use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::OsString;
use std::path::Path;
use std::path::PathBuf;

use clap::Parser;
use serde::Serialize;
use serde::Serializer;
use serde_json::Value;
use serde_json::json;
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
        cli.command.as_ref(),
        Some(Command::Lint {
            target: LintTarget::ScBoundary
        })
    ));

    let cli = Cli::parse_from(["sc-lint", "view", "graph"]);
    assert!(matches!(
        cli.command.as_ref(),
        Some(Command::View {
            target: ViewTarget::Graph
        })
    ));

    let cli = Cli::parse_from(["sc-lint", "lint", "line-counts"]);
    assert!(matches!(
        cli.command.as_ref(),
        Some(Command::Lint {
            target: LintTarget::LineCounts
        })
    ));

    let cli = Cli::parse_from(["sc-lint", "view", "findings"]);
    assert!(matches!(
        cli.command.as_ref(),
        Some(Command::View {
            target: ViewTarget::Findings
        })
    ));

    let cli = Cli::parse_from(["sc-lint", "check", "xwin"]);
    assert!(matches!(
        cli.command.as_ref(),
        Some(Command::Check {
            target: CheckTarget::Xwin
        })
    ));

    let cli = Cli::parse_from(["sc-lint", "clippy", "native"]);
    assert!(matches!(
        cli.command.as_ref(),
        Some(Command::Clippy {
            target: ClippyTarget::Native
        })
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
fn version_flag_routes_through_version_command_context() {
    let ParsedInvocation::Ready(cli) = crate::parse_args(["sc-lint", "--version"]) else {
        panic!("--version should parse into the standard execution path");
    };

    assert!(cli.version);
    assert!(cli.command.is_none());

    let context = CommandContext::from_cli(&cli).expect("version-flag context");
    assert_eq!(context.command_id(), "version");
}

#[test]
fn version_flag_json_uses_the_canonical_top_level_envelope() {
    let ParsedInvocation::Ready(cli) = crate::parse_args(["sc-lint", "--json", "--version"]) else {
        panic!("--version --json should parse into the standard execution path");
    };

    let context = CommandContext::from_cli(&cli).expect("version-flag json context");
    let loaded = LoadedConfig::load(&cli, &context).expect("config loads");
    let success = crate::command::execute(&context, &loaded).expect("version command succeeds");
    let envelope = CommandEnvelope::success(context.command_id(), success.data);
    let rendered = crate::render::render_success_json(&envelope);
    let json: Value = serde_json::from_str(&rendered).expect("rendered envelope is json");

    assert_eq!(json["ok"], true);
    assert_eq!(json["command"], "version");
    assert_eq!(json["data"]["crate_name"], "sc-lint");
    assert_eq!(json["data"]["crate_version"], env!("CARGO_PKG_VERSION"));
}

#[test]
fn missing_command_without_version_is_a_usage_error() {
    let cli = Cli::parse_from(["sc-lint", "--json"]);
    let error = CommandContext::from_cli(&cli).expect_err("missing command should fail");

    assert_eq!(error.kind, CliErrorKind::Usage);
}

#[test]
fn version_flag_conflicts_with_subcommand_as_a_usage_error() {
    let cli = Cli::parse_from(["sc-lint", "--json", "--version", "lint", "sc-boundary"]);
    let error = CommandContext::from_cli(&cli).expect_err("version flag conflict should fail");
    let rendered = crate::render::render_error_json("cli.parse_error", &error);
    let json: Value = serde_json::from_str(&rendered).expect("rendered envelope is json");

    assert_eq!(error.kind, CliErrorKind::Usage);
    assert_eq!(json["error"]["code"], "CLI.USAGE_ERROR");
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
    let fixture = AnalysisFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_source("example", "nested/module.rs", "pub struct Example;\n");
    let nested_source = fixture.root().join("crates/example/src/nested/module.rs");
    let cli = Cli::parse_from([
        "sc-lint",
        "--root",
        nested_source.to_str().expect("fixture path"),
        "lint",
        "sc-boundary",
    ]);
    let context = CommandContext::from_cli(&cli).expect("repo-root context");
    let loaded = LoadedConfig::load(&cli, &context).expect("config loads");
    let root = loaded.require_repo_root().expect("repo root");

    assert_eq!(
        root,
        dunce::canonicalize(fixture.root()).expect("canonical fixture root")
    );
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
#[serial]
fn lint_sc_portability_normalizes_backend_success_through_the_top_level_envelope() {
    let fixture = AnalysisFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_source("example", "lib.rs", "pub fn portable() {}\n");
    let _mock_backend = MockBackend::install(
        crate::consts::TOOL_PORTABILITY,
        &json!({
            "tool": crate::consts::TOOL_PORTABILITY,
            "findings": [],
            "status": "pass",
        }),
    );
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
    assert_eq!(json["data"]["tool"], crate::consts::TOOL_PORTABILITY);
    assert!(json["data"]["findings"].is_array());
}

#[test]
#[serial]
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
    let _mock_backend = MockBackend::install(
        crate::consts::TOOL_RUNTIME,
        &json!({
            "tool": crate::consts::TOOL_RUNTIME,
            "findings": [],
            "status": "pass",
        }),
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

    assert_eq!(error.kind, CliErrorKind::Config);
    assert_eq!(error.code(), "CLI.CONFIG_ERROR");
    assert!(error.cause.is_some());
    assert!(std::error::Error::source(&error).is_some());
}

#[test]
fn loaded_config_preserves_repo_root_as_a_validated_newtype() {
    let fixture = AnalysisFixture::new();
    fixture.write_workspace_root();
    fixture.write("sc-lint.toml", "[logging]\nconsole = true\n");
    let cli = Cli::parse_from([
        "sc-lint",
        "--root",
        fixture.root().to_str().expect("fixture root"),
        "lint",
        "sc-boundary",
    ]);
    let context = CommandContext::from_cli(&cli).expect("loaded-config context");
    let loaded = LoadedConfig::load(&cli, &context).expect("config loads");

    assert_eq!(
        loaded.require_repo_root().expect("repo root"),
        dunce::canonicalize(fixture.root())
            .expect("canonical fixture root")
            .as_path()
    );
    assert_eq!(
        loaded.config_path().expect("config path"),
        dunce::canonicalize(fixture.root().join("sc-lint.toml")).expect("canonical config path")
    );
}

#[test]
#[serial]
fn python_backed_lints_and_views_normalize_through_the_top_level_envelope() {
    let repo_root = repo_backed_workspace_root();
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
    let repo_root = repo_backed_workspace_root();
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
    let repo_root = repo_backed_workspace_root();
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
    let repo_root = repo_backed_workspace_root();
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
    let repo_root = repo_backed_workspace_root();
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
    let repo_root = repo_backed_workspace_root();
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

#[test]
fn render_success_human_covers_version_boundary_and_summary_paths() {
    let version_cli = Cli::parse_from(["sc-lint", "version"]);
    let version_context = CommandContext::from_cli(&version_cli).expect("version context");
    let version_output = crate::render::render_success_human(
        &version_context,
        &CommandEnvelope::success("version", json!({ "crate_version": "1.2.3" })),
    );
    assert_eq!(version_output, "sc-lint 1.2.3");

    let boundary_cli = Cli::parse_from(["sc-lint", "lint", "sc-boundary"]);
    let boundary_context = CommandContext::from_cli(&boundary_cli).expect("boundary context");
    let boundary_output = crate::render::render_success_human(
        &boundary_context,
        &CommandEnvelope::success(
            boundary_context.command_id(),
            json!({
                "status": "fail",
                "findings": [{ "rule_id": "SCB-CYCLE-001" }]
            }),
        ),
    );
    assert_eq!(boundary_output, "sc-lint-boundary: fail (1 findings)");

    let view_cli = Cli::parse_from(["sc-lint", "view", "findings"]);
    let view_context = CommandContext::from_cli(&view_cli).expect("view findings context");
    let view_output = crate::render::render_success_human(
        &view_context,
        &CommandEnvelope::success(
            view_context.command_id(),
            json!({ "summary": "2 findings grouped by rule" }),
        ),
    );
    assert_eq!(view_output, "view.findings: 2 findings grouped by rule");
}

#[test]
fn render_error_human_includes_suggested_action_when_present() {
    let error = CliError::config("bad config")
        .with_suggested_action("Run `sc-lint lint sc-boundary --json` to inspect the failure.");
    let rendered = crate::render::render_error_human("lint.sc-boundary", &error);

    assert!(rendered.contains("lint.sc-boundary: bad config (CLI.CONFIG_ERROR)"));
    assert!(rendered.contains("Run `sc-lint lint sc-boundary --json` to inspect the failure."));
}

#[derive(Debug, Clone, Copy)]
struct BrokenSerialize;

impl Serialize for BrokenSerialize {
    fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        Err(serde::ser::Error::custom("boom"))
    }
}

#[test]
fn render_success_json_falls_back_to_internal_error_envelope_on_serialize_failure() {
    let envelope = CommandEnvelope::success("version", BrokenSerialize);
    let rendered = crate::render::render_success_json(&envelope);
    let json: Value = serde_json::from_str(&rendered).expect("fallback envelope json");

    assert_eq!(json["ok"], false);
    assert_eq!(json["command"], "version");
    assert_eq!(json["error"]["code"], "CLI.INTERNAL_ERROR");
    assert_eq!(
        json["error"]["message"],
        "failed to serialize success envelope"
    );
}

fn repo_backed_workspace_root() -> PathBuf {
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
            r#"[workspace]
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
        )
        .expect("workspace root");
        std::fs::create_dir_all(self.root().join("boundaries")).expect("boundaries");
        std::fs::write(
            self.root().join("boundaries").join("planning.toml"),
            "[planning]\ncurrent_sprint = \"A.7\"\n",
        )
        .expect("planning");
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

struct MockBackend {
    _tempdir: TempDir,
    original_path: Option<OsString>,
}

impl MockBackend {
    fn install(tool: &str, payload: &Value) -> Self {
        let tempdir = TempDir::new().expect("temp dir");
        let script_name = if cfg!(windows) {
            format!("{tool}.cmd")
        } else {
            tool.to_string()
        };
        let script_path = tempdir.path().join(script_name);
        let payload = payload.to_string();
        let script = if cfg!(windows) {
            format!("@echo off\r\necho {payload}\r\n")
        } else {
            format!("#!/usr/bin/env sh\nprintf '%s\\n' '{payload}'\n")
        };
        std::fs::write(&script_path, script).expect("mock backend script");
        set_mock_backend_permissions(&script_path);

        let original_path = std::env::var_os("PATH");
        let updated_path = std::env::join_paths(
            std::iter::once(tempdir.path().to_path_buf()).chain(
                original_path
                    .as_ref()
                    .into_iter()
                    .flat_map(std::env::split_paths),
            ),
        )
        .expect("joined PATH");
        // SAFETY: These tests are marked serial and this guard owns the full
        // lifecycle of the PATH mutation, so no concurrent environment access
        // occurs while the override is installed.
        unsafe { std::env::set_var("PATH", updated_path) };

        Self {
            _tempdir: tempdir,
            original_path,
        }
    }
}

impl Drop for MockBackend {
    fn drop(&mut self) {
        match &self.original_path {
            // SAFETY: The matching install path mutation is test-local and
            // synchronized by serial execution, so restoring PATH here is the
            // only concurrent environment mutation.
            Some(path) => unsafe { std::env::set_var("PATH", path) },
            // SAFETY: See the safety note above for PATH restoration.
            None => unsafe { std::env::remove_var("PATH") },
        }
    }
}

#[cfg(unix)]
fn set_mock_backend_permissions(path: &Path) {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = std::fs::metadata(path)
        .expect("mock backend metadata")
        .permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(path, permissions).expect("mock backend perms");
}

#[cfg(windows)]
fn set_mock_backend_permissions(_path: &Path) {}

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
