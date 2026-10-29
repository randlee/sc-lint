//! k-7 declared tool provisioning: public bootstrap and binary argv/status
//! contracts (ADR-017 change 6).
//!
//! The managed venv is a real virtual environment created by the host Python;
//! only its `sc_lint` package is a stub that records how the bootstrap invoked
//! `sc_lint.setup_tools`. Provisioning itself is covered by the Python tests.

use serde_json::{Value, json};
use std::fs;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Output, Stdio};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::thread;
use std::time::Duration;
use tempfile::TempDir;

const BOOTSTRAP: &str = include_str!("../assets/bootstrap.py");

const DECLARED_TOOLS: &str = r#"{"minimum_version":"0.7.0","profiles":{"ci":["cargo","check"]},"ci":{"os":["linux"],"enabled":false},"analyzers":{},"tools":{"cargo":{"cargo-xwin":"0.18.4"},"rustup_targets":["x86_64-pc-windows-msvc"]}}"#;
const WITHOUT_TOOLS: &str = r#"{"minimum_version":"0.7.0","profiles":{"ci":["cargo","check"]},"ci":{"os":["linux"],"enabled":false},"analyzers":{}}"#;
const EMPTY_TOOLS: &str =
    r#"{"minimum_version":"0.7.0","profiles":{},"ci":{"enabled":false},"analyzers":{},"tools":{}}"#;
const EMPTY_COLLECTIONS: &str = r#"{"minimum_version":"0.7.0","profiles":{},"ci":{"enabled":false},"analyzers":{},"tools":{"cargo":{},"rustup_targets":[]}}"#;
const MALFORMED_TOOLS: &str =
    r#"{"minimum_version":"0.7.0","profiles":{},"ci":{"enabled":false},"analyzers":{},"tools":[]}"#;

const SUBPROCESS_TIMEOUT: Duration = Duration::from_secs(120);

/// Failure ceiling for a child that never reports readiness. It is not the
/// budget for the child's work: the deadline starts only after readiness.
const READY_CEILING: Duration = Duration::from_secs(60);

const STUB_HELPER: &str = "import json, os, sys\n\nif __name__ == '__main__':\n    with open(os.environ['SC_LINT_HELPER_LOG'], 'a', encoding='utf-8') as handle:\n        handle.write(json.dumps({'argv': sys.argv[1:], 'cwd': os.getcwd(), 'cargo_home': os.environ.get('CARGO_HOME')}) + '\\n')\n    sys.exit(int(os.environ.get('SC_LINT_HELPER_STATUS', '0')))\n";

/// Run a fixture subprocess with captured output and a hard deadline.
fn run_bounded(command: &mut Command, stdin: Option<&[u8]>) -> Output {
    run_bounded_for(command, stdin, SUBPROCESS_TIMEOUT)
}

fn run_bounded_for(command: &mut Command, stdin: Option<&[u8]>, timeout: Duration) -> Output {
    run_bounded_after_readiness(command, stdin, None, timeout)
}

/// Run a fixture subprocess with captured output and a hard deadline.
///
/// When `ready_marker` is set, the deadline starts only after the child has
/// written that marker to stderr, so startup time never consumes the budget.
fn run_bounded_after_readiness(
    command: &mut Command,
    stdin: Option<&[u8]>,
    ready_marker: Option<&str>,
    timeout: Duration,
) -> Output {
    let label = format!(
        "{:?} {:?}",
        command.get_program(),
        command.get_args().collect::<Vec<_>>()
    );
    // Captures live at named paths so the readiness check can read them while
    // the child still holds its own descriptors open.
    let capture = tempfile::tempdir().expect("create subprocess capture directory");
    let stdout_path = capture.path().join("stdout");
    let stderr_path = capture.path().join("stderr");
    let stdout = fs::File::create(&stdout_path).expect("create subprocess stdout capture");
    let input = stdin.map(|contents| {
        let mut file = tempfile::tempfile().expect("create subprocess stdin");
        file.write_all(contents).expect("write subprocess stdin");
        file.seek(SeekFrom::Start(0))
            .expect("rewind subprocess stdin");
        file
    });
    command
        .stdin(input.map_or(Stdio::null(), Stdio::from))
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::piped());

    let mut child = command
        .spawn()
        .unwrap_or_else(|error| panic!("failed to start {label}: {error}"));
    // A copier thread drains stderr into the capture file and signals readiness
    // once the marker is in it, so the readiness wait is a blocking receive.
    let stderr_pipe = child.stderr.take().expect("piped stderr");
    let (ready_tx, ready_rx) = mpsc::channel();
    let (copied_tx, copied_rx) = mpsc::channel();
    let marker = ready_marker.map(str::to_owned);
    let copy_path = stderr_path.clone();
    thread::spawn(move || {
        copy_stderr(stderr_pipe, &copy_path, marker.as_deref(), &ready_tx);
        let _ = copied_tx.send(());
    });
    if let Some(marker) = ready_marker {
        match ready_rx.recv_timeout(READY_CEILING) {
            Ok(()) => {}
            Err(RecvTimeoutError::Timeout) => {
                let _ = child.kill();
                let _ = child.wait();
                panic!("subprocess did not print {marker:?} within {READY_CEILING:?}: {label}");
            }
            Err(RecvTimeoutError::Disconnected) => {
                let status = child.wait().expect("reap subprocess that closed stderr");
                let stderr = read_stderr_after_copy(&copied_rx, &stderr_path, &label);
                panic!(
                    "subprocess exited before printing {marker:?}: {label}; status: {status}; stderr: {}",
                    String::from_utf8_lossy(&stderr)
                );
            }
        }
    }

    match wait_or_kill(child, timeout)
        .unwrap_or_else(|error| panic!("failed waiting for subprocess {label}: {error}"))
    {
        Waited::Exited(status) => Output {
            status,
            stdout: read_capture(&stdout_path, "subprocess stdout"),
            stderr: read_stderr_after_copy(&copied_rx, &stderr_path, &label),
        },
        Waited::TimedOut => {
            let stderr = read_stderr_after_copy(&copied_rx, &stderr_path, &label);
            panic!(
                "subprocess exceeded {timeout:?}: {label}; stderr: {}",
                String::from_utf8_lossy(&stderr)
            );
        }
    }
}

enum Waited {
    Exited(ExitStatus),
    TimedOut,
}

/// Waits for the child on a thread that owns its handle, so the deadline is a
/// blocking receive rather than a polling loop. A missed deadline kills the
/// child by pid and then waits, under a failure ceiling, for the owner to reap it.
fn wait_or_kill(child: Child, timeout: Duration) -> io::Result<Waited> {
    let pid = child.id();
    let (status_tx, status_rx) = mpsc::channel();
    thread::spawn(move || {
        let mut child = child;
        let _ = status_tx.send(child.wait());
    });
    match status_rx.recv_timeout(timeout) {
        Ok(status) => Ok(Waited::Exited(status?)),
        Err(RecvTimeoutError::Timeout) => {
            kill_process(pid);
            let _ = status_rx.recv_timeout(READY_CEILING);
            Ok(Waited::TimedOut)
        }
        Err(RecvTimeoutError::Disconnected) => Err(io::Error::other(
            "child waiter stopped without an exit status",
        )),
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

/// Copies the child's stderr into `path` until the pipe closes, and signals
/// `ready` once `marker` appears in what has been copied so far.
fn copy_stderr(mut pipe: impl Read, path: &Path, marker: Option<&str>, ready: &mpsc::Sender<()>) {
    let Ok(mut file) = fs::File::create(path) else {
        return;
    };
    let mut seen = Vec::new();
    let mut chunk = [0u8; 4096];
    let mut announced = false;
    loop {
        let read = match pipe.read(&mut chunk) {
            Ok(0) | Err(_) => break,
            Ok(read) => read,
        };
        let _ = file.write_all(&chunk[..read]);
        seen.extend_from_slice(&chunk[..read]);
        if !announced
            && marker.is_some_and(|marker| String::from_utf8_lossy(&seen).contains(marker))
        {
            announced = true;
            let _ = ready.send(());
        }
    }
}

/// Reads the captured stderr once the copier has reached the end of the pipe.
/// A pipe held open past the child's exit fails here under a ceiling.
fn read_stderr_after_copy(copied: &mpsc::Receiver<()>, path: &Path, label: &str) -> Vec<u8> {
    copied
        .recv_timeout(READY_CEILING)
        .unwrap_or_else(|_| panic!("stderr of {label} did not close within {READY_CEILING:?}"));
    read_capture(path, "subprocess stderr")
}

#[test]
fn bounded_runner_timeout_child() {
    if std::env::var_os("SC_LINT_TIMEOUT_TEST_CHILD").is_some() {
        eprintln!("intentional timeout probe");
        // Parks until the runner kills it. Unlike a sleep, nothing here ends
        // the child on its own, so the hang outcome does not depend on timing.
        loop {
            thread::park();
        }
    }
}

#[test]
fn bounded_runner_kills_and_reports_a_hung_child() {
    let mut command = Command::new(std::env::current_exe().expect("test executable"));
    command
        .args(["--exact", "bounded_runner_timeout_child", "--nocapture"])
        .env("SC_LINT_TIMEOUT_TEST_CHILD", "1");
    let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        run_bounded_after_readiness(
            &mut command,
            None,
            Some("intentional timeout probe"),
            Duration::from_secs(2),
        );
    }))
    .expect_err("hung subprocess should hit its deadline");
    let panic = match panic.downcast::<String>() {
        Ok(message) => *message,
        Err(payload) => payload
            .downcast::<&str>()
            .map(|message| (*message).to_owned())
            .expect("timeout panic has a string payload"),
    };
    assert!(panic.contains("bounded_runner_timeout_child"), "{panic}");
    assert!(panic.contains("intentional timeout probe"), "{panic}");
}

fn read_capture(path: &Path, description: &str) -> Vec<u8> {
    fs::read(path).unwrap_or_else(|error| panic!("read {description}: {error}"))
}

fn host_python() -> PathBuf {
    let names: &[&str] = if cfg!(windows) {
        &["python.exe", "python3.exe"]
    } else {
        &["python3", "python"]
    };
    let path = std::env::var_os("PATH").expect("host PATH");
    std::env::split_paths(&path)
        .flat_map(|directory| names.iter().map(move |name| directory.join(name)))
        .find(|candidate| candidate.is_file())
        .expect("a host Python on PATH seeds the managed venv")
}

fn host_python_directory() -> PathBuf {
    host_python()
        .parent()
        .expect("host Python directory")
        .to_path_buf()
}

/// Create a real `.sc-lint/venv` whose `sc_lint` package is a recording stub.
fn stub_venv(root: &Path, with_helper: bool) {
    let venv = root.join(".sc-lint").join("venv");
    let created = run_bounded(
        Command::new(host_python())
            .args(["-m", "venv", "--without-pip"])
            .arg(&venv),
        None,
    );
    assert!(
        created.status.success(),
        "stub venv creation failed: {}",
        stderr(&created)
    );
    let python = if cfg!(windows) {
        venv.join("Scripts").join("python.exe")
    } else {
        venv.join("bin").join("python3")
    };
    let purelib = run_bounded(
        Command::new(&python).args([
            "-c",
            "import sysconfig; print(sysconfig.get_paths()['purelib'])",
        ]),
        None,
    );
    assert!(
        purelib.status.success(),
        "stub venv purelib probe failed: {}",
        stderr(&purelib)
    );
    let package = PathBuf::from(String::from_utf8_lossy(&purelib.stdout).trim()).join("sc_lint");
    fs::create_dir_all(&package).expect("stub sc_lint package");
    fs::write(package.join("__init__.py"), "__version__ = '0.7.0'\n").expect("stub sc_lint");
    if with_helper {
        fs::write(package.join("setup_tools.py"), STUB_HELPER).expect("stub helper");
    }
}

fn consumer(parent: &Path, name: &str, install_input: Option<&str>) -> PathBuf {
    let root = parent.join(name);
    fs::create_dir_all(root.join(".sc-lint")).expect("consumer bootstrap directory");
    fs::write(
        root.join("sc-lint.toml"),
        "[tool.sc-lint]\nminimum_version = \"0.7.0\"\n",
    )
    .expect("consumer config");
    fs::write(root.join(".sc-lint/bootstrap.py"), BOOTSTRAP).expect("consumer Python bootstrap");
    if let Some(input) = install_input {
        fs::write(root.join(".sc-lint/install.json"), input).expect("persisted install input");
    }
    root
}

fn helper_calls(log: &Path) -> Vec<Value> {
    fs::read_to_string(log)
        .map(|text| {
            text.lines()
                .map(|line| serde_json::from_str(line).expect("helper log record"))
                .collect()
        })
        .unwrap_or_default()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn same_path(actual: &Value, expected: &Path) -> bool {
    let actual = PathBuf::from(actual.as_str().expect("logged path"))
        .canonicalize()
        .expect("canonical logged path");
    let expected = expected.canonicalize().expect("canonical expected path");
    actual
        .to_string_lossy()
        .eq_ignore_ascii_case(&expected.to_string_lossy())
}

#[cfg(unix)]
fn write_executable(path: &Path, contents: &str) {
    // Write in a child so the test process never holds a writable descriptor
    // for a file the bootstrap is about to execute.
    let output = run_bounded(
        Command::new("sh")
            .args(["-c", "cat > \"$1\" && chmod 755 \"$1\"", "write-executable"])
            .arg(path),
        Some(contents.as_bytes()),
    );
    assert!(
        output.status.success(),
        "executable fixture writer failed: {}",
        stderr(&output)
    );
}

/// Run consumer bootstrap `setup|lint` with a stub product that always passes.
#[cfg(unix)]
fn bootstrap(fixture: &Path, root: &Path, arguments: &[&str], extra: &[(&str, &str)]) -> Output {
    let managed = fixture.join("managed");
    if !managed.join("sc-lint").exists() {
        fs::create_dir_all(&managed).expect("managed directory");
        write_executable(&managed.join("sc-lint"), "#!/bin/sh\nexit 0\n");
    }
    let path = format!("{}:/usr/bin:/bin", host_python_directory().display());
    let mut command = Command::new(host_python());
    command
        .arg(root.join(".sc-lint/bootstrap.py"))
        .args(arguments)
        .current_dir(root)
        .env_clear()
        .env("PATH", path)
        .env("HOME", fixture)
        .env("SC_LINT_INSTALL_DIR", &managed);
    for (name, value) in extra {
        command.env(name, value);
    }
    run_bounded(&mut command, None)
}

/// Run consumer bootstrap `setup|lint` with a stub product that always passes.
#[cfg(windows)]
fn bootstrap(fixture: &Path, root: &Path, arguments: &[&str], extra: &[(&str, &str)]) -> Output {
    let product = fixture.join("product").join("sc-lint.cmd");
    if !product.exists() {
        fs::create_dir_all(product.parent().expect("product directory"))
            .expect("product directory");
        fs::write(&product, "@exit /b 0\r\n").expect("stub product");
    }
    let system_root = std::env::var_os("SystemRoot").expect("Windows SystemRoot");
    let system32 = PathBuf::from(&system_root).join("System32");
    let path = std::env::join_paths([host_python_directory(), system32.clone()])
        .expect("valid Windows PATH");
    let mut command = Command::new(host_python());
    command
        .arg(root.join(".sc-lint/bootstrap.py"))
        .args(arguments)
        .current_dir(root)
        .env_clear()
        .env("SystemRoot", &system_root)
        .env("PATH", path)
        .env("ComSpec", system32.join("cmd.exe"))
        .env("USERPROFILE", fixture)
        .env("SC_LINT_BIN", &product)
        .env("SC_LINT_INSTALL_DIR", fixture.join("missing-install"));
    for name in ["TEMP", "TMP", "PATHEXT"] {
        if let Some(value) = std::env::var_os(name) {
            command.env(name, value);
        }
    }
    for (name, value) in extra {
        command.env(name, value);
    }
    run_bounded(&mut command, None)
}

#[test]
fn bootstrap_setup_hands_declared_tools_to_the_managed_helper_with_exact_config() {
    let fixture = TempDir::new().expect("fixture root");
    let cargo_home = fixture.path().join("consumer cargo home");
    let cargo_home = cargo_home.to_str().expect("cargo home path");
    for name in ["widget-service", "consumer-derived-from-atm-core"] {
        let root = consumer(fixture.path(), name, Some(DECLARED_TOOLS));
        stub_venv(&root, true);
        let log = fixture.path().join(format!("{name}-helper.log"));
        let log_path = log.to_str().expect("log path");
        let config = root.join("sc-lint.toml");
        let config = config.to_str().expect("config path");
        for (arguments, expected) in [
            (
                vec!["setup", "--config", config],
                json!(["--config", config]),
            ),
            (
                vec!["setup", "--config", "sc-lint.toml", "--dry-run"],
                json!(["--config", "sc-lint.toml", "--check"]),
            ),
            (
                vec!["setup", "--config", "sc-lint.toml", "--check"],
                json!(["--config", "sc-lint.toml", "--check"]),
            ),
        ] {
            let _ = fs::remove_file(&log);
            let output = bootstrap(
                fixture.path(),
                &root,
                &arguments,
                &[("SC_LINT_HELPER_LOG", log_path), ("CARGO_HOME", cargo_home)],
            );
            assert!(
                output.status.success(),
                "{arguments:?}: {}",
                stderr(&output)
            );
            let calls = helper_calls(&log);
            assert_eq!(calls.len(), 1, "{arguments:?}: {calls:?}");
            assert_eq!(calls[0]["argv"], expected, "{arguments:?}");
            assert!(same_path(&calls[0]["cwd"], &root), "{calls:?}");
            assert_eq!(calls[0]["cargo_home"], json!(cargo_home));
        }

        let _ = fs::remove_file(&log);
        let failed = bootstrap(
            fixture.path(),
            &root,
            &["setup", "--config", "sc-lint.toml"],
            &[
                ("SC_LINT_HELPER_LOG", log_path),
                ("SC_LINT_HELPER_STATUS", "4"),
            ],
        );
        assert_eq!(failed.status.code(), Some(4), "{}", stderr(&failed));
        assert_eq!(helper_calls(&log).len(), 1);
    }
}

#[test]
fn bootstrap_without_declared_tools_never_invokes_the_helper() {
    let fixture = TempDir::new().expect("fixture root");
    let log = fixture.path().join("helper.log");
    let log_path = log.to_str().expect("log path");
    for (name, input) in [
        ("simple-consumer", None),
        ("no-tools-consumer", Some(WITHOUT_TOOLS)),
        ("empty-tools-consumer", Some(EMPTY_TOOLS)),
        ("empty-collections-consumer", Some(EMPTY_COLLECTIONS)),
    ] {
        let root = consumer(fixture.path(), name, input);
        stub_venv(&root, true);
        for arguments in [
            vec!["setup", "--config", "sc-lint.toml"],
            vec!["setup", "--config", "sc-lint.toml", "--dry-run"],
        ] {
            let output = bootstrap(
                fixture.path(),
                &root,
                &arguments,
                &[("SC_LINT_HELPER_LOG", log_path)],
            );
            assert!(output.status.success(), "{name}: {}", stderr(&output));
            assert!(!stderr(&output).contains("SETUP_TOOLS"), "{name}");
        }
    }
    assert!(helper_calls(&log).is_empty(), "{:?}", helper_calls(&log));

    let declared = consumer(fixture.path(), "declared-consumer", Some(DECLARED_TOOLS));
    stub_venv(&declared, true);
    let lint = bootstrap(
        fixture.path(),
        &declared,
        &["lint", "--config", "sc-lint.toml"],
        &[("SC_LINT_HELPER_LOG", log_path)],
    );
    assert!(lint.status.success(), "{}", stderr(&lint));
    assert!(
        helper_calls(&log).is_empty(),
        "lint must never provision tools"
    );

    let malformed = consumer(fixture.path(), "malformed-consumer", Some(MALFORMED_TOOLS));
    stub_venv(&malformed, true);
    let output = bootstrap(
        fixture.path(),
        &malformed,
        &["setup", "--config", "sc-lint.toml"],
        &[("SC_LINT_HELPER_LOG", log_path)],
    );
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(
        helper_calls(&log).len(),
        1,
        "malformed tools are validated by the managed helper"
    );
}

#[test]
fn bootstrap_reports_an_unavailable_helper_without_bootstrapping() {
    let fixture = TempDir::new().expect("fixture root");
    let root = consumer(fixture.path(), "widget-service", Some(DECLARED_TOOLS));
    let dry_run = bootstrap(
        fixture.path(),
        &root,
        &["setup", "--config", "sc-lint.toml", "--dry-run"],
        &[],
    );
    assert_eq!(dry_run.status.code(), Some(4), "{}", stderr(&dry_run));
    let message = stderr(&dry_run);
    assert!(
        message.contains("declares tools that cannot be inspected"),
        "{message}"
    );
    assert!(
        message.contains("(SETUP_TOOLS.INSPECTION_UNAVAILABLE); run just setup"),
        "{message}"
    );
    assert!(!root.join(".sc-lint/venv").exists());

    stub_venv(&root, false);
    let setup = bootstrap(
        fixture.path(),
        &root,
        &["setup", "--config", "sc-lint.toml"],
        &[],
    );
    assert_eq!(setup.status.code(), Some(4), "{}", stderr(&setup));
    let message = stderr(&setup);
    assert!(
        message.contains("does not provide sc_lint.setup_tools"),
        "{message}"
    );
    assert!(
        message.contains("raise minimum_version, then run just setup"),
        "{message}"
    );
}

#[test]
fn direct_binary_setup_keeps_its_meaning_with_declared_tools() {
    let fixture = TempDir::new().expect("fixture root");
    let root = consumer(fixture.path(), "widget-service", Some(DECLARED_TOOLS));
    let tools = fixture.path().join("tools");
    fs::create_dir_all(&tools).expect("fake tool directory");
    let log = fixture.path().join("tools.log");
    for tool in ["cargo", "rustup"] {
        #[cfg(unix)]
        write_executable(
            &tools.join(tool),
            &format!(
                "#!/bin/sh\nprintf '{tool} %s\\n' \"$*\" >> '{}'\nexit 0\n",
                log.display()
            ),
        );
        #[cfg(windows)]
        fs::write(
            tools.join(format!("{tool}.cmd")),
            format!("@echo {tool} %* >> \"{}\"\r\n@exit /b 0\r\n", log.display()),
        )
        .expect("fake tool");
    }
    let mut path = vec![tools.clone()];
    path.extend(std::env::split_paths(
        &std::env::var_os("PATH").expect("host PATH"),
    ));
    let output = run_bounded(
        Command::new(env!("CARGO_BIN_EXE_sc-lint"))
            .args(["--config", "sc-lint.toml", "setup", "--dry-run"])
            .current_dir(&root)
            .env("PATH", std::env::join_paths(path).expect("PATH"))
            .env("SC_LINT_INSTALL_DIR", fixture.path().join("empty-install"))
            .env("CARGO_HOME", fixture.path().join("cargo-home")),
        None,
    );
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(
        !log.exists(),
        "direct binary setup must not inspect or install declared tools: {}",
        fs::read_to_string(&log).unwrap_or_default()
    );
    assert!(!fixture.path().join("cargo-home").exists());
}
