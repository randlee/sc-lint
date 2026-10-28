#![cfg(test)]

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use sc_lint_schema::OutputFormat;
use sc_lint_schema::ReportStatus;
use tempfile::TempDir;

#[test]
fn outside_source_path_reports_repository_relative_path_error() {
    let root = TempDir::new().unwrap();
    let outside_source = root.path().join("..\u{2f}outside.rs");

    let error = super::portability::repo_relative_source_path(root.path(), &outside_source)
        .expect_err("outside paths must produce a diagnostic");
    assert!(error.to_string().contains("outside.rs"));
    assert!(error.to_string().contains("repository-relative"));
}

use super::*;
use sc_lint_schema::NodeId;
use sc_lint_schema::OwnerId;

fn installer_catalog() -> serde_json::Value {
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
    serde_json::json!({
        "analyzer": PORTABILITY_CATALOG.analyzer.as_str(),
        "rules": PORTABILITY_CATALOG.rules.iter().map(|rule| rule.as_str()).collect::<Vec<_>>(),
        "families": families,
        "own_keys": PORTABILITY_CATALOG.own_keys,
    })
}

#[test]
#[ignore]
fn emit_installer_catalog() {
    let output = std::env::var("SC_LINT_INSTALLER_CATALOG_OUTPUT")
        .expect("catalog output path must be provided");
    fs::write(
        output,
        serde_json::to_vec_pretty(&installer_catalog()).unwrap(),
    )
    .unwrap();
}

#[test]
fn installer_catalog_matches_rust() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../packages/sc-lint-adoption/analyzer-catalog/portability.json");
    let actual: serde_json::Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();

    assert_eq!(actual, installer_catalog());
}

#[test]
fn rule_ids_match_frozen_contract_baseline() {
    let expected: serde_json::Value =
        serde_json::from_str(include_str!("../tests/fixtures/contracts/rule-ids.json"))
            .expect("rule-ID baseline is valid JSON");
    let actual = serde_json::json!(
        PORTABILITY_CATALOG
            .rules
            .iter()
            .map(|rule| rule.as_str())
            .collect::<Vec<_>>()
    );

    assert_eq!(
        actual, expected,
        "update the frozen baseline explicitly for additions"
    );
}

#[test]
fn machine_report_matches_frozen_json_contract() {
    let report = FindingsReport {
        tool: "sc-lint-portability",
        version: "0.1.0",
        schema_version: "0.1.0",
        status: ReportStatus::Fail,
        scanned_crates: 1,
        findings: vec![Finding {
            rule_id: RuleId::Port001,
            kind: "contract_fixture".into(),
            message: "machine contract fixture".to_string(),
            owner_ids: vec![OwnerId::new("crate::example::example")],
            node_ids: vec![NodeId::new("crate::example::example::item")],
        }],
    };
    let expected: serde_json::Value = serde_json::from_str(include_str!(
        "../tests/fixtures/contracts/machine-report.json"
    ))
    .expect("machine-report baseline is valid JSON");

    assert_eq!(
        expected["schema_version"].as_str(),
        Some(SC_LINT_SCHEMA_VERSION),
        "golden schema_version must match analyzer reports"
    );
    assert_eq!(serde_json::to_value(report).unwrap(), expected);
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
fn loads_workspace_metadata_once_per_analyzer_run() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_source(
        "example",
        "lib.rs",
        "#[cfg(unix)] mod unix_socket;\n#[cfg(windows)] mod named_pipe;\npub struct Example;",
    );
    fixture.write_source("example", "unix_socket.rs", "fn connect() {}");
    fixture.write_source("example", "named_pipe.rs", "fn connect() {}");
    fixture.write_lint_config(
        "[tool.sc-lint.analyzers.portability]\n[[tool.sc-lint.analyzers.portability.platform_adapters]]\nunix_module = \"crates/example/src/unix_socket.rs\"\nwindows_module = \"crates/example/src/named_pipe.rs\"\nreason = \"Private transport adapters\"\n",
    );
    let options = AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
    };
    let metadata_loads = std::cell::Cell::new(0);

    let report = analyze_workspace_configured_with_metadata_loader(&options, None, None, |root| {
        metadata_loads.set(metadata_loads.get() + 1);
        sc_lint_analyzer_support::load_workspace_metadata(root)
    })
    .expect("portability analysis succeeds");

    assert_eq!(metadata_loads.get(), 1);
    assert_eq!(report.scanned_crates, 1);
}

#[test]
fn metadata_once_preserves_text_and_json_output() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_source(
        "example",
        "lib.rs",
        "use std::os::unix::net::UnixStream;\nuse std::sync::{Condvar, Mutex};\n\npub fn unix_stream() { let _ = UnixStream::pair(); }\npub fn bad_wait(c: &Condvar, m: &Mutex<bool>) { let g = m.lock().unwrap(); let _ = c.wait(g).unwrap(); }\n",
    );
    let options = AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
    };
    let metadata_loads = std::cell::Cell::new(0);

    let report = analyze_workspace_configured_with_metadata_loader(&options, None, None, |root| {
        metadata_loads.set(metadata_loads.get() + 1);
        sc_lint_analyzer_support::load_workspace_metadata(root)
    })
    .expect("portability analysis succeeds");
    assert_eq!(metadata_loads.get(), 1);

    // Windows cargo metadata can retain its 8.3 path alias while canonicalize
    // returns the long path, so preserve the temp directory's spelling there.
    #[cfg(windows)]
    let root = fixture.root().to_path_buf();
    #[cfg(not(windows))]
    let root = fs::canonicalize(fixture.root()).unwrap();
    let mut text = normalize_fixture_root(render_findings_report(&report), &root);
    text.push('\n');
    let mut json = normalize_fixture_root(serde_json::to_string_pretty(&report).unwrap(), &root);
    json.push('\n');

    assert_eq!(
        text.as_bytes(),
        include_bytes!("../tests/fixtures/metadata-once/text.txt"),
        "metadata reuse must preserve the pre-optimization text output byte-for-byte"
    );
    assert_eq!(
        json.as_bytes(),
        include_bytes!("../tests/fixtures/metadata-once/report.json"),
        "metadata reuse must preserve the pre-optimization JSON output byte-for-byte"
    );
}

fn normalize_fixture_root(output: String, root: &Path) -> String {
    normalize_fixture_root_with_windows_alias(output, root, cfg!(windows))
}

fn normalize_fixture_root_with_windows_alias(
    output: String,
    root: &Path,
    normalize_windows_alias: bool,
) -> String {
    let root = root.to_string_lossy().replace('\\', "/");
    // JSON escapes every Windows separator, so normalize escaped pairs before
    // individual separators. Reversing this order turns `\\\\` into `//`.
    let mut output = output.replace("\\\\", "/").replace('\\', "/");
    output = output.replace(&root, "<ROOT>");
    // Cargo metadata can preserve an 8.3 alias (for example RUNNER~1)
    // while tempfile exposes the long root spelling. Both refer to the
    // fixture root; retain the remainder byte-for-byte for the golden.
    if normalize_windows_alias {
        if let Some(message) = output.find("\"message\": \"") {
            let path_start = message + "\"message\": \"".len();
            if let Some(relative) = output[path_start..].find("/crates/") {
                output.replace_range(path_start..path_start + relative, "<ROOT>");
            }
        } else if let Some(index) = output.find("/crates/") {
            output.replace_range(..index, "<ROOT>");
        }
    }
    output
}

#[test]
fn metadata_golden_root_normalization_handles_raw_and_json_windows_short_paths() {
    let fixture = WorkspaceFixture::new();
    let cases = [
        (
            "raw cargo metadata path",
            r"C:\Users\RUNNER~1\AppData\Local\Temp\fixture\crates\example\src\lib.rs",
            "<ROOT>/crates/example/src/lib.rs",
        ),
        (
            "JSON-escaped cargo metadata path",
            r#"{"message": "C:\\Users\\RUNNER~1\\AppData\\Local\\Temp\\fixture\\crates\\example\\src\\lib.rs"}"#,
            r#"{"message": "<ROOT>/crates/example/src/lib.rs"}"#,
        ),
    ];

    for (description, input, expected) in cases {
        let normalized =
            normalize_fixture_root_with_windows_alias(input.to_string(), fixture.root(), true);
        assert_eq!(
            normalized, expected,
            "{description} must retain one separator per path component"
        );
    }
}

const PARALLEL_FILE_COUNT: usize = 10;

fn in_pool<T: Send>(threads: usize, work: impl FnOnce() -> T + Send) -> T {
    rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()
        .unwrap()
        .install(work)
}

/// Extract `(file name, line)` from a finding message of the form
/// `<path>:<line>: <detail>`.
fn finding_location(message: &str) -> (String, usize) {
    let head = message.split(": ").next().unwrap();
    let (path, line) = head.rsplit_once(':').unwrap();
    let file = path.rsplit(['/', '\\']).next().unwrap();
    (file.to_string(), line.parse().unwrap())
}

#[test]
fn parallel_file_analysis_renders_identically_for_multifile_workspaces() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    // Declare modules in reverse order so scheduling or declaration order
    // cannot coincide with the expected sorted order by accident.
    let mut lib = String::new();
    for index in (0..PARALLEL_FILE_COUNT).rev() {
        lib.push_str(&format!("mod file_{index:02};\n"));
        fixture.write_source("example", &format!("file_{index:02}.rs"), &format!(
                "pub fn a_{index:02}() {{ let _ = (\"/tmp/a\", \"/tmp/b\"); }}\npub fn b_{index:02}() {{ let _ = \"/tmp/c\"; }}\n"
            ));
    }
    fixture.write_source("example", "lib.rs", &lib);
    let options = AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
    };
    let analyze = || {
        let report = analyze_workspace(&options).unwrap();
        (
            render_findings_report(&report),
            serde_json::to_vec(&report).unwrap(),
            report,
        )
    };
    // Oracle: a single-thread pool analyzes files strictly sequentially.
    let (expected_text, expected_json, expected) = in_pool(1, analyze);

    let locations: Vec<(String, usize)> = expected
        .findings
        .iter()
        .map(|finding| finding_location(&finding.message))
        .collect();
    assert!(locations.len() >= PARALLEL_FILE_COUNT * 2, "{locations:#?}");
    let mut sorted = locations.clone();
    sorted.sort();
    assert_eq!(locations, sorted, "findings ordered by (source_path, line)");
    // Same-line findings appear for every file (two literals on line 1).
    for index in 0..PARALLEL_FILE_COUNT {
        let file = format!("file_{index:02}.rs");
        let in_file: Vec<usize> = locations
            .iter()
            .filter(|(name, _)| *name == file)
            .map(|(_, line)| *line)
            .collect();
        assert!(in_file.len() >= 2, "{file}: {in_file:?}");
        assert_eq!(in_file.first(), Some(&1), "{file}: {in_file:?}");
        assert_eq!(in_file.last(), Some(&2), "{file}: {in_file:?}");
    }

    for _ in 0..8 {
        let (text, json, report) = in_pool(4, analyze);
        assert_eq!(text, expected_text);
        assert_eq!(json, expected_json);
        assert_eq!(report.findings, expected.findings);
    }
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
    fixture.write_source(
        "example",
        "lib.rs",
        "pub fn path() { let _ = \"/home/user/file\"; }",
    );

    for pattern in ["crates/example/src/**", r"crates\\example\\src\\**"] {
        fixture.write_lint_config(&format!(
            "[tool.sc-lint.analyzers.portability]\nexclude_paths = [\"{pattern}\"]\n"
        ));
        let report = analyze_workspace(&AnalyzeOptions {
            root: fixture.root().to_path_buf(),
            format: OutputFormat::Json,
        })
        .unwrap();

        assert_eq!(report.status, ReportStatus::Pass, "pattern: {pattern}");
        assert!(report.findings.is_empty(), "pattern: {pattern}");
    }
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
    assert_eq!(baseline.status, ReportStatus::Fail);
    assert_eq!(
        baseline
            .findings
            .iter()
            .filter(|finding| finding.rule_id == RuleId::Port006)
            .count(),
        1
    );
    assert_eq!(
        baseline
            .findings
            .iter()
            .filter(|finding| finding.rule_id == RuleId::Port007)
            .count(),
        1
    );

    for pattern in ["crates/example/src/lib.rs", r"crates\\example\\src\\lib.rs"] {
        fixture.write_lint_config(&format!(
            "[tool.sc-lint.analyzers.portability]\nexclude_paths = [\"{pattern}\"]\n"
        ));
        let whole_file_excluded = analyze_workspace(&AnalyzeOptions {
            root: fixture.root().to_path_buf(),
            format: OutputFormat::Json,
        })
        .unwrap();

        assert_eq!(
            whole_file_excluded.status,
            ReportStatus::Pass,
            "pattern: {pattern}"
        );
        assert!(
            whole_file_excluded.findings.is_empty(),
            "pattern: {pattern}"
        );

        fixture.write_lint_config(&format!(
            "[tool.sc-lint.analyzers.portability.exclude_paths_by_rule]\n\"PORT-007\" = [\"{pattern}\"]\n"
        ));
        let report = analyze_workspace(&AnalyzeOptions {
            root: fixture.root().to_path_buf(),
            format: OutputFormat::Json,
        })
        .unwrap();

        assert_eq!(report.status, ReportStatus::Fail, "pattern: {pattern}");
        assert_eq!(report.findings.len(), 1, "pattern: {pattern}");
        assert_eq!(report.findings[0].rule_id, RuleId::Port006);
    }
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
fn port006_suppressed_under_target_os_and_not_windows_cfg() {
    let report = analyze_single_source(
        r#"
        #[cfg(target_os = "linux")]
        fn linux() -> &'static str { "/home/a/x" }
        #[cfg(target_os = "macos")]
        fn macos() -> &'static str { "/home/b/x" }
        #[cfg(not(windows))]
        fn not_windows() -> &'static str { "/home/c/x" }
        #[cfg(all(not(windows), feature = "x"))]
        fn all_not_windows() -> &'static str { "/home/d/x" }
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        fn any_unix_os() -> &'static str { "/home/e/x" }
        pub fn blocks() {
            #[cfg(target_os = "linux")] { let _ = "/home/f/x"; }
            #[cfg(not(windows))] { let _ = "/home/g/x"; }
        }
        "#,
    );

    assert!(report.findings.is_empty(), "{:#?}", report.findings);
}

#[test]
fn port006_statement_cfg_suppresses_unix_path_literal() {
    let report = analyze_single_source(
        r#"
        pub fn gated_statements() {
            #[cfg(target_os = "linux")]
            let _ = "/var/tmp/atm-ports";
            #[cfg(windows)]
            { let _ = "C:\\atm-ports"; let _ = "\\\\server\\share\\atm-ports"; }
            let _ = "/var/tmp/ungated";
        }
        "#,
    );

    assert_eq!(report.findings.len(), 1, "{:#?}", report.findings);
    assert!(report.findings[0].message.contains("/var/tmp/ungated"));
}

#[test]
fn port006_still_fires_on_ungated_item() {
    let report = analyze_single_source(
        r#"
        pub fn ungated() -> &'static str { "/home/ungated/x" }
        #[cfg(not(unix))]
        fn not_unix() -> &'static str { "/home/notunix/x" }
        #[cfg(target_os = "freebsd")]
        fn freebsd() -> &'static str { "/home/freebsd/x" }
        #[cfg(target_family = "unix")]
        fn family() -> &'static str { "/home/family/x" }
        #[cfg(not(target_os = "linux"))]
        fn not_linux() -> &'static str { "/home/notlinux/x" }
        "#,
    );
    let port006 = report
        .findings
        .iter()
        .filter(|finding| finding.rule_id == RuleId::Port006)
        .collect::<Vec<_>>();
    assert_eq!(port006.len(), 5, "{port006:#?}");
    for path in [
        "/home/ungated/x",
        "/home/notunix/x",
        "/home/freebsd/x",
        "/home/family/x",
        "/home/notlinux/x",
    ] {
        assert!(
            port006.iter().any(|finding| finding.message.contains(path)),
            "missing finding for {path}: {port006:#?}"
        );
    }
}

#[test]
fn port007_to_009_suppressed_under_target_os_cfg() {
    let report = analyze_single_source(
        r#"
        #[cfg(target_os = "linux")]
        fn linux_only() {
            let _ = "C:\\Users\\x";
            let _ = std::env::var("HOME");
            let _ = std::process::Command::new("sh");
        }
        "#,
    );

    assert!(report.findings.is_empty(), "{:#?}", report.findings);
}

#[test]
fn port010_impl_method_parity_kept_under_target_os_gated_impl() {
    let report = analyze_single_source(
        r#"
        pub struct W;
        #[cfg(target_os = "linux")]
        impl W {
            #[cfg(unix)]
            fn only_unix(&self) {}
        }
        "#,
    );
    let port010 = report
        .findings
        .iter()
        .filter(|finding| finding.rule_id == RuleId::Port010)
        .collect::<Vec<_>>();

    assert_eq!(port010.len(), 1, "{port010:#?}");
    assert!(port010[0].message.contains("only_unix"));
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
fn port007_ignores_verbatim_prefix() {
    let report = analyze_single_source(
        r#"
        pub fn display() {
            let _ = "\\\\?\\";
            let _ = "\\\\?\\C:\\x";
        }
        "#,
    );

    assert!(
        report
            .findings
            .iter()
            .all(|finding| finding.rule_id != RuleId::Port007),
        "{:#?}",
        report.findings
    );
}

#[test]
fn port007_ignores_device_prefix() {
    let report = analyze_single_source(
        r#"
        pub fn display() {
            let _ = "\\\\.\\";
            let _ = "\\\\.\\pipe\\x";
        }
        "#,
    );

    assert!(
        report
            .findings
            .iter()
            .all(|finding| finding.rule_id != RuleId::Port007),
        "{:#?}",
        report.findings
    );
}

#[test]
fn port007_still_flags_other_unc_literal() {
    let report = analyze_single_source(
        r#"
        pub fn display() {
            let _ = "\\\\server\\share";
            let _ = "\\\\?x";
        }
        "#,
    );
    let port007 = report
        .findings
        .iter()
        .filter(|finding| finding.rule_id == RuleId::Port007)
        .collect::<Vec<_>>();

    assert_eq!(port007.len(), 2, "{port007:#?}");
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
fn port008_user_read_with_username_fallback_not_reported() {
    let report = analyze_single_source(
        r#"
        pub fn current_user() -> String {
            std::env::var("USER")
                .or_else(|_| std::env::var("USERNAME"))
                .unwrap_or_default()
        }
        "#,
    );

    assert!(
        report
            .findings
            .iter()
            .all(|finding| finding.rule_id != RuleId::Port008),
        "{:#?}",
        report.findings
    );
}

#[test]
fn port008_user_read_without_fallback_reported() {
    let report = analyze_single_source(
        r#"
        pub fn only_user() { let _ = std::env::var("USER"); }
        pub fn other_fn() { let _ = std::env::var("USERNAME"); }
        pub fn user_again() { let _ = std::env::var("USER"); }
        "#,
    );
    let user_findings = report
        .findings
        .iter()
        .filter(|finding| finding.rule_id == RuleId::Port008)
        .collect::<Vec<_>>();

    assert_eq!(user_findings.len(), 2, "{user_findings:#?}");
    assert!(
        user_findings
            .iter()
            .all(|finding| finding.message.contains("`USER`"))
    );
}

#[test]
fn port008_impl_method_body_is_checked() {
    let report = analyze_single_source(
        r#"
        pub struct Config;
        impl Config {
            pub fn load(&self) {
                let _ = std::env::var("XDG_CONFIG_HOME");
            }
        }
        "#,
    );

    assert_eq!(
        report
            .findings
            .iter()
            .filter(|finding| finding.rule_id == RuleId::Port008)
            .count(),
        1,
        "{:#?}",
        report.findings
    );
}

#[test]
fn port008_const_and_static_initializers_do_not_use_username_fallback() {
    let report = analyze_single_source(
        r#"
        const CONST_USER: () = {
            let _ = std::env::var("USER");
            let _ = std::env::var("USERNAME");
        };
        static STATIC_USER: () = {
            let _ = std::env::var("USER");
            let _ = std::env::var("USERNAME");
        };
        "#,
    );
    let user_findings = report
        .findings
        .iter()
        .filter(|finding| finding.rule_id == RuleId::Port008)
        .collect::<Vec<_>>();

    assert_eq!(user_findings.len(), 2, "{user_findings:#?}");
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

#[test]
fn issue372_separate_adapter_pair() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_lint_config(
        r#"
        [tool.sc-lint.analyzers.portability]
        [[tool.sc-lint.analyzers.portability.platform_adapters]]
        unix_module = "crates/example/src/unix_socket.rs"
        windows_module = "crates/example/src/named_pipe.rs"
        reason = "Private transport adapters"
        "#,
    );
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
        #[cfg(unix)]
        mod unix_socket;
        #[cfg(windows)]
        mod named_pipe;
        #[cfg(unix)]
        pub fn unrelated_public_api() {}
        #[cfg(unix)]
        pub(crate) fn unrelated_restricted_api() {}
        "#,
    );
    fixture.write_source("example", "unix_socket.rs", "fn connect() {}");
    fixture.write_source("example", "named_pipe.rs", "fn connect() {}");

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
    })
    .unwrap();

    let port010 = report
        .findings
        .iter()
        .filter(|finding| finding.rule_id == RuleId::Port010)
        .collect::<Vec<_>>();
    assert_eq!(port010.len(), 2);
    assert!(port010[0].message.contains("unrelated_public_api"));
    assert!(port010[1].message.contains("unrelated_restricted_api"));
    assert!(
        port010
            .iter()
            .all(|finding| !finding.message.contains("unix_socket"))
    );
}

#[test]
fn issue372_adapter_pair_inherits_enclosing_cfg_before_path_exclusions() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_lint_config(
        r#"
        [tool.sc-lint.analyzers.portability]
        exclude_paths = ["crates/example/src/unix/transport.rs"]
        [[tool.sc-lint.analyzers.portability.platform_adapters]]
        unix_module = "crates/example/src/unix/transport.rs"
        windows_module = "crates/example/src/windows/transport.rs"
        reason = "Private transport adapters"
        "#,
    );
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
        #[cfg(unix)]
        mod unix {
            mod transport;
        }

        #[cfg(windows)]
        mod windows {
            mod transport;
        }
        "#,
    );
    fixture.write_source("example", "unix/transport.rs", "fn connect() {}");
    fixture.write_source("example", "windows/transport.rs", "fn connect() {}");

    let report = analyze_workspace_configured(
        &AnalyzeOptions {
            root: fixture.root().to_path_buf(),
            format: OutputFormat::Json,
        },
        None,
        Some("PORT-001"),
    )
    .expect("enclosing cfg validates before excluding a configured adapter file");

    assert!(report.findings.is_empty());
}

#[test]
fn issue372_retains_public_and_unrelated_parity() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_lint_config(
        r#"
        [tool.sc-lint.analyzers.portability]
        [[tool.sc-lint.analyzers.portability.platform_adapters]]
        unix_module = "crates/example/src/unix_socket.rs"
        windows_module = "crates/example/src/named_pipe.rs"
        reason = "Private transport adapters"
        "#,
    );
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
        #[cfg(unix)]
        mod unix_socket;
        #[cfg(windows)]
        mod named_pipe;
        #[cfg(unix)]
        mod unpaired_unix_only;
        use std::os::unix::net::UnixStream;
        #[cfg(unix)]
        pub fn unrelated_public_api() {}
        #[cfg(unix)]
        pub(crate) fn unrelated_restricted_api() {}
        #[cfg(unix)]
        pub fn unrelated_sibling_api() {}
        pub mod nested {
            #[cfg(unix)]
            pub fn unrelated_child_api() {}
        }
        "#,
    );
    fixture.write_source("example", "unix_socket.rs", "fn connect() {}");
    fixture.write_source("example", "named_pipe.rs", "fn connect() {}");
    fixture.write_source("example", "unpaired_unix_only.rs", "fn only_unix() {}");

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
    })
    .unwrap();

    let port010 = report
        .findings
        .iter()
        .filter(|finding| finding.rule_id == RuleId::Port010)
        .collect::<Vec<_>>();
    for name in [
        "unrelated_public_api",
        "unrelated_restricted_api",
        "unrelated_sibling_api",
        "unrelated_child_api",
        "unpaired_unix_only",
    ] {
        assert!(
            port010.iter().any(|finding| finding.message.contains(name)),
            "PORT-010 must retain `{name}`: {port010:#?}"
        );
    }
    assert!(
        port010
            .iter()
            .all(|finding| !finding.message.contains("unix_socket")),
        "the adapter pair must suppress only its own module: {port010:#?}"
    );
    let port004 = report
        .findings
        .iter()
        .filter(|finding| finding.rule_id == RuleId::Port004)
        .count();
    assert_eq!(
        port004, 1,
        "other rules are retained: {:#?}",
        report.findings
    );
}

#[test]
fn port010_unix_helper_in_paired_adapter_file_not_reported() {
    let fixture = paired_adapter_fixture(
        "#[cfg(unix)] fn helper() {}\n#[cfg(unix)] mod nested { fn nested_helper() {} }",
    );
    let report = analyze_fixture(&fixture);
    let port010 = report
        .findings
        .iter()
        .filter(|finding| finding.rule_id == RuleId::Port010)
        .collect::<Vec<_>>();

    assert!(port010.is_empty(), "{port010:#?}");
}

#[test]
fn port010_unix_fn_in_unpaired_file_reported() {
    let fixture = paired_adapter_fixture("fn connect() {}\n");
    fixture.write_source(
        "example",
        "lib.rs",
        "#[cfg(unix)] mod unix_socket;\n#[cfg(windows)] mod named_pipe;\n#[cfg(unix)] mod other;\n",
    );
    fixture.write_source("example", "other.rs", "#[cfg(unix)] fn helper() {}\n");
    let report = analyze_fixture(&fixture);
    let port010 = report
        .findings
        .iter()
        .filter(|finding| finding.rule_id == RuleId::Port010)
        .collect::<Vec<_>>();

    assert!(
        port010
            .iter()
            .any(|finding| finding.message.contains("helper")),
        "{port010:#?}"
    );
}

#[test]
fn port010_pub_unix_item_in_paired_file_keeps_parity_check() {
    let fixture = paired_adapter_fixture(
        "#[cfg(unix)] pub fn exposed() {}\n#[cfg(unix)] pub(crate) fn restricted() {}\n#[cfg(unix)] pub(super) fn super_restricted() {}\n#[cfg(unix)] pub(in crate) fn crate_restricted() {}\n",
    );
    let report = analyze_fixture(&fixture);
    let port010 = report
        .findings
        .iter()
        .filter(|finding| finding.rule_id == RuleId::Port010)
        .collect::<Vec<_>>();

    for name in [
        "exposed",
        "restricted",
        "super_restricted",
        "crate_restricted",
    ] {
        assert!(
            port010.iter().any(|finding| finding.message.contains(name)),
            "missing {name}: {port010:#?}"
        );
    }
}

#[test]
fn port010_private_unix_method_in_paired_adapter_file_not_reported() {
    let fixture = paired_adapter_fixture(
        "pub struct Conn;\nimpl Conn {\n#[cfg(unix)] fn private_helper(&self) {}\n#[cfg(unix)] pub fn public_helper(&self) {}\n}\n",
    );
    let report = analyze_fixture(&fixture);
    let port010 = report
        .findings
        .iter()
        .filter(|finding| finding.rule_id == RuleId::Port010)
        .collect::<Vec<_>>();

    assert!(
        port010
            .iter()
            .all(|finding| !finding.message.contains("private_helper")),
        "{port010:#?}"
    );
    assert!(
        port010
            .iter()
            .any(|finding| finding.message.contains("public_helper")),
        "{port010:#?}"
    );
}

#[test]
fn port010_trait_impl_unix_method_in_paired_file_keeps_parity_check() {
    let fixture = paired_adapter_fixture(
        "pub struct Conn;\npub trait Probe { fn probe(&self); }\nimpl Probe for Conn { #[cfg(unix)] fn probe(&self) {} }\n",
    );
    let report = analyze_fixture(&fixture);
    let port010 = report
        .findings
        .iter()
        .filter(|finding| finding.rule_id == RuleId::Port010)
        .collect::<Vec<_>>();

    assert!(
        port010
            .iter()
            .any(|finding| finding.message.contains("probe")),
        "{port010:#?}"
    );
}

#[test]
fn issue372_nested_file_module_inherits_enclosing_cfg() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_lint_config(
        r#"
        [tool.sc-lint.analyzers.portability]
        [[tool.sc-lint.analyzers.portability.platform_adapters]]
        unix_module = "crates/example/src/unix/transport.rs"
        windows_module = "crates/example/src/windows/transport.rs"
        reason = "Private transport adapters"
        "#,
    );
    fixture.write_source(
        "example",
        "lib.rs",
        "#[cfg(unix)]\nmod unix;\n#[cfg(windows)]\nmod windows;\n",
    );
    fixture.write_source("example", "unix.rs", "mod transport;\n");
    fixture.write_source("example", "windows.rs", "mod transport;\n");
    fixture.write_source("example", "unix/transport.rs", "fn connect() {}");
    fixture.write_source("example", "windows/transport.rs", "fn connect() {}");

    let report = analyze_workspace_configured(
        &AnalyzeOptions {
            root: fixture.root().to_path_buf(),
            format: OutputFormat::Json,
        },
        None,
        None,
    )
    .expect("a file module declared under cfg(unix) inherits the gate for its adapter pair");
    assert!(
        report
            .findings
            .iter()
            .all(|finding| !finding.message.contains("transport")),
        "{:#?}",
        report.findings
    );
}

#[test]
fn issue372_excluded_configured_file_keeps_pair_valid() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_lint_config(
        r#"
        [tool.sc-lint.analyzers.portability]
        exclude_paths = ["crates/example/src/unix_socket.rs"]
        [[tool.sc-lint.analyzers.portability.platform_adapters]]
        unix_module = "crates/example/src/unix_socket.rs"
        windows_module = "crates/example/src/named_pipe.rs"
        reason = "Private transport adapters"
        "#,
    );
    fixture.write_source(
        "example",
        "lib.rs",
        "#[cfg(unix)]\nmod unix_socket;\n#[cfg(windows)]\nmod named_pipe;\n",
    );
    fixture.write_source("example", "unix_socket.rs", "fn connect() {}");
    fixture.write_source("example", "named_pipe.rs", "fn connect() {}");

    let report = analyze_workspace_configured(
        &AnalyzeOptions {
            root: fixture.root().to_path_buf(),
            format: OutputFormat::Json,
        },
        None,
        None,
    )
    .expect("excluding a configured adapter file keeps its pair valid");
    assert!(
        report
            .findings
            .iter()
            .all(|finding| !finding.message.contains("unix_socket")),
        "{:#?}",
        report.findings
    );
}

#[test]
fn issue372_invalid_adapter_matrix_reports_semantic_error_before_filtering() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_lint_config(
        r#"
        [tool.sc-lint.analyzers.portability]
        [[tool.sc-lint.analyzers.portability.platform_adapters]]
        unix_module = "crates/example/src/missing.rs"
        windows_module = "crates/example/src/named_pipe.rs"
        reason = "Private transport adapters"
        "#,
    );
    fixture.write_source("example", "lib.rs", "pub struct Example;");

    let error = analyze_workspace_configured(
        &AnalyzeOptions {
            root: fixture.root().to_path_buf(),
            format: OutputFormat::Json,
        },
        None,
        Some("PORT-001"),
    )
    .unwrap_err();

    assert!(matches!(error, PortabilityError::AdapterConfig(_)));
    assert!(error.to_string().contains("ADAPTER_MISSING"));
}

#[test]
fn issue372_invalid_adapter_matrix_covers_path_duplicate_cfg_and_test_cases() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
        #[cfg(unix)] mod unix_socket;
        #[cfg(windows)] mod named_pipe;
        #[cfg(test)] mod test_only;
        "#,
    );
    fixture.write_source("example", "unix_socket.rs", "fn connect() {}");
    fixture.write_source("example", "named_pipe.rs", "fn connect() {}");
    fixture.write_source("example", "test_only.rs", "fn connect() {}");

    let analyze = |unix_module: &str, windows_module: &str| {
        fixture.write_lint_config(&format!(
            "[tool.sc-lint.analyzers.portability]\n[[tool.sc-lint.analyzers.portability.platform_adapters]]\nunix_module = \"{unix_module}\"\nwindows_module = \"{windows_module}\"\nreason = \"Private transport adapters\"\n"
        ));
        let error = analyze_workspace(&AnalyzeOptions {
            root: fixture.root().to_path_buf(),
            format: OutputFormat::Json,
        })
        .unwrap_err();
        let PortabilityError::AdapterConfig(adapter) = &error else {
            panic!("expected an adapter configuration error, got {error}");
        };
        let message = error.to_string();
        assert!(
            message.starts_with(adapter.code()),
            "display keeps the stable code: {message}"
        );
        adapter.code()
    };

    let named_pipe = "crates/example/src/named_pipe.rs";
    for (unix_module, windows_module, code) in [
        ("/tmp/socket.rs", named_pipe, "ANALYZER_CONFIG.ADAPTER_PATH"),
        ("../socket.rs", named_pipe, "ANALYZER_CONFIG.ADAPTER_PATH"),
        (
            "crates/example/src/unix_socket.txt",
            named_pipe,
            "ANALYZER_CONFIG.ADAPTER_PATH",
        ),
        (
            "crates/example/src/missing.rs",
            named_pipe,
            "ANALYZER_CONFIG.ADAPTER_MISSING",
        ),
        (
            "crates/example/src/unix_socket.rs",
            "crates/example/src/unix_socket.rs",
            "ANALYZER_CONFIG.ADAPTER_DUPLICATE",
        ),
        (
            named_pipe,
            "crates/example/src/unix_socket.rs",
            "ANALYZER_CONFIG.ADAPTER_CFG",
        ),
        (
            "crates/example/src/test_only.rs",
            named_pipe,
            "ANALYZER_CONFIG.ADAPTER_TEST_ONLY",
        ),
    ] {
        assert_eq!(analyze(unix_module, windows_module), code, "{unix_module}");
    }
}

#[test]
fn issue372_adapter_scan_failure_keeps_its_cause() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_source(
        "example",
        "lib.rs",
        "#[cfg(unix)] mod unix_socket;\nfn broken( {",
    );
    fixture.write_source("example", "unix_socket.rs", "fn connect() {}");
    fixture.write_lint_config(
        "[tool.sc-lint.analyzers.portability]\n[[tool.sc-lint.analyzers.portability.platform_adapters]]\nunix_module = \"crates/example/src/unix_socket.rs\"\nwindows_module = \"crates/example/src/named_pipe.rs\"\nreason = \"Private transport adapters\"\n",
    );

    let error = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
    })
    .unwrap_err();

    assert!(
        matches!(error, PortabilityError::AdapterScan { .. }),
        "scan failure is not a configuration error: {error}"
    );
    let message = error.to_string();
    assert!(!message.contains("ADAPTER_MISSING"), "{message}");
    assert!(std::error::Error::source(&error).is_some(), "{message}");
}

#[test]
fn issue372_invalid_adapter_matrix_covers_every_row_and_stable_code() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
        #[cfg(unix)] mod unix_socket;
        #[cfg(windows)] mod named_pipe;
        #[cfg(windows)] mod windows_only;
        #[cfg(test)] mod test_only;
        "#,
    );
    fixture.write_source("example", "unix_socket.rs", "fn connect() {}");
    fixture.write_source("example", "named_pipe.rs", "fn connect() {}");
    fixture.write_source("example", "windows_only.rs", "fn connect() {}");
    fixture.write_source("example", "test_only.rs", "fn connect() {}");

    let valid_pair = |unix_module: &str, windows_module: &str| {
        format!(
            "[tool.sc-lint.analyzers.portability]\n[[tool.sc-lint.analyzers.portability.platform_adapters]]\nunix_module = \"{unix_module}\"\nwindows_module = \"{windows_module}\"\nreason = \"Private transport adapters\"\n"
        )
    };
    let repeated_pair = format!(
        "{}[[tool.sc-lint.analyzers.portability.platform_adapters]]\nunix_module = \"crates/example/src/unix_socket.rs\"\nwindows_module = \"crates/example/src/named_pipe.rs\"\nreason = \"Repeated pair\"\n",
        valid_pair(
            "crates/example/src/unix_socket.rs",
            "crates/example/src/named_pipe.rs"
        )
    );
    let cases = [
        (
            "wrong type",
            "ANALYZER_CONFIG.WRONG_TYPE",
            "[tool.sc-lint.analyzers.portability]\nplatform_adapters = \"invalid\"\n".to_owned(),
        ),
        (
            "unknown key",
            "ANALYZER_CONFIG.UNKNOWN_KEY",
            "[tool.sc-lint.analyzers.portability]\n[[tool.sc-lint.analyzers.portability.platform_adapters]]\nunix_module = \"crates/example/src/unix_socket.rs\"\nwindows_module = \"crates/example/src/named_pipe.rs\"\nreason = \"Private transport adapters\"\nextra = true\n".to_owned(),
        ),
        (
            "non-rs path",
            "ANALYZER_CONFIG.ADAPTER_PATH",
            valid_pair(
                "crates/example/src/socket.txt",
                "crates/example/src/named_pipe.rs",
            ),
        ),
        (
            "missing adapter",
            "ANALYZER_CONFIG.ADAPTER_MISSING",
            valid_pair(
                "crates/example/src/missing.rs",
                "crates/example/src/named_pipe.rs",
            ),
        ),
        (
            "repeated pair",
            "ANALYZER_CONFIG.ADAPTER_DUPLICATE",
            repeated_pair,
        ),
        (
            "wrong cfg",
            "ANALYZER_CONFIG.ADAPTER_CFG",
            valid_pair(
                "crates/example/src/windows_only.rs",
                "crates/example/src/named_pipe.rs",
            ),
        ),
        (
            "test-only adapter",
            "ANALYZER_CONFIG.ADAPTER_TEST_ONLY",
            valid_pair(
                "crates/example/src/test_only.rs",
                "crates/example/src/named_pipe.rs",
            ),
        ),
    ];

    let mut mismatches = Vec::new();
    for (label, expected_code, config) in cases {
        fixture.write_lint_config(&config);
        let error = analyze_workspace(&AnalyzeOptions {
            root: fixture.root().to_path_buf(),
            format: OutputFormat::Json,
        })
        .unwrap_err();
        let actual_code = match error {
            PortabilityError::AnalyzerConfig(error) => error.code().to_owned(),
            PortabilityError::AdapterConfig(error) => error.code().to_owned(),
            other => panic!("{label} returned the wrong error family: {other}"),
        };
        if actual_code != expected_code {
            mismatches.push(format!(
                "{label}: expected {expected_code}, got {actual_code}"
            ));
        }
    }
    assert!(
        mismatches.is_empty(),
        "stable-code mismatches: {mismatches:#?}"
    );
}

#[test]
fn issue372_filter_precedence_and_empty() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_source("example", "lib.rs", "#[cfg(unix)] fn unix_only() {}");

    let absent = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
    })
    .unwrap();
    fixture.write_lint_config("[tool.sc-lint.analyzers.portability]\nplatform_adapters = []\n");
    let empty = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
    })
    .unwrap();

    assert!(
        !absent.findings.is_empty(),
        "the fixture must produce a finding"
    );
    assert_eq!(
        serde_json::to_vec(&absent.findings).unwrap(),
        serde_json::to_vec(&empty.findings).unwrap(),
        "absent and [] platform_adapters must serialize byte-equivalent findings"
    );

    for pattern in ["crates/example/src/lib.rs", r"crates\\example\\src\\lib.rs"] {
        fixture.write_lint_config("[tool.sc-lint.analyzers.portability]\n");
        let baseline = analyze_workspace_configured(
            &AnalyzeOptions {
                root: fixture.root().to_path_buf(),
                format: OutputFormat::Json,
            },
            None,
            Some("PORT-010"),
        )
        .expect("an unexcluded filtered baseline is valid");
        assert!(
            baseline
                .findings
                .iter()
                .any(|finding| finding.rule_id == RuleId::Port010),
            "the unexcluded fixture must report PORT-010 before pattern {pattern:?}: {:#?}",
            baseline.findings
        );

        fixture.write_lint_config(&format!(
            "[tool.sc-lint.analyzers.portability]\nexclude_paths = [\"{pattern}\"]\n\
             [tool.sc-lint.analyzers.portability.exclude_paths_by_rule]\n\
             \"PORT-010\" = [\"{pattern}\"]\n"
        ));
        let excluded = analyze_workspace_configured(
            &AnalyzeOptions {
                root: fixture.root().to_path_buf(),
                format: OutputFormat::Json,
            },
            None,
            Some("PORT-010"),
        )
        .expect("filtering keeps optional adapter validation independent");
        assert!(excluded.findings.is_empty(), "pattern: {pattern}");

        fixture.write_lint_config(&format!(
            "[tool.sc-lint.analyzers.portability.exclude_paths_by_rule]\n\
             \"PORT-010\" = [\"{pattern}\"]\n"
        ));
        let per_rule_only = analyze_workspace_configured(
            &AnalyzeOptions {
                root: fixture.root().to_path_buf(),
                format: OutputFormat::Json,
            },
            None,
            Some("PORT-010"),
        )
        .expect("a per-rule exclusion works with a rule filter");
        assert!(per_rule_only.findings.is_empty(), "pattern: {pattern}");
    }

    fixture.write_lint_config("[tool.sc-lint.analyzers.portability]\n");
    let nonmatching = analyze_workspace_configured(
        &AnalyzeOptions {
            root: fixture.root().to_path_buf(),
            format: OutputFormat::Json,
        },
        None,
        Some("PORT-004"),
    )
    .expect("a valid rule selector with no matching findings is not an error");
    assert!(
        nonmatching.findings.is_empty(),
        "a non-matching --rule-filter must select nothing: {:#?}",
        nonmatching.findings
    );
}

#[test]
fn issue372_filter_keeps_configured_pair_valid_when_adapter_file_is_excluded() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_lint_config(
        r#"
        [tool.sc-lint.analyzers.portability]
        exclude_paths = ["crates/example/src/unix_socket.rs"]
        [[tool.sc-lint.analyzers.portability.platform_adapters]]
        unix_module = "crates/example/src/unix_socket.rs"
        windows_module = "crates/example/src/named_pipe.rs"
        reason = "Private transport adapters"
        "#,
    );
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
        #[cfg(unix)]
        mod unix_socket;
        #[cfg(windows)]
        mod named_pipe;
        "#,
    );
    fixture.write_source("example", "unix_socket.rs", "fn connect() {}");
    fixture.write_source("example", "named_pipe.rs", "fn connect() {}");

    let report = analyze_workspace_configured(
        &AnalyzeOptions {
            root: fixture.root().to_path_buf(),
            format: OutputFormat::Json,
        },
        None,
        Some("PORT-010"),
    )
    .expect("excluding a configured adapter file keeps its pair valid");
    assert!(
        report
            .findings
            .iter()
            .all(|finding| !finding.message.contains("unix_socket")),
        "{:#?}",
        report.findings
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

fn analyze_single_source(source: &str) -> FindingsReport {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_source("example", "lib.rs", source);
    analyze_fixture(&fixture)
}

fn analyze_fixture(fixture: &WorkspaceFixture) -> FindingsReport {
    analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
    })
    .expect("fixture analysis succeeds")
}

fn paired_adapter_fixture(unix_source: &str) -> WorkspaceFixture {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_lint_config(
        r#"
        [tool.sc-lint.analyzers.portability]
        [[tool.sc-lint.analyzers.portability.platform_adapters]]
        unix_module = "crates/example/src/unix_socket.rs"
        windows_module = "crates/example/src/named_pipe.rs"
        reason = "Private transport adapters"
        "#,
    );
    fixture.write_source(
        "example",
        "lib.rs",
        "#[cfg(unix)] mod unix_socket;\n#[cfg(windows)] mod named_pipe;\n",
    );
    fixture.write_source("example", "unix_socket.rs", unix_source);
    fixture.write_source(
        "example",
        "named_pipe.rs",
        "#[cfg(unix)] fn windows_private_helper() {}\n",
    );
    fixture
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

#[test]
fn platform_adapter_reason_rejects_empty_and_blank_text() {
    use crate::platform_adapters::NonEmptyReason;

    assert_eq!(NonEmptyReason::new(String::new()), None);
    assert_eq!(NonEmptyReason::new(" \t\n".to_owned()), None);
    assert!(NonEmptyReason::new("Reviewed private transport adapter.".to_owned()).is_some());
}
