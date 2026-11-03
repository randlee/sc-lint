use std::collections::BTreeSet;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};
use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::thread;
use std::time::Duration;

use serde_json::Value;

const ANALYZERS: [&str; 3] = ["sc-lint-boundary", "sc-lint-portability", "sc-lint-runtime"];
const ANALYZER_SUPPORT: &str = "sc-lint-analyzer-support";
const CLI: &str = "sc-lint";
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

/// Runs `command` to completion or kills it at `timeout`. Output goes to
/// temporary files, so child exit is the only success-path completion signal;
/// reader-thread scheduling and inherited output handles cannot delay it. The
/// child is owned by a waiter thread that blocks in `wait()`, so there is no
/// polling loop: the deadline is a blocking receive, and a miss kills the child
/// by pid.
fn command_output_with_timeout(mut command: Command, timeout: Duration) -> io::Result<Output> {
    let description = format!("{command:?}");
    let mut stdout_file = tempfile::tempfile()?;
    let mut stderr_file = tempfile::tempfile()?;
    command
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout_file.try_clone()?))
        .stderr(Stdio::from(stderr_file.try_clone()?));
    let mut child = command.spawn()?;
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
            let stdout = read_capture(&mut stdout_file).map_err(|error| {
                io::Error::new(
                    error.kind(),
                    format!("{description} stdout capture: {error}"),
                )
            })?;
            let stderr = read_capture(&mut stderr_file).map_err(|error| {
                io::Error::new(
                    error.kind(),
                    format!("{description} stderr capture: {error}"),
                )
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
            let reap_state = if reaped {
                "killed and reaped"
            } else {
                "killed; exit not confirmed within the reap ceiling"
            };
            Err(io::Error::new(
                io::ErrorKind::TimedOut,
                format!("{description} timed out after {timeout:?}; child was {reap_state}"),
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

fn read_capture(file: &mut impl ReadSeek) -> io::Result<Vec<u8>> {
    file.seek(SeekFrom::Start(0))?;
    let mut output = Vec::new();
    file.read_to_end(&mut output)?;
    Ok(output)
}

trait ReadSeek: Read + Seek {}

impl<T: Read + Seek> ReadSeek for T {}

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
fn timeout_child_writes_output_when_requested() {
    if std::env::var_os("SC_LINT_TEST_CHILD_OUTPUT").is_some() {
        io::stdout()
            .write_all(b"controlled child stdout")
            .expect("write child stdout");
        io::stderr()
            .write_all(b"controlled child stderr")
            .expect("write child stderr");
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
fn command_output_captures_both_streams_after_child_exit() {
    let mut command = Command::new(std::env::current_exe().expect("test executable path"));
    command
        .args(["--exact", "timeout_child_writes_output_when_requested"])
        .env("SC_LINT_TEST_CHILD_OUTPUT", "1");

    let output = command_output_with_timeout(command, Duration::from_secs(30))
        .expect("child completes and its output is captured");
    assert!(output.status.success());
    assert!(
        output
            .stdout
            .windows(b"controlled child stdout".len())
            .any(|window| { window == b"controlled child stdout" }),
        "stdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(
        output
            .stderr
            .windows(b"controlled child stderr".len())
            .any(|window| { window == b"controlled child stderr" }),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn capture_reads_output_while_another_handle_remains_open() {
    let mut capture = tempfile::tempfile().expect("create output capture");
    let mut inherited_handle = capture.try_clone().expect("clone output capture");
    inherited_handle
        .write_all(b"captured")
        .expect("write child output");

    let output = read_capture(&mut capture).expect("read capture without waiting for EOF");
    assert_eq!(output, b"captured");
}

// All network deadlines below are hang guards. Readiness and release are
// explicit messages; no passing assertion depends on elapsed time.
const GRANDCHILD_HANG_GUARD: Duration = Duration::from_secs(30);

struct PendingConnection {
    address: SocketAddr,
    receiver: mpsc::Receiver<io::Result<TcpStream>>,
    worker: Option<thread::JoinHandle<()>>,
}

impl PendingConnection {
    fn listen() -> Self {
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind control listener");
        let address = listener.local_addr().expect("control address");
        let (sender, receiver) = mpsc::channel();
        let worker = thread::spawn(move || {
            let _ = sender.send(listener.accept().map(|(stream, _)| stream));
        });
        Self {
            address,
            receiver,
            worker: Some(worker),
        }
    }

    fn accept(&mut self) -> TcpStream {
        let stream = self
            .receiver
            .recv_timeout(GRANDCHILD_HANG_GUARD)
            .expect("control peer connects before the hang guard")
            .expect("accept control peer");
        self.worker
            .take()
            .expect("accept worker")
            .join()
            .expect("join accept worker");
        stream
            .set_read_timeout(Some(GRANDCHILD_HANG_GUARD))
            .expect("control read guard");
        stream
            .set_write_timeout(Some(GRANDCHILD_HANG_GUARD))
            .expect("control write guard");
        stream
    }
}

impl Drop for PendingConnection {
    fn drop(&mut self) {
        if let Some(worker) = self.worker.take() {
            // Unblock accept even if spawning/collecting the child failed.
            let _ = TcpStream::connect_timeout(&self.address, GRANDCHILD_HANG_GUARD);
            let _ = worker.join();
        }
    }
}

struct ParkedGrandchild {
    pid: Option<u32>,
    control: TcpStream,
}

impl ParkedGrandchild {
    fn receive(mut control: TcpStream) -> Self {
        let mut pid = [0; 4];
        control
            .read_exact(&mut pid)
            .expect("grandchild sends its pid before parking");
        Self {
            pid: Some(u32::from_be_bytes(pid)),
            control,
        }
    }

    fn kill_and_confirm_exit(mut self) {
        kill_process(self.pid.take().expect("live grandchild pid"));
        let mut byte = [0];
        match self.control.read(&mut byte) {
            Ok(0) => {}
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::ConnectionReset | io::ErrorKind::ConnectionAborted
                ) => {}
            other => panic!("grandchild must close control on exit: {other:?}"),
        }
    }
}

impl Drop for ParkedGrandchild {
    fn drop(&mut self) {
        if let Some(pid) = self.pid.take() {
            // Also clean up when an output/status assertion panics.
            kill_process(pid);
        }
        // EOF is a fallback release if process killing fails on the host.
        let _ = self.control.shutdown(Shutdown::Both);
    }
}

#[test]
fn timeout_grandchild_parks_when_requested() {
    let Some(address) = std::env::var_os("SC_LINT_TEST_GRANDCHILD_CONTROL") else {
        return;
    };
    let address: SocketAddr = address
        .to_str()
        .expect("control address string")
        .parse()
        .expect("control address");
    let mut control = TcpStream::connect_timeout(&address, GRANDCHILD_HANG_GUARD)
        .expect("connect test controller");
    control
        .set_read_timeout(Some(GRANDCHILD_HANG_GUARD))
        .expect("release hang guard");
    control
        .write_all(&std::process::id().to_be_bytes())
        .expect("send grandchild pid");
    io::stdout().flush().expect("flush startup stdout");
    io::stderr().flush().expect("flush startup stderr");
    let ready: SocketAddr = std::env::var("SC_LINT_TEST_GRANDCHILD_READY")
        .expect("readiness address")
        .parse()
        .expect("parse readiness address");
    let mut readiness =
        TcpStream::connect_timeout(&ready, GRANDCHILD_HANG_GUARD).expect("connect child readiness");
    readiness
        .write_all(b"R")
        .expect("signal ready with inherited streams open");
    drop(readiness);
    let mut release = [0];
    // A release byte or EOF both release the helper. Keep the stream alive
    // until process exit, so the controller observes process termination.
    match control.read(&mut release) {
        Ok(0 | 1) => std::process::exit(0),
        other => panic!("read grandchild release: {other:?}"),
    }
}

#[test]
fn timeout_child_spawns_lingering_grandchild_when_requested() {
    let Some(address) = std::env::var_os("SC_LINT_TEST_LINGERING_GRANDCHILD") else {
        return;
    };
    let mut readiness = PendingConnection::listen();
    let _grandchild = Command::new(std::env::current_exe().expect("test executable path"))
        .args([
            "--exact",
            "timeout_grandchild_parks_when_requested",
            "--nocapture",
        ])
        .env("SC_LINT_TEST_GRANDCHILD_CONTROL", address)
        .env(
            "SC_LINT_TEST_GRANDCHILD_READY",
            readiness.address.to_string(),
        )
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("spawn grandchild with inherited output handles");
    let mut peer = readiness.accept();
    let mut ready = [0];
    peer.read_exact(&mut ready)
        .expect("grandchild signals readiness");
    assert_eq!(ready, *b"R");
    io::stdout()
        .write_all(b"lingering child stdout")
        .expect("write child stdout");
    io::stdout().flush().expect("flush child stdout");
    io::stderr()
        .write_all(b"lingering child stderr")
        .expect("write child stderr");
    io::stderr().flush().expect("flush child stderr");
    std::process::exit(17);
}

#[test]
fn command_output_returns_child_status_and_streams_while_grandchild_is_parked() {
    let mut controller = PendingConnection::listen();
    let mut command = Command::new(std::env::current_exe().expect("test executable path"));
    command
        .args([
            "--exact",
            "timeout_child_spawns_lingering_grandchild_when_requested",
            "--nocapture",
        ])
        .env(
            "SC_LINT_TEST_LINGERING_GRANDCHILD",
            controller.address.to_string(),
        );
    let output = command_output_with_timeout(command, GRANDCHILD_HANG_GUARD)
        .expect("child exit returns output while grandchild holds both capture handles");
    let grandchild = ParkedGrandchild::receive(controller.accept());
    assert_eq!(output.status.code(), Some(17));
    for (stream, marker) in [
        (&output.stdout, b"lingering child stdout".as_slice()),
        (&output.stderr, b"lingering child stderr".as_slice()),
    ] {
        assert!(
            stream.windows(marker.len()).any(|window| window == marker),
            "{}",
            String::from_utf8_lossy(stream)
        );
    }
    // Nothing releases the grandchild before the status/output assertions.
    // Killing it and observing EOF confirms no process is left parked.
    grandchild.kill_and_confirm_exit();
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
