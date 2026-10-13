#![cfg(test)]

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use sc_lint_schema::OutputFormat;
use sc_lint_schema::ReportStatus;
use tempfile::TempDir;

use super::*;
use sc_lint_schema::NodeId;
use sc_lint_schema::OwnerId;

#[test]
#[ignore]
fn emit_installer_catalog() {
    let output = std::env::var("SC_LINT_INSTALLER_CATALOG_OUTPUT")
        .expect("catalog output path must be provided");
    let families = PORTABILITY_CATALOG
        .families
        .iter()
        .map(|(name, rules)| {
            (
                *name,
                rules.iter().map(|rule| rule.as_str()).collect::<Vec<_>>(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    fs::write(
        output,
        serde_json::to_vec_pretty(&serde_json::json!({
            "analyzer": PORTABILITY_CATALOG.analyzer.as_str(),
            "rules": PORTABILITY_CATALOG.rules.iter().map(|rule| rule.as_str()).collect::<Vec<_>>(),
            "families": families,
            "own_keys": PORTABILITY_CATALOG.own_keys,
        }))
        .unwrap(),
    )
    .unwrap();
}

#[test]
fn portability_string_literal_visitor_arms_decode_once() {
    fn method<'a>(file: &'a syn::File, name: &str) -> &'a syn::ImplItemFn {
        file.items
            .iter()
            .filter_map(|item| match item {
                syn::Item::Impl(item_impl) => Some(&item_impl.items),
                _ => None,
            })
            .flatten()
            .find_map(|item| match item {
                syn::ImplItem::Fn(method) if method.sig.ident == name => Some(method),
                _ => None,
            })
            .unwrap_or_else(|| panic!("missing method {name}"))
    }

    fn literal_value_call_count(method: &syn::ImplItemFn) -> usize {
        struct LiteralValueCalls(usize);

        impl<'ast> syn::visit::Visit<'ast> for LiteralValueCalls {
            fn visit_expr_method_call(&mut self, call: &'ast syn::ExprMethodCall) {
                if call.method == "value"
                    && matches!(
                        &*call.receiver,
                        syn::Expr::Path(path) if path.path.is_ident("lit")
                    )
                {
                    self.0 += 1;
                }
                syn::visit::visit_expr_method_call(self, call);
            }
        }

        let mut calls = LiteralValueCalls(0);
        syn::visit::Visit::visit_block(&mut calls, &method.block);
        calls.0
    }

    let source = syn::parse_file(include_str!("portability.rs")).unwrap();

    assert_eq!(
        literal_value_call_count(method(&source, "visit_expr_for_portability")),
        1,
        "the test-code portability visitor should decode each string literal once"
    );
    assert_eq!(
        literal_value_call_count(method(&source, "visit_expr_lit")),
        1,
        "the production portability visitor should decode each string literal once"
    );
}

#[test]
fn findings_report_text_is_stable() {
    let report = FindingsReport {
        tool: "sc-lint-portability",
        version: "0.1.0",
        schema_version: "0.1.0",
        status: ReportStatus::Pass,
        scanned_crates: 2,
        findings: Vec::new(),
    };
    assert_eq!(
        render_findings_report(&report),
        "sc-lint-portability 0.1.0 status=pass scanned_crates=2 findings=0"
    );
}

#[test]
fn findings_report_text_renders_failure_status_and_count() {
    let report = FindingsReport {
        tool: "sc-lint-portability",
        version: "0.1.0",
        schema_version: "0.1.0",
        status: ReportStatus::Fail,
        scanned_crates: 4,
        findings: vec![Finding {
            rule_id: RuleId::Port004,
            kind: "ungated_std_os_unix_import".into(),
            message: "ungated import".to_string(),
            owner_ids: vec![OwnerId::new("crate::example::example")],
            node_ids: vec![NodeId::new("crate::example::example::portability")],
        }],
    };

    assert_eq!(
        render_findings_report(&report),
        "sc-lint-portability 0.1.0 status=fail scanned_crates=4 findings=1"
    );
}

#[test]
fn analyze_workspace_counts_crate_targets() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_source("example", "lib.rs", "pub struct Example;");

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
    })
    .unwrap();

    assert_eq!(report.scanned_crates, 1);
    assert_eq!(report.status, ReportStatus::Pass);
    assert!(report.findings.is_empty());
}

#[test]
fn analyzer_table_can_disable_portability() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_lint_config("[tool.sc-lint.analyzers.portability]\nenabled = false\n");
    fixture.write_source(
        "example",
        "lib.rs",
        "pub fn path() { let _ = \"/tmp/file\"; }",
    );

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
    })
    .unwrap();

    assert_eq!(report.status, ReportStatus::Pass);
    assert!(report.findings.is_empty());
    assert_eq!(report.scanned_crates, 0);
}

#[test]
fn rule_filter_selects_portability_rule() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_source(
        "example",
        "lib.rs",
        "pub fn path() { let _ = \"/home/user/file\"; let _ = \"/bin/sh\"; }",
    );

    let report = analyze_workspace_configured(
        &AnalyzeOptions {
            root: fixture.root().to_path_buf(),
            format: OutputFormat::Json,
        },
        None,
        Some("PORT-006"),
    )
    .unwrap();

    assert!(!report.findings.is_empty());
    assert!(
        report
            .findings
            .iter()
            .all(|finding| finding.rule_id == RuleId::Port006)
    );

    let unfiltered = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
    })
    .unwrap();
    assert!(
        unfiltered
            .findings
            .iter()
            .any(|finding| finding.rule_id != RuleId::Port006)
    );
}

#[test]
fn configured_unix_prefix_flags_production_path() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_lint_config(
        "[tool.sc-lint.analyzers.portability]\nunix_path_prefixes = [\"/opt/\"]\n",
    );
    fixture.write_source(
        "example",
        "lib.rs",
        "pub fn path() { let _ = \"/opt/tool/data\"; }\n#[cfg(test)] mod tests { fn path() { let _ = \"/opt/test/data\"; } }",
    );

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
    })
    .unwrap();

    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.rule_id == RuleId::Port006)
    );
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.rule_id == RuleId::Port001)
    );
}

#[test]
fn builtin_prefixes_still_apply_when_configured_list_is_set() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_lint_config(
        "[tool.sc-lint.analyzers.portability]\nunix_path_prefixes = [\"/opt/\"]\n",
    );
    fixture.write_source(
        "example",
        "lib.rs",
        "pub fn production_path() { let _ = \"/var/lib/cache\"; }\n#[cfg(test)] mod tests { fn path() { let _ = \"/private/tmp/cache\"; } }",
    );

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
    })
    .unwrap();

    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.rule_id == RuleId::Port001)
    );
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.rule_id == RuleId::Port006)
    );
}

#[test]
fn unknown_table_key_is_config_error() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_lint_config("[tool.sc-lint.analyzers.portability]\nunknown = true\n");

    let Err(PortabilityError::AnalyzerConfig(error)) = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
    }) else {
        panic!("expected analyzer config error");
    };

    assert_eq!(error.code(), "ANALYZER_CONFIG.UNKNOWN_KEY");
    assert!(error.to_string().contains("unknown"));
}

#[test]
fn wrong_prefix_type_is_config_error() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_lint_config(
        "[tool.sc-lint.analyzers.portability]\nunix_path_prefixes = \"/opt/\"\n",
    );

    let Err(PortabilityError::AnalyzerConfig(error)) = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
    }) else {
        panic!("expected analyzer config error");
    };

    assert_eq!(error.code(), "ANALYZER_CONFIG.WRONG_TYPE");
    assert!(error.to_string().contains("unix_path_prefixes"));
}

#[test]
fn unknown_rule_filter_is_a_selector_error() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");

    let Err(PortabilityError::AnalyzerConfig(error)) = analyze_workspace_configured(
        &AnalyzeOptions {
            root: fixture.root().to_path_buf(),
            format: OutputFormat::Json,
        },
        None,
        Some("PORT-999"),
    ) else {
        panic!("expected analyzer config error");
    };

    assert_eq!(error.code(), "ANALYZER_CONFIG.SELECTOR");
}

#[test]
fn exclude_paths_drops_findings_from_matching_files() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_lint_config(
        "[tool.sc-lint.analyzers.portability]\nexclude_paths = [\"crates/example/src/**\"]\n",
    );
    fixture.write_source(
        "example",
        "lib.rs",
        "pub fn path() { let _ = \"/home/user/file\"; }",
    );

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
    })
    .unwrap();

    assert_eq!(report.status, ReportStatus::Pass);
    assert!(report.findings.is_empty());
}

#[test]
fn issue385_rule_path_exclusion_retains_other_rule() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_source(
        "example",
        "lib.rs",
        include_str!("../tests/fixtures/issue385.rs"),
    );

    let baseline = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
    })
    .unwrap();
    assert!(
        baseline
            .findings
            .iter()
            .any(|finding| finding.rule_id == RuleId::Port006)
    );
    assert!(
        baseline
            .findings
            .iter()
            .any(|finding| finding.rule_id == RuleId::Port007)
    );

    fixture.write_lint_config(
        r#"
        [tool.sc-lint.analyzers.portability.exclude_paths_by_rule]
        "PORT-007" = ["crates/example/src/lib.rs"]
        "#,
    );
    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
    })
    .unwrap();

    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.rule_id == RuleId::Port006)
    );
    assert!(
        !report
            .findings
            .iter()
            .any(|finding| finding.rule_id == RuleId::Port007)
    );
}

#[test]
fn exclude_rules_drops_matching_findings() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_lint_config(
        "[tool.sc-lint.analyzers.portability]\nexclude_rules = [\"PORT-006\"]\n",
    );
    fixture.write_source(
        "example",
        "lib.rs",
        "pub fn path() { let _ = \"/home/user/file\"; }",
    );

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
    })
    .unwrap();

    assert_eq!(report.status, ReportStatus::Pass);
    assert!(report.findings.is_empty());
}

#[test]
fn fail_rules_makes_findings_report_only() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_lint_config("[tool.sc-lint.analyzers.portability]\nfail_rules = []\n");
    fixture.write_source(
        "example",
        "lib.rs",
        "pub fn path() { let _ = \"/home/user/file\"; }",
    );

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
    })
    .unwrap();

    assert_eq!(report.status, ReportStatus::Pass);
    assert_eq!(report.findings.len(), 1);
    assert_eq!(report.findings[0].rule_id, RuleId::Port006);
}

#[test]
fn explicit_config_path_is_read() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_source(
        "example",
        "lib.rs",
        "pub fn path() { let _ = \"/home/user/file\"; }",
    );
    fixture.write(
        "config/lint.toml",
        "[tool.sc-lint.analyzers.portability]\nexclude_rules = [\"PORT-006\"]\n",
    );
    let config = fixture.root().join("config/lint.toml");

    let report = analyze_workspace_configured(
        &AnalyzeOptions {
            root: fixture.root().to_path_buf(),
            format: OutputFormat::Json,
        },
        Some(&config),
        None,
    )
    .unwrap();

    assert!(report.findings.is_empty());
}

fn analyze_with_unix_path_prefix(prefix: &str) -> Result<FindingsReport, PortabilityError> {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_lint_config(&format!(
        "[tool.sc-lint.analyzers.portability]\nunix_path_prefixes = [\"{prefix}\"]\n"
    ));
    fixture.write_source(
        "example",
        "lib.rs",
        "pub fn path() { let _ = \"/opt/tool/data\"; }",
    );

    analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
    })
}

#[test]
fn empty_unix_path_prefix_is_config_error() {
    let Err(PortabilityError::AnalyzerConfig(error)) = analyze_with_unix_path_prefix("") else {
        panic!("expected an analyzer config error");
    };

    assert_eq!(error.code(), "ANALYZER_CONFIG.WRONG_TYPE");
    assert_eq!(
        error.to_string(),
        "portability: `unix_path_prefixes` in [tool.sc-lint.analyzers.portability] must be an array of non-empty strings starting with `/`"
    );
}

#[test]
fn relative_unix_path_prefix_is_config_error() {
    let Err(PortabilityError::AnalyzerConfig(error)) = analyze_with_unix_path_prefix("opt") else {
        panic!("expected an analyzer config error");
    };

    assert_eq!(error.code(), "ANALYZER_CONFIG.WRONG_TYPE");
    assert_eq!(
        error.to_string(),
        "portability: `unix_path_prefixes` in [tool.sc-lint.analyzers.portability] must be an array of non-empty strings starting with `/`"
    );
}

fn prefix_findings(config: Option<&str>, source: &str) -> Vec<(RuleId, String)> {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    if let Some(config) = config {
        fixture.write_lint_config(config);
    }
    fixture.write_source("example", "lib.rs", source);

    analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
    })
    .unwrap()
    .findings
    .into_iter()
    .filter(|finding| matches!(finding.rule_id, RuleId::Port001 | RuleId::Port006))
    .map(|finding| {
        let literal = finding
            .message
            .split('`')
            .nth(1)
            .unwrap_or_default()
            .to_string();
        (finding.rule_id, literal)
    })
    .collect()
}

const SHARED_BUILTIN_SOURCE: &str = "pub fn production() { let _ = \"/private/tmp/prod\"; let _ = \"/var/tmp/prod\"; }\n#[cfg(test)] mod tests { fn fixture() { let _ = \"/private/tmp/test\"; let _ = \"/var/tmp/test\"; } }";

fn assert_shared_builtins_flagged(findings: &[(RuleId, String)]) {
    for (rule_id, literal) in [
        (RuleId::Port006, concat!("/private", "/tmp/prod")),
        (RuleId::Port006, concat!("/var", "/tmp/prod")),
        (RuleId::Port001, concat!("/private", "/tmp/test")),
        (RuleId::Port001, concat!("/var", "/tmp/test")),
    ] {
        assert!(
            findings.contains(&(rule_id, literal.to_string())),
            "{rule_id:?} {literal} missing from {findings:?}"
        );
    }
}

#[test]
fn defaults_only_shared_builtin_prefixes_flag_both_rules() {
    assert_shared_builtins_flagged(&prefix_findings(None, SHARED_BUILTIN_SOURCE));
}

#[test]
fn configured_prefixes_do_not_suppress_shared_builtin_detection() {
    let configured = prefix_findings(
        Some("[tool.sc-lint.analyzers.portability]\nunix_path_prefixes = [\"/opt/\"]\n"),
        SHARED_BUILTIN_SOURCE,
    );

    assert_shared_builtins_flagged(&configured);
    assert_eq!(configured, prefix_findings(None, SHARED_BUILTIN_SOURCE));
}

#[test]
fn configured_prefixes_extend_both_rules() {
    let findings = prefix_findings(
        Some("[tool.sc-lint.analyzers.portability]\nunix_path_prefixes = [\"/opt/\"]\n"),
        "pub fn production() { let _ = \"/opt/prod\"; }\n#[cfg(test)] mod tests { fn fixture() { let _ = \"/opt/test\"; } }",
    );

    assert!(
        findings.contains(&(RuleId::Port006, "/opt/prod".to_string())),
        "{findings:?}"
    );
    assert!(
        findings.contains(&(RuleId::Port001, "/opt/test".to_string())),
        "{findings:?}"
    );
}

#[test]
fn analyzer_config_home_env_overrides_legacy_table() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_lint_config("[portability]\nconfig_home_env = \"LEGACY_HOME\"\n\n[tool.sc-lint.analyzers.portability]\nconfig_home_env = \"ANALYZER_HOME\"\n");
    fixture.write_source(
        "example",
        "lib.rs",
        "#[cfg(test)] mod tests { fn config() { let _ = dirs::home_dir(); } }",
    );

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
    })
    .unwrap();

    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.message.contains("ANALYZER_HOME"))
    );
}

#[test]
fn malformed_legacy_config_home_env_is_analyzer_config_error() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_lint_config("[portability]\nconfig_home_env = []\n");
    fixture.write_source("example", "lib.rs", "pub struct Example;");

    let error = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
    })
    .unwrap_err();
    assert!(matches!(error, PortabilityError::AnalyzerConfig(_)));
}

#[test]
fn flags_hardcoded_tmp_path_in_test_scope() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_lint_config(
        r#"
        [portability]
        config_home_env = "ATM_CONFIG_HOME"
        "#,
    );
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
            #[cfg(test)]
            mod tests {
                use std::path::PathBuf;

                #[test]
                fn writes_artifact() {
                    // Intentional PORT-001 fixture: hardcoded /tmp/ path should be flagged.
                    let _ = PathBuf::from("/tmp/test-artifact");
                }
            }
        "#,
    );

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
    })
    .unwrap();

    assert_eq!(report.status, ReportStatus::Fail);
    assert_eq!(report.findings.len(), 1);
    assert_eq!(report.findings[0].rule_id, RuleId::Port001);
}

#[test]
fn passes_temp_dir_usage_in_test_scope() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_lint_config(
        r#"
        [portability]
        config_home_env = "ATM_CONFIG_HOME"
        "#,
    );
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
            #[cfg(test)]
            mod tests {
                #[test]
                fn writes_artifact() {
                    let _ = std::env::temp_dir().join("test-artifact");
                }
            }
        "#,
    );

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
    })
    .unwrap();

    assert_eq!(report.status, ReportStatus::Pass);
    assert!(report.findings.is_empty());
}

#[test]
fn passes_cfg_unix_gated_unix_path_in_production_code() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_lint_config(
        r#"
        [portability]
        config_home_env = "ATM_CONFIG_HOME"
        "#,
    );
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
            #[cfg(unix)]
            pub fn socket_path() -> std::path::PathBuf {
                std::path::PathBuf::from("/tmp/runtime-socket")
            }

            #[cfg(windows)]
            pub fn socket_path() -> std::path::PathBuf {
                std::path::PathBuf::from(r"\\.\pipe\sc-lint")
            }
        "#,
    );

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
    })
    .unwrap();

    assert_eq!(report.status, ReportStatus::Pass);
    assert!(report.findings.is_empty());
}

#[test]
fn flags_hardcoded_unix_path_in_production_code() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_lint_config(
        r#"
        [portability]
        config_home_env = "ATM_CONFIG_HOME"
        "#,
    );
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
            pub fn socket_dir() -> std::path::PathBuf {
                std::path::PathBuf::from("/var/run/sc-lint")
            }
        "#,
    );

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
    })
    .unwrap();

    assert_eq!(report.status, ReportStatus::Fail);
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.rule_id == RuleId::Port006)
    );
}

#[test]
fn passes_dirs_cache_dir_in_production_code() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_lint_config(
        r#"
        [portability]
        config_home_env = "ATM_CONFIG_HOME"
        "#,
    );
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
            pub fn cache_dir() -> std::path::PathBuf {
                dirs::cache_dir().expect("cache directory")
            }
        "#,
    );

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
    })
    .unwrap();

    assert_eq!(report.status, ReportStatus::Pass);
    assert!(report.findings.is_empty());
}

#[test]
fn flags_hardcoded_windows_path_in_production_code() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_lint_config(
        r#"
        [portability]
        config_home_env = "ATM_CONFIG_HOME"
        "#,
    );
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
            pub fn cache_file() -> std::path::PathBuf {
                let _ = r"\\server\share\sc-lint\cache.json";
                std::path::PathBuf::from(r"C:\ProgramData\sc-lint\cache.json")
            }
        "#,
    );

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
    })
    .unwrap();

    assert_eq!(report.status, ReportStatus::Fail);
    assert_eq!(
        report
            .findings
            .iter()
            .filter(|finding| finding.rule_id == RuleId::Port007)
            .count(),
        2
    );
}

#[test]
fn flags_home_env_lookup_in_production_code() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_lint_config(
        r#"
        [portability]
        config_home_env = "ATM_CONFIG_HOME"
        "#,
    );
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
            pub fn config_root() -> std::path::PathBuf {
                let home = std::env::var("HOME").expect("HOME");
                std::path::PathBuf::from(home).join(".config").join("sc-lint")
            }
        "#,
    );

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
    })
    .unwrap();

    assert_eq!(report.status, ReportStatus::Fail);
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.rule_id == RuleId::Port008)
    );
}

#[test]
fn flags_xdg_config_home_lookup_in_production_code() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_lint_config(
        r#"
        [portability]
        config_home_env = "ATM_CONFIG_HOME"
        "#,
    );
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
            pub fn config_root() -> std::path::PathBuf {
                let root = std::env::var_os("XDG_CONFIG_HOME").expect("XDG_CONFIG_HOME");
                std::path::PathBuf::from(root).join("sc-lint")
            }
        "#,
    );

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
    })
    .unwrap();

    assert_eq!(report.status, ReportStatus::Fail);
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.rule_id == RuleId::Port008)
    );
}

#[test]
fn flags_user_env_lookup_in_production_code() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_lint_config(
        r#"
        [portability]
        config_home_env = "ATM_CONFIG_HOME"
        "#,
    );
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
            pub fn current_user() -> String {
                std::env::var("USER").expect("USER")
            }
        "#,
    );

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
    })
    .unwrap();

    assert_eq!(report.status, ReportStatus::Fail);
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.rule_id == RuleId::Port008)
    );
}

#[test]
fn passes_dirs_data_dir_in_production_code() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_lint_config(
        r#"
        [portability]
        config_home_env = "ATM_CONFIG_HOME"
        "#,
    );
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
            pub fn data_root() -> std::path::PathBuf {
                dirs::data_dir().expect("data directory")
            }
        "#,
    );

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
    })
    .unwrap();

    assert_eq!(report.status, ReportStatus::Pass);
    assert!(report.findings.is_empty());
}

#[test]
fn passes_cfg_unix_gated_home_lookup_in_production_code() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_lint_config(
        r#"
        [portability]
        config_home_env = "ATM_CONFIG_HOME"
        "#,
    );
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
            #[cfg(unix)]
            pub fn config_root() -> std::path::PathBuf {
                let home = std::env::var("HOME").expect("HOME");
                std::path::PathBuf::from(home).join(".config").join("sc-lint")
            }

            #[cfg(windows)]
            pub fn config_root() -> std::path::PathBuf {
                dirs::config_dir().expect("config directory").join("sc-lint")
            }
        "#,
    );

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
    })
    .unwrap();

    assert_eq!(report.status, ReportStatus::Pass);
    assert!(report.findings.is_empty());
}

#[test]
fn flags_command_new_sh_in_production_code() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_lint_config(
        r#"
        [portability]
        config_home_env = "ATM_CONFIG_HOME"
        "#,
    );
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
            pub fn run_hook() -> std::io::Result<std::process::ExitStatus> {
                std::process::Command::new("sh")
                    .arg("-c")
                    .arg("git status --short")
                    .status()
            }
        "#,
    );

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
    })
    .unwrap();

    assert_eq!(report.status, ReportStatus::Fail);
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.rule_id == RuleId::Port009)
    );
}

#[test]
fn flags_command_new_bash_in_production_code() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_lint_config(
        r#"
        [portability]
        config_home_env = "ATM_CONFIG_HOME"
        "#,
    );
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
            pub fn run_hook() -> std::io::Result<std::process::ExitStatus> {
                std::process::Command::new("bash")
                    .arg("-lc")
                    .arg("true")
                    .status()
            }
        "#,
    );

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
    })
    .unwrap();

    assert_eq!(report.status, ReportStatus::Fail);
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.rule_id == RuleId::Port009)
    );
}

#[test]
fn flags_bin_bash_path_in_production_code() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_lint_config(
        r#"
        [portability]
        config_home_env = "ATM_CONFIG_HOME"
        "#,
    );
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
            pub fn run_hook() -> std::io::Result<std::process::ExitStatus> {
                std::process::Command::new("/bin/bash")
                    .arg("-lc")
                    .arg("true")
                    .status()
            }
        "#,
    );

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
    })
    .unwrap();

    assert_eq!(report.status, ReportStatus::Fail);
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.rule_id == RuleId::Port009)
    );
}

#[test]
fn flags_bin_sh_path_literal_in_production_code() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_lint_config(
        r#"
        [portability]
        config_home_env = "ATM_CONFIG_HOME"
        "#,
    );
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
            pub const SHELL_PATH: &str = "/bin/sh";
        "#,
    );

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
    })
    .unwrap();

    assert_eq!(report.status, ReportStatus::Fail);
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.rule_id == RuleId::Port009)
    );
}

#[test]
fn passes_std_process_command_with_binary_name_in_production_code() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_lint_config(
        r#"
        [portability]
        config_home_env = "ATM_CONFIG_HOME"
        "#,
    );
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
            pub fn run_git() -> std::io::Result<std::process::ExitStatus> {
                std::process::Command::new("git")
                    .arg("status")
                    .arg("--short")
                    .status()
            }
        "#,
    );

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
    })
    .unwrap();

    assert_eq!(report.status, ReportStatus::Pass);
    assert!(report.findings.is_empty());
}

#[test]
fn passes_cfg_unix_gated_shell_invocation_in_production_code() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_lint_config(
        r#"
        [portability]
        config_home_env = "ATM_CONFIG_HOME"
        "#,
    );
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
            #[cfg(unix)]
            pub fn run_unix_hook() -> std::io::Result<std::process::ExitStatus> {
                std::process::Command::new("sh")
                    .arg("-c")
                    .arg("true")
                    .status()
            }

            #[cfg(windows)]
            pub fn run_unix_hook() -> std::io::Result<std::process::ExitStatus> {
                std::process::Command::new("cmd")
                    .arg("/C")
                    .arg("ver")
                    .status()
            }
        "#,
    );

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
    })
    .unwrap();

    assert_eq!(report.status, ReportStatus::Pass);
    assert!(report.findings.is_empty());
}

#[test]
fn flags_cfg_unix_item_without_windows_companion() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_lint_config(
        r#"
        [portability]
        config_home_env = "ATM_CONFIG_HOME"
        "#,
    );
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
            #[cfg(unix)]
            pub fn runtime_socket_name() -> &'static str {
                "/var/run/sc-lint.sock"
            }
        "#,
    );

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
    })
    .unwrap();

    assert_eq!(report.status, ReportStatus::Fail);
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.rule_id == RuleId::Port010)
    );
}

#[test]
fn flags_cfg_unix_item_without_portable_fallback() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_lint_config(
        r#"
        [portability]
        config_home_env = "ATM_CONFIG_HOME"
        "#,
    );
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
            #[cfg(unix)]
            pub mod runtime_socket_name {
                pub const VALUE: &str = "/var/run/sc-lint.sock";
            }
        "#,
    );

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
    })
    .unwrap();

    assert_eq!(report.status, ReportStatus::Fail);
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.rule_id == RuleId::Port010)
    );
}

#[test]
fn does_not_pass_cfg_unix_item_with_cfg_test_only_sibling() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_lint_config(
        r#"
        [portability]
        config_home_env = "ATM_CONFIG_HOME"
        "#,
    );
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
            #[cfg(unix)]
            pub fn runtime_socket_name() -> &'static str {
                "/var/run/sc-lint.sock"
            }

            #[cfg(test)]
            pub fn runtime_socket_name() -> &'static str {
                "test-socket"
            }
        "#,
    );

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
    })
    .unwrap();

    assert_eq!(report.status, ReportStatus::Fail);
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.rule_id == RuleId::Port010)
    );
}

#[test]
fn passes_cfg_unix_item_with_windows_companion() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_lint_config(
        r#"
        [portability]
        config_home_env = "ATM_CONFIG_HOME"
        "#,
    );
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
            #[cfg(unix)]
            pub fn runtime_socket_name() -> &'static str {
                "/var/run/sc-lint.sock"
            }

            #[cfg(windows)]
            pub fn runtime_socket_name() -> &'static str {
                r"\\.\pipe\sc-lint"
            }
        "#,
    );

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
    })
    .unwrap();

    assert_eq!(report.status, ReportStatus::Pass);
    assert!(report.findings.is_empty());
}

#[test]
fn passes_cfg_unix_item_with_portable_fallback() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_lint_config(
        r#"
        [portability]
        config_home_env = "ATM_CONFIG_HOME"
        "#,
    );
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
            #[cfg(unix)]
            pub fn runtime_socket_name() -> &'static str {
                "/var/run/sc-lint.sock"
            }

            pub fn runtime_socket_name() -> &'static str {
                "sc-lint.sock"
            }
        "#,
    );

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
    })
    .unwrap();

    assert_eq!(report.status, ReportStatus::Pass);
    assert!(report.findings.is_empty());
}

#[test]
fn passes_cfg_unix_impl_method_with_windows_companion() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_lint_config(
        r#"
        [portability]
        config_home_env = "ATM_CONFIG_HOME"
        "#,
    );
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
            pub struct RuntimeSocket;

            impl RuntimeSocket {
                #[cfg(unix)]
                pub fn socket_name() -> &'static str {
                    "/var/run/sc-lint.sock"
                }

                #[cfg(windows)]
                pub fn socket_name() -> &'static str {
                    r"\\.\pipe\sc-lint"
                }
            }
        "#,
    );

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
    })
    .unwrap();

    assert_eq!(report.status, ReportStatus::Pass);
    assert!(report.findings.is_empty());
}

#[test]
fn flags_cfg_unix_impl_method_without_windows_companion() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_lint_config(
        r#"
        [portability]
        config_home_env = "ATM_CONFIG_HOME"
        "#,
    );
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
            pub struct RuntimeSocket;

            impl RuntimeSocket {
                #[cfg(unix)]
                pub fn name() -> &'static str {
                    "/var/run/sc-lint.sock"
                }
            }
        "#,
    );

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
    })
    .unwrap();

    assert_eq!(report.status, ReportStatus::Fail);
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.rule_id == RuleId::Port010)
    );
}

#[test]
fn flags_dirs_home_dir_without_override_check() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_lint_config(
        r#"
        [portability]
        config_home_env = "ATM_CONFIG_HOME"
        "#,
    );
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
            #[cfg(test)]
            mod tests {
                fn config_root() -> std::path::PathBuf {
                    dirs::home_dir().expect("home")
                }
            }
        "#,
    );

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
    })
    .unwrap();

    assert_eq!(report.status, ReportStatus::Fail);
    assert_eq!(report.findings.len(), 1);
    assert_eq!(report.findings[0].rule_id, RuleId::Port002);
    assert!(report.findings[0].message.contains("ATM_CONFIG_HOME"));
}

#[test]
fn passes_dirs_home_dir_after_override_check() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_lint_config(
        r#"
        [portability]
        config_home_env = "ATM_CONFIG_HOME"
        "#,
    );
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
            #[cfg(test)]
            mod tests {
                fn get_os_home_dir() -> std::path::PathBuf {
                    if let Ok(home) = std::env::var("ATM_CONFIG_HOME") {
                        return std::path::PathBuf::from(home);
                    }
                    dirs::home_dir().expect("home")
                }
            }
        "#,
    );

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
    })
    .unwrap();

    assert_eq!(report.status, ReportStatus::Pass);
    assert!(report.findings.is_empty());
}

#[test]
fn flags_set_var_in_test_scope() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_lint_config(
        r#"
        [portability]
        config_home_env = "ATM_CONFIG_HOME"
        "#,
    );
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
            #[cfg(test)]
            mod tests {
                #[test]
                fn mutates_env() {
                    unsafe { std::env::set_var("HOME", "test-home") };
                }
            }
        "#,
    );

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
    })
    .unwrap();

    assert_eq!(report.status, ReportStatus::Fail);
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.rule_id == RuleId::Port003)
    );
}

#[test]
fn flags_ungated_std_os_unix_import_in_production_code() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_lint_config(
        r#"
        [portability]
        config_home_env = "ATM_CONFIG_HOME"
        "#,
    );
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
            pub fn os_string(bytes: Vec<u8>) -> std::ffi::OsString {
                use std::os::unix::ffi::OsStringExt;
                std::ffi::OsString::from_vec(bytes)
            }
        "#,
    );

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
    })
    .unwrap();

    assert_eq!(report.status, ReportStatus::Fail);
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.rule_id == RuleId::Port004)
    );
}

#[test]
fn passes_cfg_unix_gated_std_os_unix_import_in_production_code() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_lint_config(
        r#"
        [portability]
        config_home_env = "ATM_CONFIG_HOME"
        "#,
    );
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
            #[cfg(unix)]
            pub fn os_string(bytes: Vec<u8>) -> std::ffi::OsString {
                use std::os::unix::ffi::OsStringExt;
                std::ffi::OsString::from_vec(bytes)
            }

            #[cfg(windows)]
            pub fn os_string(bytes: Vec<u8>) -> std::ffi::OsString {
                std::ffi::OsString::from(String::from_utf8_lossy(&bytes).into_owned())
            }
        "#,
    );

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
    })
    .unwrap();

    assert_eq!(report.status, ReportStatus::Pass);
    assert!(report.findings.is_empty());
}

#[test]
fn flags_cfg_attr_not_unix_allow_dead_code_in_production_code() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_lint_config(
        r#"
        [portability]
        config_home_env = "ATM_CONFIG_HOME"
        "#,
    );
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
            #[cfg_attr(not(unix), allow(dead_code))]
            pub fn unix_socket_path() -> &'static str {
                // Intentional PORT-001 fixture: hardcoded /tmp/ path should be flagged.
                "/tmp/runtime.sock"
            }
        "#,
    );

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
    })
    .unwrap();

    assert_eq!(report.status, ReportStatus::Fail);
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.rule_id == RuleId::Port005)
    );
}

#[test]
fn catalog_lists_every_rule_id() {
    use RuleId::*;

    let every = [
        Port001, Port002, Port003, Port004, Port005, Port006, Port007, Port008, Port009, Port010,
    ];
    for id in every {
        match id {
            Port001 | Port002 | Port003 | Port004 | Port005 | Port006 | Port007 | Port008
            | Port009 | Port010 => {}
        }
    }
    assert_eq!(
        ALL_PORT_IDS
            .iter()
            .map(|rule| rule.as_str())
            .collect::<Vec<_>>(),
        every.map(RuleId::as_str).to_vec()
    );
}

struct WorkspaceFixture {
    tempdir: TempDir,
}

impl WorkspaceFixture {
    fn new() -> Self {
        Self {
            tempdir: TempDir::new().unwrap(),
        }
    }

    fn root(&self) -> &Path {
        self.tempdir.path()
    }

    fn write_workspace_root(&self) {
        self.write(
            "Cargo.toml",
            r#"
                    [workspace]
                    members = ["crates/example"]
                    resolver = "2"
                "#,
        );
    }

    fn write_package_manifest(&self, package_name: &str) {
        self.write(
            &format!("crates/{package_name}/Cargo.toml"),
            &format!(
                r#"
                        [package]
                        name = "{package_name}"
                        version = "0.1.0"
                        edition = "2024"
                    "#
            ),
        );
    }

    fn write_source(&self, package_name: &str, relative_path: &str, contents: &str) {
        self.write(
            &format!("crates/{package_name}/src/{relative_path}"),
            contents,
        );
    }

    fn write_lint_config(&self, contents: &str) {
        self.write("sc-lint.toml", contents);
    }

    fn write(&self, relative_path: &str, contents: &str) {
        let path = self.root().join(relative_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, trim_indentation(contents)).unwrap();
    }
}

fn trim_indentation(input: &str) -> String {
    let lines: Vec<_> = input.lines().collect();
    let first_content = lines
        .iter()
        .find(|line| !line.trim().is_empty())
        .map(|line| line.chars().take_while(|ch| ch.is_whitespace()).count())
        .unwrap_or(0);

    let mut output = String::new();
    for line in lines {
        let trimmed = if line.len() >= first_content {
            &line[first_content..]
        } else {
            line.trim_end()
        };
        output.push_str(trimmed);
        output.push('\n');
    }
    output
}
