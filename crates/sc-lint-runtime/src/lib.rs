use std::path::PathBuf;

use sc_lint_schema::Finding as SchemaFinding;
use sc_lint_schema::FindingsReport as SchemaFindingsReport;
use sc_lint_schema::NodeId;
use sc_lint_schema::OutputFormat;
use sc_lint_schema::OwnerId;
use sc_lint_schema::ReportStatus;
use serde::Serialize;
use serde::Serializer;
use thiserror::Error;

mod render;
mod runtime;
mod source_scan;
#[cfg(test)]
mod tests;

const SC_LINT_SCHEMA_VERSION: &str = "0.1.0";
const SC_LINT_RUNTIME_TOOL: &str = "sc-lint-runtime";
const SC_LINT_RUNTIME_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnalyzeOptions {
    pub root: PathBuf,
    pub format: OutputFormat,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CrateId(String);

impl CrateId {
    pub(crate) fn from_parts(package_name: &str, target_name: &str) -> Self {
        Self(format!("crate::{package_name}::{target_name}"))
    }
}

impl From<CrateId> for String {
    fn from(value: CrateId) -> Self {
        value.0
    }
}

pub type FindingsReport = SchemaFindingsReport<RuleId>;
pub type Finding = SchemaFinding<RuleId>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
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

#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error("failed to analyze runtime findings for root `{}`: {detail}", root.display())]
    AnalyzeFindings { root: PathBuf, detail: String },
    #[error("failed to count scanned crates for root `{}`: {detail}", root.display())]
    CountScannedCrates { root: PathBuf, detail: String },
}

pub fn analyze_workspace(
    options: &AnalyzeOptions,
) -> std::result::Result<FindingsReport, RuntimeError> {
    let findings = runtime::analyze_runtime_liveness(&options.root).map_err(|source| {
        RuntimeError::AnalyzeFindings {
            root: options.root.clone(),
            detail: source.to_string(),
        }
    })?;
    let scanned_crates = source_scan::count_scanned_crates(&options.root).map_err(|source| {
        RuntimeError::CountScannedCrates {
            root: options.root.clone(),
            detail: source.to_string(),
        }
    })?;
    let status = if findings.is_empty() {
        ReportStatus::Pass
    } else {
        ReportStatus::Fail
    };
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
    render::render_findings_report(report)
}
