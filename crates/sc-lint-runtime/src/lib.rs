//! Runtime analyzer for `sc-lint`: reports blocking waits, sleeps, executor
//! entry and `Mutex` guards held across `.await` as rules `SCB-RUNTIME-001` to
//! `SCB-RUNTIME-007`. Run it as `sc-lint lint sc-runtime`.
//!
//! The primary product is the [`sc-lint`](https://crates.io/crates/sc-lint) CLI;
//! install it rather than this crate unless you are building on the library.
//! Documentation and source: <https://github.com/randlee/sc-lint>.

use std::path::Path;
use std::path::PathBuf;

use anyhow as anyhow_crate;
/// Shared analysis inputs from `sc-lint-analyzer-support`.
#[doc(inline)]
pub use sc_lint_analyzer_support::AnalyzeOptions;
use sc_lint_analyzer_support::AnalyzerConfigError;
use sc_lint_analyzer_support::AnalyzerName;
use sc_lint_analyzer_support::CatalogRuleId;
use sc_lint_analyzer_support::RuleCatalog;
use sc_lint_analyzer_support::WorkspaceMetadata;
use sc_lint_analyzer_support::load_analyzer_settings;
use sc_lint_analyzer_support::load_workspace_metadata;
use sc_lint_schema::CrateId;
use sc_lint_schema::Finding as SchemaFinding;
use sc_lint_schema::FindingsReport as SchemaFindingsReport;
use sc_lint_schema::NodeId;

use sc_lint_schema::OwnerId;
use serde::Serialize;
use serde::Serializer;
use thiserror::Error;

mod api;
mod imports;
mod panicking_calls;
mod position;
mod practices;
mod runtime;
mod scope;
mod sleep;
#[cfg(test)]
mod tests;

const SC_LINT_RUNTIME_TOOL: &str = "sc-lint-runtime";
const SC_LINT_RUNTIME_VERSION: &str = env!("CARGO_PKG_VERSION");

macro_rules! runtime_rules {
    ($($variant:ident => ($code:literal, $kind:literal)),+ $(,)?) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
        #[non_exhaustive]
        pub enum RuleId { $($variant),+ }
        impl RuleId {
            pub(crate) const ALL: &'static [Self] = &[$(Self::$variant),+];
            pub const fn as_str(self) -> &'static str { match self { $(Self::$variant => $code),+ } }
            pub(crate) const fn kind(self) -> &'static str { match self { $(Self::$variant => $kind),+ } }
            pub(crate) fn from_known(value: &str) -> Option<Self> {
                Self::ALL.iter().copied().find(|rule| rule.as_str() == value)
            }
        }
        pub(crate) const KNOWN_RULES: &[&str] = &[$($code),+];
        const CATALOG_RULES: &[CatalogRuleId] = &[$(CatalogRuleId::new($code)),+];
    };
}

runtime_rules! {
    ScbRuntime001 => ("SCB-RUNTIME-001", "condvar_wait_without_timeout"),
    ScbRuntime002 => ("SCB-RUNTIME-002", "discarded_wait_timeout_result"),
    ScbRuntime003 => ("SCB-RUNTIME-003", "blocking_thread_sleep_in_tokio_package"),
    ScbRuntime004 => ("SCB-RUNTIME-004", "blocking_thread_sleep_until"),
    ScbRuntime005 => ("SCB-RUNTIME-005", "tokio_timer_sleep"),
    ScbRuntime006 => ("SCB-RUNTIME-006", "blocking_executor_entry"),
    ScbRuntime007 => ("SCB-RUNTIME-007", "mutex_guard_across_await"),
    ScbRuntime008 => ("SCB-RUNTIME-008", "panicking_call"),
}

pub const RUNTIME_CATALOG: RuleCatalog = RuleCatalog {
    analyzer: AnalyzerName::new("runtime"),
    rules: CATALOG_RULES,
    path_addressable_rules: CATALOG_RULES,
    families: &[],
    default_fail: CATALOG_RULES,
    own_keys: &["panic_rule_packages"],
};

pub type FindingsReport = SchemaFindingsReport<RuleId>;
pub type Finding = SchemaFinding<RuleId>;

impl Serialize for RuleId {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("{0}")]
pub struct RuntimeErrorSource(Box<str>);

impl From<anyhow_crate::Error> for RuntimeErrorSource {
    fn from(value: anyhow_crate::Error) -> Self {
        Self(format!("{value:#}").into_boxed_str())
    }
}

#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error("failed to analyze runtime findings for root `{}`: {source:#}", root.display())]
    AnalyzeFindings {
        root: PathBuf,
        #[source]
        source: RuntimeErrorSource,
    },
    #[error(transparent)]
    AnalyzerConfig(#[from] AnalyzerConfigError),
    #[error(transparent)]
    PanicPackages(#[from] PanicPackageError),
}

pub fn analyze_workspace(
    options: &AnalyzeOptions,
) -> std::result::Result<FindingsReport, RuntimeError> {
    analyze_workspace_configured(options, None, None)
}

#[derive(Debug, Error)]
pub enum PanicPackageError {
    #[error(
        "panic_rule_packages contains an empty package name; remove it or provide a workspace package name"
    )]
    Empty,
    #[error("panic_rule_packages contains duplicate package `{0}`; list each package once")]
    Duplicate(String),
    #[error(
        "panic_rule_packages names unknown workspace package `{0}`; use a workspace package name"
    )]
    Unknown(String),
}

fn panic_packages(
    names: Vec<String>,
    workspace: &WorkspaceMetadata,
) -> Result<std::collections::BTreeSet<String>, RuntimeError> {
    let available = workspace
        .workspace_packages()
        .into_iter()
        .map(|package| package.name.as_str().to_owned())
        .collect::<std::collections::BTreeSet<_>>();
    let mut selected = std::collections::BTreeSet::new();
    for name in names {
        if name.trim().is_empty() {
            return Err(PanicPackageError::Empty.into());
        }
        if !selected.insert(name.clone()) {
            return Err(PanicPackageError::Duplicate(name).into());
        }
        if !available.contains(&name) {
            return Err(PanicPackageError::Unknown(name).into());
        }
    }
    Ok(selected)
}

pub fn analyze_workspace_configured(
    options: &AnalyzeOptions,
    config: Option<&Path>,
    rule_filter: Option<&str>,
) -> std::result::Result<FindingsReport, RuntimeError> {
    analyze_workspace_configured_with_metadata_loader(
        options,
        config,
        rule_filter,
        load_workspace_metadata,
    )
}

fn analyze_workspace_configured_with_metadata_loader(
    options: &AnalyzeOptions,
    config: Option<&Path>,
    rule_filter: Option<&str>,
    mut load_metadata: impl FnMut(&Path) -> anyhow_crate::Result<WorkspaceMetadata>,
) -> std::result::Result<FindingsReport, RuntimeError> {
    let settings = load_analyzer_settings(&RUNTIME_CATALOG, &options.root, config, rule_filter)?;
    let configured = settings
        .own_strings("panic_rule_packages")?
        .unwrap_or_default();
    let (findings, scanned_crates) = if settings.enabled || !configured.is_empty() {
        let workspace =
            load_metadata(&options.root).map_err(|source| RuntimeError::AnalyzeFindings {
                root: options.root.clone(),
                source: source.into(),
            })?;
        let selected = panic_packages(configured, &workspace)?;
        if settings.enabled {
            let findings = runtime::analyze_runtime_liveness(&options.root, &workspace, &selected)
                .map_err(|source| RuntimeError::AnalyzeFindings {
                    root: options.root.clone(),
                    source: source.into(),
                })?;
            let scanned_crates =
                sc_lint_analyzer_support::count_scanned_crates_with_metadata(&workspace);
            (findings, scanned_crates)
        } else {
            (Vec::new(), 0)
        }
    } else {
        (Vec::new(), 0)
    };
    let (findings, status) =
        settings.apply(findings, |rule_id| CatalogRuleId::new(rule_id.as_str()));
    Ok(FindingsReport {
        tool: SC_LINT_RUNTIME_TOOL,
        version: SC_LINT_RUNTIME_VERSION,
        schema_version: sc_lint_schema::ANALYZER_REPORT_SCHEMA_VERSION_V1,
        status,
        scanned_crates,
        findings,
        allowed: Vec::new(),
    })
}

pub fn render_findings_report(report: &FindingsReport) -> String {
    sc_lint_analyzer_support::render_findings_report(report)
}
