use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fmt;
use std::path::PathBuf;

use crate::Finding;
use crate::OwnerId;
use crate::RuleId;

use super::BoundaryInventory;
use super::BoundaryRecord;
use super::WorkspacePackageResolver;
use super::dependency_policy::WorkspacePackageName;
use super::types::Constructor;
use super::types::Visibility;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct RustIdent(String);

impl RustIdent {
    pub(crate) fn parse(value: &str) -> Result<Self, SemanticPathError> {
        if value.trim() != value || value.is_empty() {
            return Err(SemanticPathError::InvalidIdentifier(value.to_owned()));
        }
        syn::parse_str::<syn::Ident>(value)
            .map(|_| Self(value.to_owned()))
            .map_err(|_| SemanticPathError::InvalidIdentifier(value.to_owned()))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct PublicRustPath(Vec<RustIdent>);

impl PublicRustPath {
    pub(crate) fn parse(value: &str) -> Result<Self, SemanticPathError> {
        if value.trim() != value || value.is_empty() || value.split("::").any(str::is_empty) {
            return Err(SemanticPathError::InvalidPublicPath(value.to_owned()));
        }
        let segments = value
            .split("::")
            .map(RustIdent::parse)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self(segments))
    }

    pub(crate) fn first(&self) -> &RustIdent {
        self.0
            .first()
            .expect("validated public paths have a segment")
    }

    pub(crate) fn as_string(&self) -> String {
        self.0
            .iter()
            .map(RustIdent::as_str)
            .collect::<Vec<_>>()
            .join("::")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SemanticPathError {
    InvalidIdentifier(String),
    InvalidPublicPath(String),
}

impl fmt::Display for SemanticPathError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidIdentifier(value) => {
                write!(formatter, "invalid Rust identifier `{value}`")
            }
            Self::InvalidPublicPath(value) => {
                write!(formatter, "invalid public Rust path `{value}`")
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum InventoryDiagnosticKind {
    InvalidIdentifier,
    InvalidPublicPath,
    TraitOnlyConstructor,
    SelfForbiddenEdge,
    DuplicateOwnerName,
    UnapprovedCompositionRoot,
}

impl InventoryDiagnosticKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::InvalidIdentifier => "invalid_identifier",
            Self::InvalidPublicPath => "invalid_public_path",
            Self::TraitOnlyConstructor => "trait_only_constructor",
            Self::SelfForbiddenEdge => "self_forbidden_edge",
            Self::DuplicateOwnerName => "duplicate_owner_name",
            Self::UnapprovedCompositionRoot => "unapproved_composition_root",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InventoryDiagnostic {
    record_path: PathBuf,
    boundary_id: String,
    field: String,
    kind: InventoryDiagnosticKind,
    cause: String,
    recovery: String,
}

impl InventoryDiagnostic {
    fn new(
        record: &BoundaryRecord,
        field: impl Into<String>,
        kind: InventoryDiagnosticKind,
        cause: impl Into<String>,
        recovery: impl Into<String>,
    ) -> Self {
        Self {
            record_path: record.record_path.clone(),
            boundary_id: record.boundary_id.to_string(),
            field: field.into(),
            kind,
            cause: cause.into(),
            recovery: recovery.into(),
        }
    }

    pub(crate) fn finding(&self) -> Finding {
        Finding {
            rule_id: RuleId::ScbBoundary004,
            kind: self.kind.as_str().into(),
            message: format!(
                "boundary `{}` record `{}` field `{}`: {}; recovery: {}",
                self.boundary_id,
                self.record_path.display(),
                self.field,
                self.cause,
                self.recovery,
            ),
            owner_ids: vec![OwnerId::new(self.boundary_id.clone())],
            node_ids: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ValidatedRecord {
    record: BoundaryRecord,
    #[expect(
        dead_code,
        reason = "Retained as the typed semantic contract for graph and policy passes."
    )]
    public_surfaces: Vec<PublicRustPath>,
    #[expect(
        dead_code,
        reason = "Retained as the typed semantic contract for graph and policy passes."
    )]
    composition_roots: Vec<PublicRustPath>,
    #[expect(
        dead_code,
        reason = "The canonical identity is reserved for package policy consumers."
    )]
    owner: Option<WorkspacePackageName>,
}

#[derive(Debug, Clone)]
pub(crate) struct ValidatedInventory {
    records: Vec<ValidatedRecord>,
    planning: super::types::PlanningMetadata,
}

impl ValidatedInventory {
    pub(crate) fn records(&self) -> impl Iterator<Item = &BoundaryRecord> {
        self.records.iter().map(|record| &record.record)
    }

    pub(crate) fn as_raw_inventory(&self) -> BoundaryInventory {
        BoundaryInventory {
            records: self.records().cloned().collect(),
            planning: self.planning.clone(),
        }
    }

    pub(crate) fn summary(&self) -> super::types::InventorySummary {
        super::types::InventorySummary {
            boundary_count: self.records.len(),
            planned_item_count: self.planning.planned_items.len(),
        }
    }
}

pub(crate) fn validate_inventory(
    raw: BoundaryInventory,
    resolver: &WorkspacePackageResolver,
) -> (ValidatedInventory, Vec<InventoryDiagnostic>) {
    let mut diagnostics = Vec::new();
    let mut records = Vec::<ValidatedRecord>::new();
    let mut owner_names = BTreeMap::<(WorkspacePackageName, String), String>::new();
    let mut ambiguous_boundary_ids = BTreeSet::new();

    for record in raw.records {
        let mut valid = true;
        let public_value = record
            .public
            .facade
            .as_deref()
            .or(record.public.trait_name.as_deref())
            .expect("structural loader validation requires a public surface");
        let public_surfaces = match parse_public_surfaces(public_value) {
            Ok(paths) => paths,
            Err(error) => {
                diagnostics.push(path_diagnostic(&record, "public", error));
                valid = false;
                Vec::new()
            }
        };

        let owner_context = format!("owner_package for boundary `{}`", record.boundary_id);
        let owner = resolver
            .resolve_if_known(record.owner_package.as_ref(), &owner_context)
            .ok()
            .flatten();
        if let Some(owner) = &owner {
            let key = (owner.clone(), record.name.clone());
            if let Some(previous) = owner_names.get(&key).cloned() {
                ambiguous_boundary_ids.insert(previous.clone());
                ambiguous_boundary_ids.insert(record.boundary_id.to_string());
                if let Some(previous_record) = records.iter().find(|previous_record| {
                    previous_record.record.boundary_id.to_string() == previous
                }) {
                    diagnostics.push(InventoryDiagnostic::new(
                        &previous_record.record,
                        "name",
                        InventoryDiagnosticKind::DuplicateOwnerName,
                        format!(
                            "canonical owner `{owner}` and name `{}` are also declared by boundary `{}`",
                            previous_record.record.name, record.boundary_id
                        ),
                        "give each boundary for an owner a distinct name",
                    ));
                }
                diagnostics.push(InventoryDiagnostic::new(
                    &record,
                    "name",
                    InventoryDiagnosticKind::DuplicateOwnerName,
                    format!(
                        "canonical owner `{owner}` and name `{}` are already declared by boundary `{previous}`",
                        record.name
                    ),
                    "give each boundary for an owner a distinct name",
                ));
                valid = false;
            } else {
                owner_names.insert(key, record.boundary_id.to_string());
            }
        }

        if record.implementation.visibility == Visibility::TraitOnly
            && matches!(
                record.implementation.constructor,
                Some(Constructor::Public | Constructor::Private | Constructor::PubCrate)
            )
        {
            diagnostics.push(InventoryDiagnostic::new(
                &record,
                "implementation.constructor",
                InventoryDiagnosticKind::TraitOnlyConstructor,
                "trait_only boundaries cannot declare a concrete constructor",
                "omit implementation.constructor or set it to `none`",
            ));
            valid = false;
        }

        let mut composition_roots = Vec::new();
        for (index, root) in record.composition.roots.iter().enumerate() {
            match PublicRustPath::parse(root) {
                Ok(path) => {
                    validate_external_root(
                        &record,
                        &path,
                        resolver,
                        index,
                        &mut diagnostics,
                        &mut valid,
                    );
                    composition_roots.push(path);
                }
                Err(error) => {
                    diagnostics.push(path_diagnostic(
                        &record,
                        format!("composition.roots[{index}]"),
                        error,
                    ));
                    valid = false;
                }
            }
        }

        for edge in &record.dependencies.forbidden_edges {
            let from_context = format!(
                "dependencies.forbidden_edges[].from for boundary `{}`",
                record.boundary_id
            );
            let to_context = format!(
                "dependencies.forbidden_edges[].to for boundary `{}`",
                record.boundary_id
            );
            let from = resolver.resolve_if_known(edge.from.as_ref(), &from_context);
            let to = resolver.resolve_if_known(edge.to.as_ref(), &to_context);
            if matches!((from, to), (Ok(Some(from)), Ok(Some(to))) if from == to) {
                diagnostics.push(InventoryDiagnostic::new(
                    &record,
                    "dependencies.forbidden_edges",
                    InventoryDiagnosticKind::SelfForbiddenEdge,
                    format!(
                        "forbidden edge `{}` -> `{}` resolves to the same workspace package",
                        edge.from, edge.to
                    ),
                    "remove the self edge or name two distinct workspace packages",
                ));
                valid = false;
            }
        }

        if valid {
            records.push(ValidatedRecord {
                record,
                public_surfaces,
                composition_roots,
                owner,
            });
        }
    }

    records
        .retain(|record| !ambiguous_boundary_ids.contains(&record.record.boundary_id.to_string()));

    diagnostics.sort_by(|left, right| {
        left.record_path
            .cmp(&right.record_path)
            .then_with(|| left.field.cmp(&right.field))
            .then_with(|| left.kind.cmp(&right.kind))
            .then_with(|| left.cause.cmp(&right.cause))
    });
    (
        ValidatedInventory {
            records,
            planning: raw.planning,
        },
        diagnostics,
    )
}

fn parse_public_surfaces(value: &str) -> Result<Vec<PublicRustPath>, SemanticPathError> {
    value
        .split(',')
        .map(str::trim)
        .map(PublicRustPath::parse)
        .collect()
}

fn path_diagnostic(
    record: &BoundaryRecord,
    field: impl Into<String>,
    error: SemanticPathError,
) -> InventoryDiagnostic {
    let kind = match error {
        SemanticPathError::InvalidIdentifier(_) => InventoryDiagnosticKind::InvalidIdentifier,
        SemanticPathError::InvalidPublicPath(_) => InventoryDiagnosticKind::InvalidPublicPath,
    };
    InventoryDiagnostic::new(
        record,
        field,
        kind,
        error.to_string(),
        "supply a valid Rust identifier or crate-qualified public path",
    )
}

fn validate_external_root(
    record: &BoundaryRecord,
    root: &PublicRustPath,
    resolver: &WorkspacePackageResolver,
    index: usize,
    diagnostics: &mut Vec<InventoryDiagnostic>,
    valid: &mut bool,
) {
    let root_context = format!("composition root for boundary `{}`", record.boundary_id);
    let root_package = resolver.resolve_if_known(root.first().as_str(), &root_context);
    let Ok(Some(root_package)) = root_package else {
        return;
    };
    let owner_context = format!("owner_package for boundary `{}`", record.boundary_id);
    let owner_is_local = resolver
        .resolve_if_known(record.owner_package.as_ref(), &owner_context)
        .ok()
        .flatten()
        .is_some_and(|owner| owner == root_package);
    if owner_is_local
        || record
            .dependencies
            .allowed_dependencies
            .contains(&root_package)
    {
        return;
    }
    diagnostics.push(InventoryDiagnostic::new(
        record,
        format!("composition.roots[{index}]"),
        InventoryDiagnosticKind::UnapprovedCompositionRoot,
        format!(
            "external composition root `{}` resolves to `{root_package}`, which is not in dependencies.allowed_dependencies",
            root.as_string()
        ),
        "add the external package to dependencies.allowed_dependencies or keep the root owner-local",
    ));
    *valid = false;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_raw_identifiers_and_public_paths() {
        assert_eq!(RustIdent::parse("r#type").unwrap().as_str(), "r#type");
        assert_eq!(
            PublicRustPath::parse("api::r#type").unwrap().as_string(),
            "api::r#type"
        );
    }

    #[test]
    fn rejects_invalid_path_segments() {
        assert!(PublicRustPath::parse("api::").is_err());
        assert!(RustIdent::parse("not-valid").is_err());
    }
}
