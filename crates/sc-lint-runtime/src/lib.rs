use std::path::PathBuf;

use anyhow as anyhow_crate;
use sc_lint_schema::CrateId;
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
}

pub fn analyze_workspace(
    options: &AnalyzeOptions,
) -> std::result::Result<FindingsReport, RuntimeError> {
    let findings = runtime::analyze_runtime_liveness(&options.root).map_err(|source| {
        RuntimeError::AnalyzeFindings {
            root: options.root.clone(),
            source: source.into(),
        }
    })?;
    let scanned_crates = source_scan::count_scanned_crates(&options.root).map_err(|source| {
        RuntimeError::CountScannedCrates {
            root: options.root.clone(),
            source: source.into(),
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
