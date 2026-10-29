//! Shared, rule-neutral support for `sc-lint` AST analyzer crates.

mod analyzer_config;
mod relative_path;
mod render;
mod source_scan;

pub use analyzer_config::AnalyzerConfigError;
pub use analyzer_config::AnalyzerName;
pub use analyzer_config::AnalyzerSettings;
pub use analyzer_config::CatalogRuleId;
pub use analyzer_config::RuleCatalog;
pub use analyzer_config::analyzer_settings_from;
pub use analyzer_config::load_analyzer_settings;
pub use analyzer_config::read_config_document;
pub use relative_path::RelativePathError;
pub use relative_path::RepoRelativePath;
pub use render::render_findings_report;
pub use source_scan::FileContext;
pub use source_scan::PackageDependency;
pub use source_scan::PackageDependencyKind;
pub use source_scan::PackageName;
pub use source_scan::PackageTarget;
pub use source_scan::PackageTargetKind;
pub use source_scan::ScopeKind;
pub use source_scan::TargetName;
pub use source_scan::WorkspaceMetadata;
pub use source_scan::WorkspacePackageInfo;
pub use source_scan::attr_is_cfg_test;
pub use source_scan::attr_is_test;
pub use source_scan::classify_scope;
pub use source_scan::count_scanned_crates;
pub use source_scan::count_scanned_crates_with_metadata;
pub use source_scan::discover_source_files;
pub use source_scan::discover_source_files_with_metadata;
pub use source_scan::item_attrs;
pub use source_scan::item_identifier;
pub use source_scan::item_name_hint_is_tests;
pub use source_scan::load_workspace_metadata;
pub use source_scan::relative_slash_path;
pub use source_scan::span_start_line;

#[cfg(test)]
mod tests;
