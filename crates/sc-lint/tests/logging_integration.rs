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
            "check",
            "xwin",
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

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root")
        .to_path_buf()
}
