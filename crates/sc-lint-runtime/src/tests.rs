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
fn runtime_error_source_renders_the_full_cause_chain() {
    let error = anyhow::anyhow!("root cause").context("outer context");

    let source = RuntimeErrorSource::from(error);

    assert_eq!(source.to_string(), "outer context: root cause");
}

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

    assert_eq!(
        expected["schema_version"].as_str(),
        Some(SC_LINT_SCHEMA_VERSION),
        "golden schema_version must match analyzer reports"
    );
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
    .expect("runtime analysis succeeds");
    assert_eq!(metadata_loads.get(), 1);

    // Windows cargo metadata can retain its 8.3 path alias while canonicalize
    // returns the long path, so preserve the temp directory's spelling there.
    #[cfg(windows)]
    let root = fixture.root().to_path_buf();
    #[cfg(not(windows))]
    let root = std::fs::canonicalize(fixture.root()).unwrap();
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

fn normalize_fixture_root(output: String, root: &std::path::Path) -> String {
    normalize_fixture_root_with_windows_alias(output, root, cfg!(windows))
}

fn normalize_fixture_root_with_windows_alias(
    output: String,
    root: &std::path::Path,
    normalize_windows_alias: bool,
) -> String {
    let root = root.to_string_lossy().replace('\\', "/");
    // JSON escapes every Windows separator, so normalize escaped pairs before
    // individual separators. Reversing this order turns `\\\\` into `//`.
    let mut output = output.replace("\\\\", "/").replace('\\', "/");
    output = output.replace(&root, "<ROOT>");
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

fn finding_relative_location(root: &WorkspaceFixture, message: &str) -> (String, usize) {
    let head = message.split(": ").next().unwrap();
    let (path, line) = head.rsplit_once(':').unwrap();
    let canonical_root = dunce::canonicalize(root.root()).unwrap();
    let canonical_source = dunce::canonicalize(path).unwrap();
    let relative = canonical_source.strip_prefix(&canonical_root).unwrap();
    (
        relative.to_string_lossy().replace('\\', "/"),
        line.parse().unwrap(),
    )
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
        self.write_package_manifest_with(package_name, "");
    }

    fn write_package_manifest_with(&self, package_name: &str, extra: &str) {
        self.write(
            &format!("crates/{package_name}/Cargo.toml"),
            &format!(
                r#"
                    [package]
                    name = "{package_name}"
                    version = "0.1.0"
                    edition = "2024"
                    {extra}
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

    fn write_tokio_workspace(&self, members: &[&str]) {
        let members = members
            .iter()
            .map(|member| format!("\"crates/{member}\""))
            .collect::<Vec<_>>()
            .join(", ");
        self.write(
            "Cargo.toml",
            &format!(
                "[workspace]\nmembers = [{members}]\nexclude = [\"vendor/tokio\", \"vendor/tokio-util\"]\nresolver = \"2\"\n"
            ),
        );
        self.write(
            "vendor/tokio/Cargo.toml",
            "[package]\nname = \"tokio\"\nversion = \"1.0.0\"\nedition = \"2024\"\n",
        );
        self.write("vendor/tokio/src/lib.rs", "");
        self.write(
            "vendor/tokio-util/Cargo.toml",
            "[package]\nname = \"tokio-util\"\nversion = \"1.0.0\"\nedition = \"2024\"\n",
        );
        self.write("vendor/tokio-util/src/lib.rs", "");
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

fn tokio_fixture(source: &str, dependency: &str) -> WorkspaceFixture {
    let fixture = WorkspaceFixture::new();
    fixture.write_tokio_workspace(&["example"]);
    fixture.write_package_manifest_with("example", dependency);
    fixture.write_source("example", "lib.rs", source);
    fixture
}

fn assert_sleep_report(report: &FindingsReport, line: usize) {
    assert_eq!(rule_ids(report), vec![RuleId::ScbRuntime003]);
    assert_eq!(report.status, ReportStatus::Fail);
    assert_eq!(
        finding_location(&report.findings[0].message),
        ("lib.rs".into(), line)
    );
    assert!(report.findings[0].message.ends_with(
        "SCB-RUNTIME-003 std::thread::sleep blocks the thread in Tokio package `example`; use `tokio::time::sleep(..).await` instead"
    ));
}

#[test]
fn tokio_package_std_thread_sleep_rejected() {
    let fixture = tokio_fixture(
        "pub fn blocks(d: std::time::Duration) {\n    std::thread::sleep(d);\n}\n",
        "[dependencies]\ntokio = { path = \"../../vendor/tokio\" }",
    );
    let report = analyze_workspace(&analyze_options(&fixture)).unwrap();
    assert_sleep_report(&report, 2);
    assert_eq!(
        finding_relative_location(&fixture, &report.findings[0].message),
        ("crates/example/src/lib.rs".into(), 2)
    );
}

#[test]
fn tokio_sleep_scan_ignores_line_and_doc_comments() {
    let fixture = tokio_fixture(
        "// std::thread::sleep(d);\n/// `std::thread::sleep(d)` is blocking.\npub fn blocks(d: std::time::Duration) { std::thread::sleep(d); }\n",
        "[dependencies]\ntokio = { path = \"../../vendor/tokio\" }",
    );
    let report = analyze_workspace(&analyze_options(&fixture)).unwrap();

    assert_sleep_report(&report, 3);
    assert_eq!(
        report.findings.len(),
        1,
        "comment text is not executable code"
    );
}

#[test]
fn tokio_sleep_report_matches_frozen_json_contract() {
    let fixture = tokio_fixture(
        "pub fn blocks(d: std::time::Duration) { std::thread::sleep(d); }\n",
        "[dependencies]\ntokio = { path = \"../../vendor/tokio\" }",
    );
    let report = analyze_workspace(&analyze_options(&fixture)).unwrap();
    let mut actual = serde_json::to_value(report).unwrap();
    actual["version"] = "<version>".into();
    let message = actual["findings"][0]["message"].as_str().unwrap();
    let detail = message.split_once(": ").unwrap().1;
    let (relative, line) = finding_relative_location(&fixture, message);
    actual["findings"][0]["message"] = format!("<root>/{relative}:{line}: {detail}").into();
    let expected: serde_json::Value = serde_json::from_str(include_str!(
        "../tests/fixtures/contracts/runtime-003-report.json"
    ))
    .expect("runtime-003 baseline is valid JSON");

    assert_eq!(actual, expected);
}

#[test]
fn tokio_package_import_forms_rejected() {
    let dependency = "[dependencies]\ntokio = { path = \"../../vendor/tokio\" }";
    let message_suffix = "SCB-RUNTIME-003 std::thread::sleep blocks the thread in Tokio package `example`; use `tokio::time::sleep(..).await` instead";
    let cases: [(&str, &[usize]); 5] = [
        (
            "use std::{thread as t};\npub fn blocks(d: std::time::Duration) {\n    t::sleep(d);\n}\n",
            &[3],
        ),
        (
            "use std::thread::{self, sleep};\npub fn blocks(d: std::time::Duration) {\n    thread::sleep(d);\n    sleep(d);\n}\n",
            &[3, 4],
        ),
        (
            "use std::thread::*;\npub fn blocks(d: std::time::Duration) {\n    sleep(d);\n}\n",
            &[3],
        ),
        (
            "use std::*;\npub fn blocks(d: std::time::Duration) {\n    thread::sleep(d);\n}\n",
            &[3],
        ),
        (
            "pub fn blocks(d: std::time::Duration) {\n    ::std::thread::sleep(d);\n}\n",
            &[2],
        ),
    ];

    for (source, expected_lines) in cases {
        let fixture = tokio_fixture(source, dependency);
        let report = analyze_workspace(&analyze_options(&fixture)).unwrap();

        assert_eq!(report.status, ReportStatus::Fail);
        assert_eq!(
            rule_ids(&report),
            vec![RuleId::ScbRuntime003; expected_lines.len()]
        );
        assert_eq!(report.findings.len(), expected_lines.len());
        for (finding, expected_line) in report.findings.iter().zip(expected_lines) {
            assert_eq!(
                finding_location(&finding.message),
                ("lib.rs".into(), *expected_line)
            );
            assert!(finding.message.ends_with(message_suffix));
        }
    }
}

#[test]
fn tokio_package_aliased_sleep_import_rejected() {
    let fixture = tokio_fixture(
        "use std::thread::sleep as nap;\npub fn blocks(d: std::time::Duration) { nap(d); }\n",
        "[dependencies]\ntokio = { path = \"../../vendor/tokio\" }",
    );
    assert_sleep_report(&analyze_workspace(&analyze_options(&fixture)).unwrap(), 2);
}

#[test]
fn test_only_thread_sleep_allowed() {
    let fixture = tokio_fixture(
        "#[cfg(test)] use std::thread::sleep as test_sleep;\n#[cfg(test)] mod checks { pub fn blocks(d: std::time::Duration) { std::thread::sleep(d); } }\n#[test] fn unit_test(d: std::time::Duration) { std::thread::sleep(d); }\npub fn uses_test_import(d: std::time::Duration) { test_sleep(d); }\n",
        "[dependencies]\ntokio = { path = \"../../vendor/tokio\" }",
    );
    let report = analyze_workspace(&analyze_options(&fixture)).unwrap();
    assert_eq!(report.status, ReportStatus::Pass);
    assert!(report.findings.is_empty());
}

#[test]
fn tokio_dev_dependency_only_allowed() {
    let fixture = tokio_fixture(
        "pub fn blocks(d: std::time::Duration) { std::thread::sleep(d); }\n",
        "[dev-dependencies]\ntokio = { path = \"../../vendor/tokio\" }",
    );
    let report = analyze_workspace(&analyze_options(&fixture)).unwrap();
    assert_eq!(report.status, ReportStatus::Pass);
    assert!(report.findings.is_empty());
}

#[test]
fn tokio_sleep_excluded_by_exclude_paths_by_rule() {
    let fixture = WorkspaceFixture::new();
    fixture.write_tokio_workspace(&["example", "other"]);
    let dependency = "[dependencies]\ntokio = { path = \"../../vendor/tokio\" }";
    fixture.write_package_manifest_with("example", dependency);
    fixture.write_package_manifest_with("other", dependency);
    let source = "pub fn blocks(d: std::time::Duration) { std::thread::sleep(d); }\n";
    fixture.write_source("example", "lib.rs", source);
    fixture.write_source("other", "lib.rs", source);
    fixture.write_lint_config(
        r#"
        [tool.sc-lint.analyzers.runtime]
        reason = "staged migration to tokio::time::sleep"

        [tool.sc-lint.analyzers.runtime.exclude_paths_by_rule]
        "SCB-RUNTIME-003" = ["crates/example/src/lib.rs"]
        "#,
    );
    let report = analyze_workspace(&analyze_options(&fixture)).unwrap();
    assert_eq!(rule_ids(&report), vec![RuleId::ScbRuntime003]);
    assert_eq!(report.status, ReportStatus::Fail);
    assert_eq!(report.findings.len(), 1);
    assert_eq!(
        finding_location(&report.findings[0].message),
        ("lib.rs".into(), 1)
    );
    assert!(
        report.findings[0]
            .message
            .contains("in Tokio package `other`; use")
    );

    fixture.write_lint_config(
        r#"
        [tool.sc-lint.analyzers.runtime]
        reason = "staged migration to tokio::time::sleep"

        [tool.sc-lint.analyzers.runtime.exclude_paths_by_rule]
        "SCB-RUNTIME-003" = ["**/lib.rs"]
        "#,
    );
    let report = analyze_workspace(&analyze_options(&fixture)).unwrap();
    assert_eq!(report.status, ReportStatus::Pass);
    assert!(report.findings.is_empty());
}

const TOKIO_DEP: &str = "[dependencies]\ntokio = { path = \"../../vendor/tokio\" }";

fn sleep_ids(source: &str, dependency: &str) -> Vec<RuleId> {
    let fixture = tokio_fixture(source, dependency);
    rule_ids(&analyze_workspace(&analyze_options(&fixture)).unwrap())
}

#[test]
fn tokio_package_thread_sleep_after_use_rejected() {
    assert_eq!(
        sleep_ids(
            "use std::thread; fn f(d: std::time::Duration) { thread::sleep(d); }",
            TOKIO_DEP
        ),
        vec![RuleId::ScbRuntime003]
    );
}

#[test]
fn tokio_package_nested_sleep_sites_rejected() {
    let source = "fn f(d: std::time::Duration) { let c = || std::thread::sleep(d); c(); } struct S; impl S { fn m(d: std::time::Duration) { std::thread::sleep(d); } } trait T { fn t(d: std::time::Duration) { std::thread::sleep(d); } }";
    assert_eq!(sleep_ids(source, TOKIO_DEP), vec![RuleId::ScbRuntime003; 3]);
}

#[test]
fn bin_target_thread_sleep_rejected() {
    let fixture = tokio_fixture("fn ignored() {}", TOKIO_DEP);
    fixture.write(
        "crates/example/src/main.rs",
        "fn main() { std::thread::sleep(std::time::Duration::ZERO); }",
    );
    let report = analyze_workspace(&analyze_options(&fixture)).unwrap();
    assert!(rule_ids(&report).contains(&RuleId::ScbRuntime003));
}

#[test]
fn tokio_in_build_dependencies_detected() {
    assert_eq!(
        sleep_ids(
            "fn f(d: std::time::Duration) { std::thread::sleep(d); }",
            "[build-dependencies]\ntokio = { path = \"../../vendor/tokio\" }"
        ),
        vec![RuleId::ScbRuntime003]
    );
}

#[test]
fn tokio_in_target_specific_dependencies_detected() {
    assert_eq!(
        sleep_ids(
            "fn f(d: std::time::Duration) { std::thread::sleep(d); }",
            "[target.'cfg(unix)'.dependencies]\ntokio = { path = \"../../vendor/tokio\" }"
        ),
        vec![RuleId::ScbRuntime003]
    );
}

#[test]
fn tokio_renamed_dependency_detected() {
    assert_eq!(
        sleep_ids(
            "fn f(d: std::time::Duration) { std::thread::sleep(d); }",
            "[dependencies]\nrt = { package = \"tokio\", path = \"../../vendor/tokio\" }"
        ),
        vec![RuleId::ScbRuntime003]
    );
}

#[test]
fn cfg_any_and_not_test_thread_sleep_rejected() {
    assert_eq!(
        sleep_ids(
            "#[cfg(any(test, unix))] fn f(d: std::time::Duration) { std::thread::sleep(d); }\n#[cfg(not(test))] fn g(d: std::time::Duration) { std::thread::sleep(d); }",
            TOKIO_DEP
        ),
        vec![RuleId::ScbRuntime003; 2]
    );
}

#[test]
fn module_named_tests_without_cfg_test_is_checked() {
    assert_eq!(
        sleep_ids(
            "mod tests { pub fn f(d: std::time::Duration) { std::thread::sleep(d); } }",
            TOKIO_DEP
        ),
        vec![RuleId::ScbRuntime003]
    );
}

#[test]
fn test_only_const_and_static_imports_bind_nothing() {
    assert!(
        sleep_ids(
            "#[cfg(test)] const _: () = { use std::thread::sleep as nap; };\n#[cfg(test)] static S: () = { use std::thread; };\nfn f(d: std::time::Duration) { nap(d); thread::sleep(d); }",
            TOKIO_DEP
        )
        .is_empty()
    );
}

#[test]
fn cfg_attr_test_is_ignored_and_thread_sleep_rejected() {
    assert_eq!(
        sleep_ids(
            "#[cfg_attr(test, allow(dead_code))] fn f(d: std::time::Duration) { std::thread::sleep(d); }\n#[cfg_attr(test, test)] fn g(d: std::time::Duration) { std::thread::sleep(d); }\n#[cfg_attr(test, path = \"x.rs\")] mod m { pub fn h(d: std::time::Duration) { std::thread::sleep(d); } }",
            TOKIO_DEP
        ),
        vec![RuleId::ScbRuntime003; 3]
    );
}

#[test]
fn fn_named_tests_without_test_attribute_is_checked() {
    assert_eq!(
        sleep_ids(
            "fn tests(d: std::time::Duration) { std::thread::sleep(d); }",
            TOKIO_DEP
        ),
        vec![RuleId::ScbRuntime003]
    );
}

#[test]
fn single_segment_test_path_other_than_test_attribute_is_checked() {
    // Only `#[test]` or a path of two or more segments ending in `test` is
    // test-only; `#[::test]` names an extern crate path, not the attribute.
    assert_eq!(
        sleep_ids(
            "#[::test] fn f(d: std::time::Duration) { std::thread::sleep(d); }",
            TOKIO_DEP
        ),
        vec![RuleId::ScbRuntime003]
    );
}

#[test]
fn tokio_time_sleep_allowed() {
    assert!(sleep_ids("fn f() { tokio::time::sleep(()); }", TOKIO_DEP).is_empty());
}

#[test]
fn non_thread_sleep_calls_allowed() {
    assert!(
        sleep_ids(
            "fn sleep(_: std::time::Duration) {} fn f(d: std::time::Duration) { sleep(d); }",
            TOKIO_DEP
        )
        .is_empty()
    );
}

#[test]
fn tokio_test_attribute_fn_thread_sleep_allowed() {
    assert!(
        sleep_ids(
            "#[tokio::test] async fn f() { std::thread::sleep(std::time::Duration::ZERO); }",
            TOKIO_DEP
        )
        .is_empty()
    );
}

#[test]
fn cfg_all_test_thread_sleep_allowed() {
    assert!(
        sleep_ids(
            "#[cfg(all(test, unix))] fn f(d: std::time::Duration) { std::thread::sleep(d); }",
            TOKIO_DEP
        )
        .is_empty()
    );
}

#[test]
fn non_tokio_package_thread_sleep_allowed() {
    assert!(
        sleep_ids(
            "fn f(d: std::time::Duration) { std::thread::sleep(d); }",
            ""
        )
        .is_empty()
    );
}

#[test]
fn tokio_util_only_allowed() {
    assert!(
        sleep_ids(
            "fn f(d: std::time::Duration) { std::thread::sleep(d); }",
            "[dependencies]\ntokio-util = { path = \"../../vendor/tokio-util\" }"
        )
        .is_empty()
    );
}

#[test]
fn tokio_workspace_inherited_dependency_detected() {
    let fixture = tokio_fixture(
        "fn f(d: std::time::Duration) { std::thread::sleep(d); }",
        "[dependencies]\ntokio.workspace = true",
    );
    fixture.write(
        "Cargo.toml",
        "[workspace]\nmembers = [\"crates/example\"]\nexclude = [\"vendor/tokio\", \"vendor/tokio-util\"]\nresolver = \"2\"\n[workspace.dependencies]\ntokio = { path = \"vendor/tokio\" }\n",
    );
    assert_eq!(
        rule_ids(&analyze_workspace(&analyze_options(&fixture)).unwrap()),
        vec![RuleId::ScbRuntime003]
    );
}

#[test]
fn bench_and_example_thread_sleep_allowed() {
    let fixture = tokio_fixture("pub fn clean() {}", TOKIO_DEP);
    fixture.write(
        "crates/example/benches/b.rs",
        "fn main() { std::thread::sleep(std::time::Duration::ZERO); }",
    );
    fixture.write(
        "crates/example/examples/e.rs",
        "fn main() { std::thread::sleep(std::time::Duration::ZERO); }",
    );
    fixture.write_package_manifest_with("example", "[dependencies]\ntokio = { path = \"../../vendor/tokio\" }\n[[bench]]\nname = \"b\"\npath = \"benches/b.rs\"\nharness = false\n[[example]]\nname = \"e\"\npath = \"examples/e.rs\"");
    assert!(rule_ids(&analyze_workspace(&analyze_options(&fixture)).unwrap()).is_empty());
}

#[test]
fn example_under_src_thread_sleep_allowed() {
    let fixture = tokio_fixture(
        "pub fn f(d: std::time::Duration) { std::thread::sleep(d); }",
        TOKIO_DEP,
    );
    fixture.write_source(
        "example",
        "ex.rs",
        "pub fn example() { std::thread::sleep(std::time::Duration::ZERO); }",
    );
    fixture.write_package_manifest_with("example", "[dependencies]\ntokio = { path = \"../../vendor/tokio\" }\n[[example]]\nname = \"ex\"\npath = \"src/ex.rs\"");
    assert_eq!(
        rule_ids(&analyze_workspace(&analyze_options(&fixture)).unwrap()),
        vec![RuleId::ScbRuntime003]
    );
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

    let every = [ScbRuntime001, ScbRuntime002, ScbRuntime003];
    for id in every {
        match id {
            ScbRuntime001 | ScbRuntime002 | ScbRuntime003 => {}
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
