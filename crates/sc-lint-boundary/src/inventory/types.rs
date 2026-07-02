use std::collections::BTreeMap;
use std::fmt;
use std::ops::Deref;

use serde::Deserialize;

use super::dependency_policy::PackageDependencyPolicy;
use super::dependency_policy::RawDependenciesSection;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum InventoryParseError {
    BoundaryId(String),
    SprintId(String),
    TrackingId(String),
    OwnerPackage(String),
    OwnerCratePath(String),
    PlanningKey(String),
    ApprovedSymbol(String),
    ApprovedCaller(String),
}

impl InventoryParseError {
    fn boundary_id(detail: impl Into<String>) -> Self {
        Self::BoundaryId(detail.into())
    }

    fn sprint_id(detail: impl Into<String>) -> Self {
        Self::SprintId(detail.into())
    }

    fn tracking_id(detail: impl Into<String>) -> Self {
        Self::TrackingId(detail.into())
    }

    fn owner_package(detail: impl Into<String>) -> Self {
        Self::OwnerPackage(detail.into())
    }

    fn owner_crate_path(detail: impl Into<String>) -> Self {
        Self::OwnerCratePath(detail.into())
    }

    fn planning_key(detail: impl Into<String>) -> Self {
        Self::PlanningKey(detail.into())
    }

    fn approved_symbol(detail: impl Into<String>) -> Self {
        Self::ApprovedSymbol(detail.into())
    }

    fn approved_caller(detail: impl Into<String>) -> Self {
        Self::ApprovedCaller(detail.into())
    }
}

impl fmt::Display for InventoryParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BoundaryId(detail)
            | Self::SprintId(detail)
            | Self::TrackingId(detail)
            | Self::OwnerPackage(detail)
            | Self::OwnerCratePath(detail)
            | Self::PlanningKey(detail)
            | Self::ApprovedSymbol(detail)
            | Self::ApprovedCaller(detail) => formatter.write_str(detail),
        }
    }
}

impl std::error::Error for InventoryParseError {}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(try_from = "String")]
pub(crate) struct BoundaryId(String);

impl BoundaryId {
    fn parse(value: String) -> std::result::Result<Self, InventoryParseError> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(InventoryParseError::boundary_id(
                "boundary ids must not be empty",
            ));
        }
        if !trimmed.starts_with("BOUNDARY-") {
            return Err(InventoryParseError::boundary_id(format!(
                "boundary ids must start with `BOUNDARY-` (got `{trimmed}`)"
            )));
        }
        Ok(Self(trimmed.to_string()))
    }

    fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for BoundaryId {
    type Error = InventoryParseError;

    fn try_from(value: String) -> std::result::Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl AsRef<str> for BoundaryId {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Deref for BoundaryId {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl fmt::Display for BoundaryId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl PartialEq<&str> for BoundaryId {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(try_from = "String")]
pub(crate) struct SprintId(String);

impl SprintId {
    fn parse(value: String) -> std::result::Result<Self, InventoryParseError> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(InventoryParseError::sprint_id(
                "sprint ids must not be empty",
            ));
        }
        let Some((phase, step)) = trimmed.split_once('.') else {
            return Err(InventoryParseError::sprint_id(format!(
                "sprint ids must use <phase>.<step> format (got `{trimmed}`)"
            )));
        };
        if phase.is_empty() || step.is_empty() || trimmed.contains(char::is_whitespace) {
            return Err(InventoryParseError::sprint_id(format!(
                "invalid sprint id `{trimmed}`"
            )));
        }
        Ok(Self(trimmed.to_string()))
    }

    pub(super) fn placeholder_empty_inventory() -> Self {
        Self("A.0".to_string())
    }

    fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for SprintId {
    type Error = InventoryParseError;

    fn try_from(value: String) -> std::result::Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl AsRef<str> for SprintId {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Deref for SprintId {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl fmt::Display for SprintId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl PartialEq<&str> for SprintId {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(try_from = "String")]
pub(crate) struct TrackingId(String);

impl TrackingId {
    fn parse(value: String) -> std::result::Result<Self, InventoryParseError> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(InventoryParseError::tracking_id(
                "tracking ids must not be empty",
            ));
        }
        if trimmed.contains(char::is_whitespace) {
            return Err(InventoryParseError::tracking_id(format!(
                "tracking ids must not contain whitespace (got `{trimmed}`)"
            )));
        }
        Ok(Self(trimmed.to_string()))
    }
}

impl TryFrom<String> for TrackingId {
    type Error = InventoryParseError;

    fn try_from(value: String) -> std::result::Result<Self, Self::Error> {
        Self::parse(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(try_from = "String")]
pub(crate) struct OwnerPackage(String);

impl OwnerPackage {
    fn parse(value: String) -> std::result::Result<Self, InventoryParseError> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(InventoryParseError::owner_package(
                "owner packages must not be empty",
            ));
        }
        Ok(Self(trimmed.to_string()))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for OwnerPackage {
    type Error = InventoryParseError;

    fn try_from(value: String) -> std::result::Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl AsRef<str> for OwnerPackage {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Deref for OwnerPackage {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl fmt::Display for OwnerPackage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl PartialEq<&str> for OwnerPackage {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(try_from = "String")]
pub(crate) struct OwnerCratePath(String);

impl OwnerCratePath {
    fn parse(value: String) -> std::result::Result<Self, InventoryParseError> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(InventoryParseError::owner_crate_path(
                "owner crate paths must not be empty",
            ));
        }
        Ok(Self(trimmed.to_string()))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for OwnerCratePath {
    type Error = InventoryParseError;

    fn try_from(value: String) -> std::result::Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl AsRef<str> for OwnerCratePath {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Deref for OwnerCratePath {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl fmt::Display for OwnerCratePath {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl PartialEq<&str> for OwnerCratePath {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(try_from = "String")]
pub(crate) struct PlanningKey(String);

impl PlanningKey {
    fn parse(value: String) -> std::result::Result<Self, InventoryParseError> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(InventoryParseError::planning_key(
                "planning keys must not be empty",
            ));
        }
        if !trimmed.starts_with("BOUNDARY-") || !trimmed.contains('.') {
            return Err(InventoryParseError::planning_key(format!(
                "planning keys must use <boundary_id>.<section>.<field>[.<subfield>] shape (got `{trimmed}`)"
            )));
        }
        Ok(Self(trimmed.to_string()))
    }

    fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for PlanningKey {
    type Error = InventoryParseError;

    fn try_from(value: String) -> std::result::Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl AsRef<str> for PlanningKey {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Deref for PlanningKey {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl fmt::Display for PlanningKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl PartialEq<&str> for PlanningKey {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BoundaryInventory {
    pub(crate) records: Vec<BoundaryRecord>,
    pub(crate) planning: PlanningMetadata,
}

impl BoundaryInventory {
    pub(crate) fn summary(&self) -> InventorySummary {
        InventorySummary {
            boundary_count: self.records.len(),
            planned_item_count: self.planning.planned_items.len(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct InventorySummary {
    pub(crate) boundary_count: usize,
    pub(crate) planned_item_count: usize,
}

impl InventorySummary {
    pub(crate) fn recommended_finding_capacity(self) -> usize {
        self.boundary_count
            .saturating_add(self.planned_item_count)
            .max(8)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawBoundaryRecord {
    pub(crate) boundary_id: BoundaryId,
    pub(crate) owner_package: OwnerPackage,
    pub(crate) owner_crate_path: OwnerCratePath,
    pub(crate) name: String,
    pub(crate) public: PublicSection,
    pub(crate) implementation: ImplementationSection,
    pub(crate) composition: CompositionSection,
    pub(crate) callers: Option<CallersSection>,
    pub(crate) dependencies: RawDependenciesSection,
    pub(crate) references: ReferencesSection,
    pub(crate) testing: TestingSection,
    pub(crate) enforcement: EnforcementSection,
    pub(crate) status: StatusSection,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BoundaryRecord {
    pub(crate) boundary_id: BoundaryId,
    pub(crate) owner_package: OwnerPackage,
    pub(crate) owner_crate_path: OwnerCratePath,
    pub(crate) name: String,
    pub(crate) public: PublicSection,
    pub(crate) implementation: ImplementationSection,
    pub(crate) composition: CompositionSection,
    pub(crate) callers: Option<CallersSection>,
    pub(crate) dependencies: PackageDependencyPolicy,
    pub(crate) references: ReferencesSection,
    pub(crate) testing: TestingSection,
    pub(crate) enforcement: EnforcementSection,
    pub(crate) status: StatusSection,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PublicSection {
    pub(crate) facade: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ImplementationSection {
    #[serde(rename = "type")]
    pub(crate) implementation_type: Option<String>,
    pub(crate) module: Option<String>,
    pub(crate) visibility: Visibility,
    pub(crate) constructor: Option<Constructor>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CompositionSection {
    pub(crate) roots: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CallersSection {
    pub(crate) approved: Vec<ApprovedCallerEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ApprovedCallerEntry {
    pub(crate) symbol: ApprovedSymbol,
    pub(crate) callers: Vec<ApprovedCaller>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReferencesSection {
    pub(crate) scope: ReferenceScope,
    pub(crate) forbidden: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TestingSection {
    pub(crate) allowed_test_double_paths: Vec<String>,
    pub(crate) forbidden_test_bypasses: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct EnforcementSection {
    pub(crate) lint_rules: Vec<String>,
    pub(crate) review_gates: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct StatusSection {
    pub(crate) state: BoundaryState,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PlanningMetadata {
    pub(crate) planning: PlanningHeader,
    #[serde(default)]
    pub(crate) planned_items: BTreeMap<PlanningKey, PlannedItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PlanningHeader {
    pub(crate) current_sprint: SprintId,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PlannedItem {
    pub(crate) scheduled_sprint: SprintId,
    pub(crate) tracking_id: TrackingId,
    pub(crate) expires_when: ExpirationPolicy,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Visibility {
    Public,
    TraitOnly,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Constructor {
    None,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ReferenceScope {
    OutsideOwnerCrate,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum BoundaryState {
    Planned,
    ConcreteLanded,
    ReservedFuture,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(try_from = "String")]
pub(crate) struct ApprovedSymbol(String);

impl ApprovedSymbol {
    fn parse(value: String) -> std::result::Result<Self, InventoryParseError> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(InventoryParseError::approved_symbol(
                "approved symbol must not be empty",
            ));
        }
        validate_rust_like_path(trimmed, InventoryParseError::approved_symbol)?;
        Ok(Self(trimmed.to_string()))
    }

    pub(crate) fn normalized(&self) -> &str {
        self.0.strip_prefix("crate::").unwrap_or(&self.0)
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for ApprovedSymbol {
    type Error = InventoryParseError;

    fn try_from(value: String) -> std::result::Result<Self, Self::Error> {
        Self::parse(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(try_from = "String")]
pub(crate) struct ApprovedCaller(String);

impl ApprovedCaller {
    fn parse(value: String) -> std::result::Result<Self, InventoryParseError> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(InventoryParseError::approved_caller(
                "approved caller must not be empty",
            ));
        }
        validate_rust_like_path(trimmed, InventoryParseError::approved_caller)?;
        if trimmed.split("::").count() < 2 {
            return Err(InventoryParseError::approved_caller(format!(
                "approved caller must include crate path plus owner path (got `{trimmed}`)"
            )));
        }
        Ok(Self(trimmed.to_string()))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for ApprovedCaller {
    type Error = InventoryParseError;

    fn try_from(value: String) -> std::result::Result<Self, Self::Error> {
        Self::parse(value)
    }
}

fn validate_rust_like_path(
    value: &str,
    error: impl FnOnce(String) -> InventoryParseError + Copy,
) -> std::result::Result<(), InventoryParseError> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.contains(char::is_whitespace) {
        return Err(error(format!(
            "path `{value}` must not be empty or contain whitespace"
        )));
    }
    let trimmed = trimmed.strip_prefix("crate::").unwrap_or(trimmed);
    for segment in trimmed.split("::") {
        if segment.is_empty() {
            return Err(error(format!(
                "path `{value}` contains an empty path segment"
            )));
        }
        let mut chars = segment.chars();
        let Some(first) = chars.next() else {
            return Err(error(format!(
                "path `{value}` contains an empty path segment"
            )));
        };
        if !(first == '_' || first.is_ascii_alphabetic()) {
            return Err(error(format!(
                "path `{value}` has invalid path segment `{segment}`"
            )));
        }
        if !chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric()) {
            return Err(error(format!(
                "path `{value}` has invalid path segment `{segment}`"
            )));
        }
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ExpirationPolicy {
    SprintBeforeCurrent,
}
