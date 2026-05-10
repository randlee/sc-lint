use std::ffi::OsString;
use std::path::Path;

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
    let data = crate::command::execute(&context).expect("version command succeeds");
    let envelope = CommandEnvelope::success(context.command_id(), data);
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
        Cli::parse_from(["sc-lint", "--json", "lint", "sc-boundary"]),
        Cli::parse_from(["sc-lint", "--json", "view", "graph"]),
        Cli::parse_from(["sc-lint", "--json", "check", "xwin"]),
        Cli::parse_from(["sc-lint", "--json", "clippy", "native"]),
        Cli::parse_from(["sc-lint", "--json", "ci"]),
    ];

    for cli in commands {
        let context = CommandContext::from_cli(&cli);
        let error = crate::command::execute(&context).expect_err("command is reserved in A.1a");
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
fn cli_error_exit_codes_are_stable_by_kind() {
    assert_eq!(CliError::usage("bad args").exit_code(), 2);
    assert_eq!(CliError::config("bad config").exit_code(), 3);
    assert_eq!(CliError::capability("missing capability").exit_code(), 4);
    assert_eq!(CliError::internal("bug").exit_code(), 1);
}

#[test]
fn logger_bootstrap_writes_entry_completion_and_error_records() {
    let temp_dir = TempDir::new().expect("temp dir");
    let temp_root = temp_dir.path().join("logs");

    let success_exit = crate::run([
        OsString::from("sc-lint"),
        OsString::from("--json"),
        OsString::from("--log-root"),
        OsString::from(temp_root.display().to_string()),
        OsString::from("version"),
    ]);
    assert_eq!(success_exit, std::process::ExitCode::from(0));

    let failure_exit = crate::run([
        OsString::from("sc-lint"),
        OsString::from("--json"),
        OsString::from("--log-root"),
        OsString::from(temp_root.display().to_string()),
        OsString::from("lint"),
        OsString::from("sc-boundary"),
    ]);
    assert_eq!(failure_exit, std::process::ExitCode::from(4));

    let log_path = temp_root.join("sc-lint").join("sc-lint.log.jsonl");
    assert_log_file_contains_action(&log_path, "cli.command.started");
    assert_log_file_contains_action(&log_path, "cli.command.completed");

    let error_log_path = temp_root.join("sc-boundary").join("sc-boundary.log.jsonl");
    assert_log_file_contains_action(&error_log_path, "cli.command.error");
}

#[test]
fn reserved_commands_report_capability_errors() {
    let cli = Cli::parse_from(["sc-lint", "check", "xwin"]);
    let context = CommandContext::from_cli(&cli);
    let error = crate::command::execute(&context).expect_err("reserved command fails");

    assert_eq!(error.kind, CliErrorKind::Capability);
    assert!(error.message.contains("Sprint A.1a"));
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
