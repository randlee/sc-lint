use std::collections::BTreeSet;
use std::fmt;

use serde::Deserialize;
use thiserror::Error;

use super::types::BoundaryId;
use super::types::BoundaryRecord;
use super::types::RawBoundaryRecord;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawForbiddenPackageEdge {
    pub(crate) from: String,
    pub(crate) to: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawDependenciesSection {
    pub(crate) allowed_dependents: Vec<String>,
    pub(crate) allowed_dependencies: Vec<String>,
    pub(crate) forbidden_edges: Vec<RawForbiddenPackageEdge>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct WorkspacePackageName(String);

impl WorkspacePackageName {
    pub(crate) fn from_package_name(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for WorkspacePackageName {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for WorkspacePackageName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_ref())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct ForbiddenPackageEdge {
    pub(crate) from: WorkspacePackageName,
    pub(crate) to: WorkspacePackageName,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PackageDependencyPolicy {
    pub(crate) allowed_dependents: BTreeSet<WorkspacePackageName>,
    pub(crate) allowed_dependencies: BTreeSet<WorkspacePackageName>,
    pub(crate) forbidden_edges: Vec<ForbiddenPackageEdge>,
}

#[derive(Debug, Error)]
pub(crate) enum DependencyPolicyError {
    #[error(
        "invalid workspace package name {value:?} in `{field}` for boundary `{boundary_id}`: package names must not be empty or contain whitespace"
    )]
    InvalidFormat {
        boundary_id: BoundaryId,
        field: &'static str,
        value: String,
    },
    #[error("duplicate forbidden edge `{from} -> {to}` in boundary `{boundary_id}`")]
    DuplicateEdge {
        boundary_id: BoundaryId,
        from: WorkspacePackageName,
        to: WorkspacePackageName,
    },
    #[error("duplicate package `{package}` in `{field}` for boundary `{boundary_id}`")]
    DuplicatePackage {
        boundary_id: BoundaryId,
        field: &'static str,
        package: WorkspacePackageName,
    },
}

impl RawDependenciesSection {
    pub(crate) fn validate(
        self,
        boundary_id: &BoundaryId,
    ) -> std::result::Result<PackageDependencyPolicy, DependencyPolicyError> {
        let allowed_dependents =
            validate_package_list(boundary_id, "allowed_dependents", self.allowed_dependents)?;
        let allowed_dependencies = validate_package_list(
            boundary_id,
            "allowed_dependencies",
            self.allowed_dependencies,
        )?;

        let mut forbidden_edges = Vec::with_capacity(self.forbidden_edges.len());
        let mut seen_edges = BTreeSet::new();
        for raw_edge in self.forbidden_edges {
            let from =
                parse_workspace_package_name(raw_edge.from, boundary_id, "forbidden_edges[].from")?;
            let to =
                parse_workspace_package_name(raw_edge.to, boundary_id, "forbidden_edges[].to")?;
            let edge = ForbiddenPackageEdge {
                from: from.clone(),
                to: to.clone(),
            };
            if !seen_edges.insert(edge.clone()) {
                return Err(DependencyPolicyError::DuplicateEdge {
                    boundary_id: boundary_id.clone(),
                    from,
                    to,
                });
            }
            forbidden_edges.push(edge);
        }

        Ok(PackageDependencyPolicy {
            allowed_dependents,
            allowed_dependencies,
            forbidden_edges,
        })
    }
}

fn validate_package_list(
    boundary_id: &BoundaryId,
    field: &'static str,
    packages: Vec<String>,
) -> std::result::Result<BTreeSet<WorkspacePackageName>, DependencyPolicyError> {
    let mut validated = BTreeSet::new();
    for package in packages {
        let package = parse_workspace_package_name(package, boundary_id, field)?;
        if !validated.insert(package.clone()) {
            return Err(DependencyPolicyError::DuplicatePackage {
                boundary_id: boundary_id.clone(),
                field,
                package,
            });
        }
    }
    Ok(validated)
}

fn parse_workspace_package_name(
    value: String,
    boundary_id: &BoundaryId,
    field: &'static str,
) -> std::result::Result<WorkspacePackageName, DependencyPolicyError> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.contains(char::is_whitespace) {
        return Err(DependencyPolicyError::InvalidFormat {
            boundary_id: boundary_id.clone(),
            field,
            value,
        });
    }
    Ok(WorkspacePackageName(trimmed.to_string()))
}

impl TryFrom<RawBoundaryRecord> for BoundaryRecord {
    type Error = DependencyPolicyError;

    fn try_from(value: RawBoundaryRecord) -> std::result::Result<Self, Self::Error> {
        Ok(Self {
            dependencies: value.dependencies.validate(&value.boundary_id)?,
            boundary_id: value.boundary_id,
            owner_package: value.owner_package,
            owner_crate_path: value.owner_crate_path,
            name: value.name,
            public: value.public,
            implementation: value.implementation,
            composition: value.composition,
            callers: value.callers,
            references: value.references,
            testing: value.testing,
            enforcement: value.enforcement,
            status: value.status,
        })
    }
}
