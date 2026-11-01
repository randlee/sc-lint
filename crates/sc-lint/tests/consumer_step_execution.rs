use std::path::Path;
use std::process::Command;

use tempfile::TempDir;

const HELPER_ENV: &str = "SC_LINT_STEP_HELPER";
const HELPER_EXIT: &str = "SC_LINT_STEP_EXIT";

#[test]
fn lint_step_child_reports_environment_and_directory() {
    if std::env::var_os(HELPER_ENV).is_none() {
        return;
    }
    println!(
        "step-env={}",
        std::env::var("PYO3_PYTHON").unwrap_or_default()
    );
    println!(
        "step-cwd={}",
        std::env::current_dir().expect("child cwd").display()
    );
    println!("step-home={}", std::env::var("HOME").unwrap_or_default());
    println!(
        "step-userprofile={}",
        std::env::var("USERPROFILE").unwrap_or_default()
    );
    println!("controlled stdout");
    eprintln!("controlled stderr");
    if let Some(code) = std::env::var_os(HELPER_EXIT) {
        std::process::exit(code.to_string_lossy().parse().expect("numeric exit code"));
    }
}

#[test]
fn lint_step_env_and_cwd() {
    let fixture = TempDir::new().expect("fixture");
    std::fs::create_dir(fixture.path().join("crates")).expect("crates dir");
    std::fs::create_dir(fixture.path().join("crates/x")).expect("child directory");
    write_config(fixture.path(), &[("env-cwd", "", true)]);
    let logs = fixture.path().join("logs");
    let output = run_lint(fixture.path(), &logs);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = std::fs::read_to_string(logs.join("steps/env-cwd.log")).expect("step log");
    assert!(
        stdout.contains("step-env=.bootstrap-venv/bin/python"),
        "{stdout}"
    );
    assert!(
        stdout.contains(&format!("step-home={}", fixture.path().display())),
        "{stdout}"
    );
    assert!(
        stdout.contains(&format!("step-userprofile={}", fixture.path().display())),
        "{stdout}"
    );
    let expected = dunce::canonicalize(fixture.path().join("crates/x")).expect("expected cwd");
    let actual = stdout
        .lines()
        .find_map(|line| line.strip_prefix("step-cwd="))
        .expect("child cwd output");
    assert_eq!(dunce::canonicalize(actual).expect("reported cwd"), expected);
}

#[test]
fn consumer_failed_step_prints_both_streams() {
    let fixture = TempDir::new().expect("fixture");
    write_config(
        fixture.path(),
        &[("env-cwd", "7", false), ("pass", "", false)],
    );
    let output = run_lint(fixture.path(), &fixture.path().join("logs"));
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("controlled stdout"), "{stderr}");
    assert!(stderr.contains("controlled stderr"), "{stderr}");
}

#[test]
fn consumer_failed_step_without_log_root_prints_both_streams() {
    let fixture = TempDir::new().expect("fixture");
    write_config(
        fixture.path(),
        &[("env-cwd", "7", false), ("pass", "", false)],
    );
    let output = run_lint_without_log_root(fixture.path());
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("controlled stdout"), "{stderr}");
    assert!(stderr.contains("controlled stderr"), "{stderr}");
    assert!(
        !fixture.path().join("sc-lint/steps").exists(),
        "steps directory must not be created without a configured log root"
    );
}

#[test]
fn consumer_failed_step_reports_log_write_failure_with_step_output() {
    let fixture = TempDir::new().expect("fixture");
    write_config(fixture.path(), &[("env-cwd", "7", false)]);
    let logs = fixture.path().join("logs");
    std::fs::create_dir_all(&logs).expect("log root");
    std::fs::write(logs.join("steps"), b"block directory creation").expect("blocking file");

    let output = run_lint(fixture.path(), &logs);
    assert!(!output.status.success());
    let envelope: serde_json::Value =
        serde_json::from_slice(&output.stderr).expect("step failure error envelope");
    let failed_step = &envelope["error"]["details"]["failed_steps"][0];
    assert_eq!(failed_step["exit_code"], 7);
    assert!(
        failed_step["message"]
            .as_str()
            .unwrap()
            .contains("controlled stdout")
    );
    assert!(
        failed_step["message"]
            .as_str()
            .unwrap()
            .contains("controlled stderr")
    );
    assert!(failed_step["step_log_error"].is_string());
    let destination = failed_step["step_log_destination"]
        .as_str()
        .expect("step log destination");
    assert!(
        Path::new(destination).ends_with(Path::new("steps").join("env-cwd.log")),
        "unexpected step log destination: {destination}"
    );
}

#[test]
fn consumer_passing_step_reports_log_write_failure_with_step_output() {
    let fixture = TempDir::new().expect("fixture");
    write_config(fixture.path(), &[("env-cwd", "", false)]);
    let logs = fixture.path().join("logs");
    std::fs::create_dir_all(&logs).expect("log root");
    std::fs::write(logs.join("steps"), b"block directory creation").expect("blocking file");

    let output = run_lint(fixture.path(), &logs);
    assert_eq!(
        output.status.code(),
        Some(i32::from(sc_lint::consts::EXIT_BACKEND_FAILURE))
    );
    let envelope: serde_json::Value =
        serde_json::from_slice(&output.stderr).expect("step failure error envelope");
    let failed_steps = envelope["error"]["details"]["failed_steps"]
        .as_array()
        .expect("failed steps");
    assert_eq!(failed_steps.len(), 1);
    let failed_step = &failed_steps[0];
    assert_eq!(failed_step["exit_code"], 0);
    let message = failed_step["message"].as_str().expect("failure message");
    assert!(message.contains("controlled stdout"), "{message}");
    assert!(message.contains("controlled stderr"), "{message}");
    assert!(failed_step["step_log_error"].is_string());
    let destination = failed_step["step_log_destination"]
        .as_str()
        .expect("step log destination");
    assert!(
        Path::new(destination).ends_with(Path::new("steps").join("env-cwd.log")),
        "unexpected step log destination: {destination}"
    );
}

#[test]
fn consumer_log_root_writes_every_step() {
    let fixture = TempDir::new().expect("fixture");
    write_config(
        fixture.path(),
        &[("env-cwd", "7", false), ("pass", "", false)],
    );
    let logs = fixture.path().join("logs");
    let _ = run_lint(fixture.path(), &logs);
    let failed = std::fs::read_to_string(logs.join("steps/env-cwd.log")).expect("failed step log");
    let passed = std::fs::read_to_string(logs.join("steps/pass.log")).expect("passing step log");
    for contents in [&failed, &passed] {
        assert!(contents.contains("[stdout]\n"), "{contents}");
        assert!(contents.contains("controlled stdout"), "{contents}");
        assert!(contents.contains("[stderr]\n"), "{contents}");
        assert!(contents.contains("controlled stderr"), "{contents}");
    }
}

#[test]
fn consumer_log_root_disambiguates_sanitized_name_collisions_in_declaration_order() {
    let fixture = TempDir::new().expect("fixture");
    write_config(fixture.path(), &[("a/b", "", false), ("a?b", "", false)]);
    let logs = fixture.path().join("logs");

    let output = run_lint(fixture.path(), &logs);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(logs.join("steps/a_b.log").is_file());
    assert!(logs.join("steps/a_b-2.log").is_file());
}

fn helper_command() -> Vec<String> {
    vec![
        std::env::current_exe()
            .expect("test binary")
            .display()
            .to_string(),
        "--exact".to_string(),
        "lint_step_child_reports_environment_and_directory".to_string(),
        "--nocapture".to_string(),
    ]
}

fn write_config(root: &Path, rows: &[(&str, &str, bool)]) {
    let mut config = String::from("[tool.sc-lint]\nminimum_version = \"0.4.0\"\n");
    let command = helper_command();
    for (name, exit, with_env_cwd) in rows {
        config.push_str(&format!(
            "\n[[tool.sc-lint.lint]]\nname = {name:?}\ncommand = {command:?}\n"
        ));
        let mut env = String::from("SC_LINT_STEP_HELPER = \"1\"");
        if *with_env_cwd {
            env.push_str(", PYO3_PYTHON = \".bootstrap-venv/bin/python\"");
            config.push_str("cwd = \"crates/x\"\n");
        }
        if !exit.is_empty() {
            env.push_str(&format!(", SC_LINT_STEP_EXIT = {exit:?}"));
        }
        config.push_str(&format!("env = {{ {env} }}\n"));
    }
    config.push_str(&format!(
        "\n[[tool.sc-lint.test]]\nname = \"test\"\ncommand = {command:?}\n"
    ));
    std::fs::write(root.join("sc-lint.toml"), config).expect("write consumer config");
}

fn run_lint(root: &Path, logs: &Path) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_sc-lint"))
        .current_dir(root)
        .env("HOME", root)
        .env("USERPROFILE", root)
        .arg("--json")
        .arg("--log-root")
        .arg(logs)
        .args(["lint", "ci", "--consumer"])
        .output()
        .expect("sc-lint consumer lint runs")
}

fn run_lint_without_log_root(root: &Path) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_sc-lint"))
        .current_dir(root)
        .env("HOME", root)
        .env("USERPROFILE", root)
        .args(["--json", "lint", "ci", "--consumer"])
        .output()
        .expect("sc-lint consumer lint runs without a step log root")
}
