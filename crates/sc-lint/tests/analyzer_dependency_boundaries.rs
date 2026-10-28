use std::collections::BTreeSet;
use std::io::{self, Read};
use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::Value;

const ANALYZERS: [&str; 3] = ["sc-lint-boundary", "sc-lint-portability", "sc-lint-runtime"];
const ANALYZER_SUPPORT: &str = "sc-lint-analyzer-support";
const CLI: &str = "sc-lint";
const POST_KILL_PIPE_DRAIN_TIMEOUT: Duration = Duration::from_millis(250);
/// Failure ceiling for the killed child's exit status. It is not a budget the
/// passing path depends on: a child that is not reaped within it fails the test.
const POST_KILL_REAP_CEILING: Duration = Duration::from_secs(30);

#[test]
fn analyzer_manifests_remain_dependency_independent() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut command = Command::new("cargo");
    command
        .args(["metadata", "--locked", "--no-deps", "--format-version", "1"])
        .current_dir(&workspace);
    let output = command_output_with_timeout(command, Duration::from_secs(30))
        .unwrap_or_else(|error| panic!("cargo metadata failed: {error}"));
    assert!(
        output.status.success(),
        "cargo metadata failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let metadata: Value = serde_json::from_slice(&output.stdout).expect("parse cargo metadata");

    for analyzer in ANALYZERS {
        let dependencies = direct_dependencies(&metadata, analyzer);
        for peer in ANALYZERS {
            if analyzer != peer {
                assert!(
                    !dependencies.contains(peer),
                    "{analyzer} must not depend on analyzer crate {peer}"
                );
            }
        }
        assert!(
            !dependencies.contains(CLI),
            "{analyzer} must not depend on the {CLI} CLI crate"
        );
    }

    let support_dependencies = direct_dependencies(&metadata, ANALYZER_SUPPORT);
    assert!(
        !support_dependencies.contains(CLI),
        "{ANALYZER_SUPPORT} must not depend on the {CLI} CLI crate"
    );
}

/// Runs `command` to completion or kills it at `timeout`. The child is owned by
/// a waiter thread that blocks in `wait()`, so there is no polling loop: the
/// deadline is a blocking receive, and a miss kills the child by pid.
fn command_output_with_timeout(mut command: Command, timeout: Duration) -> io::Result<Output> {
    let description = format!("{command:?}");
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn()?;
    let stdout = child.stdout.take().expect("stdout is piped");
    let stderr = child.stderr.take().expect("stderr is piped");
    let (stdout_sender, stdout_reader) = mpsc::channel();
    let (stderr_sender, stderr_reader) = mpsc::channel();
    thread::spawn(move || {
        let _ = stdout_sender.send(read_pipe(stdout));
    });
    thread::spawn(move || {
        let _ = stderr_sender.send(read_pipe(stderr));
    });
    let pid = child.id();
    let (status_sender, status_reader) = mpsc::channel();
    thread::spawn(move || {
        let _ = status_sender.send(child.wait());
    });

    match status_reader.recv_timeout(timeout) {
        Ok(status) => {
            let status = status.map_err(|error| {
                io::Error::new(error.kind(), format!("{description} wait: {error}"))
            })?;
            // A grandchild that inherited the pipes can keep them open after
            // the child exits, so the drain has a deadline.
            let drain_deadline = Instant::now() + POST_KILL_PIPE_DRAIN_TIMEOUT;
            let stdout = drain_pipe_reader(stdout_reader, drain_deadline).map_err(|error| {
                io::Error::new(error.kind(), format!("{description} stdout: {error}"))
            })?;
            let stderr = drain_pipe_reader(stderr_reader, drain_deadline).map_err(|error| {
                io::Error::new(error.kind(), format!("{description} stderr: {error}"))
            })?;
            Ok(Output {
                status,
                stdout,
                stderr,
            })
        }
        Err(RecvTimeoutError::Timeout) => {
            kill_process(pid);
            let reaped = status_reader.recv_timeout(POST_KILL_REAP_CEILING).is_ok();
            let drain_deadline = Instant::now() + POST_KILL_PIPE_DRAIN_TIMEOUT;
            let drain_error = drain_pipe_reader(stdout_reader, drain_deadline)
                .err()
                .or_else(|| drain_pipe_reader(stderr_reader, drain_deadline).err());
            let drain_context = drain_error
                .map(|error| {
                    format!("; output pipes did not drain after child termination: {error}")
                })
                .unwrap_or_default();
            let reap_state = if reaped {
                "killed and reaped"
            } else {
                "killed; exit not confirmed within the reap ceiling"
            };
            Err(io::Error::new(
                io::ErrorKind::TimedOut,
                format!(
                    "{description} timed out after {timeout:?}; child was {reap_state}{drain_context}"
                ),
            ))
        }
        Err(RecvTimeoutError::Disconnected) => Err(io::Error::other(format!(
            "{description}: child waiter stopped without an exit status"
        ))),
    }
}

/// Kills a child that the waiter thread still owns. Only called on a deadline
/// miss, before the waiter has reaped it.
#[cfg(unix)]
fn kill_process(pid: u32) {
    let _ = Command::new("/bin/kill")
        .args(["-KILL", &pid.to_string()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

#[cfg(windows)]
fn kill_process(pid: u32) {
    let _ = Command::new("taskkill")
        .args(["/F", "/T", "/PID", &pid.to_string()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

fn read_pipe(mut pipe: impl Read) -> io::Result<Vec<u8>> {
    let mut output = Vec::new();
    pipe.read_to_end(&mut output)?;
    Ok(output)
}

fn drain_pipe_reader(
    reader: Receiver<io::Result<Vec<u8>>>,
    deadline: Instant,
) -> io::Result<Vec<u8>> {
    let remaining = deadline.saturating_duration_since(Instant::now());
    match reader.recv_timeout(remaining) {
        Ok(output) => output,
        Err(RecvTimeoutError::Timeout) => Err(io::Error::new(
            io::ErrorKind::TimedOut,
            format!("pipe reader exceeded {POST_KILL_PIPE_DRAIN_TIMEOUT:?} drain limit"),
        )),
        Err(RecvTimeoutError::Disconnected) => Err(io::Error::other(
            "child output reader stopped without returning output",
        )),
    }
}

#[test]
fn timeout_child_sleeps_when_requested() {
    if std::env::var_os("SC_LINT_TEST_CHILD_SLEEP").is_some() {
        // Parks until the deadline kills it, so the outcome does not depend on timing.
        loop {
            thread::park();
        }
    }
}

#[test]
fn command_output_deadline_kills_and_reaps_child() {
    let mut command = Command::new(std::env::current_exe().expect("test executable path"));
    command
        .args(["--exact", "timeout_child_sleeps_when_requested"])
        .env("SC_LINT_TEST_CHILD_SLEEP", "1");

    let error = command_output_with_timeout(command, Duration::from_millis(250))
        .expect_err("the sleeping test child exceeds the deadline");
    assert_eq!(error.kind(), io::ErrorKind::TimedOut);
    assert!(
        error
            .to_string()
            .contains("timeout_child_sleeps_when_requested")
    );
    assert!(error.to_string().contains("250ms"));
    assert!(error.to_string().contains("killed and reaped"));
}

#[test]
fn pipe_drain_times_out_while_a_reader_still_holds_the_pipe() {
    let (_held_sender, reader) = mpsc::channel::<io::Result<Vec<u8>>>();

    let error = drain_pipe_reader(reader, Instant::now())
        .expect_err("a reader that never returns exceeds the drain deadline");
    assert_eq!(error.kind(), io::ErrorKind::TimedOut);
    assert!(error.to_string().contains("drain limit"), "{error}");
}

#[test]
fn pipe_drain_returns_output_the_reader_already_sent() {
    let (sender, reader) = mpsc::channel();
    sender
        .send(Ok(b"drained".to_vec()))
        .expect("send reader output");

    let output = drain_pipe_reader(reader, Instant::now()).expect("output already sent");
    assert_eq!(output, b"drained");
}

#[test]
fn pipe_drain_reports_a_reader_that_stopped_without_output() {
    let (sender, reader) = mpsc::channel::<io::Result<Vec<u8>>>();
    drop(sender);

    let error =
        drain_pipe_reader(reader, Instant::now()).expect_err("a stopped reader returns no output");
    assert!(
        error
            .to_string()
            .contains("stopped without returning output"),
        "{error}"
    );
}

fn direct_dependencies(metadata: &Value, package_name: &str) -> BTreeSet<String> {
    metadata["packages"]
        .as_array()
        .expect("cargo metadata packages")
        .iter()
        .find(|package| package["name"].as_str() == Some(package_name))
        .unwrap_or_else(|| panic!("package {package_name} is present"))["dependencies"]
        .as_array()
        .expect("cargo metadata dependencies")
        .iter()
        .filter_map(|dependency| dependency["name"].as_str())
        .map(ToOwned::to_owned)
        .collect()
}
