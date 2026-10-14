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

fn run_invalid_adapter_config(config: &str) -> std::process::Output {
    let root = TempDir::new().unwrap();
    fs::write(
        root.path().join("Cargo.toml"),
        "[workspace]\nmembers = [\"crates/example\"]\nresolver = \"2\"\n",
    )
    .unwrap();
    let package = root.path().join("crates/example");
    fs::create_dir_all(package.join("src")).unwrap();
    fs::write(
        package.join("Cargo.toml"),
        "[package]\nname = \"example\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .unwrap();
    fs::write(
        package.join("src/lib.rs"),
        "#[cfg(unix)] mod unix_socket;\n#[cfg(windows)] mod named_pipe;\n#[cfg(windows)] mod windows_only;\n#[cfg(test)] mod test_only;\n",
    )
    .unwrap();
    for file in [
        "unix_socket.rs",
        "named_pipe.rs",
        "windows_only.rs",
        "test_only.rs",
    ] {
        fs::write(package.join("src").join(file), "fn connect() {}\n").unwrap();
    }
    let config_path = root.path().join("sc-lint.toml");
    fs::write(&config_path, config).unwrap();

    Command::new(env!("CARGO_BIN_EXE_sc-lint-portability"))
        .args(["analyze", "--root"])
        .arg(root.path())
        .arg("--config")
        .arg(config_path)
        .output()
        .unwrap()
}

fn run_excluded_workspace(config: &str, extra_args: &[&str]) -> std::process::Output {
    let root = TempDir::new().unwrap();
    fs::write(
        root.path().join("Cargo.toml"),
        "[workspace]\nmembers = [\"crates/example\"]\nresolver = \"2\"\n",
    )
    .unwrap();
    let package = root.path().join("crates/example");
    fs::create_dir_all(package.join("src")).unwrap();
    fs::write(
        package.join("Cargo.toml"),
        "[package]\nname = \"example\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .unwrap();
    fs::write(
        package.join("src/lib.rs"),
        "pub fn path() { let _ = \"/home/user/file\"; }\n",
    )
    .unwrap();
    let config_path = root.path().join("sc-lint.toml");
    fs::write(&config_path, config).unwrap();

    Command::new(env!("CARGO_BIN_EXE_sc-lint-portability"))
        .args(["analyze", "--root"])
        .arg(root.path())
        .arg("--config")
        .arg(config_path)
        .args(extra_args)
        .output()
        .unwrap()
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
fn issue372_invalid_adapter_matrix_exits_three_through_the_cli() {
    let valid_pair = |unix_module: &str, windows_module: &str| {
        format!(
            "[tool.sc-lint.analyzers.portability]\n[[tool.sc-lint.analyzers.portability.platform_adapters]]\nunix_module = \"{unix_module}\"\nwindows_module = \"{windows_module}\"\nreason = \"Private transport adapters\"\n"
        )
    };
    let repeated_pair = format!(
        "{}[[tool.sc-lint.analyzers.portability.platform_adapters]]\nunix_module = \"crates/example/src/unix_socket.rs\"\nwindows_module = \"crates/example/src/named_pipe.rs\"\nreason = \"Repeated pair\"\n",
        valid_pair(
            "crates/example/src/unix_socket.rs",
            "crates/example/src/named_pipe.rs"
        )
    );
    let cases = [
        (
            "wrong type",
            "[tool.sc-lint.analyzers.portability]\nplatform_adapters = \"invalid\"\n".to_owned(),
        ),
        (
            "unknown key",
            "[tool.sc-lint.analyzers.portability]\n[[tool.sc-lint.analyzers.portability.platform_adapters]]\nunix_module = \"crates/example/src/unix_socket.rs\"\nwindows_module = \"crates/example/src/named_pipe.rs\"\nreason = \"Private transport adapters\"\nextra = true\n".to_owned(),
        ),
        (
            "non-rs path",
            valid_pair(
                "crates/example/src/socket.txt",
                "crates/example/src/named_pipe.rs",
            ),
        ),
        (
            "missing adapter",
            valid_pair(
                "crates/example/src/missing.rs",
                "crates/example/src/named_pipe.rs",
            ),
        ),
        ("repeated pair", repeated_pair),
        (
            "wrong cfg",
            valid_pair(
                "crates/example/src/windows_only.rs",
                "crates/example/src/named_pipe.rs",
            ),
        ),
        (
            "test-only adapter",
            valid_pair(
                "crates/example/src/test_only.rs",
                "crates/example/src/named_pipe.rs",
            ),
        ),
    ];

    for (label, config) in cases {
        let output = run_invalid_adapter_config(&config);
        assert_eq!(output.status.code(), Some(3), "matrix row: {label}");
    }
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

#[test]
fn cli_excludes_windows_separator_paths_with_a_rule_filter() {
    let output = run_excluded_workspace(
        r#"[tool.sc-lint.analyzers.portability]
exclude_paths = ["crates\\example\\src\\lib.rs"]
"#,
        &["--rule-filter", "PORT-006"],
    );

    assert_eq!(output.status.code(), Some(0));
    assert!(
        !String::from_utf8(output.stdout)
            .unwrap()
            .contains("PORT-006"),
        "excluded file was still reported"
    );
}
