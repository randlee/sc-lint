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

fn excluded_workspace() -> TempDir {
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
    root
}

fn run_excluded_workspace(
    root: &TempDir,
    config: &str,
    extra_args: &[&str],
) -> std::process::Output {
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

fn run_site_allow_error(source: &str) -> std::process::Output {
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
    fs::write(package.join("src/lib.rs"), source).unwrap();

    Command::new(env!("CARGO_BIN_EXE_sc-lint-portability"))
        .args(["analyze", "--root"])
        .arg(root.path())
        .output()
        .unwrap()
}

#[test]
fn site_allow_errors_exit_as_configuration_errors() {
    for (source, code, recovery) in [
        (
            "#[sc_lint(allow(PORT-010))]\n#[cfg(unix)] pub fn only_unix() {}\n",
            "PORTABILITY.INVALID_SITE_ALLOW",
            "DIRECTIVE.SITE_ALLOW.MISSING_REASON",
        ),
        (
            "#[sc_lint(allow(PORT-001), reason = \"unsupported\")]\n#[cfg(unix)] pub fn only_unix() {}\n",
            "PORTABILITY.UNKNOWN_RULE",
            "recovery: use PORT-010",
        ),
    ] {
        let output = run_site_allow_error(source);
        assert_eq!(output.status.code(), Some(3));
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains(code), "{stderr}");
        assert!(stderr.contains(recovery), "{stderr}");
    }
}

fn port010_findings(shape: &str) -> Vec<serde_json::Value> {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/port010")
        .join(shape);
    let config = fixture.join("sc-lint.toml");
    let output = Command::new(env!("CARGO_BIN_EXE_sc-lint-portability"))
        .args(["analyze", "--root"])
        .arg(&fixture)
        .arg("--config")
        .arg(config)
        .args(["--format", "json"])
        .output()
        .unwrap();

    assert_eq!(
        output.status.code(),
        Some(0),
        "analyzer invocation failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    report["findings"]
        .as_array()
        .expect("findings array")
        .iter()
        .filter(|finding| finding["rule_id"] == "PORT-010")
        .cloned()
        .collect()
}

fn port010_items(shape: &str) -> Vec<String> {
    port010_findings(shape)
        .iter()
        .map(|finding| {
            let message = finding["message"].as_str().expect("message string");
            message
                .split("item `")
                .nth(1)
                .and_then(|rest| rest.split('`').next())
                .expect("PORT-010 message names an item")
                .to_string()
        })
        .collect()
}

/// Asserts the exact multiset of PORT-010 items reported for an atm-core
/// shape fixture; expected names are the analyzer's ground truth on the
/// pinned source (atm-core f4746d500).
fn assert_port010_items(shape: &str, expected: &[&str]) {
    let mut actual = port010_items(shape);
    actual.sort();
    let mut expected: Vec<String> = expected.iter().map(|name| (*name).to_string()).collect();
    expected.sort();
    assert_eq!(actual, expected, "PORT-010 items for fixture {shape}");
}

#[test]
fn port010_fixture_atm_core_atm_temp() {
    assert_port010_items(
        "atm_core_atm_temp",
        &[
            "is_owned_by_uid",
            "has_group_or_world_bits",
            "current_uid",
            "unix_default_scratch_root",
        ],
    );
}

#[test]
fn port010_fixture_atm_core_atm_temp_sweeper() {
    assert_port010_items(
        "atm_core_atm_temp_sweeper",
        &["from_metadata", "unix_epoch_time"],
    );
}

#[test]
fn port010_fixture_atm_core_persistence() {
    assert_port010_items("atm_core_persistence", &["sync_parent_directory"]);
}

#[test]
fn port010_fixture_atm_core_transfer_script() {
    assert_port010_items(
        "atm_core_transfer_script",
        &[
            "unix_synthesized_transfer_script_path",
            "is_owner_executable",
        ],
    );
}

#[test]
fn port010_fixture_atm_daemon_bootstrap_lib() {
    assert_port010_items(
        "atm_daemon_bootstrap_lib",
        &["unix_socket_config", "unix_socket_config_for_uid"],
    );
}

#[test]
fn port010_fixture_atm_daemon_bootstrap_protected_trust_root() {
    assert_port010_items(
        "atm_daemon_bootstrap_protected_trust_root",
        &["verify_protected_path", "verify_unix_metadata"],
    );
}

#[test]
fn port010_fixture_atm_daemon_client_lib() {
    assert_port010_items(
        "atm_daemon_client_lib",
        &["unix_socket_path", "try_connect_unix_socket"],
    );
}

#[test]
fn port010_fixture_atm_herdr_transport_socket() {
    assert_port010_items("atm_herdr_transport_socket", &["connect_unix"]);
}

#[test]
fn port010_fixture_atm_http_runtime_client() {
    assert_port010_items(
        "atm_http_runtime_client",
        &["runtime_directory_is_root_owned"],
    );
}

#[test]
fn port010_fixture_atm_http_runtime_client_tail() {
    assert_port010_items(
        "atm_http_runtime_client_tail",
        &[
            "unix_socket_client",
            "build_unix_socket_reqwest_client",
            "unix_socket_generation",
        ],
    );
}

#[test]
fn port010_fixture_atm_http_runtime_http1_server() {
    assert_port010_items("atm_http_runtime_http1_server", &["serve_unix_http1"]);
}

#[test]
fn port010_fixture_atm_http_runtime_lib() {
    // Two `get` findings: the impl methods on UnixSocketOwnerUid and UnixSocketMode.
    assert_port010_items("atm_http_runtime_lib", &["unix_socket", "get", "get"]);
}

#[test]
fn port010_fixture_atm_http_runtime_loopback_tcp() {
    assert_port010_items(
        "atm_http_runtime_loopback_tcp",
        &["create_private_record_file", "validate_record_parent"],
    );
}

#[test]
fn port010_fixture_atm_http_runtime_runtime_listener() {
    assert_port010_items(
        "atm_http_runtime_runtime_listener",
        &["bind_configured_unix_listener"],
    );
}

#[test]
fn port010_fixture_atm_http_runtime_runtime_setup() {
    assert_port010_items(
        "atm_http_runtime_runtime_setup",
        &["validate_unix_socket_path"],
    );
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
fn port010_fixture_unix_helper_in_paired_file() {
    assert!(port010_findings("unix_helper_in_paired_file").is_empty());
}

#[test]
fn port010_fixture_crate_root_main_current_behaviour() {
    // Records the current classification; the cfg(not(unix)) sibling is not a
    // portable fallback and this shape is not claimed fixed by L1-4.
    let findings = port010_findings("crate_root_main");
    assert_eq!(findings.len(), 1, "{findings:#?}");
    assert!(
        findings[0]["message"]
            .as_str()
            .unwrap()
            .contains("restore_default_sigpipe")
    );
}

#[test]
fn port010_fixture_unpaired_pub_item_reported() {
    let findings = port010_findings("unpaired_pub_item");
    assert_eq!(findings.len(), 1, "{findings:#?}");
    assert!(
        findings[0]["message"]
            .as_str()
            .unwrap()
            .contains("raise_descriptor_limit")
    );
}

#[test]
fn cli_excludes_windows_separator_paths_with_a_rule_filter() {
    let root = excluded_workspace();
    let args = ["--format", "json", "--rule-filter", "PORT-006"];
    let baseline = run_excluded_workspace(&root, "[tool.sc-lint.analyzers.portability]\n", &args);
    assert_eq!(baseline.status.code(), Some(0));
    let baseline_report: serde_json::Value = serde_json::from_slice(&baseline.stdout).unwrap();
    assert!(
        baseline_report["findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|finding| finding["rule_id"] == "PORT-006"),
        "the unexcluded fixture must report PORT-006: {baseline_report}"
    );
    assert_eq!(baseline_report["status"], "fail");

    let excluded = run_excluded_workspace(
        &root,
        r#"[tool.sc-lint.analyzers.portability]
exclude_paths = ["crates\\example\\src\\lib.rs"]
"#,
        &args,
    );

    assert_eq!(excluded.status.code(), Some(0));
    let excluded_report: serde_json::Value = serde_json::from_slice(&excluded.stdout).unwrap();
    assert_eq!(excluded_report["status"], "pass");
    assert!(
        excluded_report["findings"].as_array().unwrap().is_empty(),
        "excluded file was still reported"
    );
}

#[test]
fn cli_excludes_per_rule_windows_separator_paths_with_a_rule_filter() {
    let root = excluded_workspace();
    let args = ["--format", "json", "--rule-filter", "PORT-006"];
    let baseline = run_excluded_workspace(&root, "[tool.sc-lint.analyzers.portability]\n", &args);
    assert_eq!(baseline.status.code(), Some(0));
    let baseline_report: serde_json::Value = serde_json::from_slice(&baseline.stdout).unwrap();
    assert_eq!(baseline_report["status"], "fail");
    assert!(
        baseline_report["findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|finding| finding["rule_id"] == "PORT-006"),
        "the unexcluded fixture must report PORT-006: {baseline_report}"
    );

    let output = run_excluded_workspace(
        &root,
        r#"[tool.sc-lint.analyzers.portability.exclude_paths_by_rule]
"PORT-006" = ["crates\\example\\src\\lib.rs"]
"#,
        &args,
    );

    assert_eq!(output.status.code(), Some(0));
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["status"], "pass");
    assert!(report["findings"].as_array().unwrap().is_empty());
}
