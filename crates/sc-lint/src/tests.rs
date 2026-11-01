use std::collections::HashMap;
use std::ffi::OsString;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
#[cfg(any(unix, windows))]
use std::process::Command as ProcessCommand;
use std::sync::Mutex;
use std::sync::mpsc;
use std::time::Duration;

use clap::Parser;
use clap::ValueEnum;
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
use crate::DocsGuide;
use crate::LintTarget;
use crate::ParsedInvocation;
use crate::ViewTarget;
use crate::cli::OutputMode;
use crate::command::CommandContext;
use crate::command::ConsumerInitRequest;
use crate::config::ConsumerProfileStep;
use crate::config::LoadedConfig;
use crate::profile_plan::CargoVerb;
use crate::profile_plan::PROFILE_PLAN_ERROR_CODE;
use crate::profile_plan::PlanDecision;
use crate::profile_plan::PlannedStep;
use crate::profile_plan::Platform;
use crate::profile_plan::PlatformScope;
use crate::profile_plan::ProfileName;
use crate::profile_plan::ProfilePlanDetailCode;
use crate::profile_plan::Target;
use crate::profile_plan::plan_consumer_lint;
use crate::scheduler::FailurePolicy;
use crate::test_selectors::TestRequest;
use crate::workflow;

const CANONICAL_CONSUMER_JUSTFILE: &str = include_str!("../assets/consumer-Justfile");
const CLI_CONTRACT: &str = include_str!("../../../docs/sc-lint/cli-contract.md");

#[test]
fn cli_contract_exit_code_mapping_matches_code_constants() {
    let mapping_start = CLI_CONTRACT
        .split_once("## Exit-Code Mapping")
        .expect("CLI contract has an Exit-Code Mapping section")
        .1;
    let mapping = mapping_start.split("\n## ").next().unwrap_or(mapping_start);
    let documented: Vec<u8> = mapping
        .lines()
        .filter_map(|line| {
            line.trim()
                .strip_prefix("- `")
                .and_then(|code| code.strip_suffix('`'))
                .and_then(|code| code.parse().ok())
        })
        .collect();
    let expected = [
        crate::consts::EXIT_SUCCESS,
        crate::consts::EXIT_INTERNAL,
        crate::consts::EXIT_USAGE,
        crate::consts::EXIT_CONFIG,
        crate::consts::EXIT_CAPABILITY,
        crate::consts::EXIT_BACKEND_FAILURE,
        crate::consts::EXIT_BACKEND_PROTOCOL,
        crate::consts::EXIT_VERDICT_FAIL,
    ];

    assert_eq!(documented, expected);
}

#[test]
fn cli_contract_documents_current_business_verdict_exit_codes() {
    let contract = CLI_CONTRACT
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    assert!(contract.contains("`sc-lint check interfaces` is planned, not implemented"));
    assert!(!contract.contains("currently exits `0` for a completed comparison"));
    assert!(contract.contains(
        "`lint sc-boundary`, `lint sc-portability` and `lint sc-runtime` now exit `10` when `data.status = \"fail\"`; the envelope remains `ok: true`"
    ));
    assert!(contract.contains(
        "command completed successfully; analyzer lint commands use this for a passing verdict, while a failed analyzer verdict uses code `10`"
    ));

    let ParsedInvocation::Immediate(outcome) =
        crate::parse_args(["sc-lint", "--json", "check", "interfaces"])
    else {
        panic!("check interfaces must stop at parse time");
    };
    assert_eq!(outcome.exit_code, crate::consts::EXIT_USAGE);
    let rendered = outcome
        .rendered
        .stderr
        .expect("machine parse error on stderr");
    let envelope: Value = serde_json::from_str(&rendered).expect("parse-error envelope");
    assert_eq!(envelope["command"], "cli.parse_error");
    assert_eq!(envelope["error"]["kind"], "usage");
}

#[test]
fn canonical_consumer_justfile_is_thin_and_has_exactly_four_public_recipes() {
    let canonical = CANONICAL_CONSUMER_JUSTFILE.replace("\r\n", "\n");
    assert!(canonical.starts_with("set windows-shell := [\"cmd.exe\", \"/c\"]\n"));
    for (recipe, selector) in [
        ("setup", None),
        ("lint", Some("profile")),
        ("test", Some("layer")),
        ("upgrade", None),
    ] {
        if recipe == "lint" {
            assert!(canonical.contains("lint selector='' *scope:"));
            assert!(
                canonical
                    .contains("bootstrap.py lint --config sc-lint.toml \"{{selector}}\" {{scope}}")
            );
            continue;
        }
        let expected = match selector {
            Some(selector) => format!(
                "{recipe} *{selector}:\n    {{{{python}}}} .sc-lint/bootstrap.py {recipe} --config sc-lint.toml {{{{{selector}}}}}"
            ),
            None => {
                format!(
                    "{recipe}:\n    {{{{python}}}} .sc-lint/bootstrap.py {recipe} --config sc-lint.toml"
                )
            }
        };
        assert!(
            canonical.contains(&expected),
            "consumer template does not delegate `{recipe}` to the product bootstrap"
        );
    }
    assert!(canonical.contains("python := if os_family() == \"windows\""));
    assert!(!canonical.contains("compatibility"));
    assert!(!canonical.contains("_ensure-sc-lint"));
    for forbidden in ["cargo run", "sc-lint-boundary", ".just/"] {
        assert!(
            !canonical.contains(forbidden),
            "consumer template leaks source-maintainer implementation `{forbidden}`"
        );
    }
}

#[test]
fn command_surface_parses_the_initial_grouped_shape() {
    let cli = Cli::parse_from(["sc-lint", "lint", "sc-boundary"]);
    assert!(matches!(
        cli.command.as_ref(),
        Some(Command::Lint {
            target: LintTarget::ScBoundary,
            ..
        })
    ));

    let cli = Cli::parse_from(["sc-lint", "view", "graph"]);
    assert!(matches!(
        cli.command.as_ref(),
        Some(Command::View {
            target: ViewTarget::Graph
        })
    ));

    let cli = Cli::parse_from(["sc-lint", "lint", "function-length"]);
    assert!(matches!(
        cli.command.as_ref(),
        Some(Command::Lint {
            target: LintTarget::FunctionLength,
            ..
        })
    ));

    let cli = Cli::parse_from(["sc-lint", "lint", "line-counts"]);
    assert!(matches!(
        cli.command.as_ref(),
        Some(Command::Lint {
            target: LintTarget::LineCounts,
            ..
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

    for command in [
        "lint",
        "view",
        "check",
        "clippy",
        "compatibility",
        "setup",
        "upgrade",
        "init",
        "docs",
        "test",
        "version",
        "ci",
        "--json",
    ] {
        assert!(help.contains(command), "missing `{command}` in help output");
    }
    for guide in [
        "just-setup",
        "sc-lint-analyzer-support",
        "sc-lint-attributes",
        "sc-lint-schema",
    ] {
        assert!(
            help.contains(guide),
            "missing documentation guide `{guide}`"
        );
    }
}

#[test]
fn docs_command_parses_named_guides_and_path_mode() {
    let cli = Cli::parse_from(["sc-lint", "docs", "sc-lint-schema", "--path"]);
    assert!(matches!(
        cli.command,
        Some(Command::Docs {
            guide: Some(DocsGuide::ScLintSchema),
            path: true,
        })
    ));
}

#[test]
fn docs_setup_alias_resolves_the_installation_guide() {
    let cli = Cli::parse_from(["sc-lint", "docs", "setup"]);
    assert!(matches!(
        cli.command,
        Some(Command::Docs {
            guide: Some(crate::DocsGuide::Installation),
            path: false,
        })
    ));
}

#[test]
fn setup_and_upgrade_parse_the_stable_consumer_flags() {
    let setup = Cli::parse_from(["sc-lint", "setup", "--dry-run"]);
    assert!(matches!(
        setup.command,
        Some(Command::Setup { dry_run: true })
    ));
    let upgrade = Cli::parse_from(["sc-lint", "upgrade", "--check", "--dry-run"]);
    assert!(matches!(
        upgrade.command,
        Some(Command::Upgrade {
            check: true,
            dry_run: true
        })
    ));
}

#[test]
fn setup_and_upgrade_json_use_the_canonical_top_level_envelope() {
    let fixture = TempDir::new().expect("fixture");
    let config_path = fixture.path().join("sc-lint.toml");
    fs::write(
        &config_path,
        "[tool.sc-lint]\nminimum_version = \"0.4.1\"\n",
    )
    .expect("consumer config");
    let config = config_path.to_str().expect("UTF-8 config path");

    for (args, command_id) in [
        (
            vec![
                "sc-lint",
                "--json",
                "--config",
                config,
                "setup",
                "--dry-run",
            ],
            "setup",
        ),
        (
            vec![
                "sc-lint",
                "--json",
                "--config",
                config,
                "upgrade",
                "--check",
                "--dry-run",
            ],
            "upgrade",
        ),
    ] {
        let cli = Cli::parse_from(args);
        let context = CommandContext::from_cli(&cli).expect("installer context");
        let loaded = LoadedConfig::load(&cli, &context).expect("consumer config loads");
        let success = crate::command::execute(&context, &loaded).expect("dry-run succeeds");
        let envelope = CommandEnvelope::success(context.command_id(), success.data);
        let rendered = crate::render::render_success_json(&envelope);
        let json: Value = serde_json::from_str(&rendered).expect("envelope json");

        assert_eq!(json["ok"], true);
        assert_eq!(json["command"], command_id);
        assert!(json["data"].is_object());
        assert!(json["diagnostics"].is_array());
    }
}

#[test]
fn consumer_commands_require_an_explicit_consumer_mode() {
    let lint = Cli::parse_from(["sc-lint", "lint", "ci", "--consumer"]);
    assert!(matches!(
        lint.command,
        Some(Command::Lint {
            target: LintTarget::Ci,
            consumer: true,
            profile: None,
            platform_scope: None,
            rule_filter: None,
            fail_fast: false,
        })
    ));
    let test = Cli::parse_from(["sc-lint", "test"]);
    assert!(matches!(
        test.command,
        Some(Command::Test {
            ref selector_and_args,
            list: false,
            fail_fast: false,
        }) if selector_and_args.is_empty()
    ));
    let init = Cli::parse_from(["sc-lint", "init", "--just", "--check"]);
    assert!(matches!(
        init.command,
        Some(Command::Init {
            just: true,
            check: true,
            dry_run: false,
        })
    ));
    let invalid = Cli::parse_from(["sc-lint", "lint", "fast", "--consumer"]);
    let error = CommandContext::from_cli(&invalid).expect_err("only CI profile is consumer-owned");
    assert_eq!(error.kind, CliErrorKind::Usage);
    let init_with_config = Cli::parse_from(["sc-lint", "--config", "other.toml", "init", "--just"]);
    let error = CommandContext::from_cli(&init_with_config).expect_err("init owns its config path");
    assert_eq!(error.kind, CliErrorKind::Usage);
}

#[test]
fn source_and_consumer_lint_paths_are_distinct_without_path_heuristics() {
    let source = Cli::parse_from(["sc-lint", "lint", "ci"]);
    let source_context = CommandContext::from_cli(&source).expect("source context");
    let consumer = Cli::parse_from(["sc-lint", "lint", "ci", "--consumer"]);
    let consumer_context = CommandContext::from_cli(&consumer).expect("consumer context");

    assert_eq!(source_context.command_id(), "lint.ci");
    assert!(source_context.requires_repo_root());
    assert_eq!(consumer_context.command_id(), "lint.ci.consumer");
    assert!(!consumer_context.requires_repo_root());
}

#[test]
fn consumer_profile_schema_is_rejected_before_any_profile_step_runs() {
    let fixture = TempDir::new().expect("fixture");
    let config_path = fixture.path().join("sc-lint.toml");
    fs::write(
        &config_path,
        "[tool.sc-lint]\nminimum_version = \"0.4.0\"\n\n[[tool.sc-lint.test]]\nname = \"test\"\ncommand = [\"cargo\", \"test\"]\n",
    )
    .expect("config");
    let cli = Cli::parse_from([
        "sc-lint",
        "--config",
        config_path.to_str().expect("config path"),
        "lint",
        "ci",
        "--consumer",
    ]);
    let context = CommandContext::from_cli(&cli).expect("context");
    let error =
        LoadedConfig::load(&cli, &context).expect_err("empty lint profile fails before execution");
    assert_eq!(error.code(), "CLI.SC_LINT_CONFIG_MALFORMED");
    assert_eq!(error.details["profile"], "lint");
}

#[test]
fn consumer_init_is_idempotent_non_mutating_when_checked_and_preserves_user_files() {
    let fixture = TempDir::new().expect("fixture");
    let root = fixture.path();
    let readme = root.join("README.md");
    fs::write(&readme, "consumer-owned README\n").expect("README");
    let request = ConsumerInitRequest {
        just: true,
        check: false,
        dry_run: false,
    };

    let dry_run = crate::config::run_consumer_init_at(
        root,
        ConsumerInitRequest {
            just: true,
            check: false,
            dry_run: true,
        },
    )
    .expect("dry run");
    assert_eq!(dry_run["status"], "would_create");
    assert!(!root.join("sc-lint.toml").exists());

    let created = crate::config::run_consumer_init_at(root, request).expect("initializes");
    assert_eq!(created["status"], "created");
    assert!(root.join("sc-lint.toml").is_file());
    assert!(root.join("Justfile").is_file());
    assert!(root.join(".sc-lint/bootstrap.py").is_file());
    assert_eq!(
        fs::read(root.join(".sc-lint/bootstrap.py"))
            .expect("bootstrap")
            .get(..22),
        Some(&b"#!/usr/bin/env python3"[..])
    );
    assert_eq!(
        fs::read_to_string(&readme).expect("README"),
        "consumer-owned README\n"
    );

    let current = crate::config::run_consumer_init_at(root, request).expect("idempotent");
    assert_eq!(current["status"], "current");
    let checked = crate::config::run_consumer_init_at(
        root,
        ConsumerInitRequest {
            just: true,
            check: true,
            dry_run: false,
        },
    )
    .expect("check current integration");
    assert_eq!(checked["status"], "current");

    fs::write(root.join("Justfile"), "consumer-owned\n").expect("conflict");
    let error = crate::config::run_consumer_init_at(root, request)
        .expect_err("conflict is not overwritten");
    assert_eq!(error.code(), "CLI.SC_LINT_INTEGRATION_CONFLICT");
    assert_eq!(
        fs::read_to_string(root.join("Justfile")).expect("Justfile"),
        "consumer-owned\n"
    );
}

#[test]
fn consumer_profiles_run_every_member_and_fail_the_aggregate_on_member_failure() {
    let fixture = TempDir::new().expect("fixture");
    let config_path = fixture.path().join("sc-lint.toml");
    write_consumer_profile_config(&config_path);
    let lint_cli = Cli::parse_from([
        "sc-lint",
        "--config",
        config_path.to_str().expect("config path"),
        "lint",
        "ci",
        "--consumer",
    ]);
    let lint_context = CommandContext::from_cli(&lint_cli).expect("consumer lint context");
    let loaded = LoadedConfig::load(&lint_cli, &lint_context).expect("consumer config");
    let passing = FakeSystemAdapter::new(false);
    let lint = workflow::run_consumer_lint_with(
        &loaded,
        None,
        PlatformScope::Host,
        FailurePolicy::KeepGoing,
        &passing,
    )
    .expect("all lint members run");
    assert_eq!(
        step_names(lint.data["steps"].as_array().expect("steps")),
        vec!["lint-fmt", "lint-clippy"]
    );
    assert_eq!(
        passing
            .invocations
            .lock()
            .expect("fake invocation log lock")
            .as_slice(),
        ["lint-fmt", "lint-clippy"]
    );

    let test_cli = Cli::parse_from([
        "sc-lint",
        "--config",
        config_path.to_str().expect("config path"),
        "test",
    ]);
    let test_context = CommandContext::from_cli(&test_cli).expect("consumer test context");
    let loaded = LoadedConfig::load(&test_cli, &test_context).expect("consumer config");
    let mut failing = FakeSystemAdapter::new(false);
    failing
        .failures
        .insert("test-workspace", "configured test failed");
    let error = workflow::run_consumer_test_with(
        &loaded,
        &TestRequest::all(),
        FailurePolicy::KeepGoing,
        &failing,
    )
    .expect_err("a required test member fails the aggregate");
    assert_eq!(error.kind, CliErrorKind::BackendFailure);
    assert_eq!(error.code(), "profile.steps_failed");
    assert_eq!(
        failing
            .invocations
            .lock()
            .expect("fake invocation log lock")
            .as_slice(),
        ["test-workspace"]
    );
}

#[test]
fn consumer_profile_runs_parallel_steps_and_reports_every_failure() {
    let rows = "[[tool.sc-lint.lint]]\nname = \"first\"\ncommand = [\"python\", \"first.py\"]\n\n[[tool.sc-lint.lint]]\nname = \"second\"\ncommand = [\"python\", \"second.py\"]\n";
    let (_fixture, loaded) = load_consumer_toml(&consumer_toml(rows));
    let loaded = loaded.expect("consumer config loads");
    let (first_started, second_waits) = mpsc::channel();
    let (second_started, first_waits) = mpsc::channel();
    let mut adapter = FakeSystemAdapter::new(false);
    adapter.failures.insert("first", "first failed");
    adapter.failures.insert("second", "second failed");
    adapter.on_start.insert(
        "first",
        rendezvous_hook(
            first_started,
            first_waits,
            "second consumer step never started while first was running",
        ),
    );
    adapter.on_start.insert(
        "second",
        rendezvous_hook(
            second_started,
            second_waits,
            "first consumer step never started while second was running",
        ),
    );

    let error = workflow::run_consumer_lint_with(
        &loaded,
        None,
        PlatformScope::Host,
        FailurePolicy::KeepGoing,
        &adapter,
    )
    .expect_err("both failing consumer steps fail the profile");

    assert_eq!(error.code(), "profile.steps_failed");
    assert_eq!(
        error.details["failed_steps"],
        json!([
            {"index": 0, "name": "first", "exit_code": null, "message": "first failed"},
            {"index": 1, "name": "second", "exit_code": null, "message": "second failed"}
        ])
    );
    assert_eq!(sorted(adapter.invoked()), ["first", "second"]);
}

#[test]
fn consumer_selectors_pick_one_step_or_all_steps_in_declaration_order() {
    let fixture = TempDir::new().expect("fixture");
    let config_path = fixture.path().join("sc-lint.toml");
    fs::write(
        &config_path,
        "[tool.sc-lint]\nminimum_version = \"0.4.0\"\n\n[[tool.sc-lint.lint]]\nname = \"ci\"\ncommand = [\"cargo\", \"clippy\"]\n\n[[tool.sc-lint.lint]]\nname = \"fmt\"\ncommand = [\"cargo\", \"fmt\", \"--check\"]\n\n[[tool.sc-lint.test]]\nname = \"unit\"\ncommand = [\"cargo\", \"test\"]\n\n[[tool.sc-lint.test]]\nname = \"integrate\"\ncommand = [\"cargo\", \"test\", \"--test\", \"integration\"]\n",
    )
    .expect("consumer config");
    let config = config_path.to_str().expect("config path");

    let layer_cli = Cli::parse_from(["sc-lint", "--config", config, "test", "integrate"]);
    let layer_context = CommandContext::from_cli(&layer_cli).expect("test layer context");
    assert_eq!(layer_context.test_request().selector(), Some("integrate"));
    let loaded = LoadedConfig::load(&layer_cli, &layer_context).expect("consumer config");
    let adapter = FakeSystemAdapter::new(false);
    let run = workflow::run_consumer_test_with(
        &loaded,
        layer_context.test_request(),
        FailurePolicy::KeepGoing,
        &adapter,
    )
    .expect("named layer runs");
    assert_eq!(
        adapter
            .invocations
            .lock()
            .expect("fake invocation log lock")
            .as_slice(),
        ["integrate"]
    );
    assert_eq!(run.data["selector"], "integrate");

    let all_cli = Cli::parse_from(["sc-lint", "--config", config, "test", "all"]);
    let all_context = CommandContext::from_cli(&all_cli).expect("all layers context");
    let adapter = FakeSystemAdapter::new(false);
    let run = workflow::run_consumer_test_with(
        &loaded,
        all_context.test_request(),
        FailurePolicy::KeepGoing,
        &adapter,
    )
    .expect("all layers run");
    let steps = run.data["steps"].as_array().expect("steps array");
    assert_eq!(step_names(steps), ["unit", "integrate"],);
    assert_eq!(
        adapter
            .invocations
            .lock()
            .expect("fake invocation log lock")
            .as_slice(),
        ["unit", "integrate"]
    );

    let profile_cli = Cli::parse_from([
        "sc-lint",
        "--config",
        config,
        "lint",
        "ci",
        "--consumer",
        "--profile",
        "fmt",
    ]);
    let profile_context = CommandContext::from_cli(&profile_cli).expect("lint profile context");
    let adapter = FakeSystemAdapter::new(false);
    workflow::run_consumer_lint_with(
        &loaded,
        profile_context.consumer_selector(),
        PlatformScope::Host,
        FailurePolicy::KeepGoing,
        &adapter,
    )
    .expect("named lint profile runs");
    assert_eq!(
        adapter
            .invocations
            .lock()
            .expect("fake invocation log lock")
            .as_slice(),
        ["fmt"]
    );

    let error = workflow::run_consumer_test_with(
        &loaded,
        &TestRequest::named("nightly"),
        FailurePolicy::KeepGoing,
        &FakeSystemAdapter::new(false),
    )
    .expect_err("unknown layer is a configuration error");
    assert_eq!(error.code(), "test.unknown_selector");
    assert_eq!(error.details["selector"], "nightly");
    assert_eq!(error.details["available"], json!(["unit", "integrate"]));

    assert!(
        Cli::try_parse_from(["sc-lint", "lint", "ci", "--profile", "fmt"]).is_err(),
        "`--profile` requires `--consumer`"
    );
}

#[test]
fn consumer_missing_backend_is_a_structured_recovery_error() {
    let fixture = TempDir::new().expect("fixture");
    let config_path = fixture.path().join("sc-lint.toml");
    fs::write(
        &config_path,
        "[tool.sc-lint]\nminimum_version = \"0.4.0\"\n\n[[tool.sc-lint.lint]]\nname = \"missing\"\ncommand = [\"sc-lint-definitely-missing-backend\"]\n\n[[tool.sc-lint.test]]\nname = \"test\"\ncommand = [\"cargo\", \"test\"]\n",
    )
    .expect("config");
    let cli = Cli::parse_from([
        "sc-lint",
        "--config",
        config_path.to_str().expect("config path"),
        "lint",
        "ci",
        "--consumer",
    ]);
    let context = CommandContext::from_cli(&cli).expect("consumer context");
    let loaded = LoadedConfig::load(&cli, &context).expect("consumer config");
    let error = workflow::run_consumer_lint_profile(
        &loaded,
        None,
        PlatformScope::Host,
        FailurePolicy::KeepGoing,
    )
    .expect_err("missing backend fails");
    assert_eq!(error.code(), "profile.steps_failed");
    let failed = &error.details["failed_steps"][0];
    assert_eq!(failed["index"], 0);
    assert_eq!(failed["name"], "missing");
    assert_eq!(failed["exit_code"], Value::Null);
    assert!(
        failed["message"]
            .as_str()
            .is_some_and(|message| message.starts_with("missing failed to start")),
        "{failed}"
    );
    assert!(!error.to_string().to_ascii_lowercase().contains("traceback"));
}

/// Explicit opt-out for the wheel-backed consumer fixture below.
#[cfg(unix)]
const SKIP_WHEEL_FIXTURE_ENV: &str = "SC_LINT_SKIP_WHEEL_FIXTURE";

#[cfg(unix)]
#[test]
#[serial]
fn generated_consumer_fixture_runs_just_lint_and_test_after_the_shared_preflight() {
    let fixture = TempDir::new().expect("fixture");
    let root = fixture.path();
    crate::config::run_consumer_init_at(
        root,
        ConsumerInitRequest {
            just: true,
            check: false,
            dry_run: false,
        },
    )
    .expect("generate fixture");
    // The bootstrap provisions `.sc-lint/venv` from PyPI; point it at the
    // wheels `just setup` built so the fixture stays offline.
    // `just test` builds them first (`_source-venv`); a bare `cargo test` must
    // build them too or opt out explicitly, never skip silently.
    let wheel_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.sc-lint/wheels");
    if !wheel_dir.is_dir() {
        assert!(
            std::env::var_os(SKIP_WHEEL_FIXTURE_ENV).is_some(),
            "missing {}: run `just setup` (or `just test`) to build the wheels, or set {SKIP_WHEEL_FIXTURE_ENV}=1 to skip this fixture explicitly",
            wheel_dir.display()
        );
        eprintln!("skipping: {SKIP_WHEEL_FIXTURE_ENV} is set");
        return;
    }
    let bin_dir = root.join("bin");
    fs::create_dir_all(&bin_dir).expect("bin dir");
    let record = root.join("calls.txt");
    let binary = bin_dir.join("sc-lint");
    fs::write(
        &binary,
        "#!/bin/sh\nprintf '%s\\n' \"$*\" >> \"$SC_LINT_RECORD\"\n",
    )
    .expect("fake sc-lint");
    set_mock_backend_permissions(&binary);
    let path = std::env::join_paths(
        std::iter::once(bin_dir.clone()).chain(
            std::env::var_os("PATH")
                .as_ref()
                .into_iter()
                .flat_map(std::env::split_paths),
        ),
    )
    .expect("PATH");

    for (recipe, selector) in [("lint", Some("ci")), ("test", None)] {
        let output = ProcessCommand::new("just")
            .current_dir(root)
            .arg(recipe)
            .args(selector)
            .env("PATH", &path)
            .env("SC_LINT_BIN", &binary)
            .env("SC_LINT_RECORD", &record)
            .env("SC_LINT_WHEEL_DIR", &wheel_dir)
            .output()
            .expect("run just fixture");
        assert!(
            output.status.success(),
            "just {recipe} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let calls = fs::read_to_string(&record)
        .expect("calls")
        .lines()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    assert_eq!(
        calls,
        vec![
            format!(
                "--config sc-lint.toml compatibility check --binary {}",
                binary.display()
            ),
            "lint --consumer --config sc-lint.toml ci --profile ci".to_string(),
            format!(
                "--config sc-lint.toml compatibility check --binary {}",
                binary.display()
            ),
            "test --config sc-lint.toml".to_string(),
        ]
    );
}

#[cfg(windows)]
#[test]
fn generated_windows_consumer_fixture_runs_just_lint_and_test_after_shared_preflight() {
    let fixture = TempDir::new().expect("fixture");
    let root = fixture.path();
    crate::config::run_consumer_init_at(
        root,
        ConsumerInitRequest {
            just: true,
            check: false,
            dry_run: false,
        },
    )
    .expect("generate fixture");
    // The bootstrap provisions `.sc-lint/venv` from PyPI; point it at the
    // wheels `just setup` built so the fixture stays offline.
    let wheel_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.sc-lint/wheels");
    if !wheel_dir.is_dir() {
        eprintln!(
            "skipping: run `just setup` to build {}",
            wheel_dir.display()
        );
        return;
    }
    let bin_dir = root.join("bin");
    fs::create_dir_all(&bin_dir).expect("bin dir");
    let record = root.join("calls.txt");
    let binary = bin_dir.join("sc-lint.cmd");
    fs::write(&binary, "@echo off\r\necho %*>> \"%SC_LINT_RECORD%\"\r\n").expect("fake sc-lint");
    let path = std::env::join_paths(
        std::iter::once(bin_dir.clone()).chain(
            std::env::var_os("PATH")
                .as_ref()
                .into_iter()
                .flat_map(std::env::split_paths),
        ),
    )
    .expect("PATH");

    for (recipe, selector) in [("lint", Some("ci")), ("test", None)] {
        let output = ProcessCommand::new("just")
            .current_dir(root)
            .arg(recipe)
            .args(selector)
            .env("PATH", &path)
            .env("SC_LINT_BIN", &binary)
            .env("SC_LINT_RECORD", &record)
            .env("SC_LINT_WHEEL_DIR", &wheel_dir)
            .output()
            .expect("run just fixture");
        assert!(
            output.status.success(),
            "just {recipe} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let calls = fs::read_to_string(&record)
        .expect("calls")
        .lines()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    assert_eq!(
        calls,
        vec![
            format!(
                "--config sc-lint.toml compatibility check --binary {}",
                binary.display()
            ),
            "lint --consumer --config sc-lint.toml ci --profile ci".to_string(),
            format!(
                "--config sc-lint.toml compatibility check --binary {}",
                binary.display()
            ),
            "test --config sc-lint.toml".to_string(),
        ]
    );
}

#[cfg(windows)]
#[test]
fn generated_windows_bootstrap_accepts_gnu_style_flags_without_positional_errors() {
    let fixture = TempDir::new().expect("fixture");
    crate::config::run_consumer_init_at(
        fixture.path(),
        ConsumerInitRequest {
            just: true,
            check: false,
            dry_run: false,
        },
    )
    .expect("generate fixture");
    // The bootstrap provisions `.sc-lint/venv` from PyPI; point it at the
    // wheels `just setup` built so the fixture stays offline.
    let wheel_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.sc-lint/wheels");
    if !wheel_dir.is_dir() {
        eprintln!(
            "skipping: run `just setup` to build {}",
            wheel_dir.display()
        );
        return;
    }
    let bootstrap = fixture.path().join(".sc-lint/bootstrap.py");
    let record = fixture.path().join("calls.txt");
    let binary = fixture.path().join("sc-lint.cmd");
    fs::write(&binary, "@echo off\r\necho %*>> \"%SC_LINT_RECORD%\"\r\n").expect("fake sc-lint");
    let output = ProcessCommand::new("python")
        .args([
            bootstrap.to_str().expect("UTF-8 bootstrap path"),
            "setup",
            "--config",
            "sc-lint.toml",
        ])
        .current_dir(fixture.path())
        .env("SC_LINT_BIN", &binary)
        .env("SC_LINT_RECORD", &record)
        .env("SC_LINT_WHEEL_DIR", &wheel_dir)
        .output()
        .expect("run Windows bootstrap");
    assert!(
        output.status.success(),
        "bootstrap failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.to_ascii_lowercase().contains("positional parameter"));
    let calls = fs::read_to_string(&record).expect("calls");
    assert!(calls.contains("--config sc-lint.toml compatibility check"));
    assert!(calls.contains("--config sc-lint.toml setup"));
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
    assert_eq!(json["data"]["tool"], "sc-lint");
    assert_eq!(json["data"]["version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(json["data"]["contract_schema"], "sc-lint-version-v1");
    assert_eq!(json["data"]["status"], "pass");
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
    assert_eq!(json["data"]["tool"], "sc-lint");
    assert_eq!(json["data"]["version"], env!("CARGO_PKG_VERSION"));
}

#[test]
#[serial]
fn compatibility_check_uses_semver_not_lexical_comparison() {
    let fixture = TempDir::new().expect("fixture");
    let config_path = fixture.path().join("sc-lint.toml");
    fs::write(
        &config_path,
        "[tool.sc-lint]\nminimum_version = \"0.4.1\"\n",
    )
    .expect("compatibility config");

    for (version, expected_ok) in [
        ("0.4.1", true),
        ("0.4.10", true),
        ("0.5.0", true),
        ("0.4.0", false),
        ("0.4.1-alpha.1", false),
    ] {
        let backend = MockBackend::install(
            "sc-lint",
            &json!({
                "ok": true,
                "command": "version",
                "data": {
                    "tool": "sc-lint",
                    "version": version,
                    "contract_schema": "sc-lint-version-v1",
                    "status": "pass"
                },
                "diagnostics": []
            }),
        );
        let cli = Cli::parse_from([
            "sc-lint",
            "--json",
            "--config",
            config_path.to_str().expect("config path"),
            "compatibility",
            "check",
            "--binary",
            backend.path().to_str().expect("mock binary path"),
        ]);
        let context = CommandContext::from_cli(&cli).expect("compatibility context");
        let loaded = LoadedConfig::load(&cli, &context).expect("compatibility config loads");
        let result = crate::command::execute(&context, &loaded);
        assert_eq!(result.is_ok(), expected_ok, "version {version}");
        if expected_ok {
            let success = result.expect("compatible release passes");
            assert_eq!(success.data["installed_version"], version);
        } else {
            let error = result.expect_err("incompatible release fails");
            assert_eq!(error.code(), "CLI.SC_LINT_VERSION_TOO_OLD");
            assert_eq!(error.details["minimum_version"], "0.4.1");
            assert_eq!(error.details["installed_version"], version);
        }
        drop(backend);
    }
}

#[test]
#[serial]
fn compatibility_check_reports_malformed_installed_version_in_human_and_json_forms() {
    let fixture = TempDir::new().expect("fixture");
    let config_path = fixture.path().join("sc-lint.toml");
    fs::write(
        &config_path,
        "[tool.sc-lint]\nminimum_version = \"0.4.1\"\n",
    )
    .expect("compatibility config");
    let backend = MockBackend::install(
        "sc-lint",
        &json!({
            "ok": true,
            "command": "version",
            "data": {
                "tool": "sc-lint",
                "version": "not-semver",
                "contract_schema": "sc-lint-version-v1"
            },
            "diagnostics": []
        }),
    );
    let cli = Cli::parse_from([
        "sc-lint",
        "--json",
        "--config",
        config_path.to_str().expect("config path"),
        "compatibility",
        "check",
        "--binary",
        backend.path().to_str().expect("mock binary path"),
    ]);
    let context = CommandContext::from_cli(&cli).expect("compatibility context");
    let loaded = LoadedConfig::load(&cli, &context).expect("compatibility config loads");
    let error = crate::command::execute(&context, &loaded).expect_err("bad version fails");
    assert_eq!(error.code(), "CLI.SC_LINT_VERSION_UNPARSABLE");
    assert!(error.cause.is_some());
    let json: Value = serde_json::from_str(&crate::render::render_error_json(
        context.command_id(),
        &error,
    ))
    .expect("json error");
    assert_eq!(json["error"]["docs"], "sc-lint docs setup");
    assert_eq!(json["error"]["details"]["minimum_version"], "0.4.1");
    assert_eq!(json["error"]["details"]["reported_version"], "not-semver");
    let human = crate::render::render_error_human(context.command_id(), &error);
    assert!(human.contains("just setup"));
    assert!(human.contains("Docs: sc-lint docs setup"));
    assert!(human.contains("Reported version: not-semver"));
}

#[test]
fn compatibility_config_failures_identify_the_canonical_file_and_field() {
    let fixture = TempDir::new().expect("fixture");
    let missing = fixture.path().join("missing.toml");
    let cli = Cli::parse_from([
        "sc-lint",
        "--config",
        missing.to_str().expect("missing path"),
        "compatibility",
        "check",
    ]);
    let context = CommandContext::from_cli(&cli).expect("compatibility context");
    let missing_error = LoadedConfig::load(&cli, &context).expect_err("missing config fails");
    assert_eq!(missing_error.code(), "CLI.SC_LINT_CONFIG_MISSING");
    assert_eq!(
        missing_error.details["config_path"],
        missing.display().to_string()
    );
    assert_eq!(
        missing_error.details["required_field"],
        "[tool.sc-lint].minimum_version"
    );
    assert!(missing_error.cause.is_some());
    let missing_human = crate::render::render_error_human("compatibility.check", &missing_error);
    assert!(missing_human.contains(&missing.display().to_string()));
    assert!(missing_human.contains("[tool.sc-lint].minimum_version"));

    let malformed = fixture.path().join("malformed.toml");
    fs::write(
        &malformed,
        "[tool.sc-lint]\nminimum_version = \"not-semver\"\n",
    )
    .expect("malformed config");
    let cli = Cli::parse_from([
        "sc-lint",
        "--config",
        malformed.to_str().expect("malformed path"),
        "compatibility",
        "check",
    ]);
    let context = CommandContext::from_cli(&cli).expect("compatibility context");
    let malformed_error = LoadedConfig::load(&cli, &context).expect_err("bad config fails");
    assert_eq!(malformed_error.code(), "CLI.SC_LINT_CONFIG_MALFORMED");
    assert_eq!(
        malformed_error.details["config_path"],
        malformed.display().to_string()
    );
    assert_eq!(
        malformed_error.details["required_field"],
        "[tool.sc-lint].minimum_version"
    );
    assert!(malformed_error.cause.is_some());

    let absent_field = fixture.path().join("absent-field.toml");
    fs::write(&absent_field, "[tool.sc-lint]\n").expect("incomplete config");
    let cli = Cli::parse_from([
        "sc-lint",
        "--config",
        absent_field.to_str().expect("incomplete path"),
        "compatibility",
        "check",
    ]);
    let context = CommandContext::from_cli(&cli).expect("compatibility context");
    let absent_field_error =
        LoadedConfig::load(&cli, &context).expect_err("absent minimum field fails");
    assert_eq!(absent_field_error.code(), "CLI.SC_LINT_CONFIG_MALFORMED");
    assert_eq!(
        absent_field_error.details["required_field"],
        "[tool.sc-lint].minimum_version"
    );
    assert!(absent_field_error.cause.is_some());
}

#[test]
fn compatibility_check_reports_missing_binary_with_recovery_contract() {
    let fixture = TempDir::new().expect("fixture");
    let config_path = fixture.path().join("sc-lint.toml");
    let binary_path = fixture.path().join("missing-sc-lint");
    fs::write(
        &config_path,
        "[tool.sc-lint]\nminimum_version = \"0.4.1\"\n",
    )
    .expect("compatibility config");
    let cli = Cli::parse_from([
        "sc-lint",
        "--config",
        config_path.to_str().expect("config path"),
        "compatibility",
        "check",
        "--binary",
        binary_path.to_str().expect("binary path"),
    ]);
    let context = CommandContext::from_cli(&cli).expect("compatibility context");
    let loaded = LoadedConfig::load(&cli, &context).expect("compatibility config loads");
    let error = crate::command::execute(&context, &loaded).expect_err("missing binary fails");
    assert_eq!(error.code(), "CLI.SC_LINT_BINARY_NOT_FOUND");
    assert_eq!(error.details["minimum_version"], "0.4.1");
    assert_eq!(
        error.details["binary_path"],
        binary_path.display().to_string()
    );
    assert!(error.cause.is_some());
    assert_eq!(error.documentation.as_deref(), Some("sc-lint docs setup"));
}

#[test]
#[serial]
fn compatibility_check_human_error_includes_all_available_identity_details() {
    let fixture = TempDir::new().expect("fixture");
    let config_path = fixture.path().join("sc-lint.toml");
    fs::write(
        &config_path,
        "[tool.sc-lint]\nminimum_version = \"0.4.1\"\n",
    )
    .expect("compatibility config");
    let backend = MockBackend::install(
        "sc-lint",
        &json!({
            "ok": true,
            "command": "version",
            "data": {
                "tool": "sc-lint",
                "version": "0.4.0",
                "contract_schema": "sc-lint-version-v1"
            },
            "diagnostics": []
        }),
    );
    let cli = Cli::parse_from([
        "sc-lint",
        "--config",
        config_path.to_str().expect("config path"),
        "compatibility",
        "check",
        "--binary",
        backend.path().to_str().expect("mock binary path"),
    ]);
    let context = CommandContext::from_cli(&cli).expect("compatibility context");
    let loaded = LoadedConfig::load(&cli, &context).expect("compatibility config loads");
    let error = crate::command::execute(&context, &loaded).expect_err("old version fails");
    let human = crate::render::render_error_human(context.command_id(), &error);
    for required in [
        "0.4.1",
        "0.4.0",
        config_path.to_str().expect("config path"),
        backend.path().to_str().expect("mock binary path"),
    ] {
        assert!(
            human.contains(required),
            "missing `{required}` from {human}"
        );
    }
}

#[cfg(unix)]
#[test]
fn compatibility_check_reports_unexecutable_binary_with_recovery_contract() {
    let fixture = TempDir::new().expect("fixture");
    let config_path = fixture.path().join("sc-lint.toml");
    let binary_path = fixture.path().join("not-an-executable");
    fs::write(
        &config_path,
        "[tool.sc-lint]\nminimum_version = \"0.4.1\"\n",
    )
    .expect("compatibility config");
    fs::create_dir(&binary_path).expect("directory binary fixture");
    let cli = Cli::parse_from([
        "sc-lint",
        "--config",
        config_path.to_str().expect("config path"),
        "compatibility",
        "check",
        "--binary",
        binary_path.to_str().expect("binary path"),
    ]);
    let context = CommandContext::from_cli(&cli).expect("compatibility context");
    let loaded = LoadedConfig::load(&cli, &context).expect("compatibility config loads");
    let error = crate::command::execute(&context, &loaded).expect_err("unexecutable binary fails");
    assert_eq!(error.code(), "CLI.SC_LINT_BINARY_EXECUTION_FAILED");
    assert!(error.cause.is_some());
    assert!(
        error
            .suggested_action
            .as_deref()
            .is_some_and(|action| action.contains("just setup"))
    );
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
    assert_eq!(LintTarget::FunctionLength.profile(), None);
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
fn repo_root_discovery_names_the_required_sentinels_with_backticks() {
    let fixture = AnalysisFixture::new();
    let start = fixture.root().join("nested");
    fs::create_dir_all(&start).expect("nested start directory");
    let cli = Cli::parse_from([
        "sc-lint",
        "--root",
        start.to_str().expect("fixture path"),
        "lint",
        "sc-boundary",
    ]);
    let context = CommandContext::from_cli(&cli).expect("repo-root context");
    let error = LoadedConfig::load(&cli, &context).expect_err("repo root discovery fails");

    assert_eq!(error.kind, CliErrorKind::Config);
    assert!(
        error
            .message
            .contains("missing required sentinels `sc-lint.toml` or `Cargo.toml [workspace]`"),
        "{}",
        error.message
    );
    assert_eq!(
        error.details["required_sentinels"],
        json!(["sc-lint.toml", "Cargo.toml [workspace]"])
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

fn load_error_for_config(config: &str, argv: &[&str]) -> crate::error::CliError {
    let temp_dir = TempDir::new().expect("temp dir");
    std::fs::write(
        temp_dir.path().join("Cargo.toml"),
        "[workspace]\nmembers=[]\nresolver=\"2\"\n",
    )
    .expect("write manifest");
    std::fs::create_dir_all(temp_dir.path().join("boundaries")).expect("write boundaries dir");
    std::fs::write(temp_dir.path().join("sc-lint.toml"), config).expect("write config");
    let mut args = vec![
        "sc-lint",
        "--root",
        temp_dir.path().to_str().expect("temp path"),
    ];
    args.extend_from_slice(argv);
    let cli = Cli::parse_from(args);
    let context = CommandContext::from_cli(&cli).expect("context");
    LoadedConfig::load(&cli, &context).expect_err("config should fail")
}

#[test]
fn declined_setup_and_runner_tables_name_adr_020_decision_9() {
    for (table, config) in [
        (
            "tool.sc-lint.setup",
            "[tool.sc-lint]\nminimum_version = \"0.4.0\"\n[tool.sc-lint.setup]\npre = [\"python3\", \"tools/setup.py\"]\n",
        ),
        (
            "tool.sc-lint.runner",
            "[tool.sc-lint]\nminimum_version = \"0.4.0\"\n[tool.sc-lint.runner]\npython = \".venv\"\n",
        ),
        (
            "setup",
            "[setup]\npre = [\"python3\", \"x.py\"]\n[tool.sc-lint]\nminimum_version = \"0.4.0\"\n",
        ),
        (
            "runner",
            "[runner]\npython = \".venv\"\n[tool.sc-lint]\nminimum_version = \"0.4.0\"\n",
        ),
    ] {
        let temp_dir = TempDir::new().expect("temp dir");
        let config_path = temp_dir.path().join("sc-lint.toml");
        std::fs::write(&config_path, config).expect("write config");
        let compatibility = crate::compatibility::load_compatibility_requirement(&config_path)
            .expect_err("declined table must fail compatibility loading");
        let mut errors = vec![compatibility];
        for argv in [
            &["lint", "sc-boundary"][..],
            &["lint", "sc-portability"][..],
            &["lint", "ci"][..],
        ] {
            errors.push(load_error_for_config(config, argv));
        }
        // Path 0 is load_compatibility_requirement; the rest are argv loads.
        for (path, error) in errors.into_iter().enumerate() {
            assert_eq!(error.kind, CliErrorKind::Config, "{table} path {path}");
            assert_eq!(error.code(), "config.unknown_table", "{table} path {path}");
            assert_eq!(error.details.get("key"), Some(&json!(table)));
            let cause = error.cause.as_deref().expect("cause");
            assert!(cause.contains("ADR-020 decision 9"), "{cause}");
            let action = error.suggested_action.as_deref().expect("suggested action");
            assert!(action.contains("sc_lint.setup_tools"), "{action}");
            assert!(
                action.contains(".sc-lint/venv/bin/python -m sc_lint.<module>"),
                "{action}"
            );
        }
    }
}

#[test]
fn unknown_tables_are_rejected_with_a_typed_config_error() {
    for (key, config) in [
        (
            "tool.sc-lint.lints",
            "[tool.sc-lint]\nminimum_version = \"0.4.0\"\n[[tool.sc-lint.lints]]\nname = \"x\"\n",
        ),
        (
            "tool.sc-lint.minimum",
            "[tool.sc-lint]\nminimum = \"0.4.0\"\n",
        ),
    ] {
        let error = load_error_for_config(config, &["lint", "sc-boundary"]);
        assert_eq!(error.code(), "config.unknown_table", "{key}");
        assert_eq!(error.details.get("key"), Some(&json!(key)));
        let cause = error.cause.as_deref().expect("cause");
        assert!(!cause.contains("ADR-020"), "{cause}");
        assert!(
            error
                .suggested_action
                .as_deref()
                .is_some_and(|action| action.contains("supported keys")),
            "{error:?}"
        );
    }
}

#[test]
fn supported_sc_lint_tables_still_load() {
    let temp_dir = TempDir::new().expect("temp dir");
    std::fs::write(
        temp_dir.path().join("Cargo.toml"),
        "[workspace]\nmembers=[]\nresolver=\"2\"\n",
    )
    .expect("write manifest");
    std::fs::create_dir_all(temp_dir.path().join("boundaries")).expect("write boundaries dir");
    std::fs::write(
        temp_dir.path().join("sc-lint.toml"),
        "[logging]\nconsole = false\n[line_counts]\nlimit = 500\n[portability]\nx = 1\n[identities]\nx = 1\n[version_sync]\nx = 1\n[cargo_shear]\nx = 1\n[tool.sc-lint]\nminimum_version = \"0.4.0\"\n[tool.sc-lint.lint_profiles]\nfast = [\"a\"]\n[[tool.sc-lint.lint]]\nname = \"a\"\ncommand = [\"true\"]\n[[tool.sc-lint.test]]\nname = \"t\"\ncommand = [\"true\"]\n[tool.sc-lint.analyzers.sc-boundary]\nenabled = true\n",
    )
    .expect("write config");
    let cli = Cli::parse_from([
        "sc-lint",
        "--root",
        temp_dir.path().to_str().expect("temp path"),
        "lint",
        "sc-boundary",
    ]);
    let context = CommandContext::from_cli(&cli).expect("context");
    LoadedConfig::load(&cli, &context).expect("supported keys load");
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
fn lint_sc_boundary_reports_dependency_policy_failures_through_dispatch() {
    for (rule_id, configure_fixture) in [
        (
            "SCB-DEPENDENCY-001",
            AnalysisFixture::configure_disallowed_dependency as fn(&AnalysisFixture),
        ),
        (
            "SCB-DEPENDENCY-002",
            AnalysisFixture::configure_disallowed_dependent as fn(&AnalysisFixture),
        ),
        (
            "SCB-DEPENDENCY-003",
            AnalysisFixture::configure_forbidden_edge as fn(&AnalysisFixture),
        ),
    ] {
        let fixture = AnalysisFixture::new();
        configure_fixture(&fixture);

        let data = run_sc_boundary_json(fixture.root());
        assert_eq!(data["status"], "fail", "expected fail status for {rule_id}");
        assert!(
            data["findings"].as_array().is_some(),
            "missing findings for {rule_id}"
        );
        assert!(
            data["findings"]
                .as_array()
                .expect("findings array")
                .iter()
                .any(|finding| finding["rule_id"] == rule_id),
            "missing {rule_id} in top-level dispatch payload"
        );
    }
}

#[test]
fn lint_rule_filter_parses_for_analyzer_targets() {
    for target in ["sc-boundary", "sc-portability", "sc-runtime"] {
        let cli = Cli::parse_from([
            "sc-lint",
            "lint",
            target,
            "--rule-filter",
            "PORT-001,PORT-006",
        ]);
        let context = CommandContext::from_cli(&cli).expect("analyzer context");
        assert_eq!(context.rule_filter(), Some("PORT-001,PORT-006"), "{target}");
    }
    let cli = Cli::parse_from(["sc-lint", "lint", "sc-portability"]);
    let context = CommandContext::from_cli(&cli).expect("analyzer context");
    assert_eq!(context.rule_filter(), None);
}

#[test]
fn lint_rule_filter_rejected_for_non_analyzer_target() {
    let cli = Cli::parse_from(["sc-lint", "lint", "fast", "--rule-filter", "PORT-001"]);
    let error = CommandContext::from_cli(&cli).expect_err("rule filter on fast");
    assert_eq!(error.kind, CliErrorKind::Usage);
    assert_eq!(error.exit_code(), 2);
    assert!(error.message.contains("--rule-filter"));

    assert!(
        Cli::try_parse_from(["sc-lint", "lint", "ci", "--consumer", "--rule-filter", "X"]).is_err()
    );
}

#[test]
fn lint_sc_boundary_rule_filter_selects_the_rule() {
    let fixture = AnalysisFixture::new();
    fixture.configure_disallowed_dependency();
    let cli = Cli::parse_from([
        "sc-lint",
        "--json",
        "--root",
        fixture.root().to_str().expect("root"),
        "lint",
        "sc-boundary",
        "--rule-filter",
        "SCB-DEPENDENCY-001",
    ]);
    let context = CommandContext::from_cli(&cli).expect("boundary context");
    let loaded = LoadedConfig::load(&cli, &context).expect("config loads");
    let success = crate::command::execute(&context, &loaded).expect("dispatch succeeds");
    let findings = success.data["findings"].as_array().expect("findings array");
    assert!(!findings.is_empty());
    assert!(
        findings
            .iter()
            .all(|finding| finding["rule_id"] == "SCB-DEPENDENCY-001")
    );
}

#[test]
fn lint_sc_boundary_unknown_rule_filter_is_usage_error() {
    let fixture = AnalysisFixture::new();
    fixture.configure_disallowed_dependency();
    let cli = Cli::parse_from([
        "sc-lint",
        "--json",
        "--root",
        fixture.root().to_str().expect("root"),
        "lint",
        "sc-boundary",
        "--rule-filter",
        "NOPE",
    ]);
    let context = CommandContext::from_cli(&cli).expect("boundary context");
    let loaded = LoadedConfig::load(&cli, &context).expect("config loads");
    let error = crate::command::execute(&context, &loaded).expect_err("unknown selector");
    assert_eq!(error.kind, CliErrorKind::Usage);
    assert_eq!(error.exit_code(), 2);
    assert!(error.message.contains("rejected --rule-filter"));
}

fn lint_boundary_outcome(root: &Path, json_mode: bool) -> crate::entry::ExecutionOutcome {
    let root = root.to_str().expect("root");
    let cli = if json_mode {
        Cli::parse_from(["sc-lint", "--json", "--root", root, "lint", "sc-boundary"])
    } else {
        Cli::parse_from(["sc-lint", "--root", root, "lint", "sc-boundary"])
    };
    let context = CommandContext::from_cli(&cli).expect("boundary context");
    let loaded = LoadedConfig::load(&cli, &context).expect("config loads");
    crate::entry::execute(context, &loaded, json_mode)
}

#[test]
fn lint_analyzer_fail_verdict_exits_ten() {
    let fixture = AnalysisFixture::new();
    fixture.configure_disallowed_dependency();
    for json_mode in [true, false] {
        let outcome = lint_boundary_outcome(fixture.root(), json_mode);
        assert_eq!(
            outcome.exit_code,
            crate::consts::EXIT_VERDICT_FAIL,
            "json_mode = {json_mode}"
        );
        assert!(outcome.ok, "json_mode = {json_mode}");
        assert!(outcome.error.is_none(), "json_mode = {json_mode}");
    }
}

#[test]
fn lint_analyzer_pass_verdict_exits_zero() {
    let fixture = AnalysisFixture::new();
    fixture.write_workspace_root();
    fixture.write(
        "crates/example/Cargo.toml",
        "[package]\nname = \"example\"\nversion.workspace = true\nedition.workspace = true\nrust-version.workspace = true\nauthors.workspace = true\nlicense.workspace = true\nrepository.workspace = true\nhomepage.workspace = true\n",
    );
    fixture.write_source("example", "lib.rs", "pub struct Example;\n");
    let outcome = lint_boundary_outcome(fixture.root(), true);
    assert_eq!(outcome.exit_code, 0);
    assert!(outcome.ok);
    assert!(outcome.error.is_none());
}

#[test]
fn lint_sc_boundary_wrapper_resolves_workspace_identity_aliases() {
    let fixture = AnalysisFixture::new();
    fixture.write_workspace_root_with_members(&["crates/app-dir", "crates/api-dir"]);
    fixture.write(
        "crates/app-dir/Cargo.toml",
        "[package]\nname = \"app-pkg\"\nversion.workspace = true\nedition.workspace = true\nrust-version.workspace = true\nauthors.workspace = true\nlicense.workspace = true\nrepository.workspace = true\nhomepage.workspace = true\n\n[lib]\nname = \"app_lib\"\n\n[dependencies]\napi_alias = { package = \"api-pkg\", path = \"../api-dir\", version = \"0.1.0\" }\n",
    );
    fixture.write(
        "crates/api-dir/Cargo.toml",
        "[package]\nname = \"api-pkg\"\nversion.workspace = true\nedition.workspace = true\nrust-version.workspace = true\nauthors.workspace = true\nlicense.workspace = true\nrepository.workspace = true\nhomepage.workspace = true\n\n[lib]\nname = \"api_lib\"\n",
    );
    fixture.write_source("app-dir", "lib.rs", "pub struct App;\n");
    fixture.write_source("api-dir", "lib.rs", "pub struct Api;\n");
    fixture.write_custom_boundary_record("app-dir", "app-dir", "app_lib", &["api_lib"]);

    for json_mode in [true, false] {
        let outcome = lint_boundary_outcome(fixture.root(), json_mode);
        assert_eq!(
            outcome.exit_code, 0,
            "json_mode = {json_mode}, data = {:?}",
            outcome.rendered
        );
        assert!(outcome.ok, "json_mode = {json_mode}");
        assert!(outcome.error.is_none(), "json_mode = {json_mode}");
    }
}

#[test]
fn verdict_exit_code_covers_every_analyzer_target() {
    use crate::entry::verdict_exit_code;

    let fail = json!({ crate::consts::FIELD_STATUS: crate::consts::STATUS_FAIL });
    let pass = json!({ "status": "pass" });
    for id in [
        crate::command::CommandId::LintScBoundary,
        crate::command::CommandId::LintScPortability,
        crate::command::CommandId::LintScRuntime,
    ] {
        assert_eq!(
            verdict_exit_code(id, Some(&fail)),
            crate::consts::EXIT_VERDICT_FAIL,
            "{id:?}"
        );
        assert_eq!(verdict_exit_code(id, Some(&pass)), 0, "{id:?}");
        assert_eq!(verdict_exit_code(id, None), 0, "{id:?}");
    }
    assert_eq!(
        verdict_exit_code(crate::command::CommandId::CheckNative, Some(&fail)),
        0
    );
}

fn lint_mocked_analyzer_fail_outcome(target: &str, tool: &str) -> (i32, Value) {
    let fixture = AnalysisFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_source("example", "lib.rs", "pub fn example() {}\n");
    let mock_backend = MockBackend::install_native(
        tool,
        &json!({
            "tool": tool,
            "version": "0.8.0",
            "schema_version": "0.1.0",
            "scanned_crates": 1,
            "findings": [],
            "status": crate::consts::STATUS_FAIL,
        }),
    );
    let test_executable = std::env::current_exe().expect("test executable path");
    let cli_name = if cfg!(windows) {
        "sc-lint.exe"
    } else {
        "sc-lint"
    };
    let cli_source = test_executable
        .parent()
        .and_then(Path::parent)
        .map(|target_dir| target_dir.join(cli_name))
        .expect("sc-lint executable path");
    let cli_copy = mock_backend._tempdir.path().join(cli_name);
    fs::copy(cli_source, &cli_copy).expect("copy sc-lint beside mock backend");

    // The backend resolver prefers binaries next to sc-lint over PATH. Running
    // this copy makes the mocked sibling binary the one the CLI dispatches to.
    let output = ProcessCommand::new(cli_copy)
        .args(["--json", "--root"])
        .arg(fixture.root())
        .args(["lint", target])
        .output()
        .expect("run mocked analyzer command");
    let exit_code = output.status.code().expect("process exit code");
    let envelope = serde_json::from_slice(&output.stdout).expect("success envelope JSON");
    (exit_code, envelope)
}

fn run_lint_with_path_backend_without_version() -> (std::process::Output, MockBackend, TempDir) {
    let fixture = AnalysisFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_source("example", "lib.rs", "pub fn example() {}\n");

    let backend = MockBackend::install_native_without_version(crate::consts::TOOL_PORTABILITY);
    let launcher_dir = TempDir::new().expect("isolated launcher directory");
    let test_executable = std::env::current_exe().expect("test executable path");
    let cli_name = if cfg!(windows) {
        "sc-lint.exe"
    } else {
        "sc-lint"
    };
    let cli_source = test_executable
        .parent()
        .and_then(Path::parent)
        .map(|target_dir| target_dir.join(cli_name))
        .expect("sc-lint executable path");
    let cli_copy = launcher_dir.path().join(cli_name);
    fs::copy(cli_source, &cli_copy).expect("copy isolated sc-lint launcher");

    let output = ProcessCommand::new(cli_copy)
        .env("PATH", backend.search_path())
        .args(["--json", "--root"])
        .arg(fixture.root())
        .args(["lint", "sc-portability"])
        .output()
        .expect("run isolated sc-lint launcher");
    (output, backend, launcher_dir)
}

#[test]
#[serial]
fn stale_path_backend_without_version_fails_before_running_analysis() {
    let (output, backend, _launcher_dir) = run_lint_with_path_backend_without_version();
    let analysis_marker = backend._tempdir.path().join("analysis-invoked");
    let envelope: Value = serde_json::from_slice(&output.stderr).expect("error envelope JSON");
    let error = &envelope["error"];

    assert_eq!(
        output.status.code(),
        Some(i32::from(crate::consts::EXIT_BACKEND_FAILURE))
    );
    assert_eq!(envelope["ok"], false);
    assert_eq!(error["code"], "CLI.BACKEND_EXEC_FAILURE");
    assert_eq!(
        error["details"]["expected_version"],
        env!("CARGO_PKG_VERSION")
    );
    assert!(
        error["details"]["version_probe_failure"]
            .as_str()
            .unwrap()
            .contains("--version` probe exited")
    );
    assert_eq!(
        error["details"]["backend_path"],
        backend.path().display().to_string()
    );
    assert!(
        error["suggested_action"]
            .as_str()
            .unwrap()
            .contains("sc-lint setup")
    );
    assert!(
        error["message"]
            .as_str()
            .unwrap()
            .contains(&backend.path().display().to_string())
    );
    assert!(!analysis_marker.exists(), "stale backend must not analyze");
}

#[test]
fn matching_real_path_backends_run_analysis() {
    for (tool, target) in [
        (crate::consts::TOOL_BOUNDARY, "sc-boundary"),
        (crate::consts::TOOL_PORTABILITY, "sc-portability"),
        (crate::consts::TOOL_RUNTIME, "sc-runtime"),
    ] {
        let (output, _launcher_dir, _backend_dir) = run_lint_with_real_path_backend(tool, target);
        let envelope: Value =
            serde_json::from_slice(&output.stdout).expect("success envelope JSON");

        assert!(
            output.status.success()
                || output.status.code() == Some(i32::from(crate::consts::EXIT_VERDICT_FAIL)),
            "{tool} exit {:?}, stdout: {}, stderr: {}",
            output.status.code(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(envelope["ok"], true, "{tool} envelope: {envelope}");
        assert_eq!(envelope["data"]["tool"], tool);
    }
}

fn run_lint_with_real_path_backend(
    tool: &str,
    target: &str,
) -> (std::process::Output, TempDir, TempDir) {
    let fixture = AnalysisFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_source("example", "lib.rs", "pub fn example() {}\n");

    let launcher_dir = TempDir::new().expect("isolated launcher directory");
    let backend_dir = TempDir::new().expect("isolated backend directory");
    let built_binaries = crate::test_support::release_binaries();
    let binary_name = |name: &str| {
        if cfg!(windows) {
            format!("{name}.exe")
        } else {
            name.to_string()
        }
    };
    let launcher_name = binary_name(crate::consts::SERVICE_NAME);
    fs::copy(
        built_binaries
            .get(crate::consts::SERVICE_NAME)
            .expect("shared sc-lint launcher"),
        launcher_dir.path().join(&launcher_name),
    )
    .expect("copy isolated sc-lint launcher");
    for backend in [
        crate::consts::TOOL_BOUNDARY,
        crate::consts::TOOL_PORTABILITY,
        crate::consts::TOOL_RUNTIME,
    ] {
        let name = binary_name(backend);
        fs::copy(
            built_binaries.get(backend).expect("shared backend binary"),
            backend_dir.path().join(&name),
        )
        .unwrap_or_else(|error| panic!("copy built real backend {backend}: {error}"));
    }
    let path = std::env::var_os("PATH").unwrap_or_default();
    let path = std::env::join_paths(
        std::iter::once(backend_dir.path().to_path_buf()).chain(std::env::split_paths(&path)),
    )
    .expect("join isolated backend PATH");
    let output = ProcessCommand::new(launcher_dir.path().join(launcher_name))
        .env("PATH", path)
        .args(["--json", "--root"])
        .arg(fixture.root())
        .args(["lint", target])
        .output()
        .unwrap_or_else(|error| panic!("run real {tool} PATH backend: {error}"));
    (output, launcher_dir, backend_dir)
}

#[test]
#[serial]
fn lint_sc_portability_fail_verdict_exits_ten_with_success_envelope() {
    let (exit_code, envelope) =
        lint_mocked_analyzer_fail_outcome("sc-portability", crate::consts::TOOL_PORTABILITY);

    assert_eq!(exit_code, i32::from(crate::consts::EXIT_VERDICT_FAIL));
    assert_eq!(envelope["ok"], true);
    assert!(envelope.get("error").is_none());
}

#[test]
#[serial]
fn lint_sc_runtime_fail_verdict_exits_ten_with_success_envelope() {
    let (exit_code, envelope) =
        lint_mocked_analyzer_fail_outcome("sc-runtime", crate::consts::TOOL_RUNTIME);

    assert_eq!(exit_code, i32::from(crate::consts::EXIT_VERDICT_FAIL));
    assert_eq!(envelope["ok"], true);
    assert!(envelope.get("error").is_none());
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
            "version": "0.8.0",
            "schema_version": "0.1.0",
            "scanned_crates": 1,
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
    let context = CommandContext::from_cli_with_backend_search_path(
        &cli,
        Some(_mock_backend.search_path().to_os_string()),
    )
    .expect("portability context");
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
            "version": "0.8.0",
            "schema_version": "0.1.0",
            "scanned_crates": 1,
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
    let context = CommandContext::from_cli_with_backend_search_path(
        &cli,
        Some(_mock_backend.search_path().to_os_string()),
    )
    .expect("runtime context");
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
fn analyzer_schema_drift_maps_to_structured_backend_protocol_error() {
    let raw = r#"{
        "tool":"sc-lint-boundary",
        "version":"0.8.0",
        "schema_version":"0.2.0",
        "status":"pass",
        "scanned_crates":0,
        "findings":[{"rule_id":"BOUNDARY-001"}]
    }"#;
    let error = crate::dispatch::normalize_backend_json(crate::consts::TOOL_BOUNDARY, raw)
        .expect_err("schema drift must be rejected");

    assert_eq!(error.kind, CliErrorKind::BackendProtocol);
    assert_eq!(error.code(), "backend_payload_drift");
    assert!(
        error
            .cause
            .as_deref()
            .is_some_and(|cause| cause.contains("/findings/0"))
    );
    assert!(
        error.details["key_path"]
            .as_str()
            .is_some_and(|path| path.starts_with("/findings/0"))
    );
}

#[test]
fn analyzer_v2_allowed_report_normalizes_and_rejects_malformed_allowed_entries() {
    let valid = include_str!("../../sc-lint-schema/tests/fixtures/findings-report-v2.json");
    let expected: Value = serde_json::from_str(valid).expect("v2 fixture is JSON");
    let normalized = crate::dispatch::normalize_backend_json("sc-lint-portability", valid)
        .expect("valid v2 report normalizes");
    assert_eq!(normalized["schema_version"], "0.2.0");
    assert_eq!(normalized["allowed"], expected["allowed"]);

    let mut malformed = expected;
    malformed["allowed"][0]
        .as_object_mut()
        .expect("allowed entry object")
        .remove("reason");
    let error =
        crate::dispatch::normalize_backend_json("sc-lint-portability", &malformed.to_string())
            .expect_err("malformed allowed entries drift from the v2 schema");
    assert_eq!(error.kind, CliErrorKind::BackendProtocol);
    assert_eq!(error.code(), "backend_payload_drift");
    assert_eq!(error.details["key_path"], "/allowed/0");
    assert!(
        error
            .cause
            .as_deref()
            .is_some_and(|cause| cause.contains("/allowed/0"))
    );
}

#[test]
fn missing_boundary_planning_maps_to_cli_config_error() {
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
    let context = CommandContext::from_cli(&cli).expect("dispatch context");
    let loaded = LoadedConfig::load(&cli, &context).expect("config loads");
    let error = crate::command::execute(&context, &loaded).expect_err("missing planning fails");

    assert_eq!(error.kind, CliErrorKind::Config);
    assert_eq!(error.code(), "CLI.CONFIG_ERROR");
    assert!(error.cause.is_some());
    assert!(
        error
            .cause
            .as_deref()
            .is_some_and(|cause| cause.contains("planning.toml"))
    );
}

#[test]
fn empty_boundary_inventory_metadata_load_maps_to_config_error() {
    let temp_dir = TempDir::new().expect("temp dir");
    std::fs::write(
        temp_dir.path().join("Cargo.toml"),
        "[workspace]\nmembers=[]\nresolver=\"2\"\n",
    )
    .expect("write manifest");
    std::fs::create_dir_all(temp_dir.path().join("boundaries")).expect("write boundaries dir");
    std::fs::write(
        temp_dir.path().join("boundaries").join("planning.toml"),
        "[planning]\ncurrent_sprint = \"A.0\"\n",
    )
    .expect("write planning metadata");
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
                "function-length",
            ],
            "lint.function-length",
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
fn fast_and_ci_profiles_exclude_xwin_when_available() {
    for profile in [crate::cli::LintProfile::Fast, crate::cli::LintProfile::Ci] {
        let loaded = load_repo_lint_profile(profile.command_suffix());
        let adapter = FakeSystemAdapter::new(true);
        let result = workflow::run_lint_profile_with(&loaded, profile, &adapter)
            .expect("xwin-capable profile succeeds");
        let steps = result.data["steps"].as_array().expect("steps array");
        assert!(
            !step_names(steps).iter().any(|step| step.contains("xwin")),
            "{} must not plan xwin steps",
            profile.command_suffix()
        );
        assert_eq!(result.data["xwin"]["available"], true);
        assert_eq!(result.data["xwin"]["included"], false);
    }
}

fn load_repo_lint_profile(profile: &str) -> LoadedConfig {
    let repo_root = repo_backed_workspace_root();
    let cli = Cli::parse_from([
        "sc-lint",
        "--root",
        repo_root.to_str().expect("repo root"),
        "lint",
        profile,
    ]);
    let context = CommandContext::from_cli(&cli).expect("lint profile context");
    LoadedConfig::load(&cli, &context).expect("config loads")
}

/// Generous bound so a regression fails the test instead of hanging it.
const RENDEZVOUS_TIMEOUT: Duration = Duration::from_secs(30);

/// A hook that announces `announce`, then blocks until `await_signal` arrives.
fn rendezvous_hook(
    announce: mpsc::Sender<()>,
    await_signal: mpsc::Receiver<()>,
    what: &'static str,
) -> StepHook {
    let await_signal = Mutex::new(await_signal);
    Box::new(move || {
        announce.send(()).expect("peer is listening");
        await_signal
            .lock()
            .expect("rendezvous receiver lock")
            .recv_timeout(RENDEZVOUS_TIMEOUT)
            .unwrap_or_else(|_| panic!("{what}"));
    })
}

#[test]
fn lint_profile_runs_independent_steps_concurrently_in_plan_order() {
    let loaded = load_repo_lint_profile("fast");
    let mut adapter = FakeSystemAdapter::new(false);
    let (version_started, manifests_waits) = mpsc::channel();
    let (manifests_started, version_waits) = mpsc::channel();
    // Each step blocks until the other has started: this can only finish when
    // the two steps run at the same time.
    adapter.on_start.insert(
        "version",
        rendezvous_hook(
            version_started,
            version_waits,
            "manifests never started while version was running",
        ),
    );
    adapter.on_start.insert(
        "manifests",
        rendezvous_hook(
            manifests_started,
            manifests_waits,
            "version never started while manifests was running",
        ),
    );

    let success = workflow::run_lint_profile_with(&loaded, crate::cli::LintProfile::Fast, &adapter)
        .expect("fast profile succeeds");
    let steps = success.data["steps"].as_array().expect("steps array");
    assert_eq!(
        step_names(steps),
        ["fmt", "version", "manifests", "spell", "pytests"]
    );
}

#[test]
fn lint_profile_starts_cargo_lane_steps_in_plan_order() {
    let loaded = load_repo_lint_profile("full");
    let adapter = FakeSystemAdapter::new(true);

    workflow::run_lint_profile_with(&loaded, crate::cli::LintProfile::Full, &adapter)
        .expect("full profile succeeds");

    let cargo_lane = [
        "fmt",
        "clippy",
        "deny",
        "shear",
        "sc-boundary",
        "sc-portability",
        "sc-runtime",
        "check.xwin",
        "clippy.xwin",
    ];
    let started = adapter
        .invocations
        .lock()
        .expect("fake invocation log lock")
        .iter()
        .filter(|name| cargo_lane.contains(&name.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    assert_eq!(started, cargo_lane);
}

#[test]
fn lint_profile_keeps_going_and_reports_every_failed_step_by_plan_index() {
    let loaded = load_repo_lint_profile("full");
    let mut adapter = FakeSystemAdapter::new(false);
    adapter.failures.insert("clippy", "clippy failed");
    adapter.failures.insert("spell", "spell failed");
    adapter.failures.insert("sc-boundary", "boundary failed");

    let error = workflow::run_lint_profile_with(&loaded, crate::cli::LintProfile::Full, &adapter)
        .expect_err("the profile fails");
    assert_eq!(
        error.message,
        "3 of 14 steps failed: [1] clippy, [6] spell, [8] sc-boundary"
    );
    assert_eq!(error.kind, CliErrorKind::BackendFailure);
    assert_eq!(error.code(), "profile.steps_failed");
    let failed = error.details["failed_steps"]
        .as_array()
        .expect("failed_steps array");
    assert_eq!(failed.len(), 3);
    for entry in failed {
        let keys = entry
            .as_object()
            .expect("failed step object")
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>();
        assert_eq!(keys, ["exit_code", "index", "message", "name"]);
    }
    assert_eq!(failed[1]["index"], 6);
    assert_eq!(failed[1]["name"], "spell");
    let ran = adapter
        .invocations
        .lock()
        .expect("fake invocation log lock")
        .len();
    assert_eq!(ran, 14, "every step runs despite failures");
}

#[test]
fn lint_profile_reports_the_lowest_index_failure_after_reversed_completion() {
    let loaded = load_repo_lint_profile("fast");
    let mut adapter = FakeSystemAdapter::new(false);
    let (manifests_done, version_waits) = mpsc::channel();
    // `version` (plan index 1) cannot finish until `manifests` (index 2) has.
    let version_waits = Mutex::new(version_waits);
    adapter.on_start.insert(
        "version",
        Box::new(move || {
            version_waits
                .lock()
                .expect("version receiver lock")
                .recv_timeout(RENDEZVOUS_TIMEOUT)
                .expect("manifests never completed");
        }),
    );
    adapter.on_finish.insert(
        "manifests",
        Box::new(move || manifests_done.send(()).expect("version is waiting")),
    );
    adapter
        .failures
        .insert("version", "version failed first in plan");
    adapter
        .failures
        .insert("manifests", "manifests failed first in time");

    let error = workflow::run_lint_profile_with(&loaded, crate::cli::LintProfile::Fast, &adapter)
        .expect_err("the profile fails");
    assert_eq!(
        error.message,
        "2 of 5 steps failed: [1] version, [2] manifests"
    );

    let completions = adapter
        .completions
        .lock()
        .expect("fake completion log lock");
    let position = |name: &str| {
        completions
            .iter()
            .position(|done| done == name)
            .expect("step completed")
    };
    assert!(
        position("manifests") < position("version"),
        "completion order reversed"
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
    let unavailable_step_names = step_names(unavailable_steps);
    assert!(unavailable_step_names.contains(&"sc-runtime".to_string()));
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
    assert!(step_names(available_steps).contains(&"sc-runtime".to_string()));
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
    assert!(step_names(lint_steps).contains(&"sc-runtime".to_string()));
    assert_eq!(
        adapter
            .invocations
            .lock()
            .expect("fake invocation log lock")
            .iter()
            .filter(|step| step.as_str() == "sc-runtime")
            .count(),
        2,
        "lint ci and the top-level ci profile each dispatch sc-runtime"
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
fn render_success_human_covers_version_analyzer_and_summary_paths() {
    let version_cli = Cli::parse_from(["sc-lint", "version"]);
    let version_context = CommandContext::from_cli(&version_cli).expect("version context");
    let version_output = crate::render::render_success_human(
        &version_context,
        &CommandEnvelope::success("version", json!({ "version": "1.2.3" })),
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

    let portability_cli = Cli::parse_from(["sc-lint", "lint", "sc-portability"]);
    let portability_context =
        CommandContext::from_cli(&portability_cli).expect("portability context");
    let portability_output = crate::render::render_success_human(
        &portability_context,
        &CommandEnvelope::success(
            portability_context.command_id(),
            json!({ "status": "pass", "findings": [] }),
        ),
    );
    assert_eq!(portability_output, "sc-lint-portability: pass (0 findings)");

    let runtime_cli = Cli::parse_from(["sc-lint", "lint", "sc-runtime"]);
    let runtime_context = CommandContext::from_cli(&runtime_cli).expect("runtime context");
    let runtime_output = crate::render::render_success_human(
        &runtime_context,
        &CommandEnvelope::success(
            runtime_context.command_id(),
            json!({ "status": "fail", "findings": [{ "rule_id": "RUN-001" }] }),
        ),
    );
    assert_eq!(runtime_output, "sc-lint-runtime: fail (1 findings)");

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

#[test]
fn render_error_human_includes_backend_cause_and_process_diagnostics() {
    let error = CliError::backend_failure("clippy failed")
        .with_cause("process exited unsuccessfully")
        .with_detail("exit_code", json!(101))
        .with_detail("stdout", json!("compiler output"))
        .with_detail("stderr", json!("compiler error"));
    let rendered = crate::render::render_error_human("lint.full", &error);

    for expected in [
        "Cause: process exited unsuccessfully",
        "Exit code: 101",
        "Standard output: compiler output",
        "Standard error: compiler error",
    ] {
        assert!(
            rendered.contains(expected),
            "missing {expected}: {rendered}"
        );
    }
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

#[test]
fn workspace_version_is_the_single_source_of_truth_for_published_crates() {
    let workspace_root = repo_backed_workspace_root();
    let workspace_manifest: toml::Value = toml::from_str(
        &fs::read_to_string(workspace_root.join("Cargo.toml")).expect("read workspace Cargo.toml"),
    )
    .expect("parse workspace Cargo.toml");

    let workspace_version = workspace_manifest["workspace"]["package"]["version"]
        .as_str()
        .expect("workspace.package.version")
        .to_string();
    assert_eq!(workspace_version, env!("CARGO_PKG_VERSION"));

    let members = workspace_manifest["workspace"]["members"]
        .as_array()
        .expect("workspace members");
    for member in members {
        let member_manifest_path =
            workspace_root.join(member.as_str().expect("workspace member path"));
        let member_manifest: toml::Value = toml::from_str(
            &fs::read_to_string(member_manifest_path.join("Cargo.toml"))
                .expect("read member Cargo.toml"),
        )
        .expect("parse member Cargo.toml");
        assert_eq!(
            member_manifest["package"]["version"]["workspace"].as_bool(),
            Some(true),
            "expected {} to inherit workspace.package.version",
            member_manifest_path.display()
        );
    }

    for dependency_name in ["sc-lint-boundary", "sc-lint-directives", "sc-lint-schema"] {
        assert_eq!(
            workspace_manifest["workspace"]["dependencies"][dependency_name]["version"].as_str(),
            Some(workspace_version.as_str()),
            "expected workspace dependency {dependency_name} to track workspace.package.version",
        );
    }
}

fn repo_backed_workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root")
        .to_path_buf()
}

fn write_consumer_profile_config(path: &Path) {
    fs::write(
        path,
        "[tool.sc-lint]\nminimum_version = \"0.4.0\"\n\n[[tool.sc-lint.lint]]\nname = \"lint-fmt\"\ncommand = [\"cargo\", \"fmt\", \"--check\"]\n\n[[tool.sc-lint.lint]]\nname = \"lint-clippy\"\ncommand = [\"cargo\", \"clippy\", \"--workspace\"]\n\n[[tool.sc-lint.test]]\nname = \"test-workspace\"\ncommand = [\"cargo\", \"test\", \"--workspace\"]\n",
    )
    .expect("consumer config");
}

fn run_sc_boundary_json(repo_root: &Path) -> Value {
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
    crate::command::execute(&context, &loaded)
        .expect("dispatch succeeds")
        .data
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
        self.write_workspace_root_with_members(&["crates/example"]);
    }

    fn write_workspace_root_with_members(&self, members: &[&str]) {
        let members = members
            .iter()
            .map(|member| format!("\"{member}\""))
            .collect::<Vec<_>>()
            .join(", ");
        std::fs::write(
            self.root().join("Cargo.toml"),
            format!(
                r#"[workspace]
members = [{members}]
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2024"
rust-version = "1.94.1"
authors = ["sc-lint contributors"]
license = "MIT OR Apache-2.0"
repository = "https://example.invalid/sc-lint"
homepage = "https://example.invalid/sc-lint"
"#
            ),
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
        self.write_package_manifest_with_dependencies(package_name, "");
    }

    fn write_package_manifest_with_dependencies(&self, package_name: &str, dependencies: &str) {
        self.write(
            &format!("crates/{package_name}/Cargo.toml"),
            &format!(
                "[package]\nname = \"{package_name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n{dependencies}"
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

    fn write_member_boundary_record(
        &self,
        owner_package: &str,
        allowed_dependencies: &[&str],
        allowed_dependents: &[&str],
        forbidden_edges: &[(&str, &str)],
    ) {
        let owner_crate_path = match owner_package {
            "app" => "app",
            "api" => "api",
            other => panic!("unexpected fixture package {other}"),
        };
        let boundary_id = owner_package
            .split('-')
            .map(|segment| {
                let mut chars = segment.chars();
                match chars.next() {
                    Some(first) => first.to_ascii_uppercase().to_string() + chars.as_str(),
                    None => String::new(),
                }
            })
            .collect::<String>();
        let allowed_dependencies = allowed_dependencies
            .iter()
            .map(|package| format!("\"{package}\""))
            .collect::<Vec<_>>()
            .join(", ");
        let allowed_dependents = allowed_dependents
            .iter()
            .map(|package| format!("\"{package}\""))
            .collect::<Vec<_>>()
            .join(", ");
        let forbidden_edges = forbidden_edges
            .iter()
            .map(|(from, to)| format!("{{ from = \"{from}\", to = \"{to}\" }}"))
            .collect::<Vec<_>>()
            .join(", ");
        let forbidden_edges_block = if forbidden_edges.is_empty() {
            "[]".to_string()
        } else {
            format!("[{forbidden_edges}]")
        };

        self.write(
            "boundaries/planning.toml",
            "[planning]\ncurrent_sprint = \"D.1\"\n",
        );
        self.write(
            &format!("boundaries/{owner_package}/boundary.toml"),
            &format!(
                "boundary_id = \"BOUNDARY-{boundary_id}\"\nowner_package = \"{owner_package}\"\nowner_crate_path = \"{}\"\nname = \"{owner_package}\"\n\n[public]\nfacade = \"run\"\n\n[implementation]\ntype = \"run\"\nmodule = \"{}\"\nvisibility = \"public\"\nconstructor = \"none\"\n\n[composition]\nroots = [\"run\"]\n\n[dependencies]\nallowed_dependents = [{allowed_dependents}]\nallowed_dependencies = [{allowed_dependencies}]\nforbidden_edges = {forbidden_edges_block}\n\n[references]\nscope = \"outside_owner_crate\"\nforbidden = []\n\n[testing]\nallowed_test_double_paths = []\nforbidden_test_bypasses = []\n\n[enforcement]\nlint_rules = []\nreview_gates = []\n\n[status]\nstate = \"concrete_landed\"\n",
                owner_crate_path,
                owner_package,
            ),
        );
    }

    fn write_custom_boundary_record(
        &self,
        boundary_directory: &str,
        owner_package: &str,
        owner_crate_path: &str,
        allowed_dependencies: &[&str],
    ) {
        let allowed_dependencies = allowed_dependencies
            .iter()
            .map(|package| format!("\"{package}\""))
            .collect::<Vec<_>>()
            .join(", ");
        self.write(
            &format!("boundaries/{boundary_directory}/boundary.toml"),
            &format!(
                "boundary_id = \"BOUNDARY-App\"\nowner_package = \"{owner_package}\"\nowner_crate_path = \"{owner_crate_path}\"\nname = \"App\"\n\n[public]\nfacade = \"App\"\n\n[implementation]\ntype = \"App\"\nmodule = \"{owner_crate_path}\"\nvisibility = \"public\"\nconstructor = \"none\"\n\n[composition]\nroots = [\"App\"]\n\n[dependencies]\nallowed_dependents = []\nallowed_dependencies = [{allowed_dependencies}]\nforbidden_edges = []\n\n[references]\nscope = \"outside_owner_crate\"\nforbidden = []\n\n[testing]\nallowed_test_double_paths = []\nforbidden_test_bypasses = []\n\n[enforcement]\nlint_rules = []\nreview_gates = []\n\n[status]\nstate = \"concrete_landed\"\n"
            ),
        );
    }

    fn configure_dependency_policy_fixture(&self) {
        self.write_workspace_root_with_members(&["crates/app", "crates/api"]);
        self.write_package_manifest_with_dependencies(
            "app",
            "[dependencies]\napi = { path = \"../api\", version = \"0.1.0\" }\n",
        );
        self.write_package_manifest("api");
        self.write_source("app", "lib.rs", "pub struct App;\n");
        self.write_source("api", "lib.rs", "pub struct Api;\n");
    }

    fn configure_disallowed_dependency(&self) {
        self.configure_dependency_policy_fixture();
        self.write_member_boundary_record("app", &[], &[], &[]);
        self.write_member_boundary_record("api", &[], &["app"], &[]);
    }

    fn configure_disallowed_dependent(&self) {
        self.configure_dependency_policy_fixture();
        self.write_member_boundary_record("app", &["api"], &[], &[]);
        self.write_member_boundary_record("api", &[], &[], &[]);
    }

    fn configure_forbidden_edge(&self) {
        self.configure_dependency_policy_fixture();
        self.write_member_boundary_record("app", &["api"], &[], &[("app", "api")]);
        self.write_member_boundary_record("api", &[], &["app"], &[]);
    }
}

struct MockBackend {
    _tempdir: TempDir,
    executable: PathBuf,
    search_path: OsString,
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
        let version = env!("CARGO_PKG_VERSION");
        let script = if cfg!(windows) {
            format!(
                "@echo off\r\nif \"%~1\"==\"--version\" (echo {tool} {version}) else (echo {payload})\r\n"
            )
        } else {
            format!(
                "#!/usr/bin/env sh\nif [ \"$1\" = \"--version\" ]; then printf '%s\\n' '{tool} {version}'; else printf '%s\\n' '{payload}'; fi\n"
            )
        };
        std::fs::write(&script_path, script).expect("mock backend script");
        set_mock_backend_permissions(&script_path);

        Self::with_path(tempdir, script_path)
    }

    fn install_native(tool: &str, payload: &Value) -> Self {
        let tempdir = TempDir::new().expect("temp dir");
        let source_path = tempdir.path().join("mock_backend.rs");
        let executable_name = if cfg!(windows) {
            format!("{tool}.exe")
        } else {
            tool.to_string()
        };
        let executable_path = tempdir.path().join(executable_name);
        let source = format!(
            "fn main() {{ use std::io::Write; std::io::stdout().write_all(&{:?}).unwrap(); }}\n",
            payload.to_string().as_bytes()
        );
        fs::write(&source_path, source).expect("mock backend Rust source");
        let compilation = ProcessCommand::new("rustc")
            .args(["--edition=2024"])
            .arg(&source_path)
            .arg("-o")
            .arg(&executable_path)
            .output()
            .expect("compile native mock backend");
        assert!(
            compilation.status.success(),
            "native mock backend compilation failed: {}",
            String::from_utf8_lossy(&compilation.stderr)
        );

        Self::with_path(tempdir, executable_path)
    }

    fn install_native_without_version(tool: &str) -> Self {
        let tempdir = TempDir::new().expect("temp dir");
        let source_path = tempdir.path().join("mock_backend.rs");
        let executable_name = if cfg!(windows) {
            format!("{tool}.exe")
        } else {
            tool.to_string()
        };
        let executable_path = tempdir.path().join(executable_name);
        let analysis_marker = tempdir.path().join("analysis-invoked");
        let backend_payload = serde_json::json!({
            "tool": tool,
            "findings": [],
            "status": "pass",
        })
        .to_string();
        let source = format!(
            "fn main() {{\n\
                let args: Vec<String> = std::env::args().skip(1).collect();\n\
                if args.iter().any(|arg| arg == \"--version\") {{\n\
                    eprintln!(\"error: unexpected argument '--version' found\");\n\
                    std::process::exit(2);\n\
                }} else {{\n\
                    std::fs::write({analysis_marker:?}, b\"analysis\").expect(\"write analysis marker\");\n\
                    print!(\"{{}}\", {backend_payload:?});\n\
                }}\n\
            }}\n"
        );
        fs::write(&source_path, source).expect("stale backend source");
        let compilation = ProcessCommand::new("rustc")
            .args(["--edition=2024"])
            .arg(&source_path)
            .arg("-o")
            .arg(&executable_path)
            .output()
            .expect("compile native stale backend stub");
        assert!(
            compilation.status.success(),
            "native stale backend compilation failed: {}",
            String::from_utf8_lossy(&compilation.stderr)
        );

        Self::with_path(tempdir, executable_path)
    }

    fn with_path(tempdir: TempDir, executable: PathBuf) -> Self {
        let original_path = std::env::var_os("PATH");
        let search_path = std::env::join_paths(
            std::iter::once(tempdir.path().to_path_buf()).chain(
                original_path
                    .as_ref()
                    .into_iter()
                    .flat_map(std::env::split_paths),
            ),
        )
        .expect("joined PATH");

        Self {
            _tempdir: tempdir,
            executable,
            search_path,
        }
    }

    fn path(&self) -> &Path {
        &self.executable
    }

    fn search_path(&self) -> &std::ffi::OsStr {
        &self.search_path
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

/// Test hook run on the step's worker thread.
type StepHook = Box<dyn Fn() + Send + Sync>;

/// What the fake recorded for one `run_step` call.
#[derive(Debug, Clone, PartialEq, Eq)]
struct StepRecord {
    name: String,
    argv: Vec<String>,
    env: Vec<(String, String)>,
    cwd: Option<PathBuf>,
}

struct FakeSystemAdapter {
    xwin_available: bool,
    host: crate::profile_plan::Platform,
    failures: HashMap<&'static str, &'static str>,
    /// Exit code reported (as the `exit_code` detail) by an injected failure.
    exit_codes: HashMap<&'static str, i32>,
    /// `start:<name>` / `end:<name>` in the order they happened.
    events: Mutex<Vec<String>>,
    /// Every `run_step` call, in run order.
    records: Mutex<Vec<StepRecord>>,
    /// Runs after the step is logged as started, before it completes.
    on_start: HashMap<&'static str, StepHook>,
    /// Runs when the step completes, before its result is returned.
    on_finish: HashMap<&'static str, StepHook>,
    invocations: Mutex<Vec<String>>,
    /// Program and argv of every planned (consumer lint) step, in run order.
    planned_argv: Mutex<Vec<Vec<String>>>,
    /// Step names in completion order.
    completions: Mutex<Vec<String>>,
}

impl FakeSystemAdapter {
    fn new(xwin_available: bool) -> Self {
        Self {
            xwin_available,
            host: crate::profile_plan::Platform::host(),
            failures: HashMap::new(),
            exit_codes: HashMap::new(),
            events: Mutex::new(Vec::new()),
            records: Mutex::new(Vec::new()),
            on_start: HashMap::new(),
            on_finish: HashMap::new(),
            planned_argv: Mutex::new(Vec::new()),
            invocations: Mutex::new(Vec::new()),
            completions: Mutex::new(Vec::new()),
        }
    }
}

impl FakeSystemAdapter {
    /// Injects the host the planner should plan for.
    fn on_host(mut self, host: crate::profile_plan::Platform) -> Self {
        self.host = host;
        self
    }

    fn record_step(&self, name: &str) -> Result<(), Box<CliError>> {
        self.invocations
            .lock()
            .expect("fake invocation log lock")
            .push(name.to_string());
        self.event("start", name);
        if let Some(hook) = self.on_start.get(name) {
            hook();
        }
        self.completions
            .lock()
            .expect("fake completion log lock")
            .push(name.to_string());
        self.event("end", name);
        if let Some(hook) = self.on_finish.get(name) {
            hook();
        }
        if let Some(message) = self.failures.get(name) {
            let mut error = CliError::backend_failure(*message);
            if let Some(code) = self.exit_codes.get(name) {
                error = error.with_detail(crate::consts::FIELD_EXIT_CODE, json!(code));
            }
            return Err(Box::new(error));
        }
        Ok(())
    }

    fn event(&self, what: &str, name: &str) {
        self.events
            .lock()
            .expect("fake event log lock")
            .push(format!("{what}:{name}"));
    }

    fn invoked(&self) -> Vec<String> {
        self.invocations
            .lock()
            .expect("fake invocation log lock")
            .clone()
    }

    fn records(&self) -> Vec<StepRecord> {
        self.records.lock().expect("fake record log lock").clone()
    }
}

impl workflow::SystemAdapter for FakeSystemAdapter {
    fn cargo_xwin_available(&self, _repo_root: &Path) -> bool {
        self.xwin_available
    }

    fn host_platform(&self) -> crate::profile_plan::Platform {
        self.host
    }

    fn run_planned_step(
        &self,
        _repo_root: &Path,
        step: &crate::profile_plan::PlannedStep,
    ) -> Result<workflow::StepReport, CliError> {
        self.record_step(step.name()).map_err(|error| *error)?;
        let argv = std::iter::once(step.command())
            .chain(step.args())
            .map(|part| part.to_string_lossy().into_owned())
            .collect();
        self.planned_argv
            .lock()
            .expect("fake planned argv log lock")
            .push(argv);
        Ok(
            workflow::StepReport::success(&workflow::StepPlan::from_planned(step))
                .for_planned(step),
        )
    }

    fn run_step(
        &self,
        _repo_root: &Path,
        step: &workflow::StepPlan,
    ) -> Result<workflow::StepReport, CliError> {
        let lossy = |value: &OsString| value.to_string_lossy().into_owned();
        self.records
            .lock()
            .expect("fake record log lock")
            .push(StepRecord {
                name: step.name().to_string(),
                argv: step.argv().map(lossy).collect(),
                env: step
                    .env()
                    .iter()
                    .map(|(key, value)| (lossy(key), lossy(value)))
                    .collect(),
                cwd: step.cwd().map(Path::to_path_buf),
            });
        self.record_step(step.name()).map_err(|error| *error)?;
        Ok(workflow::StepReport::success(step))
    }
}

// ---------------------------------------------------------------------------
// Consumer platform/profile planning and execution (Phase K, k-9).
// ---------------------------------------------------------------------------

const PROFILE_FIXTURES: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/platform-profile-execution"
);

fn profile_fixture(relative: &str) -> PathBuf {
    Path::new(PROFILE_FIXTURES).join(relative)
}

fn plan_step(
    name: &str,
    command: &[&str],
    platform: Option<Platform>,
    target: Option<&str>,
) -> ConsumerProfileStep {
    ConsumerProfileStep::new(
        name,
        command.iter().map(|part| (*part).to_string()).collect(),
        platform,
        target.map(|triple| Target::parse(triple.to_string()).expect("valid target triple")),
    )
}

fn profile_name(value: &str) -> ProfileName {
    ProfileName::parse(value.to_string()).expect("valid profile name")
}

fn decision_labels(decisions: &[PlanDecision]) -> Vec<String> {
    decisions
        .iter()
        .map(|decision| match decision {
            PlanDecision::Run(step) => format!("run:{}", step.name()),
            PlanDecision::Skip(record) => format!("{}:{}", record.reason.as_str(), record.name),
        })
        .collect()
}

fn argv_of(step: &PlannedStep) -> Vec<String> {
    std::iter::once(step.command())
        .chain(step.args())
        .map(|part| part.to_string_lossy().into_owned())
        .collect()
}

fn planned_argv(decisions: &[PlanDecision], name: &str) -> Vec<String> {
    decisions
        .iter()
        .find_map(|decision| match decision {
            PlanDecision::Run(step) if step.name() == name => Some(argv_of(step)),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no planned run for step `{name}`"))
}

fn run_label(name: &str) -> String {
    format!("run:{name}")
}

fn cross_command_label(name: &str) -> String {
    format!("unsupported_cross_command:{name}")
}

fn cross_target_label(name: &str) -> String {
    format!("unsupported_cross_target:{name}")
}

fn matrix_steps() -> Vec<ConsumerProfileStep> {
    let msvc = "x86_64-pc-windows-msvc";
    vec![
        plan_step("plain", &["cargo", "fmt", "--check"], None, None),
        plan_step("linux-tool", &["./check.sh"], Some(Platform::Linux), None),
        plan_step("mac-tool", &["./check.sh"], Some(Platform::MacOS), None),
        plan_step("win-tool", &["check.cmd"], Some(Platform::Windows), None),
        plan_step(
            "linux-check",
            &["cargo", "check"],
            Some(Platform::Linux),
            Some("x86_64-unknown-linux-gnu"),
        ),
        plan_step(
            "mac-check",
            &["cargo", "check"],
            Some(Platform::MacOS),
            Some("aarch64-apple-darwin"),
        ),
        plan_step(
            "win-check",
            &["cargo", "clippy", "--workspace", "--", "-D", "warnings"],
            Some(Platform::Windows),
            Some(msvc),
        ),
        plan_step(
            "win-gnu",
            &["cargo", "check"],
            Some(Platform::Windows),
            Some("x86_64-pc-windows-gnu"),
        ),
    ]
}

type MatrixRow = (Platform, PlatformScope, Vec<String>);

fn labels(items: &[&str]) -> Vec<String> {
    items
        .iter()
        .map(|item| match item.split_once(':') {
            Some(("run", name)) => run_label(name),
            Some(("cmd", name)) => cross_command_label(name),
            Some(("tgt", name)) => cross_target_label(name),
            _ => panic!("bad expectation label `{item}`"),
        })
        .collect()
}

/// Expected decisions for every scope on a Linux host over [`matrix_steps`].
fn linux_matrix_rows() -> Vec<MatrixRow> {
    use Platform::{Linux, MacOS, Windows};
    let host = labels(&["run:plain", "run:linux-tool", "run:linux-check"]);
    vec![
        (Linux, PlatformScope::Host, host.clone()),
        (Linux, PlatformScope::Named(Linux), host),
        (
            Linux,
            PlatformScope::All,
            labels(&[
                "run:plain",
                "run:linux-tool",
                "cmd:mac-tool",
                "cmd:win-tool",
                "run:linux-check",
                "tgt:mac-check",
                "run:win-check",
                "tgt:win-gnu",
            ]),
        ),
        (
            Linux,
            PlatformScope::Named(MacOS),
            labels(&[
                "run:plain",
                "run:linux-tool",
                "cmd:mac-tool",
                "run:linux-check",
                "tgt:mac-check",
            ]),
        ),
        (
            Linux,
            PlatformScope::Named(Windows),
            labels(&[
                "run:plain",
                "run:linux-tool",
                "cmd:win-tool",
                "run:linux-check",
                "run:win-check",
                "tgt:win-gnu",
            ]),
        ),
    ]
}

/// Expected decisions for every scope on a macOS host over [`matrix_steps`].
fn macos_matrix_rows() -> Vec<MatrixRow> {
    use Platform::{Linux, MacOS, Windows};
    let host = labels(&["run:plain", "run:mac-tool", "run:mac-check"]);
    vec![
        (MacOS, PlatformScope::Host, host.clone()),
        (MacOS, PlatformScope::Named(MacOS), host),
        (
            MacOS,
            PlatformScope::All,
            labels(&[
                "run:plain",
                "cmd:linux-tool",
                "run:mac-tool",
                "cmd:win-tool",
                "tgt:linux-check",
                "run:mac-check",
                "run:win-check",
                "tgt:win-gnu",
            ]),
        ),
        (
            MacOS,
            PlatformScope::Named(Linux),
            labels(&[
                "run:plain",
                "cmd:linux-tool",
                "run:mac-tool",
                "tgt:linux-check",
                "run:mac-check",
            ]),
        ),
        (
            MacOS,
            PlatformScope::Named(Windows),
            labels(&[
                "run:plain",
                "run:mac-tool",
                "cmd:win-tool",
                "run:mac-check",
                "run:win-check",
                "tgt:win-gnu",
            ]),
        ),
    ]
}

/// Expected decisions for every scope on a Windows host over [`matrix_steps`].
fn windows_matrix_rows() -> Vec<MatrixRow> {
    use Platform::{Linux, MacOS, Windows};
    let host = labels(&["run:plain", "run:win-tool", "run:win-check", "run:win-gnu"]);
    vec![
        (Windows, PlatformScope::Host, host.clone()),
        (Windows, PlatformScope::Named(Windows), host),
        (
            Windows,
            PlatformScope::All,
            labels(&[
                "run:plain",
                "cmd:linux-tool",
                "cmd:mac-tool",
                "run:win-tool",
                "tgt:linux-check",
                "tgt:mac-check",
                "run:win-check",
                "run:win-gnu",
            ]),
        ),
        (
            Windows,
            PlatformScope::Named(Linux),
            labels(&[
                "run:plain",
                "cmd:linux-tool",
                "run:win-tool",
                "tgt:linux-check",
                "run:win-check",
                "run:win-gnu",
            ]),
        ),
        (
            Windows,
            PlatformScope::Named(MacOS),
            labels(&[
                "run:plain",
                "cmd:mac-tool",
                "run:win-tool",
                "tgt:mac-check",
                "run:win-check",
                "run:win-gnu",
            ]),
        ),
    ]
}

fn matrix_expectations() -> Vec<MatrixRow> {
    let mut rows = linux_matrix_rows();
    rows.extend(macos_matrix_rows());
    rows.extend(windows_matrix_rows());
    rows
}

#[test]
fn test_profile_platform_scope_matrix() {
    let steps = matrix_steps();
    let expectations = matrix_expectations();
    assert_eq!(expectations.len(), 15, "every host crosses every scope");
    for (host, scope, expected) in expectations {
        let decisions = plan_consumer_lint(&steps, None, scope, host)
            .unwrap_or_else(|error| panic!("{host} {scope:?}: {error}"));
        assert_eq!(decision_labels(&decisions), expected, "{host} {scope:?}");
    }

    // Execution matrix: argv for the same declarations on each host.
    let linux =
        plan_consumer_lint(&steps, None, PlatformScope::All, Platform::Linux).expect("linux plan");
    let windows = plan_consumer_lint(&steps, None, PlatformScope::All, Platform::Windows)
        .expect("windows plan");
    let clippy = |xwin: &[&str]| -> Vec<String> {
        let mut argv = vec!["cargo"];
        argv.extend(xwin);
        argv.extend([
            "clippy",
            "--workspace",
            "--target",
            "x86_64-pc-windows-msvc",
        ]);
        argv.extend(["--", "-D", "warnings"]);
        argv.into_iter().map(str::to_string).collect()
    };
    assert_eq!(planned_argv(&linux, "win-check"), clippy(&["xwin"]));
    assert_eq!(planned_argv(&windows, "win-check"), clippy(&[]));
    assert_eq!(
        planned_argv(&windows, "win-gnu"),
        ["cargo", "check", "--target", "x86_64-pc-windows-gnu"]
    );
    assert_eq!(
        planned_argv(&linux, "plain"),
        ["cargo", "fmt", "--check"],
        "untagged steps keep their declared argv"
    );
    let macos =
        plan_consumer_lint(&steps, None, PlatformScope::Host, Platform::MacOS).expect("macos plan");
    assert_eq!(
        planned_argv(&macos, "mac-check"),
        ["cargo", "check", "--target", "aarch64-apple-darwin"],
        "same-host targets use plain cargo"
    );
    let xwin = linux.iter().find_map(|decision| match decision {
        PlanDecision::Run(step) if step.name() == "win-check" => Some(step),
        _ => None,
    });
    assert!(xwin.expect("xwin step").uses_cargo_xwin());
    assert_eq!(
        xwin.and_then(PlannedStep::cargo_verb),
        Some(CargoVerb::Clippy)
    );
}

#[test]
fn test_profile_targeted_commands_require_literal_cargo() {
    let steps = [plan_step(
        "win-check",
        &["cargo", "check", "--workspace"],
        Some(Platform::Windows),
        Some("x86_64-pc-windows-msvc"),
    )];

    let windows = plan_consumer_lint(&steps, None, PlatformScope::All, Platform::Windows)
        .expect("windows plan");
    assert_eq!(
        planned_argv(&windows, "win-check"),
        [
            "cargo".to_string(),
            "check".to_string(),
            "--workspace".to_string(),
            "--target".to_string(),
            "x86_64-pc-windows-msvc".to_string(),
        ]
    );

    let unix =
        plan_consumer_lint(&steps, None, PlatformScope::All, Platform::Linux).expect("unix plan");
    assert_eq!(
        planned_argv(&unix, "win-check"),
        [
            "cargo".to_string(),
            "xwin".to_string(),
            "check".to_string(),
            "--workspace".to_string(),
            "--target".to_string(),
            "x86_64-pc-windows-msvc".to_string(),
        ]
    );
    assert_eq!(
        CargoVerb::from_argv(&["cargo.cmd".to_string(), "check".to_string()]),
        None
    );
    assert_eq!(
        CargoVerb::from_argv(&["tools/cargo.exe".to_string(), "check".to_string()]),
        None
    );
}

fn consumer_toml(rows: &str) -> String {
    format!(
        "[tool.sc-lint]\nminimum_version = \"0.4.0\"\n\n{rows}\n[[tool.sc-lint.test]]\nname = \"test\"\ncommand = [\"cargo\", \"test\"]\n"
    )
}

fn load_consumer_toml(text: &str) -> (TempDir, Result<LoadedConfig, CliError>) {
    let fixture = TempDir::new().expect("fixture");
    let config_path = fixture.path().join("sc-lint.toml");
    fs::write(&config_path, text).expect("consumer config");
    let cli = Cli::parse_from([
        "sc-lint",
        "--config",
        config_path.to_str().expect("config path"),
        "lint",
        "ci",
        "--consumer",
    ]);
    let context = CommandContext::from_cli(&cli).expect("consumer context");
    let loaded = LoadedConfig::load(&cli, &context);
    (fixture, loaded)
}

fn lint_rows(names: &[&str]) -> String {
    names
        .iter()
        .map(|name| {
            format!(
                "[[tool.sc-lint.lint]]\nname = \"{name}\"\ncommand = [\"cargo\", \"--version\"]\n\n"
            )
        })
        .collect()
}

fn selected_names(loaded: &LoadedConfig, selector: Option<&str>) -> Vec<String> {
    selected_names_for_host(loaded, selector, Platform::Linux)
}

fn selected_names_for_host(
    loaded: &LoadedConfig,
    selector: Option<&str>,
    host: Platform,
) -> Vec<String> {
    let input = loaded.consumer_lint_inputs(selector).expect("selection");
    let decisions = plan_consumer_lint(
        &input.steps,
        input.selector.as_ref(),
        PlatformScope::Host,
        host,
    )
    .expect("plan");
    decisions
        .iter()
        .map(|decision| match decision {
            PlanDecision::Run(step) => step.name().to_string(),
            PlanDecision::Skip(record) => record.name.clone(),
        })
        .collect()
}

#[test]
fn test_profile_selection_order_and_no_duplicates() {
    let rows = lint_rows(&["a", "b", "c", "d", "e"]);
    let text = consumer_toml(&format!(
        "[tool.sc-lint.lint_profiles]\npick = [\"e\", \"b\", \"d\"]\nsingle = [\"c\"]\n\n{rows}"
    ));
    let (_fixture, loaded) = load_consumer_toml(&text);
    let loaded = loaded.expect("valid profile configuration");

    // A named profile selects its own order, not declaration order.
    assert_eq!(selected_names(&loaded, Some("pick")), ["e", "b", "d"]);
    assert_eq!(selected_names(&loaded, Some("single")), ["c"]);
    // A step name selects exactly that step; `all` and absent keep declaration order.
    assert_eq!(selected_names(&loaded, Some("c")), ["c"]);
    assert_eq!(
        selected_names(&loaded, Some("all")),
        ["a", "b", "c", "d", "e"]
    );
    assert_eq!(selected_names(&loaded, None), ["a", "b", "c", "d", "e"]);

    // The planner itself never returns a step twice and rejects duplicate declarations.
    let twice = vec![
        plan_step("a", &["cargo", "check"], None, None),
        plan_step("a", &["cargo", "check"], None, None),
    ];
    let error = plan_consumer_lint(&twice, None, PlatformScope::Host, Platform::Linux)
        .expect_err("duplicate declarations are rejected");
    assert_eq!(error.detail_code, ProfilePlanDetailCode::DuplicateMember);
    let unique = vec![
        plan_step("a", &["cargo", "check"], None, None),
        plan_step("b", &["cargo", "check"], None, None),
    ];
    let decisions = plan_consumer_lint(&unique, None, PlatformScope::All, Platform::Linux)
        .expect("unique plan");
    assert_eq!(decision_labels(&decisions), ["run:a", "run:b"]);
    let only_b = plan_consumer_lint(
        &unique,
        Some(&profile_name("b")),
        PlatformScope::All,
        Platform::Linux,
    )
    .expect("single-step selection");
    assert_eq!(decision_labels(&only_b), ["run:b"]);
    let unknown = plan_consumer_lint(
        &unique,
        Some(&profile_name("zzz")),
        PlatformScope::All,
        Platform::Linux,
    )
    .expect_err("unknown selector");
    assert_eq!(unknown.detail_code, ProfilePlanDetailCode::UnknownMember);
}

fn invalid_profile_cases() -> Vec<(&'static str, String, &'static str)> {
    let rows = lint_rows(&["fmt", "spell"]);
    let profiles = |table: &str| format!("[tool.sc-lint.lint_profiles]\n{table}\n\n{rows}");
    vec![
        ("empty", profiles("fast = []"), "PROFILE.EMPTY_MEMBERS"),
        (
            "dup",
            profiles("fast = [\"fmt\", \"fmt\"]"),
            "PROFILE.DUPLICATE_MEMBER",
        ),
        (
            "unknown",
            profiles("fast = [\"fmt\", \"nope\"]"),
            "PROFILE.UNKNOWN_MEMBER",
        ),
        (
            "step-collision",
            profiles("fmt = [\"spell\"]"),
            "PROFILE.AMBIGUOUS_NAME",
        ),
        (
            "reserved",
            profiles("all = [\"fmt\"]"),
            "PROFILE.AMBIGUOUS_NAME",
        ),
    ]
}

fn invalid_step_cases() -> Vec<(&'static str, String, &'static str)> {
    let rows = lint_rows(&["fmt", "spell"]);
    let msvc = "x86_64-pc-windows-msvc";
    let targeted = |command: &str, platform: &str, target: &str| {
        format!(
            "{rows}[[tool.sc-lint.lint]]\nname = \"w\"\ncommand = {command}\nplatform = \"{platform}\"\ntarget = \"{target}\"\n"
        )
    };
    vec![
        (
            "unknown-field",
            format!(
                "{rows}[[tool.sc-lint.lint]]\nname = \"w\"\ncommand = [\"cargo\", \"check\"]\nplatfrom = \"linux\"\n"
            ),
            "PROFILE.UNKNOWN_STEP_FIELD",
        ),
        (
            "bad-platform",
            format!(
                "{rows}[[tool.sc-lint.lint]]\nname = \"w\"\ncommand = [\"cargo\", \"check\"]\nplatform = \"beos\"\n"
            ),
            "PROFILE.UNKNOWN_STEP_FIELD",
        ),
        (
            "target-without-platform",
            format!(
                "{rows}[[tool.sc-lint.lint]]\nname = \"w\"\ncommand = [\"cargo\", \"check\"]\ntarget = \"{msvc}\"\n"
            ),
            "PROFILE.TARGET_REQUIRES_PLATFORM",
        ),
        (
            "os-mismatch",
            targeted("[\"cargo\", \"check\"]", "linux", msvc),
            "PROFILE.TARGET_OS_MISMATCH",
        ),
        (
            "malformed-target",
            targeted("[\"cargo\", \"check\"]", "windows", "Not A Triple"),
            "PROFILE.TARGET_OS_MISMATCH",
        ),
        (
            "wrapper",
            targeted("[\"just\", \"check\"]", "windows", msvc),
            "PROFILE.UNSUPPORTED_COMMAND_SHAPE",
        ),
        (
            "embedded-target",
            targeted(
                "[\"cargo\", \"check\", \"--target=x86_64-pc-windows-msvc\"]",
                "windows",
                msvc,
            ),
            "PROFILE.UNSUPPORTED_COMMAND_SHAPE",
        ),
        (
            "other-verb",
            targeted("[\"cargo\", \"build\"]", "windows", msvc),
            "PROFILE.UNSUPPORTED_COMMAND_SHAPE",
        ),
        (
            "toolchain-override",
            targeted("[\"cargo\", \"+nightly\", \"check\"]", "windows", msvc),
            "PROFILE.UNSUPPORTED_COMMAND_SHAPE",
        ),
    ]
}

fn invalid_test_step_cases() -> Vec<(&'static str, String, &'static str)> {
    let test_row = |field: &str| {
        format!(
            "[[tool.sc-lint.test]]\nname = \"unit\"\ncommand = [\"cargo\", \"test\"]\n{field}\n"
        )
    };
    vec![
        ("platform", test_row("platform = \"windows\""), "platform"),
        (
            "target",
            test_row("target = \"x86_64-pc-windows-msvc\""),
            "target",
        ),
        ("unknown", test_row("parallel = true"), "parallel"),
    ]
}

#[test]
fn test_profile_rejects_invalid_members_and_targets() {
    for (label, rows, detail_code) in invalid_profile_cases()
        .into_iter()
        .chain(invalid_step_cases())
    {
        let (_fixture, loaded) = load_consumer_toml(&consumer_toml(&rows));
        let error = loaded.expect_err(label);
        assert_eq!(error.code(), PROFILE_PLAN_ERROR_CODE, "{label}");
        assert_eq!(error.kind, CliErrorKind::Config, "{label}");
        assert_eq!(error.details["detail_code"], detail_code, "{label}");
        assert!(error.suggested_action.is_some(), "{label}: recovery");
        assert!(error.cause.is_some(), "{label}: cause");
        assert!(error.documentation.is_some(), "{label}: docs");
    }

    // A profile is a flat list of step names: nesting is a shape error, not a panic.
    let rows = lint_rows(&["fmt"]);
    let (_fixture, loaded) = load_consumer_toml(&consumer_toml(&format!(
        "[tool.sc-lint.lint_profiles]\nfast = [[\"fmt\"]]\n\n{rows}"
    )));
    assert_eq!(
        loaded.expect_err("nested profile").code(),
        "CLI.SC_LINT_CONFIG_MALFORMED"
    );

    // The planner API returns structured errors for invalid input, never a panic.
    let orphan = [plan_step(
        "w",
        &["cargo", "check"],
        None,
        Some("x86_64-pc-windows-msvc"),
    )];
    let error = plan_consumer_lint(&orphan, None, PlatformScope::All, Platform::Linux)
        .expect_err("target needs a platform");
    assert_eq!(
        error.detail_code,
        ProfilePlanDetailCode::TargetRequiresPlatform
    );
    assert_eq!(error.code, PROFILE_PLAN_ERROR_CODE);
    assert_eq!(
        ProfileName::parse(String::new())
            .expect_err("empty")
            .detail_code,
        ProfilePlanDetailCode::AmbiguousName
    );
    assert!(Target::parse("not a triple".to_string()).is_err());
    assert_eq!(
        CargoVerb::from_argv(&["cargo".to_string(), "build".to_string()]),
        None
    );
    let all_codes = [
        ProfilePlanDetailCode::EmptyMembers,
        ProfilePlanDetailCode::DuplicateMember,
        ProfilePlanDetailCode::UnknownMember,
        ProfilePlanDetailCode::AmbiguousName,
        ProfilePlanDetailCode::UnknownStepField,
        ProfilePlanDetailCode::InvalidStepEnv,
        ProfilePlanDetailCode::InvalidStepCwd,
        ProfilePlanDetailCode::TargetRequiresPlatform,
        ProfilePlanDetailCode::TargetOsMismatch,
        ProfilePlanDetailCode::UnsupportedCommandShape,
    ];
    assert_eq!(
        all_codes.map(ProfilePlanDetailCode::as_str),
        [
            "PROFILE.EMPTY_MEMBERS",
            "PROFILE.DUPLICATE_MEMBER",
            "PROFILE.UNKNOWN_MEMBER",
            "PROFILE.AMBIGUOUS_NAME",
            "PROFILE.UNKNOWN_STEP_FIELD",
            "PROFILE.INVALID_STEP_ENV",
            "PROFILE.INVALID_STEP_CWD",
            "PROFILE.TARGET_REQUIRES_PLATFORM",
            "PROFILE.TARGET_OS_MISMATCH",
            "PROFILE.UNSUPPORTED_COMMAND_SHAPE",
        ]
    );
}

#[test]
fn consumer_profile_steps_require_unique_names_and_non_empty_argv() {
    let cases = [
        (
            "duplicate name",
            "[[tool.sc-lint.lint]]\nname = \"fmt\"\ncommand = [\"cargo\", \"fmt\"]\n\n[[tool.sc-lint.lint]]\nname = \"fmt\"\ncommand = [\"cargo\", \"check\"]\n",
            "repeats step name `fmt`",
        ),
        (
            "missing name",
            "[[tool.sc-lint.lint]]\ncommand = [\"cargo\", \"fmt\"]\n",
            "missing a non-empty `name`",
        ),
        (
            "empty argv",
            "[[tool.sc-lint.lint]]\nname = \"fmt\"\ncommand = []\n",
            "missing a command argv array",
        ),
        (
            "empty argument",
            "[[tool.sc-lint.lint]]\nname = \"fmt\"\ncommand = [\"cargo\", \"\"]\n",
            "empty command argument",
        ),
    ];

    for (label, rows, cause) in cases {
        let (_fixture, loaded) = load_consumer_toml(&consumer_toml(rows));
        let error = loaded.expect_err(label);
        assert_eq!(error.code(), "CLI.SC_LINT_CONFIG_MALFORMED", "{label}");
        assert_eq!(error.kind, CliErrorKind::Config, "{label}");
        assert!(error.message.contains(cause), "{label}: {error:?}");
    }
}

#[test]
fn test_profile_rejects_unsupported_test_row_fields() {
    for (label, rows, field) in invalid_test_step_cases() {
        let rows = format!("{}\n{rows}", lint_rows(&["fmt"]));
        let (_fixture, loaded) = load_consumer_toml(&consumer_toml(&rows));
        let error = loaded.expect_err(label);
        assert_eq!(error.code(), PROFILE_PLAN_ERROR_CODE, "{label}");
        assert_eq!(error.kind, CliErrorKind::Config, "{label}");
        assert_eq!(
            error.details["detail_code"], "PROFILE.UNKNOWN_STEP_FIELD",
            "{label}"
        );
        assert_eq!(error.details["field"], field, "{label}");
        assert_eq!(error.details["step"], "unit", "{label}");
    }
}

#[test]
fn lint_step_env_and_cwd_plan() {
    let rows = "[[tool.sc-lint.lint]]\nname = \"adapted\"\ncommand = [\"cargo\", \"check\"]\nplatform = \"windows\"\ntarget = \"x86_64-pc-windows-msvc\"\nenv = { PYO3_PYTHON = \".bootstrap-venv/bin/python\" }\ncwd = \"crates/x\"\n";
    let (_fixture, loaded) = load_consumer_toml(&consumer_toml(rows));
    let loaded = loaded.expect("lint step execution config");
    let input = loaded.consumer_lint_inputs(None).expect("lint input");
    let decisions = crate::profile_plan::plan_consumer_lint(
        &input.steps,
        None,
        PlatformScope::All,
        crate::profile_plan::Platform::Linux,
    )
    .expect("plan adapted step");
    let crate::profile_plan::PlanDecision::Run(planned) = &decisions[0] else {
        panic!("Windows xwin target should run on Linux");
    };
    assert!(planned.uses_cargo_xwin());
    let xwin = crate::workflow::StepPlan::from_planned(planned);
    assert_eq!(
        xwin.env(),
        &[(
            OsString::from("PYO3_PYTHON"),
            OsString::from(".bootstrap-venv/bin/python"),
        )]
    );
    assert_eq!(xwin.cwd(), Some(Path::new("crates/x")));
    assert_eq!(
        xwin.display_command(),
        "cargo xwin check --target x86_64-pc-windows-msvc"
    );

    let windows_decisions = crate::profile_plan::plan_consumer_lint(
        &input.steps,
        None,
        PlatformScope::All,
        crate::profile_plan::Platform::Windows,
    )
    .expect("plan native Windows target");
    let crate::profile_plan::PlanDecision::Run(native) = &windows_decisions[0] else {
        panic!("Windows host should run its native target");
    };
    let native = crate::workflow::StepPlan::from_planned(native);
    assert_eq!(
        native.env(),
        &[(
            OsString::from("PYO3_PYTHON"),
            OsString::from(".bootstrap-venv/bin/python"),
        )]
    );
    assert_eq!(native.cwd(), Some(Path::new("crates/x")));
    assert_eq!(
        native.display_command(),
        "cargo check --target x86_64-pc-windows-msvc"
    );
}

#[test]
fn lint_step_env_cwd_errors() {
    let cases = [
        (
            "bad env table",
            "env = \"bad\"",
            "env",
            "PROFILE.INVALID_STEP_ENV",
            "lint step `invalid` env must be a table of strings",
        ),
        (
            "non-string env value",
            "env = { PYO3_PYTHON = 7 }",
            "env.PYO3_PYTHON",
            "PROFILE.INVALID_STEP_ENV",
            "lint step `invalid` env must be a table of strings",
        ),
        (
            "empty env key",
            r#"env = { "" = "value" }"#,
            "env.",
            "PROFILE.INVALID_STEP_ENV",
            "lint step `invalid` env key `` is empty or contains `=` or NUL",
        ),
        (
            "equals env key",
            r#"env = { "BAD=KEY" = "value" }"#,
            "env.BAD=KEY",
            "PROFILE.INVALID_STEP_ENV",
            "lint step `invalid` env key `BAD=KEY` is empty or contains `=` or NUL",
        ),
        (
            "NUL env key",
            r#"env = { "BAD\u0000KEY" = "value" }"#,
            "env.BAD\0KEY",
            "PROFILE.INVALID_STEP_ENV",
            "lint step `invalid` env key `BAD\0KEY` is empty or contains `=` or NUL",
        ),
        (
            "escaping cwd",
            "cwd = \"../outside\"",
            "cwd",
            "PROFILE.INVALID_STEP_CWD",
            "lint step `invalid` cwd must be a non-empty repo-root-relative path without `..`",
        ),
        (
            "empty cwd",
            "cwd = \"\"",
            "cwd",
            "PROFILE.INVALID_STEP_CWD",
            "lint step `invalid` cwd must be a non-empty repo-root-relative path without `..`",
        ),
        (
            "absolute cwd",
            "cwd = \"/outside\"",
            "cwd",
            "PROFILE.INVALID_STEP_CWD",
            "lint step `invalid` cwd must be a non-empty repo-root-relative path without `..`",
        ),
        (
            "non-string cwd",
            "cwd = 7",
            "cwd",
            "PROFILE.INVALID_STEP_CWD",
            "lint step `invalid` cwd must be a non-empty repo-root-relative path without `..`",
        ),
    ];
    for (label, field_line, field, detail_code, message) in cases {
        let rows = format!(
            "[[tool.sc-lint.lint]]\nname = \"invalid\"\ncommand = [\"true\"]\n{field_line}\n"
        );
        let (_fixture, loaded) = load_consumer_toml(&consumer_toml(&rows));
        let error = loaded.expect_err(label);
        assert_eq!(error.code(), PROFILE_PLAN_ERROR_CODE);
        assert_eq!(error.details["detail_code"], detail_code);
        assert_eq!(error.details["field"], field);
        assert_eq!(error.details["step"], "invalid");
        assert_eq!(
            error.message,
            format!("consumer lint profile configuration is invalid: {message}"),
            "{label}"
        );
    }
}

fn load_named_membership() -> (TempDir, LoadedConfig) {
    let text = fs::read_to_string(profile_fixture("named-membership.toml")).expect("fixture");
    let fixture = TempDir::new().expect("fixture dir");
    let config_path = fixture.path().join("sc-lint.toml");
    fs::write(&config_path, text).expect("consumer config");
    let cli = Cli::parse_from([
        "sc-lint",
        "--config",
        config_path.to_str().expect("config path"),
        "lint",
        "ci",
        "--consumer",
    ]);
    let context = CommandContext::from_cli(&cli).expect("consumer context");
    let loaded = LoadedConfig::load(&cli, &context).expect("named membership config loads");
    (fixture, loaded)
}

#[expect(
    clippy::result_large_err,
    reason = "Tests surface the shared CliError contract unchanged."
)]
fn lint_rows_for(
    loaded: &LoadedConfig,
    selector: Option<&str>,
    scope: PlatformScope,
    adapter: &FakeSystemAdapter,
) -> Result<Vec<Value>, CliError> {
    let success = workflow::run_consumer_lint_with(
        loaded,
        selector,
        scope,
        FailurePolicy::KeepGoing,
        adapter,
    )?;
    Ok(success.data["steps"]
        .as_array()
        .expect("steps array")
        .clone())
}

// Case mapping to atm-core c1f3ee7a: `.just/run_lint.py` (`resolve_task_names`,
// `FAST_LINT_ORDER`) and `.just/tests/test_run_lint.py`. The three tests below
// port the upstream cases that name-resolve the lint task set; the fixture in
// `named-membership.toml` supplies harmless version-probe commands in place of
// the upstream lint scripts, so only task names and selection outcomes carry over.
//
// 1. upstream `test_resolve_task_names_all_includes_new_targets`
//    input:   resolve_task_names("all")
//    outcome: contains 22 names, including boundaries, manifests, spell, pytests,
//             sc-boundary, sc-portability, and read-concurrency-gates.
//    here:    `all` and the default selector both yield the declared order
//             [fmt, clippy, version, manifests, spell, pytests].
//    divergence: membership becomes an exact-order check because the fixture
//             declares six steps. Upstream's 22 names are not reproduced. The
//             windows-tagged probe joins only on a Windows host.
//
// 2. upstream `test_resolve_task_names_rejects_unknown_target`
//    input:   resolve_task_names("unknown")
//    outcome: ValueError listing the valid targets.
//    here:    selector "not-a-step-or-profile" is rejected with CliErrorKind::Config,
//             details.selector echoes the input, and details.available lists
//             "fast" and "manifests".
//    divergence: the input string differs; "not-a-step-or-profile" is neither a
//             step nor a profile in the fixture. The error is a typed CLI error,
//             not ValueError. The rejection outcome is the same.
//
// 3. upstream `test_resolve_task_names_fast_is_low_latency_subset`
//    input:   resolve_task_names("fast")
//    outcome: the 15-name FAST_LINT_ORDER [fmt, version, boundaries, manifests,
//             daemon-signing-coupling, shear, silent-emit, runtime-stderr,
//             function-length, legacy-mailbox-paths, nudge-taxonomy,
//             capability-degradation, spell, hermes-adapter, pytests].
//    here:    the `fast` profile is [fmt, version, manifests, spell, pytests]
//             and excludes clippy.
//    divergence: intentional subset. The fixture's `fast` profile is not
//             upstream's 15 names. The `ci` profile ([spell, fmt]) has no
//             upstream counterpart; on a Linux host its windows-probe step is
//             not selected, so the expected value is fixture-derived.
//
// Not ported: upstream `test_resolve_task_names_accepts_sc_boundary_manual_target`
// and `..._accepts_sc_portability_manual_target` (single-name selection of
// `sc-boundary` and `sc-portability`). The fixture declares neither step.
#[test]
fn test_resolve_task_names_all_includes_new_targets() {
    let (_fixture, loaded) = load_named_membership();
    let all = selected_names(&loaded, Some("all"));
    for name in ["fmt", "version", "manifests", "spell", "pytests"] {
        assert!(
            all.contains(&name.to_string()),
            "`all` lacks {name}: {all:?}"
        );
    }
    assert_eq!(all, selected_names(&loaded, None));
    // Declaration order is preserved for `all`; the windows-tagged probe only
    // joins on a Windows host, so it is omitted for this Linux host plan.
    assert_eq!(
        all,
        ["fmt", "clippy", "version", "manifests", "spell", "pytests"]
    );
}

#[test]
fn test_resolve_task_names_rejects_unknown_target() {
    let (_fixture, loaded) = load_named_membership();
    let error = loaded
        .consumer_lint_inputs(Some("not-a-step-or-profile"))
        .expect_err("unknown selector is rejected");
    assert_eq!(error.kind, CliErrorKind::Config);
    assert_eq!(error.details["selector"], "not-a-step-or-profile");
    let available = error.details["available"].to_string();
    assert!(
        available.contains("fast") && available.contains("manifests"),
        "{available}"
    );
}

#[test]
fn test_resolve_task_names_fast_is_low_latency_subset() {
    let (_fixture, loaded) = load_named_membership();
    assert_eq!(
        selected_names(&loaded, Some("fast")),
        ["fmt", "version", "manifests", "spell", "pytests"]
    );
    assert_eq!(selected_names(&loaded, Some("ci")), ["spell", "fmt"]);
    assert!(!selected_names(&loaded, Some("fast")).contains(&"clippy".to_string()));
}

#[test]
fn test_profile_emits_structured_skips_and_skip_only_success() {
    let rows = lint_rows(&["fmt"]);
    let text = consumer_toml(&format!(
        "{rows}[[tool.sc-lint.lint]]\nname = \"mac-only\"\ncommand = [\"./mac.sh\"]\nplatform = \"macos\"\n\n\
         [[tool.sc-lint.lint]]\nname = \"win-cross\"\ncommand = [\"cargo\", \"check\"]\nplatform = \"windows\"\ntarget = \"x86_64-pc-windows-gnu\"\n"
    ));
    let (_fixture, loaded) = load_consumer_toml(&text);
    let loaded = loaded.expect("config loads");
    let linux = FakeSystemAdapter::new(true).on_host(Platform::Linux);
    let rows = lint_rows_for(&loaded, None, PlatformScope::All, &linux).expect("plan runs");
    assert_eq!(rows.len(), 3, "{rows:?}");
    assert_eq!(rows[0]["status"], "pass");
    assert_eq!(
        rows[1],
        json!({
            "status": "skipped", "name": "mac-only", "host": "linux",
            "platform": "macos", "target": null, "reason": "unsupported_cross_command",
        })
    );
    assert_eq!(
        rows[2],
        json!({
            "status": "skipped", "name": "win-cross", "host": "linux",
            "platform": "windows", "target": "x86_64-pc-windows-gnu",
            "reason": "unsupported_cross_target",
        })
    );
    assert_eq!(
        linux
            .invocations
            .lock()
            .expect("fake invocation log lock")
            .as_slice(),
        ["fmt"],
        "skipped steps never run"
    );

    // Selecting only skipped steps is a successful run with no executions.
    let only = FakeSystemAdapter::new(false).on_host(Platform::Linux);
    let skipped = lint_rows_for(&loaded, Some("mac-only"), PlatformScope::All, &only)
        .expect("skip-only run succeeds");
    assert_eq!(skipped.len(), 1, "{skipped:?}");
    assert_eq!(skipped[0]["status"], "skipped");
    assert_eq!(skipped[0]["reason"], "unsupported_cross_command");
    assert!(
        only.invocations
            .lock()
            .expect("fake invocation log lock")
            .is_empty(),
        "nothing executed"
    );
}

#[test]
fn test_supported_execution_failure_is_nonzero() {
    let rows = lint_rows(&["first", "second"]);
    let (_fixture, loaded) = load_consumer_toml(&consumer_toml(&rows));
    let loaded = loaded.expect("config loads");

    // A failing supported step is a non-zero backend failure; the next step
    // still runs (both are cargo argv[0], so the cargo lane fixes the order).
    let mut failing = FakeSystemAdapter::new(true).on_host(Platform::Linux);
    failing.failures.insert("first", "first step exploded");
    let error = lint_rows_for(&loaded, None, PlatformScope::Host, &failing)
        .expect_err("failing step fails the run");
    assert_eq!(error.kind, CliErrorKind::BackendFailure);
    assert_eq!(error.code(), "profile.steps_failed");
    assert_eq!(error.exit_code(), crate::consts::EXIT_BACKEND_FAILURE);
    assert_eq!(
        failing
            .invocations
            .lock()
            .expect("fake invocation log lock")
            .as_slice(),
        ["first", "second"],
        "fail slow"
    );

    // A supported cross target without cargo-xwin is a capability failure, never a skip.
    let msvc = format!(
        "{rows}[[tool.sc-lint.lint]]\nname = \"win\"\ncommand = [\"cargo\", \"check\"]\nplatform = \"windows\"\ntarget = \"x86_64-pc-windows-msvc\"\n"
    );
    let (_fixture, loaded) = load_consumer_toml(&consumer_toml(&msvc));
    let loaded = loaded.expect("windows config loads");
    let no_xwin = FakeSystemAdapter::new(false).on_host(Platform::Linux);
    let error = lint_rows_for(&loaded, Some("win"), PlatformScope::All, &no_xwin)
        .expect_err("missing xwin fails");
    assert_eq!(error.kind, CliErrorKind::Capability);
    assert_eq!(error.exit_code(), crate::consts::EXIT_CAPABILITY);
    assert!(
        no_xwin
            .invocations
            .lock()
            .expect("fake invocation log lock")
            .is_empty()
    );
}

#[test]
fn test_profiles_exact_json_toml_roundtrip() {
    let fragment: Value = serde_json::from_str(
        &fs::read_to_string(profile_fixture("roundtrip-fragment.json")).expect("json fixture"),
    )
    .expect("fragment json");
    let toml_text = fs::read_to_string(profile_fixture("roundtrip-fragment.toml")).expect("toml");
    let (_fixture, loaded) = load_consumer_toml(&consumer_toml(&toml_text));
    let loaded = loaded.expect("round-trip fragment loads");

    // Every JSON profile entry is present, in the same order and shape, once loaded.
    let profiles = fragment["profiles"].as_object().expect("profiles object");
    let expected_names: Vec<&str> = profiles.keys().map(String::as_str).collect();
    assert_eq!(
        selected_names_for_host(&loaded, Some("all"), Platform::host()).len(),
        expected_names.len() - usize::from(cfg!(not(windows))),
        "windows-tagged step only joins `all` on a Windows host"
    );
    assert_eq!(
        selected_names_for_host(&loaded, Some("ci"), Platform::host()),
        fragment["lint_profiles"]["ci"]
            .as_array()
            .expect("ci members")
            .iter()
            .filter_map(Value::as_str)
            .filter(|name| cfg!(windows) || *name != "windows-check")
            .collect::<Vec<_>>()
    );
    let input = loaded.consumer_lint_inputs(Some("all")).expect("inputs");
    let windows = input
        .steps
        .iter()
        .find(|step| step.name() == "windows-check")
        .expect("windows step");
    assert_eq!(windows.platform(), Some(Platform::Windows));
    assert_eq!(
        windows.target().map(Target::as_str),
        fragment["profiles"]["windows-check"]["target"].as_str()
    );
    assert_eq!(
        windows.command(),
        ["cargo", "check", "--workspace"],
        "command argv keeps its exact order"
    );
}

#[test]
fn rust_loader_rejects_semantic_invalid_profile_fixtures() {
    let dir = profile_fixture("semantic-invalid");
    let expected: Value =
        serde_json::from_str(&fs::read_to_string(dir.join("expected.json")).expect("expected"))
            .expect("expected json");
    let cases = expected.as_object().expect("expected cases");
    assert_eq!(cases.len(), 6);
    for (case, detail_code) in cases {
        // The Rust fixture pins loader semantics. The adoption schema may reject
        // some values earlier, such as bad-platform, without replacing this check.
        let json: Value = serde_json::from_str(
            &fs::read_to_string(dir.join(format!("{case}.json"))).expect("case json"),
        )
        .expect("case json");
        assert!(json["profiles"].is_object(), "{case}");
        let toml_text = fs::read_to_string(dir.join(format!("{case}.toml"))).expect("case toml");
        let (_fixture, loaded) = load_consumer_toml(&consumer_toml(&toml_text));
        let error = loaded.expect_err(case);
        assert_eq!(error.code(), PROFILE_PLAN_ERROR_CODE, "{case}");
        assert_eq!(&error.details["detail_code"], detail_code, "{case}");
    }
}

// ---------------------------------------------------------------------------
// Shared scheduler, fail-fast and test selectors (Phase L, l-6).
// ---------------------------------------------------------------------------

fn parallel_step(name: &str) -> workflow::StepPlan {
    workflow::StepPlan::new(
        name,
        "test",
        workflow::Lane::Parallel,
        "tool",
        Vec::<String>::new(),
    )
}

fn cargo_step(name: &str) -> workflow::StepPlan {
    workflow::StepPlan::new(name, "test", workflow::Lane::Cargo, "cargo", ["test"])
}

#[expect(
    clippy::result_large_err,
    reason = "Tests surface the shared CliError contract unchanged."
)]
fn run_scheduled(
    adapter: &FakeSystemAdapter,
    plans: Vec<workflow::StepPlan>,
    policy: FailurePolicy,
) -> Result<Vec<workflow::StepReport>, CliError> {
    let steps = plans
        .into_iter()
        .enumerate()
        .map(|(index, plan)| crate::scheduler::ScheduledStep::new(index, plan))
        .collect::<Vec<_>>();
    crate::scheduler::run_steps_scheduled(Path::new("."), adapter, &steps, policy)
}

fn sorted(mut names: Vec<String>) -> Vec<String> {
    names.sort();
    names
}

fn detail_names(error: &CliError, key: &str) -> Vec<String> {
    error.details[key]
        .as_array()
        .map(|entries| {
            entries
                .iter()
                .map(|entry| entry["name"].as_str().expect("entry name").to_string())
                .collect()
        })
        .unwrap_or_default()
}

/// Position of `event` in the fake's event log.
fn event_position(adapter: &FakeSystemAdapter, event: &str) -> usize {
    adapter
        .events
        .lock()
        .expect("fake event log lock")
        .iter()
        .position(|recorded| recorded == event)
        .unwrap_or_else(|| panic!("{event} was not recorded"))
}

#[test]
fn profile_runs_all_steps_when_one_fails() {
    let mut adapter = FakeSystemAdapter::new(false);
    adapter.failures.insert("b", "b exploded");
    let error = run_scheduled(
        &adapter,
        vec![parallel_step("a"), parallel_step("b"), parallel_step("c")],
        FailurePolicy::KeepGoing,
    )
    .expect_err("b fails the run");
    assert_eq!(error.message, "1 of 3 steps failed: [1] b");
    assert_eq!(
        error.details["failed_steps"],
        json!([{"index": 1, "name": "b", "exit_code": null, "message": "b exploded"}])
    );
    assert_eq!(sorted(adapter.invoked()), ["a", "b", "c"]);
}

#[test]
fn cargo_lane_continues_after_failure() {
    let mut adapter = FakeSystemAdapter::new(false);
    adapter.failures.insert("c1", "c1 exploded");
    run_scheduled(
        &adapter,
        vec![cargo_step("c1"), cargo_step("c2"), cargo_step("c3")],
        FailurePolicy::KeepGoing,
    )
    .expect_err("c1 fails the run");
    assert_eq!(adapter.invoked(), ["c1", "c2", "c3"]);
}

#[test]
fn report_is_in_declaration_order() {
    let mut adapter = FakeSystemAdapter::new(false);
    let (y_done, x_waits) = mpsc::channel();
    let x_waits = Mutex::new(x_waits);
    // `x` (index 0) cannot finish until `y` (index 1) has.
    adapter.on_start.insert(
        "x",
        Box::new(move || {
            x_waits
                .lock()
                .expect("x receiver lock")
                .recv_timeout(RENDEZVOUS_TIMEOUT)
                .expect("y never finished");
        }),
    );
    adapter.on_finish.insert(
        "y",
        Box::new(move || y_done.send(()).expect("x is waiting")),
    );
    let reports = run_scheduled(
        &adapter,
        vec![parallel_step("x"), parallel_step("y")],
        FailurePolicy::KeepGoing,
    )
    .expect("both pass");
    let names = reports
        .iter()
        .map(|report| report.to_json()["name"].as_str().expect("name").to_string())
        .collect::<Vec<_>>();
    assert_eq!(names, ["x", "y"]);
    assert_eq!(
        adapter
            .completions
            .lock()
            .expect("fake completion log lock")
            .as_slice(),
        ["y", "x"]
    );
}

#[test]
fn panicking_step_becomes_failed_outcome() {
    let mut adapter = FakeSystemAdapter::new(false);
    adapter.on_start.insert("b", Box::new(|| panic!("boom")));
    let error = run_scheduled(
        &adapter,
        vec![parallel_step("a"), parallel_step("b"), cargo_step("c")],
        FailurePolicy::KeepGoing,
    )
    .expect_err("the panic fails the run");
    assert_eq!(error.code(), "profile.steps_failed");
    let failed = &error.details["failed_steps"];
    assert_eq!(failed.as_array().map(Vec::len), Some(1));
    assert_eq!(failed[0]["name"], "b");
    assert_eq!(failed[0]["index"], 1);
    assert_eq!(failed[0]["exit_code"], Value::Null);
    assert_eq!(failed[0]["message"], "b panicked during execution: boom");
    assert_eq!(sorted(adapter.invoked()), ["a", "b", "c"]);
}

#[test]
fn lane_is_cargo_only_for_cargo_argv0() {
    for program in ["cargo", "/usr/bin/cargo", "cargo.exe", "CARGO"] {
        assert_eq!(
            workflow::Lane::for_program(program.as_ref()),
            workflow::Lane::Cargo,
            "{program}"
        );
    }
    for program in ["cargo-nextest", "just", "python", "/usr/bin/cargo-xwin"] {
        assert_eq!(
            workflow::Lane::for_program(program.as_ref()),
            workflow::Lane::Parallel,
            "{program}"
        );
    }
}

#[test]
fn after_all_step_starts_last() {
    let adapter = FakeSystemAdapter::new(false);
    run_scheduled(
        &adapter,
        vec![
            parallel_step("tail").with_after_all(true),
            parallel_step("a"),
            cargo_step("c"),
        ],
        FailurePolicy::KeepGoing,
    )
    .expect("all pass");
    let started = event_position(&adapter, "start:tail");
    assert!(event_position(&adapter, "end:a") < started);
    assert!(event_position(&adapter, "end:c") < started);
}

#[test]
fn multiple_after_all_run_in_declaration_order() {
    let adapter = FakeSystemAdapter::new(false);
    run_scheduled(
        &adapter,
        vec![
            parallel_step("t1").with_after_all(true),
            parallel_step("a"),
            cargo_step("t2").with_after_all(true),
        ],
        FailurePolicy::KeepGoing,
    )
    .expect("all pass");
    let events = adapter.events.lock().expect("fake event log lock").clone();
    assert_eq!(events[2..], ["start:t1", "end:t1", "start:t2", "end:t2"]);
}

#[test]
fn fail_fast_stops_scheduling_and_skips_after_all() {
    let mut adapter = FakeSystemAdapter::new(false);
    adapter.failures.insert("p", "p exploded");
    let error = run_scheduled(
        &adapter,
        vec![
            parallel_step("p"),
            parallel_step("tail").with_after_all(true),
        ],
        FailurePolicy::FailFast,
    )
    .expect_err("p fails the run");
    assert_eq!(adapter.invoked(), ["p"]);
    assert_eq!(detail_names(&error, "failed_steps"), ["p"]);
    assert_eq!(detail_names(&error, "skipped_steps"), ["tail"]);
}

#[test]
fn fail_fast_skips_unstarted_cargo_steps_only() {
    let mut adapter = FakeSystemAdapter::new(false);
    adapter.failures.insert("c1", "c1 exploded");
    let (p_started, c1_waits) = mpsc::channel();
    let (c1_finished, p_waits) = mpsc::channel();
    // `c1` fails only once `p` is running; `p` finishes after `c1` has.
    let c1_waits = Mutex::new(c1_waits);
    adapter.on_start.insert(
        "c1",
        Box::new(move || {
            c1_waits
                .lock()
                .expect("c1 receiver lock")
                .recv_timeout(RENDEZVOUS_TIMEOUT)
                .expect("p never started");
        }),
    );
    adapter.on_finish.insert(
        "c1",
        Box::new(move || c1_finished.send(()).expect("p is waiting")),
    );
    adapter.on_start.insert(
        "p",
        rendezvous_hook(p_started, p_waits, "c1 never finished while p was running"),
    );
    let error = run_scheduled(
        &adapter,
        vec![
            cargo_step("c1"),
            parallel_step("p"),
            cargo_step("c2"),
            parallel_step("tail").with_after_all(true),
        ],
        FailurePolicy::FailFast,
    )
    .expect_err("c1 fails the run");
    assert_eq!(sorted(adapter.invoked()), ["c1", "p"]);
    assert_eq!(detail_names(&error, "failed_steps"), ["c1"]);
    assert_eq!(detail_names(&error, "skipped_steps"), ["c2", "tail"]);
}

#[test]
fn skipped_steps_listed_not_failed() {
    let mut adapter = FakeSystemAdapter::new(false);
    adapter.failures.insert("c1", "c1 exploded");
    let error = run_scheduled(
        &adapter,
        vec![cargo_step("c1"), cargo_step("c2")],
        FailurePolicy::FailFast,
    )
    .expect_err("c1 fails the run");
    assert_eq!(error.message, "1 of 2 steps failed: [0] c1");
    assert_eq!(
        error.details["skipped_steps"],
        json!([{"index": 1, "name": "c2", "reason": "fail-fast"}])
    );
    assert_eq!(detail_names(&error, "failed_steps"), ["c1"]);

    let error = run_scheduled(
        &adapter,
        vec![cargo_step("c1"), cargo_step("c2")],
        FailurePolicy::KeepGoing,
    )
    .expect_err("c1 fails the run");
    assert!(error.details.get("skipped_steps").is_none());
}

fn selector_config(selectors: &str) -> String {
    format!(
        "[tool.sc-lint]\nminimum_version = \"0.4.0\"\n\n[[tool.sc-lint.lint]]\nname = \"fmt\"\ncommand = [\"cargo\", \"fmt\", \"--check\"]\n\n[tool.sc-lint.test]\n{selectors}"
    )
}

/// The table-form fixture of the selector tests: `ci` groups `unit` then
/// `nested`, which reaches `unit` again through `integration`.
const SELECTOR_TABLE: &str = "default = \"unit\"\n\n[tool.sc-lint.test.selectors.unit]\ncommand = [\"cargo\", \"test\"]\n\n[tool.sc-lint.test.selectors.integration]\ncommand = [\"cargo\", \"test\", \"--test\", \"integration\"]\nenv = { RUST_LOG = \"debug\", SC_MODE = \"ci\" }\ncwd = \"crates/app\"\n\n[tool.sc-lint.test.selectors.nested]\ngroup = [\"integration\", \"unit\"]\n\n[tool.sc-lint.test.selectors.ci]\ngroup = [\"unit\", \"nested\"]\n\n[tool.sc-lint.test.selectors.win-only]\ncommand = [\"cargo\", \"test\", \"--features\", \"windows\"]\nplatforms = [\"windows\"]\n";

fn load_test_config(text: &str) -> (TempDir, Result<LoadedConfig, CliError>) {
    let fixture = TempDir::new().expect("fixture");
    let config_path = fixture.path().join("sc-lint.toml");
    fs::write(&config_path, text).expect("consumer config");
    let cli = Cli::parse_from([
        "sc-lint",
        "--config",
        config_path.to_str().expect("config path"),
        "test",
    ]);
    let context = CommandContext::from_cli(&cli).expect("consumer test context");
    let loaded = LoadedConfig::load(&cli, &context);
    (fixture, loaded)
}

#[expect(
    clippy::result_large_err,
    reason = "Tests surface the shared CliError contract unchanged."
)]
fn run_test_request(
    loaded: &LoadedConfig,
    request: &TestRequest,
    adapter: &FakeSystemAdapter,
) -> Result<crate::command::CommandSuccess, CliError> {
    workflow::run_consumer_test_with(loaded, request, FailurePolicy::KeepGoing, adapter)
}

#[test]
fn selector_default_runs_when_none_named() {
    let (_fixture, loaded) = load_test_config(&selector_config(SELECTOR_TABLE));
    let loaded = loaded.expect("selector config loads");
    let adapter = FakeSystemAdapter::new(false).on_host(Platform::Linux);
    let run = run_test_request(&loaded, &TestRequest::all(), &adapter).expect("default runs");
    assert_eq!(run.data["selector"], "unit");
    assert_eq!(adapter.invoked(), ["unit"]);
}

#[test]
fn selector_opt_in_runs_only_named() {
    let (_fixture, loaded) = load_test_config(&selector_config(SELECTOR_TABLE));
    let loaded = loaded.expect("selector config loads");
    let adapter = FakeSystemAdapter::new(false).on_host(Platform::Linux);
    run_test_request(&loaded, &TestRequest::named("integration"), &adapter)
        .expect("named selector runs");
    assert_eq!(adapter.invoked(), ["integration"]);

    let adapter = FakeSystemAdapter::new(false).on_host(Platform::Linux);
    run_test_request(&loaded, &TestRequest::all(), &adapter).expect("default runs");
    assert!(
        !adapter.invoked().contains(&"integration".to_string()),
        "a non-default selector never runs unless named"
    );
}

#[test]
fn selector_forwards_args() {
    let (_fixture, loaded) = load_test_config(&selector_config(SELECTOR_TABLE));
    let loaded = loaded.expect("selector config loads");
    let adapter = FakeSystemAdapter::new(false).on_host(Platform::Linux);
    let request = TestRequest::new(
        Some("unit".to_string()),
        vec!["--nocapture".to_string()],
        false,
    );
    run_test_request(&loaded, &request, &adapter).expect("unit runs");
    assert_eq!(adapter.records()[0].argv, ["cargo", "test", "--nocapture"]);

    let request = TestRequest::new(
        Some("ci".to_string()),
        vec!["--nocapture".to_string()],
        false,
    );
    let error = run_test_request(&loaded, &request, &FakeSystemAdapter::new(false))
        .expect_err("a group does not take forwarded args");
    assert_eq!(error.kind, CliErrorKind::Usage);
    assert_eq!(
        error.message,
        "group selector 'ci' does not accept forwarded arguments"
    );
}

/// Forwarded arguments without one named command selector (review finding
/// R-12): the rows form's every-row run is not a group, and an unresolved name
/// is an unknown selector, not a group.
#[test]
fn forwarded_args_report_every_row_runs_and_unknown_names_correctly() {
    let forwarded = || vec!["--nocapture".to_string()];
    let rows = "[tool.sc-lint]\nminimum_version = \"0.4.0\"\n\n[[tool.sc-lint.lint]]\nname = \"fmt\"\ncommand = [\"cargo\", \"fmt\", \"--check\"]\n\n[[tool.sc-lint.test]]\nname = \"unit\"\ncommand = [\"cargo\", \"test\"]\n";
    let (_rows_fixture, loaded) = load_test_config(rows);
    let loaded = loaded.expect("rows config loads");
    for selector in [None, Some("all".to_string())] {
        let request = TestRequest::new(selector.clone(), forwarded(), false);
        let error = run_test_request(&loaded, &request, &FakeSystemAdapter::new(false))
            .expect_err("every-row run does not take forwarded args");
        assert_eq!(error.kind, CliErrorKind::Usage, "{selector:?}");
        assert_eq!(
            error.message,
            "selector 'all' runs every [[tool.sc-lint.test]] row and does not accept forwarded arguments",
            "{selector:?}"
        );
        assert_eq!(
            error.suggested_action.as_deref(),
            Some("name a command selector to forward arguments"),
            "{selector:?}"
        );
    }
    let adapter = FakeSystemAdapter::new(false);
    let request = TestRequest::new(Some("unit".to_string()), forwarded(), false);
    run_test_request(&loaded, &request, &adapter).expect("a named row takes forwarded args");
    assert_eq!(adapter.records()[0].argv, ["cargo", "test", "--nocapture"]);

    let no_default = "[tool.sc-lint.test.selectors.unit]\ncommand = [\"cargo\", \"test\"]\n";
    let (_table_fixture, loaded) = load_test_config(&selector_config(no_default));
    let loaded = loaded.expect("table config loads");
    let (_, selectors) = loaded.test_selectors().expect("table selectors");
    let error = selectors
        .check_forwarded_args(&TestRequest::new(
            Some("nightly".to_string()),
            forwarded(),
            false,
        ))
        .expect_err("unresolved name");
    assert_eq!(error.code(), "test.unknown_selector");
    assert_eq!(error.details["selector"], "nightly");
    assert!(
        error
            .message
            .starts_with("selector 'nightly' is not declared in "),
        "{error:?}"
    );
    let error = selectors
        .check_forwarded_args(&TestRequest::new(None, forwarded(), false))
        .expect_err("no selector and no default");
    assert_eq!(error.code(), "test.unknown_selector");
    assert!(
        error.message.starts_with("no selector was named and "),
        "{error:?}"
    );
}

#[test]
fn selector_group_expands() {
    let (_fixture, loaded) = load_test_config(&selector_config(SELECTOR_TABLE));
    let loaded = loaded.expect("selector config loads");
    let adapter = FakeSystemAdapter::new(false).on_host(Platform::Linux);
    let run = run_test_request(&loaded, &TestRequest::named("ci"), &adapter).expect("group runs");
    let steps = run.data["steps"].as_array().expect("steps");
    // Listed order, nested groups expanded, `unit` (reached twice) runs once.
    assert_eq!(step_names(steps), ["unit", "integration"]);
    assert_eq!(adapter.invoked(), ["unit", "integration"]);
}

#[test]
fn selector_env_and_cwd_applied() {
    let (_fixture, loaded) = load_test_config(&selector_config(SELECTOR_TABLE));
    let loaded = loaded.expect("selector config loads");
    let adapter = FakeSystemAdapter::new(false).on_host(Platform::Linux);
    run_test_request(&loaded, &TestRequest::named("integration"), &adapter)
        .expect("integration runs");
    assert_eq!(
        adapter.records(),
        [StepRecord {
            name: "integration".to_string(),
            argv: ["cargo", "test", "--test", "integration"]
                .map(String::from)
                .to_vec(),
            env: vec![
                ("RUST_LOG".to_string(), "debug".to_string()),
                ("SC_MODE".to_string(), "ci".to_string()),
            ],
            cwd: Some(PathBuf::from("crates/app")),
        }]
    );
}

#[test]
fn selector_platform_mismatch_skipped() {
    let (_fixture, loaded) = load_test_config(&selector_config(SELECTOR_TABLE));
    let loaded = loaded.expect("selector config loads");
    let adapter = FakeSystemAdapter::new(false).on_host(Platform::Linux);
    let run = run_test_request(&loaded, &TestRequest::named("win-only"), &adapter)
        .expect("a platform skip is a success");
    assert_eq!(
        run.data["steps"],
        json!([{
            "name": "win-only",
            "status": "skipped",
            "reason": "platform_mismatch",
            "host": "linux",
            "platforms": ["windows"],
        }])
    );
    assert!(adapter.invoked().is_empty());
}

#[test]
fn selector_list_prints_names() {
    let (fixture, loaded) = load_test_config(&selector_config(SELECTOR_TABLE));
    let loaded = loaded.expect("selector config loads");
    let config_path = fixture.path().join("sc-lint.toml");
    let cli = Cli::parse_from([
        "sc-lint",
        "--config",
        config_path.to_str().expect("config path"),
        "test",
        "--list",
    ]);
    let context = CommandContext::from_cli(&cli).expect("list context");
    let adapter = FakeSystemAdapter::new(false);
    let listed = run_test_request(&loaded, context.test_request(), &adapter).expect("list");
    let names = listed.data["selectors"]
        .as_array()
        .expect("selectors")
        .iter()
        .map(|selector| selector["name"].as_str().expect("name").to_string())
        .collect::<Vec<_>>();
    assert_eq!(names, ["ci", "integration", "nested", "unit", "win-only"]);
    assert_eq!(listed.data["default"], "unit");
    assert!(adapter.invoked().is_empty(), "--list runs nothing");

    let envelope = CommandEnvelope::success(context.command_id(), listed.data);
    let rendered = crate::render::render_success_human(&context, &envelope);
    let expected = "ci: group [unit, nested]\nintegration: cargo test --test integration\nnested: group [integration, unit]\nunit (default): cargo test\nwin-only: cargo test --features windows [windows]";
    assert_eq!(rendered, expected);
    let windows_contract = CLI_CONTRACT.replace("\r\n", "\n").replace('\n', "\r\n");
    let normalized_contract = windows_contract.replace("\r\n", "\n");
    assert!(
        normalized_contract.contains(expected),
        "the CLI contract documents the exact selector-list output"
    );
}

#[test]
fn unknown_selector_rejected() {
    let (_fixture, loaded) = load_test_config(&selector_config(SELECTOR_TABLE));
    let loaded = loaded.expect("selector config loads");
    let error = run_test_request(
        &loaded,
        &TestRequest::named("nightly"),
        &FakeSystemAdapter::new(false),
    )
    .expect_err("unknown CLI name");
    assert_eq!(error.code(), "test.unknown_selector");
    assert_eq!(error.details["selector"], "nightly");
    assert_eq!(
        error.details["available"],
        json!(["ci", "integration", "nested", "unit", "win-only"])
    );

    let member = "[tool.sc-lint.test.selectors.unit]\ncommand = [\"cargo\", \"test\"]\n\n[tool.sc-lint.test.selectors.ci]\ngroup = [\"unit\", \"lint\"]\n";
    let (_fixture, loaded) = load_test_config(&selector_config(member));
    let error = loaded.expect_err("unknown group member");
    assert_eq!(error.code(), "test.unknown_selector");
    assert!(
        error
            .cause
            .as_deref()
            .is_some_and(|cause| cause
                .starts_with("selector 'lint' listed by group 'ci' is not declared in ")),
        "{error:?}"
    );

    let default = "default = \"nightly\"\n\n[tool.sc-lint.test.selectors.unit]\ncommand = [\"cargo\", \"test\"]\n";
    let (_fixture, loaded) = load_test_config(&selector_config(default));
    let error = loaded.expect_err("unknown default");
    assert_eq!(error.code(), "test.unknown_selector");
    assert!(
        error.cause.as_deref().is_some_and(|cause| cause
            .starts_with("default names selector 'nightly', which is not declared in ")),
        "{error:?}"
    );

    let no_default = "[tool.sc-lint.test.selectors.unit]\ncommand = [\"cargo\", \"test\"]\n";
    let (_fixture, loaded) = load_test_config(&selector_config(no_default));
    let loaded = loaded.expect("a config without default loads");
    let error = run_test_request(&loaded, &TestRequest::all(), &FakeSystemAdapter::new(false))
        .expect_err("bare invocation without a default");
    assert_eq!(error.code(), "test.unknown_selector");
    assert!(
        error
            .cause
            .as_deref()
            .is_some_and(|cause| cause.starts_with("no selector was named and ")
                && cause.ends_with(" declares no default")),
        "{error:?}"
    );
}

#[test]
fn group_cycle_rejected() {
    let cycle = "[tool.sc-lint.test.selectors.a]\ngroup = [\"b\"]\n\n[tool.sc-lint.test.selectors.b]\ngroup = [\"a\"]\n";
    let (_fixture, loaded) = load_test_config(&selector_config(cycle));
    let error = loaded.expect_err("a -> b -> a");
    assert_eq!(error.code(), "test.group_cycle");
    assert!(
        error
            .cause
            .as_deref()
            .is_some_and(|cause| cause.starts_with("selector group cycle: a -> b -> a in ")),
        "{error:?}"
    );

    let self_cycle = "[tool.sc-lint.test.selectors.a]\ngroup = [\"a\"]\n";
    let (_fixture, loaded) = load_test_config(&selector_config(self_cycle));
    let error = loaded.expect_err("a -> a");
    assert_eq!(error.code(), "test.group_cycle");
    assert!(
        error
            .cause
            .as_deref()
            .is_some_and(|cause| cause.starts_with("selector group cycle: a -> a in ")),
        "{error:?}"
    );
}

#[test]
fn after_all_on_group_rejected() {
    let group = "[tool.sc-lint.test.selectors.unit]\ncommand = [\"cargo\", \"test\"]\n\n[tool.sc-lint.test.selectors.ci]\ngroup = [\"unit\"]\nafter_all = false\n";
    let (_fixture, loaded) = load_test_config(&selector_config(group));
    let error = loaded.expect_err("a group never takes after_all");
    assert_eq!(error.code(), "profile.invalid_after_all");
    assert!(
        error
            .cause
            .as_deref()
            .is_some_and(|cause| cause.starts_with("group selector 'ci' in ")
                && cause.ends_with(" declares after_all")),
        "{error:?}"
    );
}

#[test]
fn legacy_test_rows_still_run_all() {
    let rows = "[tool.sc-lint]\nminimum_version = \"0.4.0\"\n\n[[tool.sc-lint.lint]]\nname = \"fmt\"\ncommand = [\"cargo\", \"fmt\", \"--check\"]\n\n[[tool.sc-lint.test]]\nname = \"first\"\ncommand = [\"cargo\", \"test\"]\n\n[[tool.sc-lint.test]]\nname = \"second\"\ncommand = [\"cargo\", \"test\", \"--doc\"]\n\n[[tool.sc-lint.test]]\nname = \"third\"\ncommand = [\"cargo\", \"test\", \"--release\"]\n";
    let (_fixture, loaded) = load_test_config(rows);
    let loaded = loaded.expect("rows config loads");
    for request in [TestRequest::all(), TestRequest::named("all")] {
        let adapter = FakeSystemAdapter::new(false);
        let run = run_test_request(&loaded, &request, &adapter).expect("every row runs");
        assert_eq!(run.data["selector"], "all");
        // Every row is cargo argv[0], so the cargo lane fixes the order.
        assert_eq!(adapter.invoked(), ["first", "second", "third"]);
    }
    let adapter = FakeSystemAdapter::new(false);
    run_test_request(&loaded, &TestRequest::named("second"), &adapter).expect("one row");
    assert_eq!(adapter.invoked(), ["second"]);
}

#[test]
fn mixed_test_forms_rejected() {
    let mixed = "[tool.sc-lint]\nminimum_version = \"0.4.0\"\n\n[[tool.sc-lint.lint]]\nname = \"fmt\"\ncommand = [\"cargo\", \"fmt\", \"--check\"]\n\n[[tool.sc-lint.test]]\nname = \"unit\"\ncommand = [\"cargo\", \"test\"]\n\n[tool.sc-lint.test]\ndefault = \"unit\"\n";
    let (_fixture, loaded) = load_test_config(mixed);
    let error = loaded.expect_err("both forms in one file");
    assert_eq!(error.code(), "CLI.SC_LINT_CONFIG_MALFORMED");
}

/// Selector and step shape errors carry selector recovery only, never the
/// compatibility setup recovery (phase-end review finding R-4).
#[test]
fn selector_shape_errors_carry_selector_recovery_not_compatibility_recovery() {
    let table =
        "[tool.sc-lint.test.selectors.unit]\ncommand = [\"cargo\", \"test\"]\ngroup = [\"unit\"]\n";
    let (_table_fixture, table_loaded) = load_test_config(&selector_config(table));
    let duplicate = "[[tool.sc-lint.lint]]\nname = \"fmt\"\ncommand = [\"cargo\", \"fmt\"]\n\n[[tool.sc-lint.lint]]\nname = \"fmt\"\ncommand = [\"cargo\", \"check\"]\n";
    let (_row_fixture, row_loaded) = load_consumer_toml(&consumer_toml(duplicate));
    let cases = [
        (
            "test",
            table_loaded.expect_err("selector with both command and group"),
            "selector `unit` must declare exactly one of `command` or `group`",
        ),
        (
            "lint",
            row_loaded.expect_err("duplicate lint step"),
            "consumer `lint` profile repeats step name `fmt`",
        ),
    ];
    for (profile, error, message) in cases {
        let config_path = error.details["config_path"]
            .as_str()
            .expect("config_path detail")
            .to_string();
        assert!(
            config_path.ends_with("sc-lint.toml"),
            "{profile}: {config_path}"
        );
        assert_eq!(error.kind, CliErrorKind::Config, "{profile}");
        assert_eq!(error.code(), "CLI.SC_LINT_CONFIG_MALFORMED", "{profile}");
        assert_eq!(error.message, message, "{profile}");
        assert_eq!(
            error.cause.as_deref(),
            Some(format!("{message} in {config_path}").as_str()),
            "{profile}"
        );
        assert_eq!(
            error.suggested_action.as_deref(),
            Some(
                format!(
                    "fix the `tool.sc-lint.{profile}` selectors in {config_path} as the cause describes"
                )
                .as_str()
            ),
            "{profile}"
        );
        assert_eq!(error.documentation, None, "{profile}");
        let mut detail_keys: Vec<&str> = error.details.keys().map(String::as_str).collect();
        detail_keys.sort_unstable();
        assert_eq!(detail_keys, ["config_path", "profile"], "{profile}");
        assert_eq!(error.details["profile"], profile, "{profile}");

        let human = crate::render::render_error_human("test", &error);
        let envelope = failure_envelope_json("test", error).to_string();
        for compatibility_text in [
            "just setup",
            "installer",
            "minimum_version",
            "required_field",
            "sc-lint docs setup",
        ] {
            assert!(
                !envelope.contains(compatibility_text),
                "{profile}: envelope carries {compatibility_text:?}: {envelope}"
            );
            assert!(
                !human.contains(compatibility_text),
                "{profile}: human output carries {compatibility_text:?}: {human}"
            );
        }
    }
}

#[test]
fn consumer_lint_after_all_and_platform_skips_keep_declaration_order() {
    let rows = "[[tool.sc-lint.lint]]\nname = \"tail\"\ncommand = [\"cargo\", \"--version\"]\nafter_all = true\n\n[[tool.sc-lint.lint]]\nname = \"win\"\ncommand = [\"cargo\", \"check\"]\nplatform = \"windows\"\n\n[[tool.sc-lint.lint]]\nname = \"bad\"\ncommand = [\"cargo\", \"--version\"]\n\n";
    let (_fixture, loaded) = load_consumer_toml(&consumer_toml(rows));
    let loaded = loaded.expect("lint rows load");

    let adapter = FakeSystemAdapter::new(false).on_host(Platform::Linux);
    let steps = lint_rows_for(&loaded, None, PlatformScope::All, &adapter).expect("all pass");
    assert_eq!(step_names(&steps), ["tail", "win", "bad"]);
    assert_eq!(steps[1]["status"], "skipped");
    assert!(event_position(&adapter, "end:bad") < event_position(&adapter, "start:tail"));

    let mut adapter = FakeSystemAdapter::new(false).on_host(Platform::Linux);
    adapter.failures.insert("bad", "bad exploded");
    let error =
        lint_rows_for(&loaded, None, PlatformScope::All, &adapter).expect_err("bad fails the run");
    assert_eq!(error.details["failed_steps"][0]["index"], 2);
    assert_eq!(error.details["failed_steps"][0]["name"], "bad");
}

#[test]
fn config_errors_carry_envelope_keys() {
    let invalid_after_all = "[tool.sc-lint.test.selectors.unit]\ncommand = [\"cargo\", \"test\"]\nafter_all = \"yes\"\n";
    let unknown = "[tool.sc-lint.test.selectors.ci]\ngroup = [\"missing\"]\n";
    let cycle = "[tool.sc-lint.test.selectors.a]\ngroup = [\"a\"]\n";
    for (selectors, code, name, recovery) in [
        (
            invalid_after_all,
            "profile.invalid_after_all",
            "'unit'",
            "remove after_all from that selector",
        ),
        (
            unknown,
            "test.unknown_selector",
            "'missing'",
            "use a name from sc-lint test --list",
        ),
        (
            cycle,
            "test.group_cycle",
            "a -> a",
            "break the cycle in [tool.sc-lint.test]",
        ),
    ] {
        let (fixture, loaded) = load_test_config(&selector_config(selectors));
        let error = loaded.expect_err(code);
        let serialized = serde_json::to_value(&error).expect("error serializes");
        let config = fixture.path().join("sc-lint.toml").display().to_string();
        assert_eq!(serialized["kind"], "config", "{code}");
        assert_eq!(serialized["code"], code);
        let cause = serialized["cause"].as_str().expect("cause");
        assert!(
            cause.contains(name) && cause.contains(&config),
            "{code}: {cause}"
        );
        assert_eq!(serialized["suggested_action"], recovery, "{code}");
    }
}

#[test]
fn failed_steps_report_shape() {
    let mut adapter = FakeSystemAdapter::new(false);
    adapter.failures.insert("clippy", "clippy failed");
    adapter.exit_codes.insert("clippy", 101);
    let error = run_scheduled(
        &adapter,
        vec![
            cargo_step("fmt"),
            parallel_step("spell"),
            cargo_step("clippy"),
        ],
        FailurePolicy::KeepGoing,
    )
    .expect_err("clippy fails the run");
    assert_eq!(
        error.details["failed_steps"],
        json!([{"index": 2, "name": "clippy", "exit_code": 101, "message": "clippy failed"}])
    );
    assert_eq!(error.kind, CliErrorKind::BackendFailure);
    assert_eq!(error.code(), "profile.steps_failed");
    assert_eq!(
        error.suggested_action.as_deref(),
        Some("fix the failing steps and rerun")
    );
    assert_eq!(error.exit_code(), crate::consts::EXIT_BACKEND_FAILURE);
}

#[test]
fn fail_fast_flag_parses_and_selects_policy() {
    let policy = |argv: &[&str]| {
        CommandContext::from_cli(&Cli::parse_from(argv))
            .expect("context")
            .failure_policy()
    };
    assert_eq!(
        policy(&["sc-lint", "test", "--fail-fast"]),
        FailurePolicy::FailFast
    );
    assert_eq!(
        policy(&["sc-lint", "lint", "ci", "--consumer", "--fail-fast"]),
        FailurePolicy::FailFast
    );
    assert_eq!(policy(&["sc-lint", "test"]), FailurePolicy::KeepGoing);
    assert_eq!(
        policy(&["sc-lint", "lint", "ci", "--consumer"]),
        FailurePolicy::KeepGoing
    );
    assert!(Cli::try_parse_from(["sc-lint", "lint", "ci", "--fail-fast"]).is_err());
    assert!(Cli::try_parse_from(["sc-lint", "test", "--list", "unit"]).is_err());
}

#[test]
fn forwarded_args_parse() {
    let request = |argv: &[&str]| {
        CommandContext::from_cli(&Cli::parse_from(argv))
            .expect("context")
            .test_request()
            .clone()
    };
    let unit = request(&["sc-lint", "test", "unit", "--nocapture"]);
    assert_eq!(unit.selector(), Some("unit"));
    assert_eq!(unit.args(), ["--nocapture"]);

    let cli = Cli::parse_from(["sc-lint", "test", "--fail-fast", "unit", "--fail-fast"]);
    assert!(matches!(
        cli.command,
        Some(Command::Test {
            fail_fast: true,
            ..
        })
    ));
    let forwarded = request(&["sc-lint", "test", "--fail-fast", "unit", "--fail-fast"]);
    assert_eq!(forwarded.selector(), Some("unit"));
    assert_eq!(forwarded.args(), ["--fail-fast"]);
}

#[test]
fn human_render_lists_failed_and_skipped_steps() {
    // Feed fixed outcomes to the aggregator: scheduler start order between
    // the cargo lane and parallel workers is intentionally concurrent, so
    // running real workers here would make the expected fail-fast result race.
    let steps = vec![
        crate::scheduler::ScheduledStep::new(0, cargo_step("a")),
        crate::scheduler::ScheduledStep::new(1, cargo_step("b")),
        crate::scheduler::ScheduledStep::new(2, parallel_step("c")),
    ];
    let error = crate::scheduler::aggregate_outcomes(
        &steps,
        vec![
            crate::scheduler::StepOutcome::Failed(
                CliError::backend_failure("a exploded")
                    .with_detail(crate::consts::FIELD_EXIT_CODE, json!(101)),
            ),
            crate::scheduler::StepOutcome::Skipped,
            crate::scheduler::StepOutcome::Failed(CliError::internal(
                "c panicked during execution: boom",
            )),
        ],
    )
    .expect_err("the controlled outcomes contain failures");
    assert_eq!(
        crate::render::render_error_human("test", &error),
        "test: 2 of 3 steps failed: [0] a, [2] c (profile.steps_failed)\nFailed steps:\n  [0] a (exit 101): a exploded\n  [2] c (exit n/a): c panicked during execution: boom\nSkipped steps (fail-fast): b\nfix the failing steps and rerun"
    );
}

fn context_with_repo_root(args: &[&str]) -> (Cli, CommandContext, LoadedConfig) {
    let root = repo_backed_workspace_root();
    let mut argv = vec![
        OsString::from("sc-lint"),
        OsString::from("--root"),
        root.as_os_str().to_os_string(),
    ];
    argv.extend(args.iter().map(OsString::from));
    let cli = Cli::parse_from(argv);
    let context = CommandContext::from_cli(&cli).expect("command context");
    let loaded = LoadedConfig::load(&cli, &context).expect("config loads");
    (cli, context, loaded)
}

fn success_envelope_json(command: &str, data: Value) -> Value {
    let rendered = crate::render::render_success_json(&CommandEnvelope::success(command, data));
    serde_json::from_str(&rendered).expect("success envelope is json")
}

fn failure_envelope_json(command: &str, error: CliError) -> Value {
    let rendered = crate::render::render_error_json(command, &error);
    serde_json::from_str(&rendered).expect("failure envelope is json")
}

/// The stable `command` patterns of the REQ-CLI-005I contract matrix in
/// `docs/sc-lint/cli-contract.md`, one per command-family row.
fn contract_matrix_patterns() -> Vec<String> {
    let contract = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/sc-lint/cli-contract.md");
    let text = fs::read_to_string(&contract).expect("read cli-contract.md");
    let mut lines = text
        .lines()
        .skip_while(|line| !line.starts_with("| Command family | Stable `command` pattern |"))
        .skip(2);
    let mut patterns = Vec::new();
    for line in lines.by_ref().take_while(|line| line.starts_with('|')) {
        let pattern_cell = line
            .split('|')
            .nth(2)
            .expect("matrix row has a pattern cell");
        let pattern = pattern_cell
            .split('`')
            .nth(1)
            .unwrap_or_else(|| panic!("matrix pattern is not backticked: {line}"));
        patterns.push(pattern.to_owned());
    }
    assert!(
        !patterns.is_empty(),
        "contract matrix not found in {}",
        contract.display()
    );
    patterns
}

/// The contract-matrix row a concrete command identifier belongs to: an exact
/// pattern wins, otherwise the longest `<prefix>.<placeholder>` pattern.
fn contract_matrix_row<'a>(patterns: &'a [String], command: &str) -> Option<&'a str> {
    if let Some(exact) = patterns.iter().find(|pattern| pattern.as_str() == command) {
        return Some(exact);
    }
    patterns
        .iter()
        .filter_map(|pattern| {
            let prefix = pattern.split_once('<')?.0;
            command
                .starts_with(prefix)
                .then_some((prefix.len(), pattern.as_str()))
        })
        .max_by_key(|(prefix_len, _)| *prefix_len)
        .map(|(_, pattern)| pattern)
}

/// Every contract-matrix row is exercised by at least one of `commands`, and
/// every command belongs to a row.
fn assert_contract_matrix_covered(commands: &[&str], requirement: &str) {
    let patterns = contract_matrix_patterns();
    for command in commands {
        assert!(
            contract_matrix_row(&patterns, command).is_some(),
            "{requirement}: `{command}` has no row in the cli-contract.md matrix"
        );
    }
    for pattern in &patterns {
        assert!(
            commands
                .iter()
                .any(|command| contract_matrix_row(&patterns, command) == Some(pattern.as_str())),
            "{requirement}: no case covers the cli-contract.md matrix row `{pattern}`"
        );
    }
}

#[test]
fn contract_matrix_rows_resolve_concrete_commands() {
    let patterns = contract_matrix_patterns();
    for (command, row) in [
        ("lint.sc-boundary", "lint.<tool-or-profile>"),
        ("lint.ci.consumer", "lint.ci.consumer"),
        ("check.native", "check.<target>"),
        ("clippy.xwin", "clippy.<target>"),
        ("view.graph", "view.<target>"),
        ("compatibility.check", "compatibility.check"),
        ("cli.parse_error", "cli.parse_error"),
    ] {
        assert_eq!(
            contract_matrix_row(&patterns, command),
            Some(row),
            "{command}"
        );
    }
    assert_eq!(contract_matrix_row(&patterns, "unknown"), None);
}

fn assert_envelope_keys(envelope: &Value, command: &str, success: bool, error_kind: Option<&str>) {
    let keys = envelope
        .as_object()
        .expect("envelope object")
        .keys()
        .map(String::as_str)
        .collect::<Vec<_>>();
    let expected = if success {
        vec!["command", "data", "diagnostics", "ok"]
    } else {
        vec!["command", "diagnostics", "error", "ok"]
    };
    assert_eq!(keys, expected, "envelope keys for {command}");
    assert_eq!(envelope["ok"], success);
    assert_eq!(envelope["command"], command);
    assert!(envelope["diagnostics"].is_array());
    if let Some(error_kind) = error_kind {
        assert_eq!(envelope["error"]["kind"], error_kind);
    }
}

#[test]
fn envelope_key_set_identical_across_families() {
    let mut cases: Vec<(Value, bool, String, Option<&str>)> = Vec::new();

    let (_, context, loaded) = context_with_repo_root(&["version"]);
    let version = crate::command::execute(&context, &loaded).expect("version succeeds");
    cases.push((
        success_envelope_json(context.command_id(), version.data),
        true,
        "version".to_owned(),
        None,
    ));

    let (_, context, loaded) = context_with_repo_root(&["docs", "--path"]);
    let docs = crate::command::execute(&context, &loaded).expect("docs path succeeds");
    cases.push((
        success_envelope_json(context.command_id(), docs.data),
        true,
        "docs".to_owned(),
        None,
    ));

    let installer_fixture = TempDir::new().expect("installer config fixture");
    let config_path = installer_fixture.path().join("sc-lint.toml");
    fs::write(
        &config_path,
        "[tool.sc-lint]\nminimum_version = \"0.4.1\"\n",
    )
    .expect("installer config");
    let config = config_path.to_str().expect("config path");
    let cli = Cli::parse_from(["sc-lint", "--config", config, "setup", "--dry-run"]);
    let context = CommandContext::from_cli(&cli).expect("setup context");
    let loaded = LoadedConfig::load(&cli, &context).expect("setup config loads");
    let setup = crate::command::execute(&context, &loaded).expect("setup dry-run succeeds");
    cases.push((
        success_envelope_json(context.command_id(), setup.data),
        true,
        "setup".to_owned(),
        None,
    ));
    let cli = Cli::parse_from([
        "sc-lint",
        "--config",
        config,
        "upgrade",
        "--check",
        "--dry-run",
    ]);
    let context = CommandContext::from_cli(&cli).expect("upgrade context");
    let loaded = LoadedConfig::load(&cli, &context).expect("upgrade config loads");
    let upgrade = crate::command::execute(&context, &loaded).expect("upgrade dry-run succeeds");
    cases.push((
        success_envelope_json(context.command_id(), upgrade.data),
        true,
        "upgrade".to_owned(),
        None,
    ));

    let init_fixture = TempDir::new().expect("init fixture");
    let cli = Cli::parse_from(["sc-lint", "init", "--just"]);
    let context = CommandContext::from_cli(&cli).expect("init context");
    let init = crate::config::run_consumer_init_at(
        init_fixture.path(),
        ConsumerInitRequest {
            just: true,
            check: false,
            dry_run: true,
        },
    )
    .expect("init dry-run succeeds");
    cases.push((
        success_envelope_json(context.command_id(), init),
        true,
        "init".to_owned(),
        None,
    ));

    let (_, _, loaded) = context_with_repo_root(&["ci"]);
    let ci = workflow::run_ci_with(&loaded, &FakeSystemAdapter::new(false))
        .expect("ci succeeds with fake adapter");
    cases.push((
        success_envelope_json("ci", ci.data),
        true,
        "ci".to_owned(),
        None,
    ));

    for target in ClippyTarget::value_variants() {
        let name = target.to_possible_value().expect("clippy target name");
        let (_, context, loaded) = context_with_repo_root(&["clippy", name.get_name()]);
        let clippy = workflow::run_clippy_with(&loaded, *target, &FakeSystemAdapter::new(true))
            .expect("clippy succeeds with fake adapter");
        cases.push((
            success_envelope_json(context.command_id(), clippy.data),
            true,
            context.command_id().to_owned(),
            None,
        ));
    }
    for target in CheckTarget::value_variants() {
        let name = target.to_possible_value().expect("check target name");
        let (_, context, loaded) = context_with_repo_root(&["check", name.get_name()]);
        let check = workflow::run_check_with(&loaded, *target, &FakeSystemAdapter::new(true))
            .expect("check succeeds with fake adapter");
        cases.push((
            success_envelope_json(context.command_id(), check.data),
            true,
            context.command_id().to_owned(),
            None,
        ));
    }

    let lint_fixture = AnalysisFixture::new();
    lint_fixture.write_workspace_root();
    lint_fixture.write_package_manifest("example");
    lint_fixture.write_source("example", "lib.rs", "pub struct Example;\n");
    let cli = Cli::parse_from([
        "sc-lint",
        "--root",
        lint_fixture.root().to_str().expect("lint root"),
        "lint",
        "sc-boundary",
    ]);
    let context = CommandContext::from_cli(&cli).expect("lint context");
    let loaded = LoadedConfig::load(&cli, &context).expect("lint config loads");
    let lint = crate::command::execute(&context, &loaded).expect("sc-boundary lint succeeds");
    cases.push((
        success_envelope_json(context.command_id(), lint.data),
        true,
        "lint.sc-boundary".to_owned(),
        None,
    ));

    let compat_fixture = TempDir::new().expect("compatibility fixture");
    let compat_config = compat_fixture.path().join("sc-lint.toml");
    let missing_binary = compat_fixture.path().join("missing-sc-lint");
    fs::write(
        &compat_config,
        "[tool.sc-lint]\nminimum_version = \"0.4.1\"\n",
    )
    .expect("compatibility config");
    let cli = Cli::parse_from([
        "sc-lint",
        "--config",
        compat_config.to_str().unwrap(),
        "compatibility",
        "check",
        "--binary",
        missing_binary.to_str().unwrap(),
    ]);
    let context = CommandContext::from_cli(&cli).expect("compatibility context");
    let loaded = LoadedConfig::load(&cli, &context).expect("compatibility config loads");
    let error = crate::command::execute(&context, &loaded).expect_err("missing binary fails");
    assert_eq!(error.kind, CliErrorKind::BackendFailure);
    cases.push((
        failure_envelope_json(context.command_id(), error),
        false,
        "compatibility.check".to_owned(),
        Some("backend_failure"),
    ));

    let (_, context, loaded) = context_with_repo_root(&["view", "graph"]);
    let error = crate::command::execute(&context, &loaded).expect_err("reserved view fails");
    cases.push((
        failure_envelope_json(context.command_id(), error),
        false,
        "view.graph".to_owned(),
        Some("capability"),
    ));

    let (_, _, loaded) = context_with_repo_root(&["check", "xwin"]);
    let error =
        workflow::run_check_with(&loaded, CheckTarget::Xwin, &FakeSystemAdapter::new(false))
            .expect_err("xwin requires capability");
    cases.push((
        failure_envelope_json("check.xwin", error),
        false,
        "check.xwin".to_owned(),
        Some("capability"),
    ));

    let consumer_lint_fixture = TempDir::new().expect("consumer lint config");
    let consumer_lint_config = consumer_lint_fixture.path().join("sc-lint.toml");
    fs::write(
        &consumer_lint_config,
        "[tool.sc-lint]\nminimum_version = \"0.4.1\"\n\n[[tool.sc-lint.test]]\nname = \"unit\"\ncommand = [\"cargo\", \"test\"]\n",
    )
    .expect("test-only consumer config");
    let cli = Cli::parse_from([
        "sc-lint",
        "--config",
        consumer_lint_config.to_str().unwrap(),
        "lint",
        "ci",
        "--consumer",
    ]);
    let context = CommandContext::from_cli(&cli).expect("consumer lint context");
    let error = LoadedConfig::load(&cli, &context).expect_err("lint profile is missing");
    cases.push((
        failure_envelope_json(context.command_id(), error),
        false,
        "lint.ci.consumer".to_owned(),
        Some("config"),
    ));

    let consumer_test_fixture = TempDir::new().expect("consumer test config");
    let consumer_test_config = consumer_test_fixture.path().join("sc-lint.toml");
    fs::write(
        &consumer_test_config,
        "[tool.sc-lint]\nminimum_version = \"0.4.1\"\n\n[[tool.sc-lint.lint]]\nname = \"fmt\"\ncommand = [\"cargo\", \"fmt\"]\n",
    )
    .expect("lint-only consumer config");
    let cli = Cli::parse_from([
        "sc-lint",
        "--config",
        consumer_test_config.to_str().unwrap(),
        "test",
    ]);
    let context = CommandContext::from_cli(&cli).expect("consumer test context");
    let error = LoadedConfig::load(&cli, &context).expect_err("test profile is missing");
    cases.push((
        failure_envelope_json(context.command_id(), error),
        false,
        "test".to_owned(),
        Some("config"),
    ));

    let ParsedInvocation::Immediate(parse_error) =
        crate::parse_args(["sc-lint", "--json", "unknown-command"])
    else {
        panic!("unknown command must fail during parsing");
    };
    let rendered = parse_error.rendered.stderr.expect("parse-error json");
    cases.push((
        serde_json::from_str(&rendered).expect("parse-error envelope parses"),
        false,
        "cli.parse_error".to_owned(),
        Some("usage"),
    ));

    let commands = cases
        .iter()
        .map(|(_, _, command, _)| command.as_str())
        .collect::<Vec<_>>();
    assert_contract_matrix_covered(&commands, "REQ-CLI-005I");
    for target in ["check.native", "check.xwin", "clippy.native", "clippy.xwin"] {
        assert!(
            commands.contains(&target),
            "REQ-CLI-005I: no case for {target}"
        );
    }
    for (envelope, success, command, error_kind) in cases {
        assert_envelope_keys(&envelope, &command, success, error_kind);
    }
}

#[test]
fn human_output_fields_present_in_json() {
    type HumanOutputRow<'a> = (&'a str, &'a [&'a str], Value, &'a [&'a str], bool);
    let rows: Vec<HumanOutputRow<'_>> = vec![
        (
            "analyzer-lint",
            &["lint", "sc-boundary"],
            json!({"status":"fail","findings":[{"rule_id":"SCB-CYCLE-001"}]}),
            &["/data/status", "/data/findings"],
            false,
        ),
        (
            "profile",
            &["lint", "fast"],
            json!({"status":"pass","steps":[{"name":"a"},{"name":"b"}]}),
            &["/command", "/data/status", "/data/steps"],
            false,
        ),
        (
            "python-lint-view",
            &["lint", "line-counts"],
            json!({"status":"pass","summary":"limits satisfied"}),
            &["/command", "/data/summary"],
            false,
        ),
        (
            "version",
            &["version"],
            json!({"version":"1.2.3"}),
            &["/data/version"],
            false,
        ),
        (
            "installer",
            &["setup"],
            json!({"summary":"setup complete"}),
            &["/data/summary"],
            false,
        ),
        (
            "docs",
            &["docs"],
            json!({"path":"/x/docs","content":"# Guide"}),
            &["/data/path", "/data/content"],
            false,
        ),
        (
            "test",
            &["test"],
            json!({"status":"pass","steps":[{"name":"unit"},{"name":"doc"}]}),
            &["/command", "/data/status", "/data/steps"],
            false,
        ),
        (
            "lint.ci.consumer",
            &["lint", "ci", "--consumer"],
            json!({"status":"pass","steps":[{"name":"fmt"},{"name":"clippy"}]}),
            &["/command", "/data/status", "/data/steps"],
            false,
        ),
        (
            "init",
            &["init", "--just"],
            json!({"status":"created","path":"x","count":3}),
            &["/command"],
            true,
        ),
        (
            "compatibility.check",
            &["compatibility", "check"],
            json!({"status":"checked","path":"x"}),
            &["/command"],
            true,
        ),
        (
            "view.graph",
            &["view", "graph"],
            json!({"status":"checked","path":"x"}),
            &["/command"],
            true,
        ),
        (
            "lint.full",
            &["lint", "full"],
            json!({"status":"pass","steps":[{"name":"a"},{"name":"b"}]}),
            &["/command", "/data/status", "/data/steps"],
            false,
        ),
        (
            "lint.ci",
            &["lint", "ci"],
            json!({"status":"pass","steps":[{"name":"a"},{"name":"b"}]}),
            &["/command", "/data/status", "/data/steps"],
            false,
        ),
        (
            "ci",
            &["ci"],
            json!({"status":"pass","steps":[{"name":"a"},{"name":"b"}]}),
            &["/command", "/data/status", "/data/steps"],
            false,
        ),
        (
            "check.native",
            &["check", "native"],
            json!({"status":"pass","steps":[{"name":"a"},{"name":"b"}]}),
            &["/command", "/data/status", "/data/steps"],
            false,
        ),
        (
            "check.xwin",
            &["check", "xwin"],
            json!({"status":"pass","steps":[{"name":"a"},{"name":"b"}]}),
            &["/command", "/data/status", "/data/steps"],
            false,
        ),
        (
            "clippy.native",
            &["clippy", "native"],
            json!({"status":"pass","steps":[{"name":"a"},{"name":"b"}]}),
            &["/command", "/data/status", "/data/steps"],
            false,
        ),
        (
            "clippy.xwin",
            &["clippy", "xwin"],
            json!({"status":"pass","steps":[{"name":"a"},{"name":"b"}]}),
            &["/command", "/data/status", "/data/steps"],
            false,
        ),
        (
            "lint.sc-portability",
            &["lint", "sc-portability"],
            json!({"status":"pass","findings":[]}),
            &["/data/status", "/data/findings"],
            false,
        ),
        (
            "lint.sc-runtime",
            &["lint", "sc-runtime"],
            json!({"status":"pass","findings":[]}),
            &["/data/status", "/data/findings"],
            false,
        ),
        (
            "lint.function-length",
            &["lint", "function-length"],
            json!({"status":"pass","summary":"no findings"}),
            &["/command", "/data/summary"],
            false,
        ),
        (
            "lint.identity-literals",
            &["lint", "identity-literals"],
            json!({"status":"pass","summary":"no findings"}),
            &["/command", "/data/summary"],
            false,
        ),
        (
            "view.findings",
            &["view", "findings"],
            json!({"status":"pass","summary":"no findings"}),
            &["/command", "/data/summary"],
            false,
        ),
        (
            "upgrade",
            &["upgrade"],
            json!({"summary":"upgrade complete"}),
            &["/data/summary"],
            false,
        ),
        (
            "test.list",
            &["test", "--list"],
            json!({"default":"unit","selectors":[{"name":"unit","command":["cargo","test"]},{"name":"all","group":["unit"]}]}),
            &[
                "/data/default",
                "/data/selectors/0/name",
                "/data/selectors/1/name",
            ],
            false,
        ),
    ];
    let mut family_names = Vec::new();
    let mut covered_ids = Vec::new();
    let mut covered_commands = Vec::new();
    for (family, argv, data, pointers, fallback) in rows {
        family_names.push(family);
        assert!(
            !pointers.is_empty(),
            "{family}: pointer list must not be empty"
        );
        let mut full_argv = vec!["sc-lint"];
        full_argv.extend_from_slice(argv);
        let cli = Cli::parse_from(full_argv);
        let context = CommandContext::from_cli(&cli).expect("context");
        covered_ids.push(context.id());
        covered_commands.push(context.command_id().to_owned());
        let envelope = CommandEnvelope::success(context.command_id(), data);
        let human = crate::render::render_success_human(&context, &envelope);
        if fallback {
            assert_eq!(human, format!("{}: ok", context.command_id()));
        }
        let serialized = serde_json::to_value(&envelope).expect("envelope serializes");
        for pointer in pointers {
            let value = serialized
                .pointer(pointer)
                .unwrap_or_else(|| panic!("{family}: missing {pointer}"));
            let token = match value {
                Value::Array(items) => format!("({} ", items.len()),
                Value::String(text) => text.clone(),
                other => other.to_string(),
            };
            assert!(!token.is_empty(), "{family}: empty token at {pointer}");
            assert!(
                human.contains(&token),
                "{family}: human output {human:?} omits {pointer} token {token:?}"
            );
        }
    }

    let mut error = CliError::config("minimum version missing")
        .with_cause("file not found")
        .with_suggested_action("add the field")
        .with_documentation("sc-lint docs setup");
    for (key, value) in [
        ("minimum_version", json!("1.0")),
        ("installed_version", json!("0.9")),
        ("reported_version", json!("0.8")),
        ("binary_path", json!("/bin/sc-lint")),
        ("config_path", json!("sc-lint.toml")),
        ("bundle_path", json!("docs-bundle")),
        ("required_field", json!("minimum_version")),
        ("exit_code", json!(7)),
        ("stdout", json!("stdout text")),
        ("stderr", json!("stderr text")),
    ] {
        error = error.with_detail(key, value);
    }
    let error_envelope = failure_envelope_json("compatibility.check", error.clone());
    let human = crate::render::render_error_human("compatibility.check", &error);
    family_names.push("error");
    for pointer in [
        "/command",
        "/error/message",
        "/error/code",
        "/error/cause",
        "/error/suggested_action",
        "/error/docs",
        "/error/details/minimum_version",
        "/error/details/installed_version",
        "/error/details/reported_version",
        "/error/details/binary_path",
        "/error/details/config_path",
        "/error/details/bundle_path",
        "/error/details/required_field",
        "/error/details/exit_code",
        "/error/details/stdout",
        "/error/details/stderr",
    ] {
        let value = error_envelope
            .pointer(pointer)
            .unwrap_or_else(|| panic!("error: missing {pointer}"));
        let token = value
            .as_str()
            .map(str::to_owned)
            .unwrap_or_else(|| value.to_string());
        assert!(
            human.contains(&token),
            "error human output omits {pointer} token {token:?}"
        );
    }
    let ParsedInvocation::Immediate(human_parse) =
        crate::parse_args(["sc-lint", "unknown-command"])
    else {
        panic!("unknown command must fail during parsing");
    };
    let ParsedInvocation::Immediate(json_parse) =
        crate::parse_args(["sc-lint", "--json", "unknown-command"])
    else {
        panic!("unknown command must fail during parsing");
    };
    let human = human_parse.rendered.stderr.expect("human parse error");
    let parse_envelope: Value =
        serde_json::from_str(&json_parse.rendered.stderr.expect("json parse error"))
            .expect("parse-error envelope parses");
    family_names.push("cli.parse_error");
    for pointer in [
        "/command",
        "/error/code",
        "/error/message",
        "/error/suggested_action",
    ] {
        let token = parse_envelope
            .pointer(pointer)
            .and_then(Value::as_str)
            .unwrap_or_else(|| panic!("cli.parse_error: missing {pointer}"));
        assert!(
            human.contains(token.trim_end()),
            "cli.parse_error human output omits {pointer} token {token:?}"
        );
    }
    covered_commands.push("cli.parse_error".to_owned());

    let mut unique_families = family_names.clone();
    unique_families.sort_unstable();
    unique_families.dedup();
    assert_eq!(unique_families.len(), family_names.len(), "duplicate rows");
    for id in every_command_id() {
        assert!(
            covered_ids.contains(&id),
            "REQ-CLI-005D: no human-output row for {id:?}"
        );
    }
    let commands = covered_commands
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    assert_contract_matrix_covered(&commands, "REQ-CLI-005D");
}

/// Lists every `CommandId` from one token list that also expands to a
/// wildcard-free `match`: a variant missing from the list leaves the match
/// non-exhaustive, so the test crate does not compile until the variant is
/// listed here (and so given a REQ-CLI-005D row). A duplicate entry fails the
/// check below.
macro_rules! all_command_ids {
    ($($variant:ident),+ $(,)?) => {{
        use crate::command::CommandId;
        #[allow(dead_code, reason = "exists only to force an exhaustive match")]
        fn exhaustive(id: CommandId) {
            match id {
                $(CommandId::$variant => {})+
            }
        }
        vec![$(CommandId::$variant),+]
    }};
}

fn every_command_id() -> Vec<crate::command::CommandId> {
    let ids = all_command_ids![
        Ci,
        CheckNative,
        CheckXwin,
        ClippyNative,
        ClippyXwin,
        CompatibilityCheck,
        Docs,
        Init,
        ConsumerTest,
        Setup,
        Upgrade,
        LintCi,
        ConsumerLintCi,
        LintFast,
        LintFull,
        LintFunctionLength,
        LintIdentityLiterals,
        LintLineCounts,
        LintScBoundary,
        LintScPortability,
        LintScRuntime,
        Version,
        ViewFindings,
        ViewGraph,
    ];
    for (index, id) in ids.iter().enumerate() {
        assert!(!ids[..index].contains(id), "{id:?} listed twice");
    }
    ids
}

mod host_rows;
