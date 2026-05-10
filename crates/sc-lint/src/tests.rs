use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;

use clap::Parser;
use serde_json::Value;
use serial_test::serial;
use tempfile::TempDir;

use crate::CheckTarget;
use crate::Cli;
use crate::CliError;
use crate::CliErrorKind;
use crate::ClippyTarget;
use crate::Command;
use crate::CommandEnvelope;
use crate::LintTarget;
use crate::ParsedInvocation;
use crate::ViewTarget;
use crate::cli::OutputMode;
use crate::command::CommandContext;
use crate::config::LoadedConfig;
use crate::workflow;

#[test]
fn command_surface_parses_the_initial_grouped_shape() {
    let cli = Cli::parse_from(["sc-lint", "lint", "sc-boundary"]);
    assert!(matches!(
        cli.command,
        Command::Lint {
            target: LintTarget::ScBoundary
        }
    ));

    let cli = Cli::parse_from(["sc-lint", "view", "graph"]);
    assert!(matches!(
        cli.command,
        Command::View {
            target: ViewTarget::Graph
        }
    ));

    let cli = Cli::parse_from(["sc-lint", "lint", "line-counts"]);
    assert!(matches!(
        cli.command,
        Command::Lint {
            target: LintTarget::LineCounts
        }
    ));

    let cli = Cli::parse_from(["sc-lint", "view", "findings"]);
    assert!(matches!(
        cli.command,
        Command::View {
            target: ViewTarget::Findings
        }
    ));

    let cli = Cli::parse_from(["sc-lint", "check", "xwin"]);
    assert!(matches!(
        cli.command,
        Command::Check {
            target: CheckTarget::Xwin
        }
    ));

    let cli = Cli::parse_from(["sc-lint", "clippy", "native"]);
    assert!(matches!(
        cli.command,
        Command::Clippy {
            target: ClippyTarget::Native
        }
    ));
}

#[test]
fn help_text_exposes_the_initial_grouped_surface() {
    let help = crate::help_text();

    for command in ["lint", "view", "check", "clippy", "version", "ci", "--json"] {
        assert!(help.contains(command), "missing `{command}` in help output");
    }
}

#[test]
fn version_success_uses_the_canonical_top_level_envelope() {
    let cli = Cli::parse_from(["sc-lint", "--json", "version"]);
    let context = CommandContext::from_cli(&cli).expect("version context");
    let loaded = LoadedConfig::load(&cli, &context).expect("config loads");
    let success = crate::command::execute(&context, &loaded).expect("version command succeeds");
    let envelope = CommandEnvelope::success(context.command_id(), success.data);
    let rendered = crate::render::render_success_json(&envelope);
    let json: Value = serde_json::from_str(&rendered).expect("rendered envelope is json");

    assert_eq!(json["ok"], true);
    assert_eq!(json["command"], "version");
    assert_eq!(json["data"]["crate_name"], "sc-lint");
    assert_eq!(json["data"]["contract_schema"], "v1");
    assert!(json["diagnostics"].as_array().is_some());
}

#[test]
fn reserved_view_commands_use_the_same_failure_envelope_shape() {
    let commands = [Cli::parse_from(["sc-lint", "--json", "view", "graph"])];

    for cli in commands {
        let context = CommandContext::from_cli(&cli).expect("view context");
        let loaded = LoadedConfig::load(&cli, &context).expect("config loads");
        let error =
            crate::command::execute(&context, &loaded).expect_err("view commands are reserved");
        let rendered = crate::render::render_error_json(context.command_id(), &error);
        let json: Value = serde_json::from_str(&rendered).expect("rendered envelope is json");

        assert_eq!(json["ok"], false);
        assert_eq!(json["command"], context.command_id());
        assert!(json["data"].is_null());
        assert_eq!(json["error"]["kind"], "capability");
        assert_eq!(json["error"]["code"], "CLI.CAPABILITY_ERROR");
        assert!(json["diagnostics"].as_array().is_some());
    }
}

#[test]
fn parse_errors_use_the_documented_command_identifier() {
    let ParsedInvocation::Immediate(outcome) =
        crate::parse_args(["sc-lint", "--json", "unknown-command"])
    else {
        panic!("invalid command should stop at parse time");
    };
    let rendered = outcome.rendered.stderr.expect("parse error emits stderr");
    let json: Value = serde_json::from_str(&rendered).expect("rendered parse error is json");

    assert_eq!(json["command"], "cli.parse_error");
    assert_eq!(json["error"]["code"], "CLI.USAGE_ERROR");
}

#[test]
fn version_failure_uses_the_canonical_top_level_envelope() {
    let error = CliError::internal("version rendering failure");
    let rendered = crate::render::render_error_json("version", &error);
    let json: Value = serde_json::from_str(&rendered).expect("rendered envelope is json");

    assert_eq!(json["ok"], false);
    assert_eq!(json["command"], "version");
    assert_eq!(json["error"]["code"], "CLI.INTERNAL_ERROR");
}

#[test]
fn cli_error_exit_codes_are_stable_by_kind() {
    assert_eq!(CliError::usage("bad args").exit_code(), 2);
    assert_eq!(CliError::config("bad config").exit_code(), 3);
    assert_eq!(CliError::capability("missing capability").exit_code(), 4);
    assert_eq!(CliError::backend_failure("backend failed").exit_code(), 5);
    assert_eq!(
        CliError::backend_protocol("backend malformed").exit_code(),
        6
    );
    assert_eq!(CliError::internal("bug").exit_code(), 1);
}

#[test]
fn output_mode_tracks_json_flag_and_serializes() {
    assert_eq!(OutputMode::from_json_flag(false), OutputMode::Human);
    assert_eq!(OutputMode::from_json_flag(true), OutputMode::Json);
    assert_eq!(
        serde_json::to_string(&OutputMode::Json).expect("serialize output mode"),
        "\"json\""
    );
}

#[test]
fn lint_targets_map_profile_values_stably() {
    assert_eq!(
        LintTarget::Fast.profile(),
        Some(crate::cli::LintProfile::Fast)
    );
    assert_eq!(
        LintTarget::Full.profile(),
        Some(crate::cli::LintProfile::Full)
    );
    assert_eq!(LintTarget::Ci.profile(), Some(crate::cli::LintProfile::Ci));
    assert_eq!(LintTarget::ScBoundary.profile(), None);
    assert_eq!(LintTarget::LineCounts.profile(), None);
    assert_eq!(crate::cli::LintProfile::Fast.command_suffix(), "fast");
    assert_eq!(crate::cli::LintProfile::Full.command_suffix(), "full");
    assert_eq!(crate::cli::LintProfile::Ci.command_suffix(), "ci");
}

#[test]
fn repo_root_discovery_walks_up_to_the_workspace_root() {
    let repo_root = project_workspace_root();
    let cli = Cli::parse_from([
        "sc-lint",
        "--root",
        repo_root
            .join("crates/sc-lint/src")
            .to_str()
            .expect("repo path"),
        "lint",
        "sc-boundary",
    ]);
    let context = CommandContext::from_cli(&cli).expect("repo-root context");
    let loaded = LoadedConfig::load(&cli, &context).expect("config loads");
    let root = loaded.require_repo_root().expect("repo root");

    assert!(root.join("boundaries").is_dir());
    assert!(root.join("Cargo.toml").is_file());
}

#[test]
fn malformed_repo_config_returns_cli_config_error() {
    let temp_dir = TempDir::new().expect("temp dir");
    std::fs::write(
        temp_dir.path().join("Cargo.toml"),
        "[workspace]\nmembers=[]\nresolver=\"2\"\n",
    )
    .expect("write manifest");
    std::fs::create_dir_all(temp_dir.path().join("boundaries")).expect("write boundaries dir");
    std::fs::write(temp_dir.path().join("sc-lint.toml"), "logging = [").expect("write config");

    let cli = Cli::parse_from([
        "sc-lint",
        "--root",
        temp_dir.path().to_str().expect("temp path"),
        "lint",
        "sc-boundary",
    ]);
    let context = CommandContext::from_cli(&cli).expect("config-error context");
    let error = LoadedConfig::load(&cli, &context).expect_err("config should fail");

    assert_eq!(error.kind, CliErrorKind::Config);
    assert!(error.message.contains("failed to parse repo config"));
    assert!(error.cause.is_some());
}

#[test]
fn lint_sc_boundary_normalizes_backend_success_through_the_top_level_envelope() {
    let fixture = AnalysisFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_source("example", "lib.rs", "pub struct Example;\n");
    let repo_root = fixture.root();
    let cli = Cli::parse_from([
        "sc-lint",
        "--json",
        "--root",
        repo_root.to_str().expect("repo root"),
        "lint",
        "sc-boundary",
    ]);
    let context = CommandContext::from_cli(&cli).expect("boundary context");
    let loaded = LoadedConfig::load(&cli, &context).expect("config loads");
    let success = crate::command::execute(&context, &loaded).expect("dispatch succeeds");
    let expected_finding_count = success
        .data
        .get("findings")
        .and_then(Value::as_array)
        .map_or(0, std::vec::Vec::len);
    assert_eq!(
        success
            .dispatch
            .as_ref()
            .expect("dispatch telemetry")
            .finding_count(),
        expected_finding_count
    );
    let envelope = CommandEnvelope::success(context.command_id(), success.data);
    let rendered = crate::render::render_success_json(&envelope);
    let json: Value = serde_json::from_str(&rendered).expect("success json");

    assert_eq!(json["ok"], true);
    assert_eq!(json["command"], "lint.sc-boundary");
    assert_eq!(json["data"]["tool"], crate::consts::TOOL_BOUNDARY);
    assert!(json["data"]["findings"].is_array());
}

#[test]
fn lint_sc_portability_normalizes_backend_success_through_the_top_level_envelope() {
    let fixture = AnalysisFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_source("example", "lib.rs", "pub fn portable() {}\n");
    let repo_root = fixture.root();
    let cli = Cli::parse_from([
        "sc-lint",
        "--json",
        "--root",
        repo_root.to_str().expect("repo root"),
        "lint",
        "sc-portability",
    ]);
    let context = CommandContext::from_cli(&cli).expect("portability context");
    let loaded = LoadedConfig::load(&cli, &context).expect("config loads");
    let success = crate::command::execute(&context, &loaded).expect("dispatch succeeds");
    let expected_finding_count = success
        .data
        .get("findings")
        .and_then(Value::as_array)
        .map_or(0, std::vec::Vec::len);
    assert_eq!(
        success
            .dispatch
            .as_ref()
            .expect("dispatch telemetry")
            .finding_count(),
        expected_finding_count
    );
    let envelope = CommandEnvelope::success(context.command_id(), success.data);
    let rendered = crate::render::render_success_json(&envelope);
    let json: Value = serde_json::from_str(&rendered).expect("success json");

    assert_eq!(json["ok"], true);
    assert_eq!(json["command"], "lint.sc-portability");
    assert_eq!(json["data"]["tool"], "sc-lint-portability");
    assert!(json["data"]["findings"].is_array());
}

#[test]
fn lint_sc_runtime_normalizes_backend_success_through_the_top_level_envelope() {
    let fixture = AnalysisFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
            use std::sync::{Condvar, Mutex};
            use std::time::Duration;

            pub fn inspected(condvar: &Condvar, state: &Mutex<bool>) {
                let state = state.lock().expect("lock");
                let (_guard, wait) = condvar
                    .wait_timeout(state, Duration::from_secs(1))
                    .expect("wait");
                if wait.timed_out() {
                    return;
                }
            }
        "#,
    );
    let repo_root = fixture.root();
    let cli = Cli::parse_from([
        "sc-lint",
        "--json",
        "--root",
        repo_root.to_str().expect("repo root"),
        "lint",
        "sc-runtime",
    ]);
    let context = CommandContext::from_cli(&cli).expect("runtime context");
    let loaded = LoadedConfig::load(&cli, &context).expect("config loads");
    let success = crate::command::execute(&context, &loaded).expect("dispatch succeeds");
    let expected_finding_count = success
        .data
        .get("findings")
        .and_then(Value::as_array)
        .map_or(0, std::vec::Vec::len);
    assert_eq!(
        success
            .dispatch
            .as_ref()
            .expect("dispatch telemetry")
            .finding_count(),
        expected_finding_count
    );
    let envelope = CommandEnvelope::success(context.command_id(), success.data);
    let rendered = crate::render::render_success_json(&envelope);
    let json: Value = serde_json::from_str(&rendered).expect("success json");

    assert_eq!(json["ok"], true);
    assert_eq!(json["command"], "lint.sc-runtime");
    assert_eq!(json["data"]["tool"], crate::consts::TOOL_RUNTIME);
    assert!(json["data"]["findings"].is_array());
}

#[test]
fn malformed_backend_json_maps_to_backend_protocol_error() {
    let error = crate::dispatch::normalize_backend_json(crate::consts::TOOL_BOUNDARY, "{not-json")
        .expect_err("normalization should fail");

    assert_eq!(error.kind, CliErrorKind::BackendProtocol);
    assert_eq!(error.code(), "CLI.BACKEND_PROTOCOL_ERROR");
    assert!(error.cause.is_some());
    assert!(std::error::Error::source(&error).is_some());
}

#[test]
fn backend_execution_failure_maps_to_backend_failure_error() {
    let temp_dir = TempDir::new().expect("temp dir");
    std::fs::write(
        temp_dir.path().join("Cargo.toml"),
        "[workspace]\nmembers=[]\nresolver=\"2\"\n",
    )
    .expect("write manifest");
    std::fs::create_dir_all(temp_dir.path().join("boundaries")).expect("write boundaries dir");
    std::fs::create_dir_all(temp_dir.path().join("empty")).expect("empty dir");

    let cli = Cli::parse_from([
        "sc-lint",
        "--root",
        temp_dir.path().join("empty").to_str().expect("empty path"),
        "lint",
        "sc-boundary",
    ]);
    let context = CommandContext::from_cli(&cli).expect("dispatch failure context");
    let loaded = LoadedConfig::load(&cli, &context).expect("config loads");
    let error = crate::command::execute(&context, &loaded).expect_err("dispatch should fail");

    assert_eq!(error.kind, CliErrorKind::BackendFailure);
    assert_eq!(error.code(), "CLI.BACKEND_EXEC_FAILURE");
    assert!(error.cause.is_some());
    assert!(std::error::Error::source(&error).is_some());
}

#[test]
fn loaded_config_preserves_repo_root_as_a_validated_newtype() {
    let repo_root = project_workspace_root();
    let cli = Cli::parse_from([
        "sc-lint",
        "--root",
        repo_root.to_str().expect("repo root"),
        "lint",
        "sc-boundary",
    ]);
    let context = CommandContext::from_cli(&cli).expect("loaded-config context");
    let loaded = LoadedConfig::load(&cli, &context).expect("config loads");

    assert_eq!(
        loaded.require_repo_root().expect("repo root"),
        repo_root.as_path()
    );
}

#[test]
#[serial]
fn python_backed_lints_and_views_normalize_through_the_top_level_envelope() {
    let repo_root = project_workspace_root();
    for (args, command_id) in [
        (
            [
                "sc-lint",
                "--json",
                "--root",
                repo_root.to_str().expect("repo root"),
                "lint",
                "line-counts",
            ],
            "lint.line-counts",
        ),
        (
            [
                "sc-lint",
                "--json",
                "--root",
                repo_root.to_str().expect("repo root"),
                "lint",
                "identity-literals",
            ],
            "lint.identity-literals",
        ),
        (
            [
                "sc-lint",
                "--json",
                "--root",
                repo_root.to_str().expect("repo root"),
                "view",
                "findings",
            ],
            "view.findings",
        ),
    ] {
        let cli = Cli::parse_from(args);
        let context = CommandContext::from_cli(&cli).expect("python-backed context");
        let loaded = LoadedConfig::load(&cli, &context).expect("config loads");
        let success =
            crate::command::execute(&context, &loaded).expect("python-backed command succeeds");
        let envelope = CommandEnvelope::success(context.command_id(), success.data);
        let rendered = crate::render::render_success_json(&envelope);
        let json: Value = serde_json::from_str(&rendered).expect("success json");

        assert_eq!(json["ok"], true);
        assert_eq!(json["command"], command_id);
        assert_eq!(
            json["data"]["adapter"],
            crate::python_adapter::ADAPTER_SCHEMA
        );
    }
}

#[test]
fn lint_profiles_have_stable_membership() {
    let repo_root = project_workspace_root();
    let cli = Cli::parse_from([
        "sc-lint",
        "--root",
        repo_root.to_str().expect("repo root"),
        "lint",
        "fast",
    ]);
    let context = CommandContext::from_cli(&cli).expect("lint fast context");
    let loaded = LoadedConfig::load(&cli, &context).expect("config loads");
    let adapter = FakeSystemAdapter::new(false);

    let success = workflow::run_lint_profile_with(&loaded, crate::cli::LintProfile::Fast, &adapter)
        .expect("fast profile succeeds");
    let steps = success
        .data
        .get("steps")
        .and_then(Value::as_array)
        .expect("steps array");

    assert_eq!(
        step_names(steps),
        vec!["fmt", "version", "manifests", "spell", "pytests"]
    );
}

#[test]
fn full_profile_adds_xwin_only_when_available() {
    let repo_root = project_workspace_root();
    let cli = Cli::parse_from([
        "sc-lint",
        "--root",
        repo_root.to_str().expect("repo root"),
        "lint",
        "full",
    ]);
    let context = CommandContext::from_cli(&cli).expect("lint full context");
    let loaded = LoadedConfig::load(&cli, &context).expect("config loads");

    let unavailable = workflow::run_lint_profile_with(
        &loaded,
        crate::cli::LintProfile::Full,
        &FakeSystemAdapter::new(false),
    )
    .expect("full profile succeeds without xwin");
    let unavailable_steps = unavailable
        .data
        .get("steps")
        .and_then(Value::as_array)
        .expect("steps array");
    assert!(!step_names(unavailable_steps).contains(&"check.xwin".to_string()));
    assert_eq!(unavailable.data["xwin"]["included"], false);

    let available = workflow::run_lint_profile_with(
        &loaded,
        crate::cli::LintProfile::Full,
        &FakeSystemAdapter::new(true),
    )
    .expect("full profile succeeds with xwin");
    let available_steps = available
        .data
        .get("steps")
        .and_then(Value::as_array)
        .expect("steps array");
    assert!(step_names(available_steps).contains(&"check.xwin".to_string()));
    assert!(step_names(available_steps).contains(&"clippy.xwin".to_string()));
    assert_eq!(available.data["xwin"]["included"], true);
}

#[test]
fn ci_and_lint_ci_differ_only_by_test_execution() {
    let repo_root = project_workspace_root();
    let lint_cli = Cli::parse_from([
        "sc-lint",
        "--root",
        repo_root.to_str().expect("repo root"),
        "lint",
        "ci",
    ]);
    let lint_context = CommandContext::from_cli(&lint_cli).expect("lint ci context");
    let loaded = LoadedConfig::load(&lint_cli, &lint_context).expect("config loads");
    let adapter = FakeSystemAdapter::new(false);

    let lint_ci = workflow::run_lint_profile_with(&loaded, crate::cli::LintProfile::Ci, &adapter)
        .expect("lint ci succeeds");
    let top_level_ci = workflow::run_ci_with(&loaded, &adapter).expect("ci succeeds");
    let lint_steps = lint_ci
        .data
        .get("steps")
        .and_then(Value::as_array)
        .expect("steps array");
    let ci_steps = top_level_ci
        .data
        .get("steps")
        .and_then(Value::as_array)
        .expect("steps array");

    assert_eq!(ci_steps.len(), lint_steps.len() + 1);
    assert_eq!(
        step_names(lint_steps),
        step_names(&ci_steps[..lint_steps.len()])
    );
    assert_eq!(ci_steps.last().expect("test step")["name"], "test");
    assert_eq!(top_level_ci.data["tests_included"], true);
}

#[test]
fn explicit_xwin_commands_require_capability() {
    let repo_root = project_workspace_root();
    let cli = Cli::parse_from([
        "sc-lint",
        "--root",
        repo_root.to_str().expect("repo root"),
        "check",
        "xwin",
    ]);
    let context = CommandContext::from_cli(&cli).expect("xwin check context");
    let loaded = LoadedConfig::load(&cli, &context).expect("config loads");
    let error =
        workflow::run_check_with(&loaded, CheckTarget::Xwin, &FakeSystemAdapter::new(false))
            .expect_err("xwin command should require capability");

    assert_eq!(error.kind, CliErrorKind::Capability);
    assert_eq!(error.details["command"], "check.xwin");
    assert_eq!(error.details["target"], crate::WINDOWS_XWIN_TARGET);
}

#[test]
fn native_and_xwin_preflight_commands_use_success_envelopes() {
    let repo_root = project_workspace_root();
    let cli = Cli::parse_from([
        "sc-lint",
        "--json",
        "--root",
        repo_root.to_str().expect("repo root"),
        "clippy",
        "xwin",
    ]);
    let context = CommandContext::from_cli(&cli).expect("xwin clippy context");
    let loaded = LoadedConfig::load(&cli, &context).expect("config loads");
    let success =
        workflow::run_clippy_with(&loaded, ClippyTarget::Xwin, &FakeSystemAdapter::new(true))
            .expect("xwin clippy succeeds");
    let envelope = CommandEnvelope::success(context.command_id(), success.data);
    let rendered = crate::render::render_success_json(&envelope);
    let json: Value = serde_json::from_str(&rendered).expect("json envelope");

    assert_eq!(json["ok"], true);
    assert_eq!(json["command"], "clippy.xwin");
    assert_eq!(json["data"]["xwin"]["target"], crate::WINDOWS_XWIN_TARGET);
}

fn project_workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root")
        .to_path_buf()
}

struct AnalysisFixture {
    tempdir: TempDir,
}

impl AnalysisFixture {
    fn new() -> Self {
        Self {
            tempdir: TempDir::new().expect("temp dir"),
        }
    }

    fn root(&self) -> &Path {
        self.tempdir.path()
    }

    fn write_workspace_root(&self) {
        std::fs::write(
            self.root().join("Cargo.toml"),
            "[workspace]\nmembers = [\"crates/example\"]\nresolver = \"2\"\n",
        )
        .expect("workspace root");
        std::fs::create_dir_all(self.root().join("boundaries")).expect("boundaries");
    }

    fn write_package_manifest(&self, package_name: &str) {
        self.write(
            &format!("crates/{package_name}/Cargo.toml"),
            &format!(
                "[package]\nname = \"{package_name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n"
            ),
        );
    }

    fn write_source(&self, package_name: &str, relative_path: &str, contents: &str) {
        self.write(
            &format!("crates/{package_name}/src/{relative_path}"),
            contents,
        );
    }

    fn write(&self, relative_path: &str, contents: &str) {
        let path = self.root().join(relative_path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("parent dirs");
        }
        std::fs::write(path, contents).expect("write fixture file");
    }
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

struct FakeSystemAdapter {
    xwin_available: bool,
    failures: HashMap<&'static str, &'static str>,
    invocations: RefCell<Vec<String>>,
}

impl FakeSystemAdapter {
    fn new(xwin_available: bool) -> Self {
        Self {
            xwin_available,
            failures: HashMap::new(),
            // Tests need to observe step order without requiring Sync, so
            // RefCell is sufficient for this single-threaded fake.
            invocations: RefCell::new(Vec::new()),
        }
    }
}

impl workflow::SystemAdapter for FakeSystemAdapter {
    fn cargo_xwin_available(&self, _repo_root: &Path) -> bool {
        self.xwin_available
    }

    fn run_step(
        &self,
        _repo_root: &Path,
        step: &workflow::StepPlan,
    ) -> Result<workflow::StepReport, CliError> {
        self.invocations.borrow_mut().push(step.name().to_string());

        if let Some(message) = self.failures.get(step.name()) {
            return Err(CliError::backend_failure(*message));
        }

        Ok(workflow::StepReport::success(step))
    }
}
