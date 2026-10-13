use std::fs;
use std::process::Command;

use tempfile::TempDir;

fn run_analyzer(config: &str, extra_args: &[&str]) -> std::process::Output {
    let root = TempDir::new().unwrap();
    let config_path = root.path().join("sc-lint.toml");
    fs::write(&config_path, config).unwrap();

    let mut command = Command::new(env!("CARGO_BIN_EXE_sc-lint-portability"));
    command
        .args(["analyze", "--root"])
        .arg(root.path())
        .arg("--config")
        .arg(config_path)
        .args(extra_args);

    command.output().unwrap()
}

#[test]
fn unknown_rule_filter_exits_two_and_prefixes_error() {
    let output = run_analyzer(
        "[tool.sc-lint.analyzers.portability]\n",
        &["--rule-filter", "PORT-999"],
    );

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.starts_with("error: portability: unknown --rule-filter selector"));
}

#[test]
fn invalid_analyzer_config_exits_three_and_prefixes_error() {
    let output = run_analyzer(
        "[tool.sc-lint.analyzers.portability]\nunix_path_prefixes = \"relative\"\n",
        &[],
    );

    assert_eq!(output.status.code(), Some(3));
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.starts_with("error: portability: `unix_path_prefixes`"));
}

#[test]
fn issue372_cli_configured_pair() {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/issue372");
    let config = fixture.join("sc-lint.toml");

    let output = Command::new(env!("CARGO_BIN_EXE_sc-lint-portability"))
        .args(["analyze", "--root"])
        .arg(&fixture)
        .arg("--config")
        .arg(&config)
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    assert!(
        !String::from_utf8(output.stdout)
            .unwrap()
            .contains("PORT-010")
    );
}
