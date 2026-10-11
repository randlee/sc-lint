use std::collections::BTreeSet;
use std::error::Error;
use std::fs;
use std::io::ErrorKind;

use serde::Serialize;
use tempfile::TempDir;

use sc_lint_schema::Finding;
use sc_lint_schema::FindingKind;
use sc_lint_schema::FindingsReport;
use sc_lint_schema::NodeId;
use sc_lint_schema::OwnerId;
use sc_lint_schema::ReportStatus;

use super::AnalyzerConfigError;
use super::AnalyzerName;
use super::CatalogRuleId;
use super::RuleCatalog;
use super::analyzer_settings_from;
use super::load_analyzer_settings;
use super::relative_slash_path;
use super::render_findings_report;

#[derive(Serialize)]
enum TestRule {
    Shared,
}

#[test]
fn text_report_rendering_is_shared_and_stable() {
    let report = FindingsReport {
        tool: "shared-analyzer",
        version: "0.5.0",
        schema_version: "0.1.0",
        status: ReportStatus::Pass,
        scanned_crates: 2,
        findings: vec![Finding {
            rule_id: TestRule::Shared,
            kind: FindingKind::new("shared"),
            message: "shared support fixture".to_owned(),
            owner_ids: vec![OwnerId::new("crate::fixture")],
            node_ids: vec![NodeId::new("crate::fixture::node")],
        }],
    };

    assert_eq!(
        render_findings_report(&report),
        "shared-analyzer 0.5.0 status=pass scanned_crates=2 findings=1"
    );
}

const CATALOG: RuleCatalog = RuleCatalog {
    analyzer: AnalyzerName::new("example"),
    rules: &[
        CatalogRuleId::new("EXAMPLE-001"),
        CatalogRuleId::new("EXAMPLE-002"),
    ],
    families: &[(
        "example",
        &[
            CatalogRuleId::new("EXAMPLE-001"),
            CatalogRuleId::new("EXAMPLE-002"),
        ],
    )],
    default_fail: &[CatalogRuleId::new("EXAMPLE-001")],
    own_keys: &["extension", "labels"],
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
enum ConfigRule {
    One,
    Two,
}

fn config_rule_id(rule: &ConfigRule) -> &'static str {
    match rule {
        ConfigRule::One => "EXAMPLE-001",
        ConfigRule::Two => "EXAMPLE-002",
    }
}

fn config_finding(rule_id: ConfigRule) -> Finding<ConfigRule> {
    Finding {
        rule_id,
        kind: FindingKind::new("example"),
        message: "config fixture".to_owned(),
        owner_ids: Vec::new(),
        node_ids: Vec::new(),
    }
}

fn config_dir(contents: &str) -> TempDir {
    let directory = TempDir::new().unwrap();
    fs::write(directory.path().join("sc-lint.toml"), contents).unwrap();
    directory
}

fn rule_set(rules: &[&'static str]) -> BTreeSet<CatalogRuleId> {
    rules.iter().map(|rule| CatalogRuleId::new(rule)).collect()
}

#[test]
#[should_panic(expected = "analyzer name must not be empty")]
fn empty_analyzer_name_is_rejected() {
    let _ = AnalyzerName::new("");
}

#[test]
#[should_panic(expected = "rule id must not be empty")]
fn empty_catalog_rule_id_is_rejected() {
    let _ = CatalogRuleId::new("");
}

#[test]
fn missing_config_yields_defaults() {
    let directory = TempDir::new().unwrap();
    let settings = load_analyzer_settings(&CATALOG, directory.path(), None, None).unwrap();

    assert!(settings.enabled);
    assert_eq!(settings.fail_rules, rule_set(&["EXAMPLE-001"]));
    assert!(settings.exclude_rules.is_empty());
    assert!(settings.exclude_paths.is_empty());
    assert_eq!(settings.rule_filter, None);
    assert_eq!(settings.own_string("extension").unwrap(), None);
}

#[test]
fn config_without_analyzer_table_yields_defaults() {
    let directory = config_dir("[tool.sc-lint.analyzers.other]\nenabled = false\n");
    let settings = load_analyzer_settings(&CATALOG, directory.path(), None, None).unwrap();

    assert!(settings.enabled);
    assert_eq!(settings.fail_rules, rule_set(&["EXAMPLE-001"]));
}

#[test]
fn explicit_missing_config_is_a_read_error() {
    let directory = TempDir::new().unwrap();
    let missing = directory.path().join("missing.toml");
    let error =
        load_analyzer_settings(&CATALOG, directory.path(), Some(&missing), None).unwrap_err();

    assert_eq!(error.code(), "ANALYZER_CONFIG.READ");
    assert!(Error::source(&error).is_some());
    let AnalyzerConfigError::Read { source, .. } = error else {
        panic!("expected a read error");
    };
    assert_eq!(source.kind(), ErrorKind::NotFound);
}

#[test]
fn malformed_config_preserves_toml_parse_source() {
    let directory = config_dir("[tool.sc-lint.analyzers.example");
    let error = load_analyzer_settings(&CATALOG, directory.path(), None, None).unwrap_err();

    assert_eq!(error.code(), "ANALYZER_CONFIG.PARSE");
    assert!(Error::source(&error).is_some());
    let AnalyzerConfigError::Parse { source, .. } = error else {
        panic!("expected a parse error");
    };
    assert!(!source.to_string().is_empty());
}

#[test]
fn unknown_key_names_analyzer_and_key() {
    let directory = config_dir("[tool.sc-lint.analyzers.example]\nunknown = []\n");
    let error = load_analyzer_settings(&CATALOG, directory.path(), None, None).unwrap_err();

    assert_eq!(error.code(), "ANALYZER_CONFIG.UNKNOWN_KEY");
    assert_eq!(
        error.to_string(),
        "example: unknown key `unknown` in [tool.sc-lint.analyzers.example]"
    );
}

#[test]
fn own_keys_are_preserved_and_common_keys_are_consumed() {
    let directory = config_dir(
        "[tool.sc-lint.analyzers.example]\nenabled = false\nreason = \"not needed\"\nextension = 1\nexclude_paths = [\"tests/**\"]\n",
    );
    let settings = load_analyzer_settings(&CATALOG, directory.path(), None, None).unwrap();

    assert!(!settings.enabled);
    assert_eq!(settings.exclude_paths, ["tests/**"]);
    assert_eq!(
        settings.own_string("extension").unwrap_err().code(),
        "ANALYZER_CONFIG.WRONG_TYPE"
    );
}

#[test]
fn own_key_accessors_validate_strings_and_string_arrays() {
    let directory = config_dir(
        "[tool.sc-lint.analyzers.example]\nextension = \"enabled\"\nlabels = [\"alpha\", \"beta\"]\n",
    );
    let settings = load_analyzer_settings(&CATALOG, directory.path(), None, None).unwrap();

    assert_eq!(
        settings.own_string("extension").unwrap(),
        Some("enabled".to_owned())
    );
    assert_eq!(settings.own_string("missing").unwrap(), None);
    assert_eq!(
        settings.own_strings("labels").unwrap(),
        Some(vec!["alpha".to_owned(), "beta".to_owned()])
    );
    assert_eq!(settings.own_strings("missing").unwrap(), None);
}

#[test]
fn own_key_accessors_report_structured_type_errors() {
    let directory = config_dir(
        "[tool.sc-lint.analyzers.example]\nextension = [\"enabled\"]\nlabels = \"alpha\"\n",
    );
    let settings = load_analyzer_settings(&CATALOG, directory.path(), None, None).unwrap();

    assert_eq!(
        settings.own_string("extension").unwrap_err().to_string(),
        "example: `extension` in [tool.sc-lint.analyzers.example] must be a string"
    );
    assert_eq!(
        settings.own_strings("labels").unwrap_err().to_string(),
        "example: `labels` in [tool.sc-lint.analyzers.example] must be an array of strings"
    );
}

#[test]
fn invalid_common_key_type_is_a_structured_error() {
    let directory =
        config_dir("[tool.sc-lint.analyzers.example]\nexclude_rules = \"EXAMPLE-001\"\n");
    let error = load_analyzer_settings(&CATALOG, directory.path(), None, None).unwrap_err();

    assert_eq!(error.code(), "ANALYZER_CONFIG.WRONG_TYPE");
}

#[test]
fn reason_must_be_a_string() {
    let directory = config_dir("[tool.sc-lint.analyzers.example]\nreason = 5\n");
    let error = load_analyzer_settings(&CATALOG, directory.path(), None, None).unwrap_err();

    assert_eq!(error.code(), "ANALYZER_CONFIG.WRONG_TYPE");
    assert_eq!(
        error.to_string(),
        "example: `reason` in [tool.sc-lint.analyzers.example] must be a string"
    );
}

#[test]
fn families_and_rule_filters_resolve_to_catalog_rules() {
    let directory = TempDir::new().unwrap();
    let settings =
        load_analyzer_settings(&CATALOG, directory.path(), None, Some("example")).unwrap();

    assert_eq!(
        settings.rule_filter,
        Some(rule_set(&["EXAMPLE-001", "EXAMPLE-002"]))
    );
}

#[test]
fn bad_rule_filter_is_a_selector_error() {
    let directory = TempDir::new().unwrap();
    let error =
        load_analyzer_settings(&CATALOG, directory.path(), None, Some("unknown")).unwrap_err();

    assert_eq!(error.code(), "ANALYZER_CONFIG.SELECTOR");
    let AnalyzerConfigError::Selector { analyzer, selector } = error else {
        panic!("expected a selector error");
    };
    assert_eq!(analyzer, "example");
    assert_eq!(selector, "unknown");
}

#[test]
fn empty_rule_filter_entry_is_a_selector_error() {
    let directory = TempDir::new().unwrap();
    let error =
        load_analyzer_settings(&CATALOG, directory.path(), None, Some("EXAMPLE-001,")).unwrap_err();

    assert_eq!(error.code(), "ANALYZER_CONFIG.SELECTOR");
}

#[test]
fn settings_apply_filtering_and_failure_policy() {
    let directory = config_dir(
        "[tool.sc-lint.analyzers.example]\nexclude_rules = [\"EXAMPLE-001\"]\nfail_rules = [\"EXAMPLE-002\"]\n",
    );
    let settings = load_analyzer_settings(&CATALOG, directory.path(), None, None).unwrap();
    let (findings, status) = settings.apply(
        vec![
            config_finding(ConfigRule::One),
            config_finding(ConfigRule::Two),
        ],
        config_rule_id,
    );

    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].rule_id, ConfigRule::Two);
    assert_eq!(status, ReportStatus::Fail);

    let filtered =
        load_analyzer_settings(&CATALOG, directory.path(), None, Some("EXAMPLE-001")).unwrap();
    let (findings, status) = filtered.apply(
        vec![
            config_finding(ConfigRule::One),
            config_finding(ConfigRule::Two),
        ],
        config_rule_id,
    );

    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].rule_id, ConfigRule::One);
    assert_eq!(status, ReportStatus::Pass);
}

#[test]
fn exclude_paths_support_segment_and_recursive_wildcards() {
    let directory = config_dir(
        "[tool.sc-lint.analyzers.example]\nexclude_paths = [\"tests/fixtures/**\", \"crates/*/src/gen?.rs\"]\n",
    );
    let settings = load_analyzer_settings(&CATALOG, directory.path(), None, None).unwrap();

    assert!(settings.path_excluded("tests/fixtures/a/b.rs"));
    assert!(settings.path_excluded("crates/app/src/gen1.rs"));
    assert!(!settings.path_excluded("crates/app/src/gen10.rs"));
    assert!(!settings.path_excluded("crates/app/src/nested/gen1.rs"));
}

#[test]
fn exclude_paths_handle_dense_segment_wildcards_without_backtracking() {
    let pattern = format!("{}b", "*a".repeat(64));
    let directory = config_dir(&format!(
        "[tool.sc-lint.analyzers.example]\nexclude_paths = [\"{pattern}\"]\n"
    ));
    let settings = load_analyzer_settings(&CATALOG, directory.path(), None, None).unwrap();

    assert!(!settings.path_excluded(&format!("{}c", "a".repeat(64))));
}

#[test]
fn exclude_paths_handle_dense_double_star_segments_without_backtracking() {
    let pattern = format!("{}x", "**/".repeat(32));
    let directory = config_dir(&format!(
        "[tool.sc-lint.analyzers.example]\nexclude_paths = [\"{pattern}\"]\n"
    ));
    let settings = load_analyzer_settings(&CATALOG, directory.path(), None, None).unwrap();

    assert!(!settings.path_excluded(&format!("{}y", "a/".repeat(64))));
    assert!(settings.path_excluded(&format!("{}x", "a/".repeat(64))));
}

#[test]
fn exclude_paths_keep_glob_semantics() {
    let directory = config_dir(
        "[tool.sc-lint.analyzers.example]\nexclude_paths = [\"crates/*/src/**/*.rs\", \"docs/?.md\"]\n",
    );
    let settings = load_analyzer_settings(&CATALOG, directory.path(), None, None).unwrap();

    assert!(settings.path_excluded("crates/app/src/lib.rs"));
    assert!(settings.path_excluded("crates/app/src/a/b/mod.rs"));
    assert!(settings.path_excluded("docs/a.md"));
    assert!(!settings.path_excluded("crates/app/tests/lib.rs"));
    assert!(!settings.path_excluded("crates/app/src/lib.txt"));
    assert!(!settings.path_excluded("docs/ab.md"));
    assert!(!settings.path_excluded("crates/a/b/src/lib.rs"));
}

#[test]
fn settings_from_document_matches_file_loading() {
    let text = "[tool.sc-lint.analyzers.example]\nexclude_paths = [\"tests/**\"]\n";
    let directory = config_dir(text);
    let document: toml::Table = toml::from_str(text).unwrap();
    let from_file = load_analyzer_settings(&CATALOG, directory.path(), None, None).unwrap();
    let from_document = analyzer_settings_from(&CATALOG, &document, None).unwrap();

    assert_eq!(from_file, from_document);
}

#[test]
fn relative_slash_path_is_root_relative() {
    let directory = TempDir::new().unwrap();
    let file = directory.path().join("crates/example/src/lib.rs");
    fs::create_dir_all(file.parent().unwrap()).unwrap();
    fs::write(&file, "").unwrap();

    assert_eq!(
        relative_slash_path(directory.path(), &file),
        "crates/example/src/lib.rs"
    );
}
