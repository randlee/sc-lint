//! k-7 declared tool provisioning: public bootstrap and binary argv/status
//! contracts (ADR-017 change 6).
//!
//! The managed venv is a real virtual environment created by the host Python;
//! only its `sc_lint` package is a stub that records how the bootstrap invoked
//! `sc_lint.setup_tools`. Provisioning itself is covered by the Python tests.

use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use tempfile::TempDir;

#[cfg(unix)]
const BOOTSTRAP: &str = include_str!("../assets/bootstrap");
#[cfg(windows)]
const BOOTSTRAP_POWERSHELL: &str = include_str!("../assets/bootstrap.ps1");

const DECLARED_TOOLS: &str = r#"{"minimum_version":"0.7.0","profiles":{"ci":["cargo","check"]},"ci":{"os":["linux"],"enabled":false},"analyzers":{},"tools":{"cargo":{"cargo-xwin":"0.18.4"},"rustup_targets":["x86_64-pc-windows-msvc"]}}"#;
const WITHOUT_TOOLS: &str = r#"{"minimum_version":"0.7.0","profiles":{"ci":["cargo","check"]},"ci":{"os":["linux"],"enabled":false},"analyzers":{}}"#;
const EMPTY_TOOLS: &str =
    r#"{"minimum_version":"0.7.0","profiles":{},"ci":{"enabled":false},"analyzers":{},"tools":{}}"#;
const EMPTY_COLLECTIONS: &str = r#"{"minimum_version":"0.7.0","profiles":{},"ci":{"enabled":false},"analyzers":{},"tools":{"cargo":{},"rustup_targets":[]}}"#;
const MALFORMED_TOOLS: &str =
    r#"{"minimum_version":"0.7.0","profiles":{},"ci":{"enabled":false},"analyzers":{},"tools":[]}"#;

const STUB_HELPER: &str = "import json, os, sys\n\nif __name__ == '__main__':\n    with open(os.environ['SC_LINT_HELPER_LOG'], 'a', encoding='utf-8') as handle:\n        handle.write(json.dumps({'argv': sys.argv[1:], 'cwd': os.getcwd(), 'cargo_home': os.environ.get('CARGO_HOME')}) + '\\n')\n    sys.exit(int(os.environ.get('SC_LINT_HELPER_STATUS', '0')))\n";

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
    let created = Command::new(host_python())
        .args(["-m", "venv", "--without-pip"])
        .arg(&venv)
        .status()
        .expect("create stub venv");
    assert!(created.success(), "stub venv creation failed");
    let python = if cfg!(windows) {
        venv.join("Scripts").join("python.exe")
    } else {
        venv.join("bin").join("python3")
    };
    let purelib = Command::new(&python)
        .args([
            "-c",
            "import sysconfig; print(sysconfig.get_paths()['purelib'])",
        ])
        .output()
        .expect("read stub venv purelib");
    assert!(purelib.status.success(), "stub venv purelib probe failed");
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
    #[cfg(unix)]
    write_executable(&root.join(".sc-lint/bootstrap"), BOOTSTRAP);
    #[cfg(windows)]
    fs::write(root.join(".sc-lint/bootstrap.ps1"), BOOTSTRAP_POWERSHELL)
        .expect("consumer PowerShell bootstrap");
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
    use std::io::Write;
    use std::process::Stdio;

    // Write in a child so the test process never holds a writable descriptor
    // for a file the bootstrap is about to execute.
    let mut child = Command::new("sh")
        .args(["-c", "cat > \"$1\" && chmod 755 \"$1\"", "write-executable"])
        .arg(path)
        .stdin(Stdio::piped())
        .spawn()
        .expect("executable fixture writer starts");
    let mut stdin = child.stdin.take().expect("executable fixture writer stdin");
    stdin
        .write_all(contents.as_bytes())
        .expect("write executable fixture contents");
    drop(stdin);
    assert!(
        child
            .wait()
            .expect("executable fixture writer waits")
            .success(),
        "executable fixture writer succeeds"
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
    let mut command = Command::new("/bin/sh");
    command
        .arg(root.join(".sc-lint/bootstrap"))
        .args(arguments)
        .current_dir(root)
        .env_clear()
        .env("PATH", path)
        .env("HOME", fixture)
        .env("SC_LINT_INSTALL_DIR", &managed);
    for (name, value) in extra {
        command.env(name, value);
    }
    command.output().expect("run consumer bootstrap")
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
    let inherited_path = std::env::var_os("PATH").expect("Windows PATH");
    let pwsh = std::env::split_paths(&inherited_path)
        .map(|directory| directory.join("pwsh.exe"))
        .find(|candidate| candidate.is_file())
        .expect("pwsh.exe on Windows PATH");
    let path = std::env::join_paths([
        pwsh.parent().expect("pwsh parent").to_path_buf(),
        host_python_directory(),
        system32.clone(),
    ])
    .expect("valid Windows PATH");
    let mut command = Command::new(&pwsh);
    command
        .args(["-NoLogo", "-NoProfile", "-NonInteractive", "-File"])
        .arg(root.join(".sc-lint/bootstrap.ps1"))
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
    command.output().expect("run PowerShell consumer bootstrap")
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
    let output = Command::new(env!("CARGO_BIN_EXE_sc-lint"))
        .args(["--config", "sc-lint.toml", "setup", "--dry-run"])
        .current_dir(&root)
        .env("PATH", std::env::join_paths(path).expect("PATH"))
        .env("SC_LINT_INSTALL_DIR", fixture.path().join("empty-install"))
        .env("CARGO_HOME", fixture.path().join("cargo-home"))
        .output()
        .expect("run product setup");
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(
        !log.exists(),
        "direct binary setup must not inspect or install declared tools: {}",
        fs::read_to_string(&log).unwrap_or_default()
    );
    assert!(!fixture.path().join("cargo-home").exists());
}
