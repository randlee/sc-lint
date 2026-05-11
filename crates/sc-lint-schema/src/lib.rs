use std::fmt;
use std::ops::Deref;

use serde::Serialize;

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Finding<R>
where
    R: Serialize,
{
    pub rule_id: R,
    pub kind: String,
    pub message: String,
    pub owner_ids: Vec<OwnerId>,
    pub node_ids: Vec<NodeId>,
}
