use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::OsString;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
#[cfg(any(unix, windows))]
use std::process::Command as ProcessCommand;

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
use crate::DocsGuide;
use crate::LintTarget;
use crate::ParsedInvocation;
use crate::ViewTarget;
use crate::cli::OutputMode;
use crate::command::CommandContext;
use crate::command::ConsumerInitRequest;
use crate::config::ConsumerProfile;
use crate::config::LoadedConfig;
use crate::workflow;

const CANONICAL_CONSUMER_JUSTFILE: &str = include_str!("../assets/consumer-Justfile");

#[test]
fn canonical_consumer_justfile_is_thin_and_has_exactly_four_public_recipes() {
    let canonical = CANONICAL_CONSUMER_JUSTFILE.replace("\r\n", "\n");
    assert!(canonical.starts_with("set windows-shell := [\"pwsh\", \"-NoLogo\", \"-Command\"]\n"));
    for recipe in ["setup", "lint", "test", "upgrade"] {
        assert!(
            canonical.contains(&format!(
                "{recipe}:\n    {{{{bootstrap_command}}}} {recipe} --config sc-lint.toml"
            )),
            "consumer template does not delegate `{recipe}` to the product bootstrap"
        );
    }
    assert!(canonical.contains("bootstrap_command := if os_family() == \"windows\""));
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
        })
    ));
    let test = Cli::parse_from(["sc-lint", "test"]);
    assert!(matches!(test.command, Some(Command::Test)));
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
    assert!(root.join(".sc-lint/bootstrap").is_file());
    assert!(root.join(".sc-lint/bootstrap.ps1").is_file());
    assert_eq!(
        fs::read(root.join(".sc-lint/bootstrap"))
            .expect("bootstrap")
            .get(..9),
        Some(&b"#!/bin/sh"[..])
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
    let lint = workflow::run_consumer_profile_with(&loaded, ConsumerProfile::Lint, &passing)
        .expect("all lint members run");
    assert_eq!(
        step_names(lint.data["steps"].as_array().expect("steps")),
        vec!["lint-fmt", "lint-clippy"]
    );
    assert_eq!(
        *passing.invocations.borrow(),
        vec!["lint-fmt", "lint-clippy"]
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
    let error = workflow::run_consumer_profile_with(&loaded, ConsumerProfile::Test, &failing)
        .expect_err("a required test member fails the aggregate");
    assert_eq!(error.kind, CliErrorKind::BackendFailure);
    assert_eq!(*failing.invocations.borrow(), vec!["test-workspace"]);
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
    let error = workflow::run_consumer_lint_profile(&loaded).expect_err("missing backend fails");
    assert_eq!(error.code(), "CLI.SC_LINT_BACKEND_NOT_FOUND");
    assert_eq!(error.documentation.as_deref(), Some("sc-lint docs setup"));
    assert!(!error.to_string().to_ascii_lowercase().contains("traceback"));
}

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

    for recipe in ["lint", "test"] {
        let output = ProcessCommand::new("just")
            .current_dir(root)
            .arg(recipe)
            .env("PATH", &path)
            .env("SC_LINT_BIN", &binary)
            .env("SC_LINT_RECORD", &record)
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
            "lint --consumer --config sc-lint.toml ci".to_string(),
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

    for recipe in ["lint", "test"] {
        let output = ProcessCommand::new("just")
            .current_dir(root)
            .arg(recipe)
            .env("PATH", &path)
            .env("SC_LINT_BIN", &binary)
            .env("SC_LINT_RECORD", &record)
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
            "lint --consumer --config sc-lint.toml ci".to_string(),
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
    let bootstrap = fixture.path().join(".sc-lint/bootstrap.ps1");
    let record = fixture.path().join("calls.txt");
    let binary = fixture.path().join("sc-lint.cmd");
    fs::write(&binary, "@echo off\r\necho %*>> \"%SC_LINT_RECORD%\"\r\n").expect("fake sc-lint");
    let output = ProcessCommand::new("pwsh")
        .args([
            "-NoLogo",
            "-NonInteractive",
            "-File",
            bootstrap.to_str().expect("UTF-8 bootstrap path"),
            "setup",
            "--config",
            "sc-lint.toml",
        ])
        .current_dir(fixture.path())
        .env("SC_LINT_BIN", &binary)
        .env("SC_LINT_RECORD", &record)
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
                owner_package.replace('-', "_"),
                owner_package.replace('-', "_"),
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
            executable: script_path,
            original_path,
        }
    }

    fn path(&self) -> &Path {
        &self.executable
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
