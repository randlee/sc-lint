use std::path::PathBuf;

use anyhow::Error as AnyhowError;
use sc_lint_schema::NodeId;
use sc_lint_schema::OutputFormat;
use sc_lint_schema::OwnerId;
use sc_lint_schema::ReportStatus;
use serde::Serialize;
use serde::Serializer;
use thiserror::Error;

mod portability;
mod render;
mod source_scan;
#[cfg(test)]
mod tests;

const SC_LINT_SCHEMA_VERSION: &str = "0.1.0";
const DEFAULT_RULES_TOML: &str = include_str!("../config/defaults.toml");

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

pub type FindingsReport = sc_lint_schema::FindingsReport<RuleId>;
pub type Finding = sc_lint_schema::Finding<RuleId>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RuleId {
    Port001,
    Port002,
    Port003,
    Port004,
    Port005,
}

impl RuleId {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Port001 => "PORT-001",
            Self::Port002 => "PORT-002",
            Self::Port003 => "PORT-003",
            Self::Port004 => "PORT-004",
            Self::Port005 => "PORT-005",
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
pub enum PortabilityError {
    #[error("failed to analyze portability findings for root `{}`: {source:#}", root.display())]
    AnalyzeFindings {
        root: PathBuf,
        #[source]
        source: AnyhowError,
    },
    #[error("failed to count scanned crates for root `{}`: {source:#}", root.display())]
    CountScannedCrates {
        root: PathBuf,
        #[source]
        source: AnyhowError,
    },
}

pub fn analyze_workspace(
    options: &AnalyzeOptions,
) -> std::result::Result<FindingsReport, PortabilityError> {
    let findings = portability::analyze_portability(&options.root).map_err(|source| {
        PortabilityError::AnalyzeFindings {
            root: options.root.clone(),
            source,
        }
    })?;
    let scanned_crates = portability::count_scanned_crates(&options.root).map_err(|source| {
        PortabilityError::CountScannedCrates {
            root: options.root.clone(),
            source,
        }
    })?;
    let status = if findings.is_empty() {
        ReportStatus::Pass
    } else {
        ReportStatus::Fail
    };
    Ok(FindingsReport {
        tool: "sc-lint-portability",
        version: env!("CARGO_PKG_VERSION"),
        schema_version: SC_LINT_SCHEMA_VERSION,
        status,
        scanned_crates,
        findings,
    })
}

pub fn render_findings_report(report: &FindingsReport) -> String {
    render::render_findings_report(report)
}
