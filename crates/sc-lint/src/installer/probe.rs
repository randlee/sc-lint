//! Backend and version probes for installed sc-lint binaries.

use super::*;
use std::sync::mpsc::{self, RecvTimeoutError};

const BACKEND_OUTPUT_DRAIN_TIMEOUT: Duration = Duration::from_millis(50);
const BACKEND_OUTPUT_STOP_TIMEOUT: Duration = Duration::from_millis(250);
const VERSION_PROBE_ARGUMENTS: [&str; 2] = ["--json", "version"];
const VERSION_PROBE_DESCRIPTION: &str = "--json version";

/// Why a bounded probe produced no exit status.
#[derive(Debug)]
pub(crate) enum ProbeFailure {
    /// The binary could not be started; the kind tells a missing binary apart.
    Start(io::Error),
    /// The probe started but timed out, could not be waited on, or its output
    /// could not be drained; the message names the binary and the probe.
    Probe(String),
}

impl ProbeFailure {
    fn into_message(self, binary: &Path, expected_name: &str) -> String {
        match self {
            Self::Start(error) => format!(
                "could not start backend companion `{expected_name}` at `{}`: {error}",
                binary.display()
            ),
            Self::Probe(message) => message,
        }
    }
}

/// Probe the supported CLI help contract and analyzer command surface.
pub(super) fn backend_cli_failure(
    binary: &Path,
    expected_name: &str,
    timeout: Duration,
) -> Option<String> {
    let Some(expected_about) = backend_about(expected_name) else {
        return Some(format!("unknown backend companion `{expected_name}`"));
    };
    let (status, output) = match run_backend_probe(binary, expected_name, "--help", timeout) {
        Ok(result) => result,
        Err(problem) => return Some(problem),
    };
    backend_help_contract_failure(
        &output,
        status.success(),
        binary,
        expected_name,
        expected_about,
    )
}

pub(crate) fn probe_backend_version(binary: &Path, expected_name: &str) -> Result<Version, String> {
    let (status, output) =
        run_backend_probe(binary, expected_name, "--version", BACKEND_PROBE_TIMEOUT)?;
    if !status.success() {
        return Err(format!("`--version` probe exited with {status}"));
    }
    let output = String::from_utf8(output)
        .map_err(|error| format!("`--version` probe output was not UTF-8: {error}"))?;
    let mut fields = output.split_whitespace();
    let reported_name = fields
        .next()
        .ok_or_else(|| "`--version` probe returned no identity or version".to_string())?;
    let reported_version = fields
        .next()
        .ok_or_else(|| "`--version` probe returned no version".to_string())?;
    if reported_name != expected_name {
        return Err(format!(
            "`--version` probe reported backend `{reported_name}`, expected `{expected_name}`"
        ));
    }
    if fields.next().is_some() {
        return Err("`--version` probe returned unexpected trailing output".to_string());
    }
    Version::parse(reported_version).map_err(|error| {
        format!("`--version` probe reported invalid version `{reported_version}`: {error}")
    })
}

pub(super) fn run_backend_probe(
    binary: &Path,
    expected_name: &str,
    argument: &str,
    timeout: Duration,
) -> Result<(std::process::ExitStatus, Vec<u8>), String> {
    run_backend_probe_with_arguments(binary, expected_name, &[argument], argument, timeout)
}

fn run_backend_probe_with_arguments(
    binary: &Path,
    expected_name: &str,
    arguments: &[&str],
    description: &str,
    timeout: Duration,
) -> Result<(std::process::ExitStatus, Vec<u8>), String> {
    run_probe(binary, expected_name, arguments, description, timeout)
        .map_err(|failure| failure.into_message(binary, expected_name))
}

/// Runs `<binary> --json version` bounded by `timeout`, with the same
/// process-tree cleanup and output-drain bounds as the backend probes.
pub(crate) fn run_version_command(
    binary: &Path,
    timeout: Duration,
) -> Result<(std::process::ExitStatus, Vec<u8>), ProbeFailure> {
    run_probe(
        binary,
        crate::consts::SERVICE_NAME,
        &VERSION_PROBE_ARGUMENTS,
        VERSION_PROBE_DESCRIPTION,
        timeout,
    )
}

fn run_probe(
    binary: &Path,
    expected_name: &str,
    arguments: &[&str],
    description: &str,
    timeout: Duration,
) -> Result<(std::process::ExitStatus, Vec<u8>), ProbeFailure> {
    let (mut child, reader) = spawn_probe(binary, expected_name, arguments, description)?;
    let status = wait_for_backend_probe(&mut child, binary, expected_name, description, timeout)
        .map_err(ProbeFailure::Probe)?;
    // `wait_for_backend_probe` reaps a successful leader. Its PID may then be
    // reused, so the output-drain path must not signal the process group by
    // that PID. A lingering descendant is reported as a bounded output timeout.
    let output = finish_backend_output_after_leader_exit(
        reader,
        BACKEND_OUTPUT_DRAIN_TIMEOUT,
        BACKEND_OUTPUT_STOP_TIMEOUT,
    )
    .map_err(|error| {
        ProbeFailure::Probe(error.to_probe_message(binary, expected_name, description))
    })?;
    Ok((status, output))
}

pub(super) fn spawn_probe(
    binary: &Path,
    expected_name: &str,
    arguments: &[&str],
    description: &str,
) -> Result<(Child, ProbeOutputReader), ProbeFailure> {
    let mut command = Command::new(binary);
    command
        .args(arguments)
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    #[cfg(unix)]
    command.process_group(0);
    let mut child = command.spawn().map_err(ProbeFailure::Start)?;
    let Some(stdout) = child.stdout.take() else {
        terminate_probe_processes(&mut child);
        return Err(ProbeFailure::Probe(format!(
            "backend companion `{expected_name}` at `{}` did not provide a readable `{description}` stream",
            binary.display(),
        )));
    };
    let (sender, reader) = mpsc::sync_channel(1);
    // The reader is intentionally detached: a descendant that escaped the probe
    // process group can retain stdout after a timeout, leaving this thread blocked
    // until it closes the pipe. `finish_backend_output` bounds the caller's wait,
    // but cannot reclaim that escaped descendant or reader thread here.
    thread::spawn(move || {
        let _ = sender.send(read_probe_output(stdout));
    });
    Ok((child, reader))
}

pub(super) fn wait_for_backend_probe(
    child: &mut Child,
    binary: &Path,
    expected_name: &str,
    argument: &str,
    timeout: Duration,
) -> Result<std::process::ExitStatus, String> {
    let deadline = Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Ok(status),
            Ok(None) if Instant::now() < deadline => {
                thread::sleep(BACKEND_PROBE_POLL_INTERVAL);
            }
            Ok(None) => {
                terminate_probe_processes(child);
                return Err(format!(
                    "backend companion `{expected_name}` at `{}` timed out after {} ms during its `{argument}` probe; process-tree cleanup was attempted",
                    binary.display(),
                    timeout.as_millis()
                ));
            }
            Err(error) => {
                terminate_probe_processes(child);
                return Err(format!(
                    "could not wait for backend companion `{expected_name}` at `{}`: {error}",
                    binary.display()
                ));
            }
        }
    }
}

fn finish_backend_output_after_leader_exit(
    reader: ProbeOutputReader,
    drain_timeout: Duration,
    stop_timeout: Duration,
) -> Result<Vec<u8>, BackendOutputError> {
    finish_backend_output_after_stop(
        reader,
        drain_timeout,
        stop_timeout,
        None::<fn() -> io::Result<ProbeTreeStop>>,
    )
}

pub(super) fn finish_backend_output_after_stop(
    reader: ProbeOutputReader,
    drain_timeout: Duration,
    stop_timeout: Duration,
    stop_process_tree: Option<impl FnOnce() -> io::Result<ProbeTreeStop>>,
) -> Result<Vec<u8>, BackendOutputError> {
    // A descendant can outlive the direct child while holding stdout open.
    // Give the reader a short chance to drain normal output, then stop the
    // process group instead of waiting indefinitely for EOF.
    match reader.recv_timeout(drain_timeout) {
        Ok(result) => return format_probe_output_result(result),
        Err(RecvTimeoutError::Disconnected) => {
            return Err(BackendOutputError::ReaderStopped);
        }
        Err(RecvTimeoutError::Timeout) => {}
    }
    if let Some(stop_process_tree) = stop_process_tree {
        stop_process_tree().map_err(BackendOutputError::ProcessTreeStop)?;
    }
    match reader.recv_timeout(stop_timeout) {
        Ok(result) => format_probe_output_result(result),
        Err(RecvTimeoutError::Timeout) => Err(BackendOutputError::Timeout { wait: stop_timeout }),
        Err(RecvTimeoutError::Disconnected) => Err(BackendOutputError::ReaderStopped),
    }
}

fn format_probe_output_result(result: io::Result<Vec<u8>>) -> Result<Vec<u8>, BackendOutputError> {
    result.map_err(BackendOutputError::Read)
}

#[derive(Debug)]
pub(super) enum BackendOutputError {
    Read(io::Error),
    ReaderStopped,
    ProcessTreeStop(io::Error),
    Timeout { wait: Duration },
}

impl BackendOutputError {
    fn to_probe_message(&self, binary: &Path, expected_name: &str, argument: &str) -> String {
        match self {
            Self::Read(error) => format!(
                "could not read `{argument}` output from backend companion `{expected_name}` at `{}`: {error}",
                binary.display()
            ),
            Self::ReaderStopped => format!(
                "`{argument}` output reader stopped without a result for backend companion `{expected_name}` at `{}`",
                binary.display()
            ),
            Self::ProcessTreeStop(error) => format!(
                "could not stop backend companion `{expected_name}` at `{}` while stopping its probe process tree after output drain timed out during `{argument}`: {error}",
                binary.display()
            ),
            Self::Timeout { wait } => format!(
                "backend companion `{expected_name}` at `{}` timed out waiting for `{argument}` output to close after {} ms; a descendant may still hold the pipe",
                binary.display(),
                wait.as_millis()
            ),
        }
    }
}

pub(super) fn backend_help_contract_failure(
    output: &[u8],
    successful_exit: bool,
    binary: &Path,
    expected_name: &str,
    expected_about: &str,
) -> Option<String> {
    if !successful_exit {
        return Some(format!(
            "backend companion `{expected_name}` at `{}` exited unsuccessfully",
            binary.display()
        ));
    }
    let help = String::from_utf8_lossy(output);
    let healthy = help.contains(expected_about)
        && help
            .lines()
            .any(|line| line.trim_start().starts_with("Usage:") && line.contains("<COMMAND>"))
        && help
            .lines()
            .any(|line| line.trim_start().starts_with("analyze "));
    if healthy {
        None
    } else {
        Some(format!(
            "backend companion `{expected_name}` at `{}` did not satisfy the documented `--help` contract",
            binary.display()
        ))
    }
}

pub(super) fn backend_about(name: &str) -> Option<&'static str> {
    match name {
        crate::consts::TOOL_BOUNDARY => Some("AST-sensitive Rust boundary analyzer"),
        crate::consts::TOOL_PORTABILITY => Some("AST-sensitive Rust portability analyzer"),
        crate::consts::TOOL_RUNTIME => Some("AST-sensitive Rust runtime analyzer"),
        _ => None,
    }
}

pub(super) fn read_probe_output(mut stdout: impl Read) -> io::Result<Vec<u8>> {
    let mut output = Vec::with_capacity(BACKEND_PROBE_OUTPUT_LIMIT.min(4096));
    let mut buffer = [0u8; 4096];
    loop {
        let count = stdout.read(&mut buffer)?;
        if count == 0 {
            return Ok(output);
        }
        let remaining = BACKEND_PROBE_OUTPUT_LIMIT.saturating_sub(output.len());
        output.extend_from_slice(&buffer[..count.min(remaining)]);
    }
}

pub(super) fn terminate_probe_processes(child: &mut Child) {
    let process_id = child.id();
    let _ = terminate_probe_process_tree(process_id);
    let _ = child.kill();
    let _ = child.wait();
}

/// What `terminate_probe_process_tree` found when it signalled a probe group.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ProbeTreeStop {
    /// The group existed and every member was sent SIGKILL.
    Killed,
    /// No process remained in the group, so there was nothing left to stop.
    /// Only the Unix group signal can observe this; `taskkill` reports a missing
    /// process as a failure, so the Windows path never constructs it.
    #[cfg_attr(not(unix), allow(dead_code))]
    AlreadyStopped,
}

#[cfg(unix)]
pub(super) fn terminate_probe_process_tree(process_id: u32) -> io::Result<ProbeTreeStop> {
    let group = libc::pid_t::try_from(process_id)
        .ok()
        .filter(|group| *group > 0)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("probe process id {process_id} cannot name a process group"),
            )
        })?;
    // SAFETY: `kill` takes a plain pid and signal number and reads no memory;
    // a negative pid addresses the process group whose id is `group`.
    if unsafe { libc::kill(-group, libc::SIGKILL) } == 0 {
        return Ok(ProbeTreeStop::Killed);
    }
    let error = io::Error::last_os_error();
    if error.raw_os_error() == Some(libc::ESRCH) {
        // ESRCH means the group has no members left; only other errors are failures.
        Ok(ProbeTreeStop::AlreadyStopped)
    } else {
        Err(error)
    }
}

/// `taskkill` exit code for a PID that names no running process.
#[cfg_attr(not(windows), allow(dead_code))]
const TASKKILL_NOT_FOUND: i32 = 128;

#[cfg(windows)]
pub(super) fn terminate_probe_process_tree(process_id: u32) -> io::Result<ProbeTreeStop> {
    let status = Command::new("taskkill")
        .args(["/PID", &process_id.to_string(), "/T", "/F"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?;
    classify_taskkill_exit(status.code())
}

/// Maps a `taskkill` exit code to the probe stop outcome. A process that already exited is
/// `AlreadyStopped`, matching the Unix `ESRCH` case; any other failure remains an error.
/// Kept free of Windows APIs so the classification is tested on every platform.
#[cfg_attr(not(windows), allow(dead_code))]
fn classify_taskkill_exit(code: Option<i32>) -> io::Result<ProbeTreeStop> {
    match code {
        Some(0) => Ok(ProbeTreeStop::Killed),
        Some(TASKKILL_NOT_FOUND) => Ok(ProbeTreeStop::AlreadyStopped),
        Some(code) => Err(io::Error::other(format!(
            "taskkill exited with exit code: {code}"
        ))),
        None => Err(io::Error::other(
            "taskkill was terminated before it reported an exit code",
        )),
    }
}

#[cfg(not(any(unix, windows)))]
pub(super) fn terminate_probe_process_tree(_process_id: u32) -> io::Result<ProbeTreeStop> {
    Err(io::Error::other(
        "process-tree termination is not supported on this platform",
    ))
}

pub(crate) fn probe_version(binary: &Path) -> Result<Version, String> {
    probe_version_with_timeout(binary, BACKEND_PROBE_TIMEOUT)
}

fn probe_version_with_timeout(binary: &Path, timeout: Duration) -> Result<Version, String> {
    if !binary.is_file() {
        return Err("binary was not found".to_string());
    }
    let (status, output) = run_version_command(binary, timeout)
        .map_err(|failure| failure.into_message(binary, crate::consts::SERVICE_NAME))?;
    if !status.success() {
        return Err(format!("version probe exited with {status}"));
    }
    let envelope: Value = serde_json::from_slice(&output).map_err(|error| error.to_string())?;
    let data = envelope
        .get("data")
        .ok_or_else(|| "version probe did not include data".to_string())?;
    if envelope.get("ok").and_then(Value::as_bool) != Some(true)
        || envelope.get("command").and_then(Value::as_str) != Some("version")
        || data.get("tool").and_then(Value::as_str) != Some(crate::consts::SERVICE_NAME)
        || data.get("contract_schema").and_then(Value::as_str) != Some(VERSION_PROBE_SCHEMA)
    {
        return Err("version probe did not implement sc-lint-version-v1".to_string());
    }
    let version = data
        .get("version")
        .and_then(Value::as_str)
        .ok_or_else(|| "version probe did not include a version".to_string())?;
    Version::parse(version).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(unix)]
    use std::os::unix::process::ExitStatusExt;

    #[cfg(unix)]
    #[test]
    fn reaped_probe_leader_does_not_signal_its_reusable_group_id() {
        let mut child = Command::new("sh")
            .args(["-c", "exit 0"])
            .process_group(0)
            .spawn()
            .expect("spawn probe group leader");
        child.wait().expect("reap probe group leader");

        // Do not signal a process group by an identifier after its leader was
        // reaped: the PID can be reused before the output pipe finishes draining.
        let (_sender, reader) = mpsc::sync_channel(1);
        assert!(matches!(
            finish_backend_output_after_leader_exit(
                reader,
                Duration::ZERO,
                Duration::ZERO,
            ),
            Err(BackendOutputError::Timeout { wait }) if wait.is_zero()
        ));
    }

    #[cfg(unix)]
    #[test]
    fn live_probe_group_is_killed_and_reported_as_stopped() {
        let mut child = Command::new("cat")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .process_group(0)
            .spawn()
            .expect("spawn live probe group leader");
        let _stdin = child.stdin.take().expect("keep cat waiting for input");
        let process_id = child.id();

        assert_eq!(
            terminate_probe_process_tree(process_id).expect("live group can be killed"),
            ProbeTreeStop::Killed,
        );
        let status = child.wait().expect("reap killed probe group leader");
        assert_eq!(status.signal(), Some(libc::SIGKILL));
    }

    #[cfg(unix)]
    #[test]
    fn unrepresentable_probe_process_id_remains_an_error() {
        let error = terminate_probe_process_tree(u32::MAX)
            .expect_err("a process id that cannot name a group must stay an error");
        assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
    }

    #[test]
    fn taskkill_success_counts_as_killed() {
        assert_eq!(
            classify_taskkill_exit(Some(0)).expect("success is not an error"),
            ProbeTreeStop::Killed
        );
    }

    #[test]
    fn taskkill_for_an_exited_process_counts_as_already_stopped() {
        assert_eq!(
            classify_taskkill_exit(Some(TASKKILL_NOT_FOUND))
                .expect("an exited process is not an error"),
            ProbeTreeStop::AlreadyStopped
        );
    }

    #[test]
    fn other_taskkill_failures_remain_errors() {
        for code in [Some(1), Some(5), Some(129), None] {
            assert!(
                classify_taskkill_exit(code).is_err(),
                "exit {code:?} must stay an error"
            );
        }
    }

    #[test]
    fn open_probe_output_writer_returns_typed_timeout_after_stop() {
        let (_sender, reader) = mpsc::sync_channel(1);
        let mut stop_attempted = false;

        let error = finish_backend_output_after_stop(
            reader,
            Duration::ZERO,
            Duration::ZERO,
            Some(|| {
                stop_attempted = true;
                Ok(ProbeTreeStop::AlreadyStopped)
            }),
        )
        .expect_err("a connected reader that never sends must time out");

        assert!(
            stop_attempted,
            "the process tree must be stopped after drain timeout"
        );
        assert!(
            matches!(
                error,
                BackendOutputError::Timeout { wait } if wait.is_zero()
            ),
            "expected typed timeout, got {error:?}"
        );
    }

    #[test]
    fn process_tree_stop_error_names_the_output_drain_timeout() {
        let error = BackendOutputError::ProcessTreeStop(io::Error::other("stop failed"));

        let message = error.to_probe_message(Path::new("backend-bin"), "backend", "--help");

        assert!(
            message.contains("while stopping its probe process tree after output drain timed out")
        );
        assert!(!message.contains("process exited"));
    }

    #[test]
    fn probe_version_hung_candidate_returns_a_bounded_timeout() {
        let error = run_probe(
            Path::new("/bin/sh"),
            crate::consts::SERVICE_NAME,
            &["-c", "exec tail -f /dev/null", "--"],
            VERSION_PROBE_DESCRIPTION,
            Duration::from_millis(100),
        )
        .expect_err("hung version candidate must be bounded")
        .into_message(Path::new("candidate"), crate::consts::SERVICE_NAME);

        assert!(
            error.contains(
                "timed out after 100 ms during its `--json version` probe; process-tree cleanup was attempted"
            ),
            "expected timeout diagnostic, got {error}"
        );
    }
}
