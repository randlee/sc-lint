use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;

use serde_json::Value;
use serial_test::serial;
use tempfile::TempDir;

#[test]
#[serial]
fn logger_bootstrap_writes_entry_completion_dispatch_and_error_records() {
    let temp_dir = TempDir::new().expect("temp dir");
    let temp_root = temp_dir.path().join("logs");
    let fixture = WorkspaceFixture::new();
    let repo_root = fixture.root();
    let binary = env!("CARGO_BIN_EXE_sc-lint");

    let version = sc_lint_command(binary, repo_root)
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

    let dispatch = sc_lint_command(binary, repo_root)
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

    let failure = sc_lint_command(binary, repo_root)
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
    assert_log_file_contains_field(
        &cli_log_path,
        "cli.command.started",
        "command",
        "view.graph",
    );
    assert_log_file_contains_elapsed_ms(&cli_log_path);

    let dispatch_log_path = temp_root.join("sc-boundary").join("sc-boundary.log.jsonl");
    assert_log_file_contains_action(&dispatch_log_path, "cli.dispatch.started");
    assert_log_file_contains_action(&dispatch_log_path, "cli.dispatch.normalized");
    assert_log_file_contains_elapsed_ms(&dispatch_log_path);

    let runtime = sc_lint_command(binary, repo_root)
        .args([
            "--json",
            "--root",
            repo_root.to_str().expect("utf-8 repo root"),
            "--log-root",
            temp_root.to_str().expect("utf-8 temp path"),
            "lint",
            "sc-runtime",
        ])
        .output()
        .expect("runtime command runs");
    assert!(
        runtime.status.success(),
        "runtime stderr: {}",
        String::from_utf8_lossy(&runtime.stderr)
    );

    let runtime_log_path = temp_root.join("sc-runtime").join("sc-runtime.log.jsonl");
    assert_log_file_contains_action(&runtime_log_path, "cli.dispatch.started");
    assert_log_file_contains_action(&runtime_log_path, "cli.dispatch.normalized");
    assert_log_file_contains_elapsed_ms(&runtime_log_path);
}

#[test]
#[serial]
#[cfg_attr(windows, ignore = "cargo.cmd not resolved by CreateProcessW")]
fn xwin_logging_records_target_metadata_for_success_and_error_paths() {
    let temp_dir = TempDir::new().expect("temp dir");
    let fixture = WorkspaceFixture::new();
    let repo_root = fixture.root();
    let binary = env!("CARGO_BIN_EXE_sc-lint");
    let original_path = std::env::var_os("PATH");

    let success_logs = temp_dir.path().join("logs-success");
    let success_path =
        cargo_wrapper_path(temp_dir.path().join("bin-success"), CargoMode::XwinSuccess);
    let success = sc_lint_command(binary, &repo_root)
        .env(
            "PATH",
            prepend_path(&success_path, original_path.as_deref()),
        )
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
    let failure = sc_lint_command(binary, &repo_root)
        .env(
            "PATH",
            prepend_path(&failure_path, original_path.as_deref()),
        )
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
#[serial]
fn sc_boundary_logs_manifest_policy_metadata_for_completion_and_error_paths() {
    let binary = env!("CARGO_BIN_EXE_sc-lint");

    let success_fixture = WorkspaceFixture::new();
    let success_logs = TempDir::new().expect("temp dir");
    let success_output = sc_lint_command(binary, success_fixture.root())
        .args([
            "--json",
            "--root",
            success_fixture.root().to_str().expect("utf-8 repo root"),
            "--log-root",
            success_logs.path().to_str().expect("utf-8 temp path"),
            "lint",
            "sc-boundary",
        ])
        .output()
        .expect("boundary success command runs");
    assert!(
        success_output.status.success(),
        "boundary success stderr: {}",
        String::from_utf8_lossy(&success_output.stderr)
    );

    let success_log_path = success_logs
        .path()
        .join("sc-boundary")
        .join("sc-boundary.log.jsonl");
    for action in ["cli.command.started", "cli.command.completed"] {
        assert_log_file_contains_field(
            &success_log_path,
            action,
            "manifest_policy_mode",
            "rust-native",
        );
        assert_log_file_contains_field(
            &success_log_path,
            action,
            "manifest_policy_parity",
            "python-oracle",
        );
    }

    let failure_fixture = WorkspaceFixture::new();
    failure_fixture.write("crates/example/src/lib.rs", "pub fn broken( {}\n");
    let failure_logs = TempDir::new().expect("temp dir");
    let failure_output = sc_lint_command(binary, failure_fixture.root())
        .args([
            "--json",
            "--root",
            failure_fixture.root().to_str().expect("utf-8 repo root"),
            "--log-root",
            failure_logs.path().to_str().expect("utf-8 temp path"),
            "lint",
            "sc-boundary",
        ])
        .output()
        .expect("boundary failure command runs");
    assert_eq!(failure_output.status.code(), Some(5));

    let failure_log_path = failure_logs
        .path()
        .join("sc-boundary")
        .join("sc-boundary.log.jsonl");
    assert_log_file_contains_field(
        &failure_log_path,
        "cli.command.error",
        "manifest_policy_mode",
        "rust-native",
    );
    assert_log_file_contains_field(
        &failure_log_path,
        "cli.command.error",
        "manifest_policy_parity",
        "python-oracle",
    );
}

#[test]
#[serial]
fn python_backed_commands_log_adapter_metadata() {
    let temp_dir = TempDir::new().expect("temp dir");
    let temp_root = temp_dir.path().join("logs-python");
    let fixture = WorkspaceFixture::new();
    let repo_root = fixture.root();
    let binary = env!("CARGO_BIN_EXE_sc-lint");

    let output = sc_lint_command(binary, repo_root)
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
    let expected = adapter_metadata_from_output(&output.stdout);

    let log_path = temp_root.join("sc-lint").join("sc-lint.log.jsonl");
    assert_log_file_contains_field(
        &log_path,
        "cli.command.started",
        "adapter",
        &expected.adapter,
    );
    assert_log_file_contains_field(
        &log_path,
        "cli.command.started",
        "config_scope",
        &expected.config_scope,
    );
    assert_log_file_contains_field(&log_path, "cli.command.started", "script", &expected.script);
    assert_log_file_contains_field(
        &log_path,
        "cli.command.completed",
        "adapter",
        &expected.adapter,
    );
    assert_log_file_contains_field(
        &log_path,
        "cli.command.completed",
        "config_scope",
        &expected.config_scope,
    );
    assert_log_file_contains_field(
        &log_path,
        "cli.command.completed",
        "script",
        &expected.script,
    );
    assert_log_file_contains_action(&log_path, "cli.dispatch.started");
    assert_log_file_contains_action(&log_path, "cli.dispatch.normalized");
}

#[test]
#[serial]
fn line_counts_logs_adapter_metadata_for_error() {
    let fixture = WorkspaceFixture::new();
    let repo_root = fixture.root();
    let binary = env!("CARGO_BIN_EXE_sc-lint");
    let baseline = sc_lint_command(binary, repo_root)
        .args([
            "--json",
            "--root",
            repo_root.to_str().expect("utf-8 repo root"),
            "lint",
            "line-counts",
        ])
        .output()
        .expect("baseline line-counts command runs");
    assert!(
        baseline.status.success(),
        "baseline line-counts stderr: {}",
        String::from_utf8_lossy(&baseline.stderr)
    );
    let expected = adapter_metadata_from_output(&baseline.stdout);
    let config_override = LintConfigOverride::new("[line_counts]\nmax_total_lines = \"invalid\"\n");
    let temp_dir = TempDir::new().expect("temp dir");
    let temp_root = temp_dir.path().join("logs-line-counts-error");

    let output = sc_lint_command(binary, repo_root)
        .args([
            "--json",
            "--root",
            repo_root.to_str().expect("utf-8 repo root"),
            "--config",
            config_override.path().to_str().expect("utf-8 config path"),
            "--log-root",
            temp_root.to_str().expect("utf-8 temp path"),
            "lint",
            "line-counts",
        ])
        .output()
        .expect("line-counts error command runs");
    assert_eq!(
        output.status.code(),
        Some(3),
        "line-counts stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    assert!(
        output.stdout.is_empty(),
        "expected json error on stderr, got stdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    let output_json = parse_command_output(&output.stderr);
    let log_path = temp_root.join("sc-lint").join("sc-lint.log.jsonl");
    assert_log_file_contains_field(&log_path, "cli.command.error", "adapter", &expected.adapter);
    assert_log_file_contains_field(
        &log_path,
        "cli.command.error",
        "config_scope",
        &expected.config_scope,
    );
    assert_log_file_contains_field(&log_path, "cli.command.error", "script", &expected.script);
    assert_eq!(
        output_json["error"]["kind"].as_str(),
        Some("config"),
        "line-counts stdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );
}

#[test]
#[serial]
fn identity_literals_logs_adapter_metadata_for_completion() {
    let temp_dir = TempDir::new().expect("temp dir");
    let temp_root = temp_dir.path().join("logs-identity-literals");
    let fixture = WorkspaceFixture::new();
    let repo_root = fixture.root();
    let binary = env!("CARGO_BIN_EXE_sc-lint");

    let output = sc_lint_command(binary, repo_root)
        .args([
            "--json",
            "--root",
            repo_root.to_str().expect("utf-8 repo root"),
            "--log-root",
            temp_root.to_str().expect("utf-8 temp path"),
            "lint",
            "identity-literals",
        ])
        .output()
        .expect("identity-literals command runs");
    assert!(
        output.status.success(),
        "identity-literals stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let expected = adapter_metadata_from_output(&output.stdout);

    let log_path = temp_root.join("sc-lint").join("sc-lint.log.jsonl");
    for action in ["cli.command.started", "cli.command.completed"] {
        assert_log_file_contains_field(&log_path, action, "adapter", &expected.adapter);
        assert_log_file_contains_field(&log_path, action, "config_scope", &expected.config_scope);
        assert_log_file_contains_field(&log_path, action, "script", &expected.script);
    }
}

#[test]
#[serial]
fn identity_literals_logs_adapter_metadata_for_error() {
    let fixture = WorkspaceFixture::new();
    let repo_root = fixture.root();
    let binary = env!("CARGO_BIN_EXE_sc-lint");
    let baseline = sc_lint_command(binary, repo_root)
        .args([
            "--json",
            "--root",
            repo_root.to_str().expect("utf-8 repo root"),
            "lint",
            "identity-literals",
        ])
        .output()
        .expect("baseline identity-literals command runs");
    assert!(
        baseline.status.success(),
        "baseline identity-literals stderr: {}",
        String::from_utf8_lossy(&baseline.stderr)
    );
    let expected = adapter_metadata_from_output(&baseline.stdout);
    let config_override =
        LintConfigOverride::new("[identities]\nforbidden_literals = \"invalid\"\n");
    let temp_dir = TempDir::new().expect("temp dir");
    let temp_root = temp_dir.path().join("logs-identity-literals-error");

    let output = sc_lint_command(binary, repo_root)
        .args([
            "--json",
            "--root",
            repo_root.to_str().expect("utf-8 repo root"),
            "--config",
            config_override.path().to_str().expect("utf-8 config path"),
            "--log-root",
            temp_root.to_str().expect("utf-8 temp path"),
            "lint",
            "identity-literals",
        ])
        .output()
        .expect("identity-literals error command runs");
    assert_eq!(
        output.status.code(),
        Some(3),
        "identity-literals stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    assert!(
        output.stdout.is_empty(),
        "expected json error on stderr, got stdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    let output_json = parse_command_output(&output.stderr);
    let log_path = temp_root.join("sc-lint").join("sc-lint.log.jsonl");
    assert_log_file_contains_field(&log_path, "cli.command.error", "adapter", &expected.adapter);
    assert_log_file_contains_field(
        &log_path,
        "cli.command.error",
        "config_scope",
        &expected.config_scope,
    );
    assert_log_file_contains_field(&log_path, "cli.command.error", "script", &expected.script);
    assert_eq!(
        output_json["error"]["kind"].as_str(),
        Some("config"),
        "identity-literals stdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );
}

#[test]
#[serial]
fn view_findings_logs_adapter_metadata_for_error() {
    let fixture = WorkspaceFixture::new();
    let repo_root = fixture.root();
    let binary = env!("CARGO_BIN_EXE_sc-lint");
    let baseline = sc_lint_command(binary, repo_root)
        .args([
            "--json",
            "--root",
            repo_root.to_str().expect("utf-8 repo root"),
            "view",
            "findings",
        ])
        .output()
        .expect("baseline view findings command runs");
    assert!(
        baseline.status.success(),
        "baseline view findings stderr: {}",
        String::from_utf8_lossy(&baseline.stderr)
    );
    let expected = adapter_metadata_from_output(&baseline.stdout);
    let temp_dir = TempDir::new().expect("temp dir");
    let temp_root = temp_dir.path().join("logs-view-findings-error");

    let output = sc_lint_command(binary, repo_root)
        .env("PATH", isolated_path(temp_dir.path()))
        .args([
            "--json",
            "--root",
            repo_root.to_str().expect("utf-8 repo root"),
            "--log-root",
            temp_root.to_str().expect("utf-8 temp path"),
            "view",
            "findings",
        ])
        .output()
        .expect("view findings error command runs");
    assert_eq!(
        output.status.code(),
        Some(5),
        "view findings stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    assert!(
        output.stdout.is_empty(),
        "expected json error on stderr, got stdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    let output_json = parse_command_output(&output.stderr);
    let log_path = temp_root.join("sc-lint").join("sc-lint.log.jsonl");
    assert_log_file_contains_field(&log_path, "cli.command.error", "adapter", &expected.adapter);
    assert_log_file_contains_field(
        &log_path,
        "cli.command.error",
        "config_scope",
        &expected.config_scope,
    );
    assert_log_file_contains_field(&log_path, "cli.command.error", "script", &expected.script);
    assert_eq!(
        output_json["error"]["kind"].as_str(),
        Some("backend_failure"),
        "view findings stdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );
}

#[test]
#[serial]
fn view_findings_logs_adapter_metadata_for_completion() {
    let temp_dir = TempDir::new().expect("temp dir");
    let temp_root = temp_dir.path().join("logs-view-findings");
    let fixture = WorkspaceFixture::new();
    let repo_root = fixture.root();
    let binary = env!("CARGO_BIN_EXE_sc-lint");

    let output = sc_lint_command(binary, repo_root)
        .args([
            "--json",
            "--root",
            repo_root.to_str().expect("utf-8 repo root"),
            "--log-root",
            temp_root.to_str().expect("utf-8 temp path"),
            "view",
            "findings",
        ])
        .output()
        .expect("view findings command runs");
    assert!(
        output.status.success(),
        "view findings stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let expected = adapter_metadata_from_output(&output.stdout);

    let log_path = temp_root.join("sc-lint").join("sc-lint.log.jsonl");
    for action in ["cli.command.started", "cli.command.completed"] {
        assert_log_file_contains_field(&log_path, action, "adapter", &expected.adapter);
        assert_log_file_contains_field(&log_path, action, "config_scope", &expected.config_scope);
        assert_log_file_contains_field(&log_path, action, "script", &expected.script);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AdapterMetadata {
    adapter: String,
    config_scope: String,
    script: String,
}

fn adapter_metadata_from_output(output: &[u8]) -> AdapterMetadata {
    let payload = parse_command_output(output);
    let details = payload["data"].as_object().expect("data object");
    AdapterMetadata {
        adapter: details["adapter"]
            .as_str()
            .expect("adapter field")
            .to_string(),
        config_scope: details["config_scope"]
            .as_str()
            .expect("config_scope field")
            .to_string(),
        script: details["script"]
            .as_str()
            .expect("script field")
            .to_string(),
    }
}

fn parse_command_output(output: &[u8]) -> Value {
    serde_json::from_slice(output).expect("command writes json")
}

struct LintConfigOverride {
    dir: TempDir,
    path: PathBuf,
}

impl LintConfigOverride {
    fn new(contents: &str) -> Self {
        let dir = TempDir::new().expect("temp dir");
        let path = dir.path().join("lint-config.toml");
        std::fs::write(&path, contents).expect("write lint config override");
        Self { dir, path }
    }

    fn path(&self) -> &Path {
        let _keepalive = self.dir.path();
        &self.path
    }
}

struct WorkspaceFixture {
    temp_dir: TempDir,
}

impl WorkspaceFixture {
    fn new() -> Self {
        let fixture = Self {
            temp_dir: TempDir::new().expect("temp dir"),
        };
        fixture.write(
            "Cargo.toml",
            r#"
                [workspace]
                members = ["crates/example"]
                resolver = "2"

                [workspace.package]
                version = "0.1.0"
                edition = "2024"
                rust-version = "1.94.1"
                authors = ["sc-lint contributors"]
                license = "MIT OR Apache-2.0"
                repository = "https://example.invalid/sc-lint"
                homepage = "https://example.invalid/sc-lint"
            "#,
        );
        std::fs::create_dir_all(fixture.root().join("boundaries")).expect("create boundaries dir");
        fixture.write(
            "boundaries/planning.toml",
            r#"
                [planning]
                current_sprint = "A.7"
            "#,
        );
        fixture.write(
            "crates/example/Cargo.toml",
            r#"
                [package]
                name = "example"
                version.workspace = true
                edition.workspace = true
                rust-version.workspace = true
                authors.workspace = true
                license.workspace = true
                repository.workspace = true
                homepage.workspace = true
            "#,
        );
        fixture.write(
            "crates/example/src/lib.rs",
            r#"
                pub struct Example;

                impl Example {
                    pub fn id(&self) -> &'static str {
                        "example"
                    }
                }
            "#,
        );
        for relative_path in [
            ".just/lint-config.toml",
            ".just/lint_common.py",
            ".just/python_adapter.py",
            ".just/view_common.py",
            ".just/lint_line_counts.py",
            ".just/lint_identity_literals.py",
            ".just/view_findings.py",
        ] {
            let source = repo_source_root().join(relative_path);
            let contents = std::fs::read_to_string(&source).expect("read support file");
            fixture.write(relative_path, &contents);
        }
        fixture
    }

    fn root(&self) -> &Path {
        self.temp_dir.path()
    }

    fn write(&self, relative_path: &str, contents: &str) {
        let path = self.root().join(relative_path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("create fixture parent");
        }
        std::fs::write(path, contents).expect("write fixture file");
    }
}

fn sc_lint_command(binary: &str, current_dir: &Path) -> Command {
    let mut command = Command::new(binary);
    command.current_dir(current_dir);
    clear_atm_env(&mut command);
    command
}

fn clear_atm_env(command: &mut Command) {
    for (key, _) in std::env::vars_os() {
        if key.to_string_lossy().starts_with("ATM_") {
            command.env_remove(key);
        }
    }
    let atm_root = isolated_atm_root();
    command.env("ATM_HOME", atm_root.join("home"));
    command.env("ATM_CONFIG_HOME", atm_root.join("config-home"));
}

fn isolated_atm_root() -> &'static PathBuf {
    static ROOT: OnceLock<PathBuf> = OnceLock::new();
    ROOT.get_or_init(|| {
        let root = TempDir::new().expect("temp dir").keep();
        std::fs::create_dir_all(root.join("home")).expect("create ATM_HOME");
        std::fs::create_dir_all(root.join("config-home")).expect("create ATM_CONFIG_HOME");
        root
    })
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
                // Do not add a minimum elapsed-ms assertion here. Fast hosts
                // can legitimately record 0ms for short commands.
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

fn prepend_path(
    wrapper_path: &Path,
    original_path: Option<&std::ffi::OsStr>,
) -> std::ffi::OsString {
    let bin_dir = wrapper_path.parent().expect("wrapper parent");
    let mut parts = vec![bin_dir.as_os_str().to_os_string()];
    if let Some(existing) = original_path {
        parts.extend(std::env::split_paths(&existing).map(|path| path.into_os_string()));
    }
    std::env::join_paths(parts).expect("join path entries")
}

fn isolated_path(path: &Path) -> std::ffi::OsString {
    std::env::join_paths([path]).expect("join isolated path")
}

fn repo_source_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root")
        .to_path_buf()
}
