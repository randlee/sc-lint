use std::path::Path;
use std::path::PathBuf;

use clap::Parser;
use serde_json::Value;
use tempfile::TempDir;

use crate::CheckTarget;
use crate::Cli;
use crate::CliError;
use crate::CliErrorKind;
use crate::ClippyTarget;
use crate::Command;
use crate::CommandEnvelope;
use crate::LintTarget;
use crate::ViewTarget;
use crate::command::CommandContext;
use crate::config::LoadedConfig;

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
    let context = CommandContext::from_cli(&cli);
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
fn every_initial_command_family_uses_the_same_failure_envelope_shape() {
    let commands = [
        Cli::parse_from(["sc-lint", "--json", "view", "graph"]),
        Cli::parse_from(["sc-lint", "--json", "check", "xwin"]),
        Cli::parse_from(["sc-lint", "--json", "clippy", "native"]),
        Cli::parse_from(["sc-lint", "--json", "ci"]),
    ];

    for cli in commands {
        let context = CommandContext::from_cli(&cli);
        let loaded = LoadedConfig::load(&cli, &context).expect("config loads");
        let error =
            crate::command::execute(&context, &loaded).expect_err("command is reserved in A.1b");
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
fn reserved_commands_report_capability_errors() {
    let cli = Cli::parse_from(["sc-lint", "check", "xwin"]);
    let context = CommandContext::from_cli(&cli);
    let loaded = LoadedConfig::load(&cli, &context).expect("config loads");
    let error = crate::command::execute(&context, &loaded).expect_err("reserved command fails");

    assert_eq!(error.kind, CliErrorKind::Capability);
    assert!(error.message.contains("Sprint A.1b"));
}

#[test]
fn repo_root_discovery_walks_up_to_the_workspace_root() {
    let repo_root = workspace_root();
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
    let context = CommandContext::from_cli(&cli);
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
    let context = CommandContext::from_cli(&cli);
    let error = LoadedConfig::load(&cli, &context).expect_err("config should fail");

    assert_eq!(error.kind, CliErrorKind::Config);
    assert!(error.message.contains("failed to parse repo config"));
    assert!(error.cause.is_some());
}

#[test]
fn lint_sc_boundary_normalizes_backend_success_through_the_top_level_envelope() {
    let repo_root = workspace_root();
    let cli = Cli::parse_from([
        "sc-lint",
        "--json",
        "--root",
        repo_root.to_str().expect("repo root"),
        "lint",
        "sc-boundary",
    ]);
    let context = CommandContext::from_cli(&cli);
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
    assert_eq!(json["data"]["tool"], "sc-lint-boundary");
    assert!(json["data"]["findings"].is_array());
}

#[test]
fn malformed_backend_json_maps_to_backend_protocol_error() {
    let error = crate::dispatch::normalize_backend_json("sc-lint-boundary", "{not-json")
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
    let context = CommandContext::from_cli(&cli);
    let loaded = LoadedConfig::load(&cli, &context).expect("config loads");
    let error = crate::command::execute(&context, &loaded).expect_err("dispatch should fail");

    assert_eq!(error.kind, CliErrorKind::BackendFailure);
    assert_eq!(error.code(), "CLI.BACKEND_EXEC_FAILURE");
    assert!(error.cause.is_some());
    assert!(std::error::Error::source(&error).is_some());
}

#[test]
fn loaded_config_preserves_repo_root_as_a_validated_newtype() {
    let repo_root = workspace_root();
    let cli = Cli::parse_from([
        "sc-lint",
        "--root",
        repo_root.to_str().expect("repo root"),
        "lint",
        "sc-boundary",
    ]);
    let context = CommandContext::from_cli(&cli);
    let loaded = LoadedConfig::load(&cli, &context).expect("config loads");

    assert_eq!(
        loaded.require_repo_root().expect("repo root"),
        repo_root.as_path()
    );
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root")
        .to_path_buf()
}
