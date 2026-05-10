use std::fmt;
use std::ops::Deref;
use std::path::PathBuf;

use anyhow::Error as AnyhowError;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Text,
    Json,
}

impl fmt::Display for OutputFormat {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Text => formatter.write_str("text"),
            Self::Json => formatter.write_str("json"),
        }
    }
}

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

#[derive(Debug, Clone, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(transparent)]
pub struct NodeId(String);

impl NodeId {
    pub fn new(value: impl Into<String>) -> Self {
        let value = value.into();
        debug_assert!(!value.is_empty(), "node ids must not be empty");
        Self(value)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for NodeId {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Deref for NodeId {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(transparent)]
pub struct OwnerId(String);

impl OwnerId {
    pub fn new(value: impl Into<String>) -> Self {
        let value = value.into();
        debug_assert!(!value.is_empty(), "owner ids must not be empty");
        Self(value)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for OwnerId {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Deref for OwnerId {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl fmt::Display for OwnerId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct FindingsReport {
    pub tool: &'static str,
    pub version: &'static str,
    pub schema_version: &'static str,
    pub status: ReportStatus,
    pub scanned_crates: usize,
    pub findings: Vec<Finding>,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReportStatus {
    Pass,
    Fail,
}

impl ReportStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Fail => "fail",
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Finding {
    pub rule_id: RuleId,
    pub kind: String,
    pub message: String,
    pub owner_ids: Vec<OwnerId>,
    pub node_ids: Vec<NodeId>,
}

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
    #[error("failed to analyze runtime findings for root `{}`: {source:#}", root.display())]
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
) -> std::result::Result<FindingsReport, RuntimeError> {
    let findings = runtime::analyze_runtime_liveness(&options.root).map_err(|source| {
        RuntimeError::AnalyzeFindings {
            root: options.root.clone(),
            source,
        }
    })?;
    let scanned_crates = source_scan::count_scanned_crates(&options.root).map_err(|source| {
        RuntimeError::CountScannedCrates {
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
