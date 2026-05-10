use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

use serde_json::Value;
use tempfile::TempDir;

#[test]
fn logger_bootstrap_writes_entry_completion_dispatch_and_error_records() {
    let temp_dir = TempDir::new().expect("temp dir");
    let temp_root = temp_dir.path().join("logs");
    let repo_root = workspace_root();
    let binary = env!("CARGO_BIN_EXE_sc-lint");

    let version = Command::new(binary)
        .current_dir(&repo_root)
        .args([
            "--json",
            "--log-root",
            temp_root.to_str().expect("utf-8 temp path"),
            "version",
        ])
        .output()
        .expect("version command runs");
    assert!(
        version.status.success(),
        "version stderr: {}",
        String::from_utf8_lossy(&version.stderr)
    );

    let dispatch = Command::new(binary)
        .current_dir(&repo_root)
        .args([
            "--json",
            "--root",
            repo_root.to_str().expect("utf-8 repo root"),
            "--log-root",
            temp_root.to_str().expect("utf-8 temp path"),
            "lint",
            "sc-boundary",
        ])
        .output()
        .expect("dispatch command runs");
    assert!(
        dispatch.status.success(),
        "dispatch stderr: {}",
        String::from_utf8_lossy(&dispatch.stderr)
    );

    let failure = Command::new(binary)
        .current_dir(&repo_root)
        .args([
            "--json",
            "--root",
            repo_root.to_str().expect("utf-8 repo root"),
            "--log-root",
            temp_root.to_str().expect("utf-8 temp path"),
            "view",
            "graph",
        ])
        .output()
        .expect("reserved command runs");
    assert_eq!(failure.status.code(), Some(4));

    let cli_log_path = temp_root.join("sc-lint").join("sc-lint.log.jsonl");
    assert_log_file_contains_action(&cli_log_path, "cli.command.started");
    assert_log_file_contains_error_action(&cli_log_path, "cli.command.error");
    assert_log_file_contains_elapsed_ms(&cli_log_path);

    let dispatch_log_path = temp_root.join("sc-boundary").join("sc-boundary.log.jsonl");
    assert_log_file_contains_action(&dispatch_log_path, "cli.dispatch.started");
    assert_log_file_contains_action(&dispatch_log_path, "cli.dispatch.normalized");
    assert_log_file_contains_elapsed_ms(&dispatch_log_path);
}

#[test]
fn xwin_logging_records_target_metadata_for_success_and_error_paths() {
    let temp_dir = TempDir::new().expect("temp dir");
    let repo_root = workspace_root();
    let binary = env!("CARGO_BIN_EXE_sc-lint");

    let success_logs = temp_dir.path().join("logs-success");
    let success_path =
        cargo_wrapper_path(temp_dir.path().join("bin-success"), CargoMode::XwinSuccess);
    let success = Command::new(binary)
        .current_dir(&repo_root)
        .env("PATH", prepend_path(&success_path))
        .args([
            "--json",
            "--root",
            repo_root.to_str().expect("utf-8 repo root"),
            "--log-root",
            success_logs.to_str().expect("utf-8 temp path"),
            "check",
            "xwin",
        ])
        .output()
        .expect("xwin success command runs");
    assert!(
        success.status.success(),
        "xwin success stderr: {}",
        String::from_utf8_lossy(&success.stderr)
    );

    let success_log_path = success_logs.join("sc-lint").join("sc-lint.log.jsonl");
    assert_log_file_contains_field(
        &success_log_path,
        "cli.command.started",
        "target_triple",
        "x86_64-pc-windows-msvc",
    );
    assert_log_file_contains_field(
        &success_log_path,
        "cli.command.completed",
        "target_triple",
        "x86_64-pc-windows-msvc",
    );

    let failure_logs = temp_dir.path().join("logs-failure");
    let failure_path =
        cargo_wrapper_path(temp_dir.path().join("bin-failure"), CargoMode::XwinMissing);
    let failure = Command::new(binary)
        .current_dir(&repo_root)
        .env("PATH", prepend_path(&failure_path))
        .args([
            "--json",
            "--root",
            repo_root.to_str().expect("utf-8 repo root"),
            "--log-root",
            failure_logs.to_str().expect("utf-8 temp path"),
            "check",
            "xwin",
        ])
        .output()
        .expect("xwin failure command runs");
    assert_eq!(failure.status.code(), Some(4));

    let failure_log_path = failure_logs.join("sc-lint").join("sc-lint.log.jsonl");
    assert_log_file_contains_field(
        &failure_log_path,
        "cli.command.error",
        "target_triple",
        "x86_64-pc-windows-msvc",
    );
    assert_log_file_contains_field(
        &failure_log_path,
        "cli.command.error",
        "preflight_mode",
        "xwin",
    );
}

#[test]
fn python_backed_commands_log_adapter_metadata() {
    let temp_dir = TempDir::new().expect("temp dir");
    let temp_root = temp_dir.path().join("logs-python");
    let repo_root = workspace_root();
    let binary = env!("CARGO_BIN_EXE_sc-lint");

    let output = Command::new(binary)
        .current_dir(&repo_root)
        .args([
            "--json",
            "--root",
            repo_root.to_str().expect("utf-8 repo root"),
            "--log-root",
            temp_root.to_str().expect("utf-8 temp path"),
            "lint",
            "line-counts",
        ])
        .output()
        .expect("python-backed command runs");
    assert!(
        output.status.success(),
        "python-backed stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let log_path = temp_root.join("sc-lint").join("sc-lint.log.jsonl");
    assert_log_file_contains_field(
        &log_path,
        "cli.command.started",
        "adapter",
        "python-json-v1",
    );
    assert_log_file_contains_field(
        &log_path,
        "cli.command.started",
        "config_scope",
        "line_counts",
    );
    assert_log_file_contains_field(
        &log_path,
        "cli.command.started",
        "script",
        ".just/lint_line_counts.py",
    );
    assert_log_file_contains_action(&log_path, "cli.dispatch.started");
    assert_log_file_contains_action(&log_path, "cli.dispatch.normalized");
}

fn assert_log_file_contains_action(path: &Path, action: &str) {
    let contents = std::fs::read_to_string(path).expect("log file exists");
    assert!(
        contents
            .lines()
            .filter_map(|line| serde_json::from_str::<Value>(line).ok())
            .any(|line| line["action"] == action),
        "expected `{action}` in {path:?}"
    );
}

fn assert_log_file_contains_error_action(path: &Path, action: &str) {
    let contents = std::fs::read_to_string(path).expect("log file exists");
    assert!(
        contents
            .lines()
            .filter_map(|line| serde_json::from_str::<Value>(line).ok())
            .any(|line| line["action"] == action && line["fields"]["code"].is_string()),
        "expected `{action}` with code field in {path:?}"
    );
}

fn assert_log_file_contains_elapsed_ms(path: &Path) {
    let contents = std::fs::read_to_string(path).expect("log file exists");
    assert!(
        contents
            .lines()
            .filter_map(|line| serde_json::from_str::<Value>(line).ok())
            .any(|line| {
                line["action"] == "cli.command.completed"
                    && line["fields"]["elapsed_ms"].as_u64().is_some()
            }),
        "expected elapsed_ms in completion record for {path:?}"
    );
}

fn assert_log_file_contains_field(path: &Path, action: &str, field: &str, expected: &str) {
    let contents = std::fs::read_to_string(path).expect("log file exists");
    assert!(
        contents
            .lines()
            .filter_map(|line| serde_json::from_str::<Value>(line).ok())
            .any(|line| {
                line["action"] == action && line["fields"][field].as_str() == Some(expected)
            }),
        "expected `{field}={expected}` in `{action}` for {path:?}"
    );
}

#[derive(Debug, Clone, Copy)]
enum CargoMode {
    XwinSuccess,
    XwinMissing,
}

fn cargo_wrapper_path(bin_dir: PathBuf, mode: CargoMode) -> PathBuf {
    std::fs::create_dir_all(&bin_dir).expect("create bin dir");
    #[cfg(windows)]
    let path = bin_dir.join("cargo.cmd");
    #[cfg(not(windows))]
    let path = bin_dir.join("cargo");

    let body = match mode {
        CargoMode::XwinSuccess => cargo_wrapper_body_success(),
        CargoMode::XwinMissing => cargo_wrapper_body_missing(),
    };
    std::fs::write(&path, body).expect("write cargo wrapper");

    #[cfg(not(windows))]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = std::fs::metadata(&path)
            .expect("wrapper metadata")
            .permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&path, permissions).expect("wrapper permissions");
    }

    path
}

#[cfg(windows)]
fn cargo_wrapper_body_success() -> String {
    "@echo off\r\nif \"%1\"==\"xwin\" exit /b 0\r\nexit /b 1\r\n".to_string()
}

#[cfg(not(windows))]
fn cargo_wrapper_body_success() -> String {
    "#!/bin/sh\nif [ \"$1\" = \"xwin\" ]; then\n  exit 0\nfi\nexit 1\n".to_string()
}

#[cfg(windows)]
fn cargo_wrapper_body_missing() -> String {
    "@echo off\r\nexit /b 1\r\n".to_string()
}

#[cfg(not(windows))]
fn cargo_wrapper_body_missing() -> String {
    "#!/bin/sh\nexit 1\n".to_string()
}

fn prepend_path(wrapper_path: &Path) -> std::ffi::OsString {
    let bin_dir = wrapper_path.parent().expect("wrapper parent");
    let mut parts = vec![bin_dir.as_os_str().to_os_string()];
    if let Some(existing) = std::env::var_os("PATH") {
        parts.extend(std::env::split_paths(&existing).map(|path| path.into_os_string()));
    }
    std::env::join_paths(parts).expect("join path entries")
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root")
        .to_path_buf()
}
