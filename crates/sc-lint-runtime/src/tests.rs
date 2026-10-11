#![cfg(test)]

use super::*;
use sc_lint_schema::NodeId;
use sc_lint_schema::OutputFormat;
use sc_lint_schema::OwnerId;
use sc_lint_schema::ReportStatus;
use std::path::Path;

use tempfile::TempDir;

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
