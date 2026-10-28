use std::ffi::OsString;
use std::fs;
use std::io;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Read;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::process::Output;
use std::process::Stdio;
use std::sync::mpsc::RecvTimeoutError;
use std::thread;
use std::time::Duration;
use std::time::Instant;

use serde_json::Value;
use tempfile::TempDir;

// Black-box reproductions of the #197 and #199 field reports. Cargo guarantees
// CARGO_BIN_EXE_sc-lint only for integration tests, so the product binary is
// located here rather than guessed from the test executable's path.

// Mirrors `EXIT_CAPABILITY` in crates/sc-lint/src/consts.rs.
const EXIT_CAPABILITY: i32 = 4;
// Mirrors `EXIT_BACKEND_FAILURE` in crates/sc-lint/src/consts.rs.
const EXIT_BACKEND_FAILURE: i32 = 5;
// Failure ceilings only: a healthy child prints its readiness line before it
// waits, so these never bound a passing run. They turn a hang into a named error.
const READY_LINE_DEADLINE: Duration = Duration::from_secs(60);
const OUTPUT_DRAIN_DEADLINE: Duration = Duration::from_secs(10);

fn profile_fixture(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/platform-profile-execution")
        .join(relative)
}

fn step_names(steps: &[Value]) -> Vec<String> {
    steps
        .iter()
        .map(|step| {
            step.get("name")
                .and_then(Value::as_str)
                .expect("step name")
                .to_string()
        })
        .collect()
}

fn issue_failure_context(output: &Output) -> String {
    format!(
        "exit={:?}\nstdout={}\nstderr={}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn native_cargo_shim() -> TempDir {
    let directory = TempDir::new().expect("native cargo shim directory");
    let source = directory.path().join("cargo_shim.rs");
    let executable = directory
        .path()
        .join(if cfg!(windows) { "cargo.exe" } else { "cargo" });
    fs::write(
        &source,
        r#"
use std::io::Write;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut log = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(std::env::var_os("SC_LINT_SHIM_LOG").expect("shim log"))
        .expect("open shim log");
    writeln!(log, "{}", args.join(" ")).expect("write shim log");
    let first = args.first().map(String::as_str);
    let second = args.get(1).map(String::as_str);
    if first == Some("xwin") && second == Some("--version") {
        let no_xwin = std::env::var("SC_LINT_SHIM_NO_XWIN").is_ok_and(|value| value == "1");
        std::process::exit(i32::from(no_xwin));
    }
    let verb = if first == Some("xwin") { second } else { first }.unwrap_or_default();
    if let Ok(fail_verb) = std::env::var("SC_LINT_SHIM_FAIL_VERB")
        && !fail_verb.is_empty()
        && verb == fail_verb
    {
        eprintln!("shim: {verb} failed as requested");
        std::process::exit(1);
    }
}
"#,
    )
    .expect("write native cargo shim source");
    let compilation = Command::new("rustc")
        .args(["--edition=2024"])
        .arg(&source)
        .arg("-o")
        .arg(&executable)
        .output()
        .expect("compile native cargo shim");
    assert!(
        compilation.status.success(),
        "native cargo shim compilation failed: {}",
        String::from_utf8_lossy(&compilation.stderr)
    );
    directory
}

fn run_consumer_lint(config_text: &str, lint_args: &[&str], envs: &[(&str, OsString)]) -> Output {
    let fixture = TempDir::new().expect("issue fixture");
    fs::write(
        fixture.path().join("Cargo.toml"),
        "[package]\nname = \"neutral\"\nversion = \"0.0.0\"\nedition = \"2021\"\n",
    )
    .expect("neutral manifest");
    fs::create_dir_all(fixture.path().join("src")).expect("neutral src");
    fs::write(fixture.path().join("src/lib.rs"), "pub fn neutral() {}\n").expect("neutral lib");
    let config_path = fixture.path().join("sc-lint.toml");
    fs::write(&config_path, config_text).expect("issue config");
    let mut command = Command::new(env!("CARGO_BIN_EXE_sc-lint"));
    command
        .current_dir(fixture.path())
        .args(["--json", "--config"])
        .arg(&config_path)
        .args(["lint", "ci", "--consumer"])
        .args(lint_args)
        .envs(envs.iter().map(|(key, value)| (*key, value)))
        .env("CARGO_TARGET_DIR", fixture.path().join("cargo-target"));
    run_command_with_timeout(&mut command, Duration::from_secs(300))
        .unwrap_or_else(|error| panic!("run sc-lint product binary: {error}"))
}

fn run_command_with_timeout(command: &mut Command, timeout: Duration) -> io::Result<Output> {
    let display = format!("{command:?}");
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let stdout = child.stdout.take().expect("piped stdout");
    let stderr = child.stderr.take().expect("piped stderr");
    let (stdout_tx, stdout_rx) = std::sync::mpsc::channel();
    let (stderr_tx, stderr_rx) = std::sync::mpsc::channel();
    thread::spawn(move || {
        let _ = stdout_tx.send(read_child_output(stdout));
    });
    thread::spawn(move || {
        let _ = stderr_tx.send(read_child_output(stderr));
    });
    let started = Instant::now();

    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                return Ok(Output {
                    status,
                    stdout: receive_child_output(&stdout_rx, &display, "stdout")?,
                    stderr: receive_child_output(&stderr_rx, &display, "stderr")?,
                });
            }
            Ok(None) => {}
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                let stdout =
                    receive_child_output(&stdout_rx, &display, "stdout").unwrap_or_default();
                let stderr =
                    receive_child_output(&stderr_rx, &display, "stderr").unwrap_or_default();
                return Err(io::Error::new(
                    error.kind(),
                    format!(
                        "waiting for command {display} failed: {error}; stdout: {}; stderr: {}",
                        String::from_utf8_lossy(&stdout),
                        String::from_utf8_lossy(&stderr)
                    ),
                ));
            }
        }

        if started.elapsed() >= timeout {
            let _ = child.kill();
            let _status = child.wait()?;
            let stdout = receive_child_output(&stdout_rx, &display, "stdout")?;
            let stderr = receive_child_output(&stderr_rx, &display, "stderr")?;
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                format!(
                    "command {display} timed out after {timeout:?}; stdout: {}; stderr: {}",
                    String::from_utf8_lossy(&stdout),
                    String::from_utf8_lossy(&stderr)
                ),
            ));
        }

        thread::sleep(Duration::from_millis(10));
    }
}

fn read_child_output(mut stream: impl Read) -> io::Result<Vec<u8>> {
    let mut output = Vec::new();
    stream.read_to_end(&mut output)?;
    Ok(output)
}

fn read_child_output_after_ready(
    stream: impl Read + Send + 'static,
    ready: std::sync::mpsc::Sender<io::Result<()>>,
) -> io::Result<Vec<u8>> {
    let mut reader = BufReader::new(stream);
    let mut output = Vec::new();
    while !output.ends_with(b"timeout child ready\n") {
        let before = output.len();
        reader.read_until(b'\n', &mut output)?;
        if output.len() == before {
            break;
        }
    }
    ready
        .send(if output.ends_with(b"timeout child ready\n") {
            Ok(())
        } else {
            Err(io::Error::other("timeout child did not signal readiness"))
        })
        .map_err(|_| io::Error::other("readiness receiver dropped"))?;
    reader.read_to_end(&mut output)?;
    Ok(output)
}

#[test]
fn timeout_output_reader_signals_explicit_readiness() {
    let (ready_tx, ready_rx) = std::sync::mpsc::channel();
    let output = read_child_output_after_ready(
        std::io::Cursor::new(
            b"test harness preamble\ntimeout child stdout\ntimeout child ready\n".to_vec(),
        ),
        ready_tx,
    )
    .expect("the controlled output contains the readiness line");

    ready_rx
        .recv_timeout(READY_LINE_DEADLINE)
        .expect("the reader reports readiness within its deadline")
        .expect("the readiness line is accepted");
    assert_eq!(
        output,
        b"test harness preamble\ntimeout child stdout\ntimeout child ready\n"
    );
}

fn run_command_with_timeout_after_ready(
    command: &mut Command,
    timeout: Duration,
) -> io::Result<Output> {
    let display = format!("{command:?}");
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let stdout = child.stdout.take().expect("piped stdout");
    let stderr = child.stderr.take().expect("piped stderr");
    let (ready_tx, ready_rx) = std::sync::mpsc::channel();
    let (stdout_tx, stdout_rx) = std::sync::mpsc::channel();
    let (stderr_tx, stderr_rx) = std::sync::mpsc::channel();
    thread::spawn(move || {
        let _ = stdout_tx.send(read_child_output_after_ready(stdout, ready_tx));
    });
    thread::spawn(move || {
        let _ = stderr_tx.send(read_child_output(stderr));
    });
    match ready_rx.recv_timeout(READY_LINE_DEADLINE) {
        Ok(ready) => ready?,
        Err(RecvTimeoutError::Timeout) => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                format!(
                    "command {display} did not print its readiness line within {READY_LINE_DEADLINE:?}"
                ),
            ));
        }
        Err(RecvTimeoutError::Disconnected) => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(io::Error::other(format!(
                "command {display} readiness channel closed before a result"
            )));
        }
    }
    let started = Instant::now();
    loop {
        if let Some(status) = child.try_wait()? {
            return Ok(Output {
                status,
                stdout: receive_child_output(&stdout_rx, &display, "stdout")?,
                stderr: receive_child_output(&stderr_rx, &display, "stderr")?,
            });
        }
        if started.elapsed() >= timeout {
            let _ = child.kill();
            let _ = child.wait()?;
            let stdout = receive_child_output(&stdout_rx, &display, "stdout")?;
            let stderr = receive_child_output(&stderr_rx, &display, "stderr")?;
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                format!(
                    "command {display} timed out after {timeout:?}; stdout: {}; stderr: {}",
                    String::from_utf8_lossy(&stdout),
                    String::from_utf8_lossy(&stderr)
                ),
            ));
        }
        thread::sleep(Duration::from_millis(10));
    }
}

/// Receives one output pipe's result, failing with a named error instead of
/// waiting forever when a grandchild keeps the pipe open.
fn receive_child_output(
    reader: &std::sync::mpsc::Receiver<io::Result<Vec<u8>>>,
    display: &str,
    stream: &str,
) -> io::Result<Vec<u8>> {
    reader
        .recv_timeout(OUTPUT_DRAIN_DEADLINE)
        .map_err(|error| {
            io::Error::new(
                io::ErrorKind::TimedOut,
                format!("command {display} {stream} did not drain within {OUTPUT_DRAIN_DEADLINE:?}: {error}"),
            )
        })?
}

#[test]
fn issue_timeout_child_process() {
    if std::env::var_os("SC_LINT_TEST_TIMEOUT_CHILD").is_some() {
        println!("timeout child stdout");
        io::stdout().flush().expect("flush timeout child stdout");
        eprintln!("timeout child stderr");
        io::stderr().flush().expect("flush timeout child stderr");
        println!("timeout child ready");
        io::stdout().flush().expect("flush timeout child readiness");
        loop {
            thread::park();
        }
    }
}

#[test]
fn issue_command_timeout_kills_child_and_reports_output() {
    let mut command = Command::new(std::env::current_exe().expect("test executable"));
    command
        .args(["--exact", "issue_timeout_child_process", "--nocapture"])
        .env("SC_LINT_TEST_TIMEOUT_CHILD", "1");
    let error = run_command_with_timeout_after_ready(&mut command, Duration::from_millis(100))
        .expect_err("a sleeping child exceeds the deadline");
    let message = error.to_string();
    assert_eq!(error.kind(), io::ErrorKind::TimedOut, "{message}");
    assert!(message.contains("issue_timeout_child_process"), "{message}");
    assert!(message.contains("timeout child stdout"), "{message}");
    assert!(message.contains("timeout child stderr"), "{message}");
}

#[test]
fn test_supported_execution_failure_on_real_host_is_nonzero() {
    let (platform, triple) = match std::env::consts::OS {
        "linux" => ("linux", "invented-unknown-linux-gnu"),
        "macos" => ("macos", "invented-apple-darwin"),
        "windows" => ("windows", "invented-pc-windows-msvc"),
        other => panic!("unsupported test host: {other}"),
    };
    let config = format!(
        "[tool.sc-lint]\nminimum_version = \"0.4.0\"\n\n[[tool.sc-lint.lint]]\nname = \"invented\"\ncommand = [\"cargo\", \"check\"]\nplatform = \"{platform}\"\ntarget = \"{triple}\"\n\n[[tool.sc-lint.test]]\nname = \"test\"\ncommand = [\"cargo\", \"test\"]\n"
    );
    let output = run_consumer_lint(&config, &[], &[]);
    assert_eq!(
        output.status.code(),
        Some(EXIT_BACKEND_FAILURE),
        "a real failing cargo invocation is bounded and non-zero\n{}",
        issue_failure_context(&output)
    );
}

#[test]
fn test_issue_199_named_membership_reproduces_then_passes() {
    let config = fs::read_to_string(profile_fixture("named-membership.toml")).expect("fixture");
    let output = run_consumer_lint(&config, &["--profile", "fast"], &[]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "`--profile fast` must select the named membership\n{}",
        issue_failure_context(&output)
    );
    let envelope: Value = serde_json::from_slice(&output.stdout).expect("success envelope");
    let steps = envelope["data"]["steps"].as_array().expect("steps array");
    assert_eq!(
        step_names(steps),
        ["fmt", "version", "manifests", "spell", "pytests"],
        "membership order is the profile order, not declaration order"
    );
    assert!(
        steps.iter().all(|step| step["status"] == "pass"),
        "{steps:?}"
    );
    assert_eq!(envelope["data"]["selector"], "fast");
}

#[test]
fn test_issue_197_cross_target_execution_reproduces_then_passes() {
    let shim = native_cargo_shim();
    let cargo = "cargo";
    let config = format!(
        "[tool.sc-lint]\nminimum_version = \"0.4.0\"\n\n\
        [[tool.sc-lint.lint]]\nname = \"win-clippy\"\ncommand = [{cargo:?}, \"clippy\", \"--workspace\", \"--\", \"-D\", \"warnings\"]\nplatform = \"windows\"\ntarget = \"x86_64-pc-windows-msvc\"\n\n\
        [[tool.sc-lint.test]]\nname = \"test\"\ncommand = [{cargo:?}, \"test\"]\n"
    );
    let path = std::env::join_paths(std::iter::once(shim.path().to_path_buf()).chain(
        std::env::split_paths(&std::env::var_os("PATH").expect("PATH is set")),
    ))
    .expect("shimmed PATH");

    let log_dir = TempDir::new().expect("shim log dir");
    let log = log_dir.path().join("argv.log");
    let envs = [
        ("PATH", path.clone()),
        ("SC_LINT_SHIM_LOG", log.clone().into_os_string()),
    ];
    let output = run_consumer_lint(&config, &["--platform-scope", "windows"], &envs);
    assert_eq!(
        output.status.code(),
        Some(0),
        "cross-target step must execute\n{}",
        issue_failure_context(&output)
    );
    let argv = fs::read_to_string(&log).expect("shim recorded argv");
    let expected = if cfg!(windows) {
        "clippy --workspace --target x86_64-pc-windows-msvc -- -D warnings"
    } else {
        "xwin clippy --workspace --target x86_64-pc-windows-msvc -- -D warnings"
    };
    assert!(
        argv.lines().any(|line| line == expected),
        "expected `{expected}` in recorded argv:\n{argv}"
    );

    let failed_log = log_dir.path().join("failed-clippy.log");
    let envs = [
        ("PATH", path.clone()),
        ("SC_LINT_SHIM_LOG", failed_log.clone().into_os_string()),
        ("SC_LINT_SHIM_FAIL_VERB", OsString::from("clippy")),
    ];
    let output = run_consumer_lint(&config, &["--platform-scope", "windows"], &envs);
    assert_eq!(
        output.status.code(),
        Some(EXIT_BACKEND_FAILURE),
        "a failed native clippy invocation must be reported\n{}",
        issue_failure_context(&output)
    );
    let recorded = fs::read_to_string(&failed_log).expect("failed shim recorded argv");
    assert!(
        recorded.lines().any(|line| line == expected),
        "expected `{expected}` in recorded argv:\n{recorded}"
    );

    // Planted negative: without cargo-xwin the run is a capability failure,
    // and the clippy step never ran (it was not silently skipped).
    if !cfg!(windows) {
        let missing_log = log_dir.path().join("missing-xwin.log");
        let envs = [
            ("PATH", path),
            ("SC_LINT_SHIM_LOG", missing_log.clone().into_os_string()),
            ("SC_LINT_SHIM_NO_XWIN", OsString::from("1")),
        ];
        let output = run_consumer_lint(&config, &["--platform-scope", "windows"], &envs);
        assert_eq!(
            output.status.code(),
            Some(EXIT_CAPABILITY),
            "{}",
            issue_failure_context(&output)
        );
        let recorded = fs::read_to_string(&missing_log).expect("probe argv");
        assert!(!recorded.contains("clippy"), "{recorded}");
    }
}
