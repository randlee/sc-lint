//! REQ-CLI-028 acceptance cases use injected hosts and controlled execution.
use super::*;
use crate::scheduler::FailurePolicy;
use crate::test_selectors::TestRequest;

fn row(name: &str, fields: &str) -> String {
    format!(
        "[[tool.sc-lint.lint]]\nname = \"{name}\"\ncommand = [\"cargo\", \"clippy\"]\n{fields}\n"
    )
}

fn run_rows(
    text: &str,
    selector: Option<&str>,
    scope: PlatformScope,
    host: Platform,
) -> serde_json::Value {
    let (_temp, loaded) = load_consumer_toml(text);
    let loaded = loaded.expect("valid consumer");
    let adapter = FakeSystemAdapter::new(true).on_host(host);
    workflow::run_consumer_lint_with(&loaded, selector, scope, FailurePolicy::KeepGoing, &adapter)
        .expect("run")
        .data
}

#[test]
fn ac1_scope_then_host_then_actual_host_command_adaptation() {
    let text = consumer_toml(&row(
        "cross",
        "platform = 'windows'\ntarget = 'x86_64-pc-windows-msvc'\nhost = 'linux'",
    ));
    for host in Platform::ALL {
        let host_rows = run_rows(&text, None, PlatformScope::Host, host);
        if host != Platform::Windows {
            assert_eq!(host_rows["steps"], json!([]));
        }
        let all = run_rows(&text, None, PlatformScope::All, host);
        let item = &all["steps"][0];
        assert_eq!(item["host"], host.as_str());
        assert_eq!(item["host_selector"], "linux");
        if host == Platform::Linux {
            assert!(
                item["command"]
                    .as_str()
                    .unwrap()
                    .contains("cargo xwin clippy")
            );
        } else {
            assert_eq!(item["reason"], "host_mismatch");
        }
    }
    for host in [Platform::Linux, Platform::MacOS] {
        let text = consumer_toml(&row(
            "cross",
            &format!(
                "platform = 'windows'\ntarget = 'x86_64-pc-windows-msvc'\nhost = '{}'",
                host.as_str()
            ),
        ));
        assert!(
            run_rows(&text, None, PlatformScope::All, host)["steps"][0]["command"]
                .as_str()
                .unwrap()
                .contains("cargo xwin")
        );
    }
    let native = consumer_toml(&row(
        "native",
        "platform='windows'\ntarget='x86_64-pc-windows-msvc'\nhost='windows'",
    ));
    let report = run_rows(&native, None, PlatformScope::All, Platform::Windows);
    let command = report["steps"][0]["command"].as_str().unwrap();
    assert!(command.contains("cargo clippy"));
    assert!(!command.contains("xwin"));
    let text = consumer_toml(&row(
        "unsupported",
        "platform = 'windows'\ntarget = 'aarch64-pc-windows-msvc'\nhost = 'linux'\nenv_paths={PYTHON='secret/path'}",
    ));
    assert_eq!(
        run_rows(&text, None, PlatformScope::All, Platform::Linux)["steps"][0]["reason"],
        "unsupported_cross_target"
    );
    assert_eq!(
        run_rows(&text, None, PlatformScope::All, Platform::Linux)["steps"][0]["env_path_keys"],
        json!(["PYTHON"])
    );
    assert_eq!(
        run_rows(&text, None, PlatformScope::All, Platform::MacOS)["steps"][0]["reason"],
        "host_mismatch"
    );
}

#[test]
fn ac2_tagged_and_untagged_report_shapes() {
    let text = consumer_toml(
        &(row("plain", "")
            + &row("tagged", "host = 'windows'")
            + &row("legacy_cross", "platform = 'macos'")),
    );
    let report = run_rows(&text, None, PlatformScope::All, Platform::Windows);
    assert!(report["steps"][0].get("host").is_none());
    assert_eq!(report["steps"][1]["host"], "windows");
    assert_eq!(report["steps"][1]["host_selector"], "windows");
    assert_eq!(report["steps"][2]["reason"], "unsupported_cross_command");
    assert!(report["steps"][2].get("host_selector").is_none());
    assert!(report["steps"][1].get("actual_host").is_none());
}

#[test]
fn ac3_host_skips_do_not_trip_fail_fast_or_after_all_barriers() {
    for host in Platform::ALL {
        let other = if host == Platform::Windows {
            "linux"
        } else {
            "windows"
        };
        let text = consumer_toml(
            &(row("normal", "")
                + &row("skip", &format!("host = '{other}'\nafter_all = true"))
                + &row("finish", "after_all = true")),
        );
        let (_temp, loaded) = load_consumer_toml(&text);
        let adapter = FakeSystemAdapter::new(true).on_host(host);
        let report = workflow::run_consumer_lint_with(
            &loaded.unwrap(),
            None,
            PlatformScope::All,
            FailurePolicy::FailFast,
            &adapter,
        )
        .unwrap();
        assert_eq!(adapter.invoked(), ["normal", "finish"]);
        assert_eq!(report.data["steps"][1]["reason"], "host_mismatch");
        assert_eq!(report.data["steps"][2]["status"], "pass");
    }
}

fn with_default(value: &str) -> String {
    let rows = row("local_step", "") + &row("ci_step", "");
    format!(
        "[tool.sc-lint]\nminimum_version='0.4.0'\ndefault_lint_profile={value}\n[tool.sc-lint.lint_profiles]\nlocal=['local_step']\nci=['local_step','ci_step']\n{rows}\n[[tool.sc-lint.test]]\nname='unit'\ncommand=['cargo','test']\n"
    )
}

#[test]
fn ac4_default_profile_only_applies_to_bare_lint() {
    for host in Platform::ALL {
        let text = with_default("'local'");
        let bare = run_rows(&text, None, PlatformScope::All, host);
        assert_eq!(bare["steps"].as_array().unwrap().len(), 1);
        assert_eq!(
            run_rows(&text, Some("ci"), PlatformScope::All, host)["steps"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
        assert_eq!(
            run_rows(&text, Some("all"), PlatformScope::All, host)["steps"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
        let (_temp, loaded) = load_consumer_toml(&text);
        let adapter = FakeSystemAdapter::new(true).on_host(host);
        let test = workflow::run_consumer_test_with(
            &loaded.unwrap(),
            &TestRequest::all(),
            FailurePolicy::KeepGoing,
            &adapter,
        )
        .unwrap();
        assert_eq!(test.data["steps"][0]["name"], "unit");
    }
}

#[test]
fn ac5_default_resolution_is_lazy_and_profiles_only() {
    for default in ["'unknown'", "'all'", "'local_step'", "''"] {
        let (_temp, loaded) = load_consumer_toml(&with_default(default));
        let loaded = loaded.unwrap();
        let error = loaded.consumer_lint_inputs(None).unwrap_err();
        assert_eq!(
            error.details["detail_code"],
            "PROFILE.UNKNOWN_DEFAULT_LINT_PROFILE"
        );
        assert_eq!(error.details["available"], json!(["ci", "local"]));
        assert!(loaded.consumer_lint_inputs(Some("ci")).is_ok());
        let adapter = FakeSystemAdapter::new(true).on_host(Platform::Linux);
        assert!(
            workflow::run_consumer_test_with(
                &loaded,
                &TestRequest::all(),
                FailurePolicy::KeepGoing,
                &adapter
            )
            .is_ok()
        );
    }
    assert!(load_consumer_toml(&with_default("7")).1.is_err());
    for (profiles, code) in [
        ("local=[]\n", "PROFILE.EMPTY_MEMBERS"),
        ("all=['local_step']\n", "PROFILE.AMBIGUOUS_NAME"),
    ] {
        let text = format!(
            "[tool.sc-lint]\nminimum_version='0.4.0'\n[tool.sc-lint.lint_profiles]\n{profiles}{}[[tool.sc-lint.test]]\nname='unit'\ncommand=['cargo','test']",
            row("local_step", "")
        );
        assert_eq!(
            load_consumer_toml(&text).1.unwrap_err().details["detail_code"],
            code
        );
    }
}

#[test]
fn invalid_host_fails_before_platform_scope_filter() {
    for host in ["'freebsd'", "4", "['linux']", "true"] {
        let text = consumer_toml(&row("bad", &format!("platform='windows'\nhost={host}")));
        assert_eq!(
            load_consumer_toml(&text).1.unwrap_err().details["detail_code"],
            "PROFILE.INVALID_HOST"
        );
    }
}

#[test]
fn host_platform_mismatch_without_target_is_invalid() {
    let text = consumer_toml(&(row("bad", "platform='windows'\nhost='linux'") + &row("good", "")));
    let (_temp, loaded) = load_consumer_toml(&text);
    let loaded = loaded.unwrap();
    let input = loaded.consumer_lint_inputs(Some("bad")).unwrap();
    let error = plan_consumer_lint(
        &input.steps,
        input.selector.as_ref(),
        PlatformScope::Host,
        Platform::Linux,
    )
    .unwrap_err();
    assert_eq!(error.detail_code.as_str(), "PROFILE.INVALID_HOST");
    assert!(loaded.consumer_lint_inputs(Some("good")).is_ok());
    assert_eq!(
        run_rows(&text, Some("good"), PlatformScope::Host, Platform::Linux)["steps"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn host_without_platform_filters_by_host_only() {
    let text = consumer_toml(&row("host_only", "host='windows'"));
    for host in Platform::ALL {
        let report = run_rows(&text, None, PlatformScope::Host, host);
        assert_eq!(
            report["steps"][0]["status"],
            if host == Platform::Windows {
                "pass"
            } else {
                "skipped"
            }
        );
    }
}

#[test]
fn invalid_host_on_unselected_profile_fails_at_config_load() {
    let text = format!(
        "{}\n[tool.sc-lint.lint_profiles]\nselected=['good']",
        consumer_toml(&(row("good", "") + &row("bad", "host='invalid'")))
    );
    assert_eq!(
        load_consumer_toml(&text).1.unwrap_err().details["detail_code"],
        "PROFILE.INVALID_HOST"
    );
}

fn selector_config(fields: &str) -> String {
    format!(
        "[tool.sc-lint]\nminimum_version='0.4.0'\n{}\n[tool.sc-lint.test]\ndefault='unit'\n[tool.sc-lint.test.selectors.unit]\ncommand=['cargo','test']\n{fields}\n",
        row("lint", "")
    )
}

#[test]
fn ac7_env_paths_are_validated_at_load_for_every_row_and_command_selector() {
    let invalid = [
        "env_paths=[]",
        "env_paths='bad'",
        "env_paths={A=3}",
        "env_paths={A=''}",
        r#"env_paths={A="x\u0000y"}"#,
        "env_paths={'A=B'='x'}",
        "env_paths={''='x'}",
        r#"env_paths={"A\u0000B"="x"}"#,
    ];
    for fields in invalid {
        let text = consumer_toml(&(row("good", "") + &row("unselected", fields)));
        let error = load_consumer_toml(&text).1.unwrap_err();
        assert_eq!(
            error.details["detail_code"], "PROFILE.INVALID_STEP_ENV",
            "{fields}"
        );
        assert!(
            error.details["field"]
                .as_str()
                .unwrap()
                .starts_with("env_paths")
        );
        assert_eq!(
            load_consumer_toml(&selector_config(fields))
                .1
                .unwrap_err()
                .code(),
            "CLI.SC_LINT_CONFIG_MALFORMED"
        );
    }
    let duplicate = "env={A='literal'}\nenv_paths={A='relative'}";
    assert_eq!(
        load_consumer_toml(&consumer_toml(&row("bad", duplicate)))
            .1
            .unwrap_err()
            .details["detail_code"],
        "PROFILE.DUPLICATE_STEP_ENV_KEY"
    );
    assert_eq!(
        load_consumer_toml(&selector_config(duplicate))
            .1
            .unwrap_err()
            .code(),
        "CLI.SC_LINT_CONFIG_MALFORMED"
    );
    let group = format!(
        "{}\n[tool.sc-lint.test.selectors.group]\ngroup=['unit']\nenv_paths={{A='relative'}}",
        selector_config("")
    );
    assert_eq!(
        load_consumer_toml(&group).1.unwrap_err().code(),
        "CLI.SC_LINT_CONFIG_MALFORMED"
    );
    assert_eq!(
        load_consumer_toml(&selector_config("host='linux'"))
            .1
            .unwrap_err()
            .code(),
        "CLI.SC_LINT_CONFIG_MALFORMED"
    );
}

#[test]
fn env_paths_uses_config_file_parent_not_step_cwd() {
    let current = std::env::current_dir().unwrap();
    let temp = tempfile::Builder::new()
        .prefix("host-row-paths-")
        .tempdir_in(&current)
        .unwrap();
    let consumer = temp.path().join("consumer");
    fs::create_dir(&consumer).unwrap();
    let relative = temp
        .path()
        .strip_prefix(&current)
        .unwrap()
        .join("consumer/../consumer/sc-lint.toml");
    let fields = "env_paths={PYO3_PYTHON='venv/../venv/Scripts/python.exe'}\nenv={LITERAL='relative/not/a/resolved/path'}\ncwd='crates/pkg'";
    fs::write(
        consumer.join("sc-lint.toml"),
        consumer_toml(&row("path", fields)),
    )
    .unwrap();
    let cli = Cli::parse_from([
        "sc-lint",
        "--config",
        relative.to_str().unwrap(),
        "lint",
        "ci",
        "--consumer",
    ]);
    let context = CommandContext::from_cli(&cli).unwrap();
    let loaded = LoadedConfig::load(&cli, &context).unwrap();
    let input = loaded.consumer_lint_inputs(None).unwrap();
    let expected_root = current.join(&relative).parent().unwrap().to_path_buf();
    assert_eq!(input.root, expected_root);
    let expected = format!(
        "{}{}venv/../venv/Scripts/python.exe",
        expected_root.display(),
        std::path::MAIN_SEPARATOR
    );
    assert_eq!(input.steps[0].env()["PYO3_PYTHON"], expected);
    assert_eq!(
        input.steps[0].env()["LITERAL"],
        "relative/not/a/resolved/path"
    );
    assert_eq!(input.steps[0].cwd(), Some(Path::new("crates/pkg")));
    assert!(expected.contains(".."));
    assert!(!expected.contains("crates/pkg"));
}

#[test]
fn ac8_path_resolution_uses_native_absolute_rules_and_verbatim_relative_values() {
    let temp = TempDir::new().unwrap();
    let root = temp.path().join("symlink-token/../nonexistent");
    for value in [
        "venv/Scripts/python.exe",
        "nested//./../python",
        r"C:\tools\python.exe",
        r"\\server\share\python.exe",
        r"\x",
        "C:x",
        "/tools/python",
    ] {
        let resolved = crate::step_paths::resolve_path(&root, value);
        if Path::new(value).is_absolute() {
            assert_eq!(resolved, value);
        } else {
            assert_eq!(
                resolved,
                format!("{}{}{value}", root.display(), std::path::MAIN_SEPARATOR)
            );
        }
        assert!(
            !root.exists(),
            "resolution must not create or require a directory"
        );
    }
}

#[cfg(unix)]
#[test]
fn ac8_existing_symlink_is_not_resolved() {
    let temp = TempDir::new().unwrap();
    fs::create_dir(temp.path().join("actual")).unwrap();
    std::os::unix::fs::symlink(temp.path().join("actual"), temp.path().join("link")).unwrap();
    let value = crate::step_paths::resolve_path(temp.path(), "link/not-created");
    assert_eq!(value, format!("{}/link/not-created", temp.path().display()));
    assert!(!value.contains("/actual/"));
}

#[test]
fn ac9_report_keys_are_declared_names_only_for_lint_and_selectors() {
    let rows = row("plain", "env={LITERAL='do-not-disclose'}")
        + &row(
            "paths",
            "host='linux'\nenv_paths={Z='secret/not-created',A='other/not-created'}",
        )
        + &row("empty", "env_paths={}");
    let text = consumer_toml(&rows);
    for host in Platform::ALL {
        let report = run_rows(&text, None, PlatformScope::All, host);
        assert!(report["steps"][0].get("env_path_keys").is_none());
        assert_eq!(report["steps"][1]["env_path_keys"], json!(["A", "Z"]));
        assert_eq!(report["steps"][2]["env_path_keys"], json!([]));
        for item in report["steps"].as_array().unwrap() {
            assert!(item.get("env_keys").is_none());
        }
        assert!(!report.to_string().contains("not-created"));
        assert!(!report.to_string().contains("do-not-disclose"));
    }
    let (temp, loaded) = load_consumer_toml(&selector_config(
        "env={LITERAL='do-not-disclose'}\nenv_paths={PATH_VALUE='secret/not-created'}",
    ));
    let loaded = loaded.unwrap();
    let adapter = FakeSystemAdapter::new(true).on_host(Platform::Linux);
    let request = TestRequest::new(None, Vec::new(), true);
    let listed =
        workflow::run_consumer_test_with(&loaded, &request, FailurePolicy::KeepGoing, &adapter)
            .unwrap()
            .data;
    assert_eq!(listed["selectors"][0]["env_keys"], json!(["LITERAL"]));
    assert_eq!(
        listed["selectors"][0]["env_path_keys"],
        json!(["PATH_VALUE"])
    );
    assert!(!listed.to_string().contains("not-created"));
    workflow::run_consumer_test_with(
        &loaded,
        &TestRequest::all(),
        FailurePolicy::KeepGoing,
        &adapter,
    )
    .unwrap();
    let records = adapter.records();
    assert!(records[0].env.iter().any(|(key, value)| key == "PATH_VALUE"
        && value
            == &format!(
                "{}{}secret/not-created",
                temp.path().display(),
                std::path::MAIN_SEPARATOR
            )));
}
