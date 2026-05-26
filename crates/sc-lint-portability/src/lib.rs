use std::path::PathBuf;

use anyhow as anyhow_crate;
use sc_lint_schema::CrateId;
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

pub type FindingsReport = sc_lint_schema::FindingsReport<RuleId>;
pub type Finding = sc_lint_schema::Finding<RuleId>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[non_exhaustive]
pub enum RuleId {
    Port001,
    Port002,
    Port003,
    Port004,
    Port005,
    Port006,
    Port007,
    Port008,
    Port009,
    Port010,
}

impl RuleId {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Port001 => "PORT-001",
            Self::Port002 => "PORT-002",
            Self::Port003 => "PORT-003",
            Self::Port004 => "PORT-004",
            Self::Port005 => "PORT-005",
            Self::Port006 => "PORT-006",
            Self::Port007 => "PORT-007",
            Self::Port008 => "PORT-008",
            Self::Port009 => "PORT-009",
            Self::Port010 => "PORT-010",
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
pub struct PortabilityErrorSource(Box<str>);

impl From<anyhow_crate::Error> for PortabilityErrorSource {
    fn from(value: anyhow_crate::Error) -> Self {
        Self(value.to_string().into_boxed_str())
    }
}

#[derive(Debug, Error)]
pub enum PortabilityError {
    #[error("failed to analyze portability findings for root `{}`: {source:#}", root.display())]
    AnalyzeFindings {
        root: PathBuf,
        #[source]
        source: PortabilityErrorSource,
    },
    #[error("failed to count scanned crates for root `{}`: {source:#}", root.display())]
    CountScannedCrates {
        root: PathBuf,
        #[source]
        source: PortabilityErrorSource,
    },
}

pub fn analyze_workspace(
    options: &AnalyzeOptions,
) -> std::result::Result<FindingsReport, PortabilityError> {
    let findings = portability::analyze_portability(&options.root).map_err(|source| {
        PortabilityError::AnalyzeFindings {
            root: options.root.clone(),
            source: source.into(),
        }
    })?;
    let scanned_crates = portability::count_scanned_crates(&options.root).map_err(|source| {
        PortabilityError::CountScannedCrates {
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
