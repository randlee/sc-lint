//! Shared, rule-neutral support for `sc-lint` AST analyzer crates.

mod render;
mod source_scan;

pub use render::render_findings_report;
pub use source_scan::FileContext;
pub use source_scan::PackageName;
pub use source_scan::ScopeKind;
pub use source_scan::TargetName;
pub use source_scan::attr_is_cfg_test;
pub use source_scan::attr_is_test;
pub use source_scan::classify_scope;
pub use source_scan::count_scanned_crates;
pub use source_scan::discover_source_files;
pub use source_scan::item_attrs;
pub use source_scan::item_identifier;
pub use source_scan::item_name_hint_is_tests;
pub use source_scan::span_start_line;

#[cfg(test)]
mod tests;
