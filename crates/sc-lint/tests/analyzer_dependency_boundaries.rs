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
    let started = Instant::now();

    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                return Ok(Output {
                    status,
                    stdout: receive_pipe_reader(stdout_reader)?,
                    stderr: receive_pipe_reader(stderr_reader)?,
                });
            }
            Ok(None) if started.elapsed() >= timeout => {
                let _ = child.kill();
                child.wait()?;
                let drain_deadline = Instant::now() + POST_KILL_PIPE_DRAIN_TIMEOUT;
                let drain_error = drain_pipe_reader(stdout_reader, drain_deadline)
                    .err()
                    .or_else(|| drain_pipe_reader(stderr_reader, drain_deadline).err());
                let drain_context = drain_error
                    .map(|error| {
                        format!("; output pipes did not drain after child termination: {error}")
                    })
                    .unwrap_or_default();
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    format!(
                        "{description} timed out after {timeout:?}; child was killed and reaped{drain_context}"
                    ),
                ));
            }
            Ok(None) => thread::sleep(Duration::from_millis(10)),
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                let drain_deadline = Instant::now() + POST_KILL_PIPE_DRAIN_TIMEOUT;
                let drain_error = drain_pipe_reader(stdout_reader, drain_deadline)
                    .err()
                    .or_else(|| drain_pipe_reader(stderr_reader, drain_deadline).err());
                return Err(match drain_error {
                    Some(drain_error) => io::Error::new(
                        error.kind(),
                        format!(
                            "{error}; output pipes did not drain after child termination: {drain_error}"
                        ),
                    ),
                    None => error,
                });
            }
        }
    }
}

fn read_pipe(mut pipe: impl Read) -> io::Result<Vec<u8>> {
    let mut output = Vec::new();
    pipe.read_to_end(&mut output)?;
    Ok(output)
}

fn receive_pipe_reader(reader: Receiver<io::Result<Vec<u8>>>) -> io::Result<Vec<u8>> {
    reader
        .recv()
        .map_err(|_| io::Error::other("child output reader stopped without returning output"))?
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
        thread::sleep(Duration::from_secs(10));
    }
}

#[test]
fn timeout_grandchild_keeps_inherited_pipes_open() {
    if std::env::var_os("SC_LINT_TEST_GRANDCHILD_SLEEP").is_some() {
        thread::sleep(Duration::from_secs(5));
    }
}

#[test]
fn timeout_parent_spawns_pipe_inheriting_grandchild() {
    if std::env::var_os("SC_LINT_TEST_SPAWN_GRANDCHILD").is_some() {
        let mut grandchild = Command::new(std::env::current_exe().expect("test executable path"))
            .args(["--exact", "timeout_grandchild_keeps_inherited_pipes_open"])
            .env("SC_LINT_TEST_GRANDCHILD_SLEEP", "1")
            .spawn()
            .expect("spawn pipe-inheriting grandchild");
        println!("grandchild spawned");
        let _ = grandchild.wait();
        thread::sleep(Duration::from_secs(10));
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
fn command_output_timeout_does_not_wait_for_pipe_inheriting_grandchild() {
    let mut command = Command::new(std::env::current_exe().expect("test executable path"));
    command
        .args([
            "--exact",
            "timeout_parent_spawns_pipe_inheriting_grandchild",
        ])
        .env("SC_LINT_TEST_SPAWN_GRANDCHILD", "1");

    let started = Instant::now();
    let error = command_output_with_timeout(command, Duration::from_secs(1))
        .expect_err("the sleeping test child exceeds the deadline");
    assert_eq!(error.kind(), io::ErrorKind::TimedOut);
    assert!(error.to_string().contains("output pipes did not drain"));
    assert!(started.elapsed() < Duration::from_secs(4));
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
