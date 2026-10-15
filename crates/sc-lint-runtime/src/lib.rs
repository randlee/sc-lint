use std::path::Path;
use std::path::PathBuf;

use anyhow as anyhow_crate;
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
use sc_lint_schema::OutputFormat;
use sc_lint_schema::OwnerId;
use serde::Serialize;
use serde::Serializer;
use thiserror::Error;

mod runtime;
#[cfg(test)]
mod tests;

const SC_LINT_SCHEMA_VERSION: &str = "0.1.0";
const SC_LINT_RUNTIME_TOOL: &str = "sc-lint-runtime";
const SC_LINT_RUNTIME_VERSION: &str = env!("CARGO_PKG_VERSION");

pub const RUNTIME_CATALOG: RuleCatalog = RuleCatalog {
    analyzer: AnalyzerName::new("runtime"),
    rules: &[
        CatalogRuleId::new("SCB-RUNTIME-001"),
        CatalogRuleId::new("SCB-RUNTIME-002"),
    ],
    families: &[],
    default_fail: &[
        CatalogRuleId::new("SCB-RUNTIME-001"),
        CatalogRuleId::new("SCB-RUNTIME-002"),
    ],
    own_keys: &[],
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnalyzeOptions {
    pub root: PathBuf,
    pub format: OutputFormat,
}

pub type FindingsReport = SchemaFindingsReport<RuleId>;
pub type Finding = SchemaFinding<RuleId>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[non_exhaustive]
pub enum RuleId {
    ScbRuntime001,
    ScbRuntime002,
}

impl RuleId {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ScbRuntime001 => "SCB-RUNTIME-001",
            Self::ScbRuntime002 => "SCB-RUNTIME-002",
        }
    }
}

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
        Self(value.to_string().into_boxed_str())
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
    #[error("failed to count scanned crates for root `{}`: {source:#}", root.display())]
    CountScannedCrates {
        root: PathBuf,
        #[source]
        source: RuntimeErrorSource,
    },
    #[error(transparent)]
    AnalyzerConfig(#[from] AnalyzerConfigError),
}

pub fn analyze_workspace(
    options: &AnalyzeOptions,
) -> std::result::Result<FindingsReport, RuntimeError> {
    analyze_workspace_configured(options, None, None)
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
    let (findings, scanned_crates) = if settings.enabled {
        let workspace =
            load_metadata(&options.root).map_err(|source| RuntimeError::AnalyzeFindings {
                root: options.root.clone(),
                source: source.into(),
            })?;
        let findings = runtime::analyze_runtime_liveness(&options.root, &workspace, &settings)
            .map_err(|source| RuntimeError::AnalyzeFindings {
                root: options.root.clone(),
                source: source.into(),
            })?;
        let scanned_crates =
            sc_lint_analyzer_support::count_scanned_crates_with_metadata(&workspace);
        (findings, scanned_crates)
    } else {
        (Vec::new(), 0)
    };
    let (findings, status) =
        settings.apply(findings, |rule_id| CatalogRuleId::new(rule_id.as_str()));
    Ok(FindingsReport {
        tool: SC_LINT_RUNTIME_TOOL,
        version: SC_LINT_RUNTIME_VERSION,
        schema_version: SC_LINT_SCHEMA_VERSION,
        status,
        scanned_crates,
        findings,
    })
}

pub fn render_findings_report(report: &FindingsReport) -> String {
    sc_lint_analyzer_support::render_findings_report(report)
}
