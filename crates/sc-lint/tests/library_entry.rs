//! REQ-PRODUCT-002B: the command families are reachable through the `sc_lint`
//! library without the `sc-lint` binary. These tests link the library the way
//! the Python bindings do and never spawn the product executable.

use std::fs;

use sc_lint::CommandContext;
use sc_lint::ExecutionOutcome;
use sc_lint::LoadedConfig;
use sc_lint::ParsedInvocation;
use sc_lint::consts;
use serde_json::Value;
use tempfile::TempDir;

fn ready_invocation(args: &[&str]) -> sc_lint::Cli {
    match ParsedInvocation::parse(args.iter().copied()) {
        ParsedInvocation::Ready(cli) => cli,
        ParsedInvocation::Immediate(_) => panic!("{args:?} must reach the execution path"),
    }
}

fn execute(args: &[&str]) -> ExecutionOutcome {
    let cli = ready_invocation(args);
    let context = CommandContext::from_cli(&cli).expect("command context");
    let loaded = LoadedConfig::load(&cli, &context).expect("configuration loads");
    ExecutionOutcome::run(context, &loaded, cli.json)
}

#[test]
fn version_command_runs_through_the_library_entry_point() {
    let outcome = execute(&["sc-lint", "--json", "--version"]);

    assert!(outcome.ok);
    assert_eq!(outcome.exit_code, consts::EXIT_SUCCESS);
    assert!(outcome.error.is_none());
    assert!(outcome.dispatch.is_none());
    assert!(outcome.rendered.stderr.is_none());
    let stdout = outcome
        .rendered
        .stdout
        .as_deref()
        .expect("json success renders to stdout");
    let envelope: Value = serde_json::from_str(stdout).expect("stable json envelope");
    assert_eq!(envelope["ok"], true);
    assert_eq!(envelope["command"], "version");
    assert_eq!(envelope["data"]["tool"], "sc-lint");
    assert_eq!(envelope["data"]["version"], env!("CARGO_PKG_VERSION"));
    let payload: Value = serde_json::from_str(&sc_lint::version_json()).expect("version payload");
    assert_eq!(payload, envelope["data"]);
}

#[test]
fn failures_keep_the_machine_envelope_outside_the_binary() {
    let fixture = TempDir::new().expect("fixture");
    let config_path = fixture.path().join("sc-lint.toml");
    fs::write(
        &config_path,
        "[tool.sc-lint]\nminimum_version = \"0.4.1\"\n",
    )
    .expect("consumer config");
    let missing = fixture.path().join("missing-sc-lint");

    let outcome = execute(&[
        "sc-lint",
        "--json",
        "--config",
        config_path.to_str().expect("UTF-8 config path"),
        "compatibility",
        "check",
        "--binary",
        missing.to_str().expect("UTF-8 binary path"),
    ]);

    assert!(!outcome.ok);
    let error = outcome.error.as_ref().expect("structured error");
    assert_eq!(error.code(), "CLI.SC_LINT_BINARY_NOT_FOUND");
    assert_eq!(outcome.exit_code, error.exit_code());
    assert_eq!(outcome.summary, error.message);
    assert!(outcome.rendered.stdout.is_none());
    let stderr = outcome
        .rendered
        .stderr
        .as_deref()
        .expect("json failure renders to stderr");
    let envelope: Value = serde_json::from_str(stderr).expect("stable json envelope");
    assert_eq!(envelope["ok"], false);
    assert!(
        envelope["command"]
            .as_str()
            .is_some_and(|id| !id.is_empty())
    );
    assert_eq!(envelope["error"]["code"], "CLI.SC_LINT_BINARY_NOT_FOUND");
}

#[test]
fn parse_failures_are_immediate_outcomes_with_the_usage_exit_code() {
    let ParsedInvocation::Immediate(outcome) =
        ParsedInvocation::parse(["sc-lint", "--json", "unknown-command"])
    else {
        panic!("an unknown command must stop at parse time");
    };

    assert_eq!(outcome.write_code(), consts::EXIT_USAGE);
}

#[test]
fn run_code_returns_exit_codes_to_embedders() {
    assert_eq!(
        sc_lint::run_code(["sc-lint", "--json", "--version"]),
        consts::EXIT_SUCCESS
    );
    assert_eq!(
        sc_lint::run_code(["sc-lint", "--json", "unknown-command"]),
        consts::EXIT_USAGE
    );
}
