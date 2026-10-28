use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn sc_lint_runtime_cli_reports_tokio_blocking_sleep_with_full_source_path() {
    let fixture = TempDir::new().expect("fixture directory");
    let root = fixture.path();
    write(
        root,
        "Cargo.toml",
        "[workspace]\nmembers = [\"crates/example\"]\nexclude = [\"vendor/tokio\"]\nresolver = \"2\"\n",
    );
    write(
        root,
        "vendor/tokio/Cargo.toml",
        "[package]\nname = \"tokio\"\nversion = \"1.0.0\"\nedition = \"2024\"\n",
    );
    write(root, "vendor/tokio/src/lib.rs", "");
    write(
        root,
        "crates/example/Cargo.toml",
        "[package]\nname = \"example\"\nversion = \"0.1.0\"\nedition = \"2024\"\n[dependencies]\ntokio = { path = \"../../vendor/tokio\" }\n",
    );
    write(
        root,
        "crates/example/src/lib.rs",
        "pub fn blocks(d: std::time::Duration) { std::thread::sleep(d); }\n",
    );

    let output = Command::new(env!("CARGO_BIN_EXE_sc-lint"))
        .args([
            "--json",
            "--root",
            root.to_str().expect("UTF-8 fixture path"),
            "lint",
            "sc-runtime",
        ])
        .output()
        .expect("sc-lint CLI runs");

    assert_eq!(
        output.status.code(),
        Some(10),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let envelope: Value = serde_json::from_slice(&output.stdout).expect("CLI JSON envelope");
    assert_eq!(envelope["command"], "lint.sc-runtime");
    assert_eq!(envelope["data"]["status"], "fail");
    let findings = envelope["data"]["findings"]
        .as_array()
        .expect("finding list");
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0]["rule_id"], "SCB-RUNTIME-003");
    let message = findings[0]["message"].as_str().expect("finding message");
    let source_path = message.split(":1: ").next().expect("source path prefix");
    let relative = Path::new(source_path)
        .strip_prefix(root.canonicalize().expect("canonical fixture root"))
        .expect("source under fixture root");
    assert_eq!(
        relative.to_string_lossy().replace('\\', "/"),
        "crates/example/src/lib.rs"
    );
}

fn write(root: &Path, relative: &str, contents: &str) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().expect("parent directory")).expect("fixture parent");
    fs::write(path, contents).expect("fixture file");
}
