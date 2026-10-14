use std::ffi::OsString;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::process::Output;

use serde_json::Value;
use tempfile::TempDir;

// Black-box reproductions of the #197 and #199 field reports. Cargo guarantees
// CARGO_BIN_EXE_sc-lint only for integration tests, so the product binary is
// located here rather than guessed from the test executable's path.

// Mirrors `EXIT_CAPABILITY` in crates/sc-lint/src/consts.rs.
const EXIT_CAPABILITY: i32 = 4;
// Mirrors `EXIT_BACKEND_FAILURE` in crates/sc-lint/src/consts.rs.
const EXIT_BACKEND_FAILURE: i32 = 5;

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
    let config_path = fixture.path().join("sc-lint.toml");
    fs::write(&config_path, config_text).expect("issue config");
    Command::new(env!("CARGO_BIN_EXE_sc-lint"))
        .args(["--json", "--config"])
        .arg(&config_path)
        .args(["lint", "ci", "--consumer"])
        .args(lint_args)
        .envs(envs.iter().map(|(key, value)| (*key, value)))
        .output()
        .expect("run sc-lint product binary")
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
