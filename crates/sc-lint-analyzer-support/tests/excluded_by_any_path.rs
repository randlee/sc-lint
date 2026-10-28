use std::path::Path;

use sc_lint_analyzer_support::AnalyzerName;
use sc_lint_analyzer_support::AnalyzerSettings;
use sc_lint_analyzer_support::CatalogRuleId;
use sc_lint_analyzer_support::RuleCatalog;
use sc_lint_analyzer_support::analyzer_settings_from;

const IDS: &[CatalogRuleId] = &[CatalogRuleId::new("T-001"), CatalogRuleId::new("T-002")];

const CATALOG: RuleCatalog = RuleCatalog {
    analyzer: AnalyzerName::new("demo"),
    rules: IDS,
    path_addressable_rules: IDS,
    families: &[],
    default_fail: &[],
    own_keys: &[],
};

const CONFIG: &str = r#"
[tool.sc-lint.analyzers.demo]
exclude_paths = ["vendor/**"]

[tool.sc-lint.analyzers.demo.exclude_paths_by_rule]
"T-001" = ["src/b.rs"]
"#;

fn settings() -> AnalyzerSettings {
    let table = toml::from_str::<toml::Table>(CONFIG).expect("test config parses");
    analyzer_settings_from(&CATALOG, &table, None).expect("test config is valid")
}

fn excluded(settings: &AnalyzerSettings, rule: &'static str, paths: &[&str]) -> bool {
    let paths: Vec<&Path> = paths.iter().map(Path::new).collect();
    settings.excluded_by_any_path(CatalogRuleId::new(rule), &paths)
}

#[test]
fn excluded_by_any_path_matches_any_listed_path() {
    let settings = settings();
    assert!(excluded(&settings, "T-001", &["src/a.rs", "src/b.rs"]));
    assert!(!excluded(&settings, "T-001", &["src/a.rs"]));
    assert!(excluded(&settings, "T-001", &["src/a.rs", "vendor/x/y.rs"]));
    assert!(!excluded(&settings, "T-002", &["src/b.rs"]));
    assert!(!excluded(&settings, "T-001", &[]));
    assert!(!excluded(&settings, "T-001", &["/abs/src/b.rs"]));
    assert!(!excluded(&settings, "T-001", &["../escape/src/b.rs"]));
    assert!(excluded(&settings, "T-999", &["vendor/x.rs"]));
    assert!(!excluded(&settings, "T-999", &["src/b.rs"]));
}
