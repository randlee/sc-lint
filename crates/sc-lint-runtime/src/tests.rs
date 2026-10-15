#![cfg(test)]

use super::*;
use sc_lint_schema::NodeId;
use sc_lint_schema::OutputFormat;
use sc_lint_schema::OwnerId;
use sc_lint_schema::ReportStatus;
use std::collections::BTreeMap;
use std::path::Path;

use tempfile::TempDir;

#[test]
fn outside_source_path_reports_repository_relative_path_error() {
    let root = TempDir::new().unwrap();
    let outside_source = root.path().join("..\u{2f}outside.rs");

    let error = super::runtime::repo_relative_source_path(root.path(), &outside_source)
        .expect_err("outside paths must produce a diagnostic");
    assert!(error.to_string().contains("outside.rs"));
    assert!(error.to_string().contains("repository-relative"));
}

fn installer_catalog() -> serde_json::Value {
    let families = RUNTIME_CATALOG
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
        "analyzer": RUNTIME_CATALOG.analyzer.as_str(),
        "rules": RUNTIME_CATALOG.rules.iter().map(|rule| rule.as_str()).collect::<Vec<_>>(),
        "families": families,
        "own_keys": RUNTIME_CATALOG.own_keys,
    })
}

#[test]
#[ignore]
fn emit_installer_catalog() {
    let output = std::env::var("SC_LINT_INSTALLER_CATALOG_OUTPUT")
        .expect("catalog output path must be provided");
    std::fs::write(
        output,
        serde_json::to_vec_pretty(&installer_catalog()).unwrap(),
    )
    .unwrap();
}

#[test]
fn installer_catalog_matches_rust() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../packages/sc-lint-adoption/analyzer-catalog/runtime.json");
    let actual: serde_json::Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();

    assert_eq!(actual, installer_catalog());
}

#[test]
fn rule_ids_match_frozen_contract_baseline() {
    let expected: serde_json::Value =
        serde_json::from_str(include_str!("../tests/fixtures/contracts/rule-ids.json"))
            .expect("rule-ID baseline is valid JSON");
    let actual = serde_json::json!(
        RUNTIME_CATALOG
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
        tool: "sc-lint-runtime",
        version: "0.1.0",
        schema_version: "0.1.0",
        status: ReportStatus::Fail,
        scanned_crates: 1,
        findings: vec![Finding {
            rule_id: RuleId::ScbRuntime001,
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

    assert_eq!(serde_json::to_value(report).unwrap(), expected);
}

#[test]
fn findings_report_text_is_stable() {
    let report = FindingsReport {
        tool: "sc-lint-runtime",
        version: "0.1.0",
        schema_version: "0.1.0",
        status: ReportStatus::Pass,
        scanned_crates: 2,
        findings: Vec::new(),
    };
    assert_eq!(
        render_findings_report(&report),
        "sc-lint-runtime 0.1.0 status=pass scanned_crates=2 findings=0"
    );
}

#[test]
fn findings_report_text_renders_failure_status_and_count() {
    let report = FindingsReport {
        tool: "sc-lint-runtime",
        version: "0.1.0",
        schema_version: "0.1.0",
        status: ReportStatus::Fail,
        scanned_crates: 3,
        findings: vec![Finding {
            rule_id: RuleId::ScbRuntime001,
            kind: "condvar_wait".into(),
            message: "wait used".to_string(),
            owner_ids: vec![OwnerId::new("crate::example::example")],
            node_ids: vec![NodeId::new("crate::example::example::block")],
        }],
    };

    assert_eq!(
        render_findings_report(&report),
        "sc-lint-runtime 0.1.0 status=fail scanned_crates=3 findings=1"
    );
}

#[test]
fn loads_workspace_metadata_once_per_analyzer_run() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_source("example", "lib.rs", "pub struct Example;");
    let options = AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
    };
    let metadata_loads = std::cell::Cell::new(0);

    let report = analyze_workspace_configured_with_metadata_loader(&options, None, None, |root| {
        metadata_loads.set(metadata_loads.get() + 1);
        sc_lint_analyzer_support::load_workspace_metadata(root)
    })
    .expect("runtime analysis succeeds");

    assert_eq!(metadata_loads.get(), 1);
    assert_eq!(report.scanned_crates, 1);
}

#[test]
fn flags_bare_wait_in_production_code() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
            use std::sync::{Condvar, Mutex};

            pub fn block_until_ready(condvar: &Condvar, state: &Mutex<bool>) {
                let state = state.lock().expect("lock");
                let _guard = condvar.wait(state).expect("wait");
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
            .any(|finding| finding.rule_id == RuleId::ScbRuntime001)
    );
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
                r#"
use std::sync::{{Condvar, Mutex}};
pub fn wait_a_{index:02}(c: &Condvar, s: &Mutex<bool>) {{ let g = s.lock().unwrap(); let _ = c.wait(g).unwrap(); }}
pub fn wait_b_{index:02}(c: &Condvar, s: &Mutex<bool>) {{
    let g = s.lock().unwrap();
    let _ = c.wait(g).unwrap();
}}
"#
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
    for index in 0..PARALLEL_FILE_COUNT {
        let file = format!("file_{index:02}.rs");
        let in_file = locations.iter().filter(|(name, _)| *name == file).count();
        assert!(in_file >= 2, "{file}: {in_file}");
    }

    for _ in 0..8 {
        let (text, json, report) = in_pool(4, analyze);
        assert_eq!(text, expected_text);
        assert_eq!(json, expected_json);
        assert_eq!(report.findings, expected.findings);
    }
}

#[test]
fn flags_discarded_wait_timeout_result_in_production_code() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
            use std::sync::{Condvar, Mutex};
            use std::time::Duration;

            pub fn block_until_ready(condvar: &Condvar, state: &Mutex<bool>) {
                let state = state.lock().expect("lock");
                condvar.wait_timeout(state, Duration::from_secs(1)).expect("wait");
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
            .any(|finding| finding.rule_id == RuleId::ScbRuntime002)
    );
}

#[test]
fn passes_inspected_wait_timeout_result_in_production_code() {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    fixture.write_source(
        "example",
        "lib.rs",
        r#"
            use std::sync::{Condvar, Mutex};
            use std::time::Duration;

            pub fn block_until_ready(condvar: &Condvar, state: &Mutex<bool>) {
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

    let report = analyze_workspace(&AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
    })
    .unwrap();

    assert_eq!(report.status, ReportStatus::Pass);
    assert!(!report.findings.iter().any(|finding| matches!(
        finding.rule_id,
        RuleId::ScbRuntime001 | RuleId::ScbRuntime002
    )));
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
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, contents).unwrap();
    }
}

/// One SCB-RUNTIME-001 finding (bare `wait`) and one SCB-RUNTIME-002 finding
/// (discarded `wait_timeout` result).
const BOTH_RULES_SOURCE: &str = r#"
use std::sync::{Condvar, Mutex};
use std::time::Duration;

pub fn block_until_ready(condvar: &Condvar, state: &Mutex<bool>) {
    let state = state.lock().expect("lock");
    let _guard = condvar.wait(state).expect("wait");
}

pub fn block_with_timeout(condvar: &Condvar, state: &Mutex<bool>) {
    let state = state.lock().expect("lock");
    condvar.wait_timeout(state, Duration::from_secs(1)).expect("wait");
}
"#;

fn runtime_fixture(config: Option<&str>) -> WorkspaceFixture {
    let fixture = WorkspaceFixture::new();
    fixture.write_workspace_root();
    fixture.write_package_manifest("example");
    if let Some(config) = config {
        fixture.write_lint_config(config);
    }
    fixture.write_source("example", "lib.rs", BOTH_RULES_SOURCE);
    fixture
}

fn analyze_options(fixture: &WorkspaceFixture) -> AnalyzeOptions {
    AnalyzeOptions {
        root: fixture.root().to_path_buf(),
        format: OutputFormat::Json,
    }
}

fn rule_ids(report: &FindingsReport) -> Vec<RuleId> {
    report
        .findings
        .iter()
        .map(|finding| finding.rule_id)
        .collect()
}

#[test]
fn fixture_without_config_reports_both_rules() {
    let fixture = runtime_fixture(None);

    let report = analyze_workspace(&analyze_options(&fixture)).unwrap();

    assert_eq!(
        rule_ids(&report),
        vec![RuleId::ScbRuntime001, RuleId::ScbRuntime002]
    );
    assert_eq!(report.status, ReportStatus::Fail);
}

#[test]
fn disabled_table_reports_pass_without_findings() {
    let fixture = runtime_fixture(Some(
        "[tool.sc-lint.analyzers.runtime]\nenabled = false\nreason = \"covered elsewhere\"\n",
    ));

    let report = analyze_workspace(&analyze_options(&fixture)).unwrap();

    assert_eq!(report.status, ReportStatus::Pass);
    assert!(report.findings.is_empty());
    assert_eq!(report.scanned_crates, 0);
}

#[test]
fn unknown_table_key_is_config_error() {
    let fixture = runtime_fixture(Some(
        "[tool.sc-lint.analyzers.runtime]\ntargets = [\"linux\"]\n",
    ));

    let Err(RuntimeError::AnalyzerConfig(error)) = analyze_workspace(&analyze_options(&fixture))
    else {
        panic!("expected an analyzer config error");
    };

    assert_eq!(error.code(), "ANALYZER_CONFIG.UNKNOWN_KEY");
    assert_eq!(
        error.to_string(),
        "runtime: unknown key `targets` in [tool.sc-lint.analyzers.runtime]"
    );
}

#[test]
fn rule_filter_selects_exactly_the_rule() {
    let fixture = runtime_fixture(None);

    let report =
        analyze_workspace_configured(&analyze_options(&fixture), None, Some("SCB-RUNTIME-002"))
            .unwrap();

    assert_eq!(rule_ids(&report), vec![RuleId::ScbRuntime002]);
    assert_eq!(report.status, ReportStatus::Fail);
}

#[test]
fn unknown_rule_filter_is_a_selector_error() {
    let fixture = runtime_fixture(None);

    let Err(RuntimeError::AnalyzerConfig(error)) =
        analyze_workspace_configured(&analyze_options(&fixture), None, Some("SCB-RUNTIME-999"))
    else {
        panic!("expected an analyzer config error");
    };

    assert_eq!(error.code(), "ANALYZER_CONFIG.SELECTOR");
}

#[test]
fn exclude_paths_drops_findings_from_matching_files() {
    let fixture = runtime_fixture(Some(
        "[tool.sc-lint.analyzers.runtime]\nexclude_paths = [\"crates/example/src/**\"]\n",
    ));

    let report = analyze_workspace(&analyze_options(&fixture)).unwrap();

    assert_eq!(report.status, ReportStatus::Pass);
    assert!(report.findings.is_empty());
}

#[test]
fn per_rule_path_exclusion_keeps_other_runtime_findings() {
    let fixture = runtime_fixture(Some(
        r#"
        [tool.sc-lint.analyzers.runtime.exclude_paths_by_rule]
        "SCB-RUNTIME-001" = ["crates/example/src/lib.rs"]
        "#,
    ));

    let report = analyze_workspace(&analyze_options(&fixture)).unwrap();

    assert_eq!(rule_ids(&report), vec![RuleId::ScbRuntime002]);
}

#[test]
fn issue385_runtime_paths() {
    let fixture = runtime_fixture(None);
    let baseline = analyze_workspace(&analyze_options(&fixture)).unwrap();
    let baseline_json = serde_json::to_vec(&baseline).unwrap();

    fixture.write_lint_config("[tool.sc-lint.analyzers.runtime]\n");
    assert_eq!(
        serde_json::to_vec(&analyze_workspace(&analyze_options(&fixture)).unwrap()).unwrap(),
        baseline_json
    );

    fixture.write_lint_config("[tool.sc-lint.analyzers.runtime.exclude_paths_by_rule]\n");
    assert_eq!(
        serde_json::to_vec(&analyze_workspace(&analyze_options(&fixture)).unwrap()).unwrap(),
        baseline_json
    );

    fixture.write_lint_config(
        r#"
        [tool.sc-lint.analyzers.runtime]
        exclude_paths = ["crates/example/src/lib.rs"]
        [tool.sc-lint.analyzers.runtime.exclude_paths_by_rule]
        "SCB-RUNTIME-001" = ["crates/other/src/lib.rs"]
        "#,
    );
    let excluded = analyze_workspace(&analyze_options(&fixture)).unwrap();
    assert!(excluded.findings.is_empty());

    fixture.write_lint_config(
        r#"
        [tool.sc-lint.analyzers.runtime.exclude_paths_by_rule]
        "SCB-RUNTIME-001" = ["crates/example/src/lib.rs"]
        "#,
    );
    let settings = sc_lint_analyzer_support::load_analyzer_settings(
        &RUNTIME_CATALOG,
        fixture.root(),
        None,
        None,
    )
    .unwrap();
    let (findings, status) = settings.apply(
        vec![
            (
                baseline.findings[0].clone(),
                Some(
                    sc_lint_analyzer_support::RepoRelativePath::parse(
                        "crates\\example\\src\\lib.rs",
                    )
                    .unwrap(),
                ),
            ),
            (baseline.findings[1].clone(), None),
            (
                baseline.findings[0].clone(),
                Some(
                    sc_lint_analyzer_support::RepoRelativePath::parse(
                        "crates/example/src/other.rs",
                    )
                    .unwrap(),
                ),
            ),
        ],
        |rule| sc_lint_analyzer_support::CatalogRuleId::new(rule.as_str()),
    );
    assert_eq!(
        rule_ids(&FindingsReport {
            findings,
            ..baseline
        }),
        vec![RuleId::ScbRuntime002, RuleId::ScbRuntime001,]
    );
    assert_eq!(status, ReportStatus::Fail);
}

#[test]
fn exclude_and_fail_rules_apply_to_runtime_findings() {
    let fixture = runtime_fixture(Some(
        "[tool.sc-lint.analyzers.runtime]\nexclude_rules = [\"SCB-RUNTIME-001\"]\nfail_rules = []\n",
    ));

    let report = analyze_workspace(&analyze_options(&fixture)).unwrap();

    assert_eq!(rule_ids(&report), vec![RuleId::ScbRuntime002]);
    assert_eq!(report.status, ReportStatus::Pass);
}

#[test]
fn catalog_lists_every_rule_id() {
    use RuleId::*;

    let every = [ScbRuntime001, ScbRuntime002];
    for id in every {
        match id {
            ScbRuntime001 | ScbRuntime002 => {}
        }
    }
    assert_eq!(
        RUNTIME_CATALOG
            .rules
            .iter()
            .map(|rule| rule.as_str())
            .collect::<Vec<_>>(),
        every.map(RuleId::as_str).to_vec()
    );
}

#[test]
fn explicit_config_path_is_read() {
    let fixture = runtime_fixture(None);
    fixture.write(
        "config/lint.toml",
        r#"
        [tool.sc-lint.analyzers.runtime]
        exclude_rules = ["SCB-RUNTIME-001", "SCB-RUNTIME-002"]
        "#,
    );
    let config = fixture.root().join("config/lint.toml");

    let report =
        analyze_workspace_configured(&analyze_options(&fixture), Some(config.as_path()), None)
            .unwrap();

    assert!(rule_ids(&report).is_empty());
}
