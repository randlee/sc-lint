use std::path::Path;
use std::process::Command;

use serde_json::Value;
use tempfile::TempDir;

#[test]
fn logger_bootstrap_writes_entry_completion_error_and_elapsed_time_records() {
    let temp_dir = TempDir::new().expect("temp dir");
    let temp_root = temp_dir.path().join("logs");
    let binary = env!("CARGO_BIN_EXE_sc-lint");

    let success = Command::new(binary)
        .args([
            "--json",
            "--log-root",
            temp_root.to_str().expect("utf-8 temp path"),
            "version",
        ])
        .output()
        .expect("version command runs");
    assert!(
        success.status.success(),
        "version stderr: {}",
        String::from_utf8_lossy(&success.stderr)
    );

    let failure = Command::new(binary)
        .args([
            "--json",
            "--log-root",
            temp_root.to_str().expect("utf-8 temp path"),
            "lint",
            "sc-boundary",
        ])
        .output()
        .expect("reserved command runs");
    assert_eq!(failure.status.code(), Some(4));

    let success_log_path = temp_root.join("sc-lint").join("sc-lint.log.jsonl");
    assert_log_file_contains_action(&success_log_path, "cli.command.started");
    assert_log_file_contains_elapsed_ms(&success_log_path);

    let error_log_path = temp_root.join("sc-boundary").join("sc-boundary.log.jsonl");
    assert_log_file_contains_action(&error_log_path, "cli.command.error");
    assert_log_file_contains_elapsed_ms(&error_log_path);
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
