use std::fmt;
use std::ops::Deref;

use serde::Serialize;

mod adapter_payload;
mod analyzer_report;

pub use adapter_payload::{PayloadDrift, validate_adapter_payload};
pub use analyzer_report::{
    AnalyzerReportDrift, analyzer_report_schema, analyzer_report_v1_schema,
    validate_analyzer_report,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
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

impl From<String> for NodeId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for NodeId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl PartialEq<&str> for NodeId {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl PartialEq<String> for NodeId {
    fn eq(&self, other: &String) -> bool {
        self.as_str() == other
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

impl From<String> for OwnerId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for OwnerId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl PartialEq<&str> for OwnerId {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl PartialEq<String> for OwnerId {
    fn eq(&self, other: &String) -> bool {
        self.as_str() == other
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(transparent)]
pub struct CrateId(String);

impl CrateId {
    pub fn new(value: impl Into<String>) -> Self {
        let value = value.into();
        debug_assert!(!value.is_empty(), "crate ids must not be empty");
        Self(value)
    }

    pub fn from_parts(package_name: &str, target_name: &str) -> Self {
        Self::new(format!("crate::{package_name}::{target_name}"))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for CrateId {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Deref for CrateId {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl fmt::Display for CrateId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl From<String> for CrateId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for CrateId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<CrateId> for String {
    fn from(value: CrateId) -> Self {
        value.0
    }
}

impl From<&CrateId> for String {
    fn from(value: &CrateId) -> Self {
        value.0.clone()
    }
}

impl From<CrateId> for NodeId {
    fn from(value: CrateId) -> Self {
        Self::new(String::from(value))
    }
}

impl From<&CrateId> for NodeId {
    fn from(value: &CrateId) -> Self {
        Self::new(String::from(value))
    }
}

impl PartialEq<&str> for CrateId {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl PartialEq<String> for CrateId {
    fn eq(&self, other: &String) -> bool {
        self.as_str() == other
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FindingsReport<R>
where
    R: Serialize,
{
    pub tool: &'static str,
    pub version: &'static str,
    pub schema_version: &'static str,
    pub status: ReportStatus,
    pub scanned_crates: usize,
    pub findings: Vec<Finding<R>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub allowed: Vec<AllowedFinding<R>>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[non_exhaustive]
pub struct AllowedFinding<R>
where
    R: Serialize,
{
    rule_id: R,
    reason: String,
    item: String,
    source_path: String,
    line: usize,
}

impl<R> AllowedFinding<R>
where
    R: Serialize,
{
    pub fn new(
        rule_id: R,
        reason: impl Into<String>,
        item: impl Into<String>,
        source_path: impl Into<String>,
        line: usize,
    ) -> Result<Self, AllowedFindingError> {
        let reason = reason.into();
        let item = item.into();
        let source_path = source_path.into();
        if reason.trim().is_empty() {
            return Err(AllowedFindingError::EmptyReason);
        }
        if item.trim().is_empty() {
            return Err(AllowedFindingError::EmptyItem);
        }
        if source_path.trim().is_empty() {
            return Err(AllowedFindingError::EmptySourcePath);
        }
        if line == 0 {
            return Err(AllowedFindingError::InvalidLine);
        }
        Ok(Self {
            rule_id,
            reason,
            item,
            source_path,
            line,
        })
    }

    pub fn rule_id(&self) -> &R {
        &self.rule_id
    }

    pub fn reason(&self) -> &str {
        &self.reason
    }

    pub fn item(&self) -> &str {
        &self.item
    }

    pub fn source_path(&self) -> &str {
        &self.source_path
    }

    pub fn line(&self) -> usize {
        self.line
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllowedFindingError {
    EmptyReason,
    EmptyItem,
    EmptySourcePath,
    InvalidLine,
}

impl fmt::Display for AllowedFindingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::EmptyReason => "allowed finding reason must not be empty",
            Self::EmptyItem => "allowed finding item must not be empty",
            Self::EmptySourcePath => "allowed finding source path must not be empty",
            Self::InvalidLine => "allowed finding line must be one-based",
        })
    }
}

impl std::error::Error for AllowedFindingError {}

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct FindingKind(String);

impl FindingKind {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for FindingKind {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl From<&str> for FindingKind {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for FindingKind {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl fmt::Display for FindingKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl PartialEq<&str> for FindingKind {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Finding<R>
where
    R: Serialize,
{
    pub rule_id: R,
    pub kind: FindingKind,
    pub message: String,
    pub owner_ids: Vec<OwnerId>,
    pub node_ids: Vec<NodeId>,
}
