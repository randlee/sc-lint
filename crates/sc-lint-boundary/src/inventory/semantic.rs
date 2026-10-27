use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::path::Path;

use anyhow::Result;
use std::fmt;
use std::path::PathBuf;

use crate::Finding;
use crate::OwnerId;
use crate::RuleId;

use super::BoundaryInventory;
use super::BoundaryRecord;
use super::FeatureIndependence;
use super::WorkspacePackageResolver;
use super::dependency_policy::PackageRef;
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
    public_surfaces: Vec<PublicRustPath>,
    composition_roots: Vec<PublicRustPath>,
    owner: Option<WorkspacePackageName>,
    policy: Option<ValidatedDependencyPolicy>,
}

#[derive(Debug, Clone)]
pub(crate) struct ValidatedDependencyPolicy {
    pub(crate) owner: WorkspacePackageName,
    pub(crate) allowed_dependencies: BTreeSet<WorkspacePackageName>,
    pub(crate) allowed_dev_dependencies: BTreeSet<WorkspacePackageName>,
    pub(crate) allowed_feature_dependencies: FeatureIndependence<BTreeSet<WorkspacePackageName>>,
    pub(crate) allowed_dependents: BTreeSet<WorkspacePackageName>,
    pub(crate) allowed_dev_dependents: Option<BTreeSet<WorkspacePackageName>>,
    pub(crate) forbidden_edges: BTreeSet<ValidatedForbiddenPackageEdge>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ValidatedForbiddenPackageEdge {
    pub(crate) from: WorkspacePackageName,
    pub(crate) to: WorkspacePackageName,
}

struct ValidationContext<'a> {
    resolver: &'a WorkspacePackageResolver,
    known_non_workspace_packages: &'a BTreeSet<String>,
    policy_owners: &'a BTreeSet<WorkspacePackageName>,
    validate_package_policy: bool,
}

impl ValidatedRecord {
    pub(crate) fn boundary_id(&self) -> &str {
        self.record.boundary_id.as_ref()
    }

    pub(crate) fn record_path(&self) -> &Path {
        &self.record.record_path
    }

    pub(crate) fn public_surfaces(&self) -> &[PublicRustPath] {
        &self.public_surfaces
    }

    pub(crate) fn composition_roots(&self) -> &[PublicRustPath] {
        &self.composition_roots
    }

    pub(crate) fn owner(&self) -> Option<&WorkspacePackageName> {
        self.owner.as_ref()
    }

    pub(crate) fn policy(&self) -> Option<&ValidatedDependencyPolicy> {
        self.policy.as_ref()
    }

    pub(crate) fn requires_workspace_package(&self) -> bool {
        self.record.status.state.requires_workspace_package()
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ValidatedInventory {
    records: Vec<ValidatedRecord>,
    planning: super::types::PlanningMetadata,
}

impl ValidatedInventory {
    pub(crate) fn records(&self) -> impl Iterator<Item = &ValidatedRecord> {
        self.records.iter()
    }

    /// Adapts the validated inventory to graph passes that still require the legacy raw shape.
    pub(crate) fn as_raw_inventory(&self) -> BoundaryInventory {
        for record in &self.records {
            debug_assert!(!record.public_surfaces().is_empty());
            debug_assert_eq!(
                record.composition_roots().len(),
                record.record.composition.roots.len()
            );
            debug_assert!(record.policy().is_none() || record.owner().is_some());
        }
        BoundaryInventory {
            records: self
                .records
                .iter()
                .map(|record| record.record.clone())
                .collect(),
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
    known_non_workspace_packages: &BTreeSet<String>,
    validate_package_policy: bool,
) -> Result<(ValidatedInventory, Vec<InventoryDiagnostic>)> {
    let policy_owners = raw
        .records
        .iter()
        .filter(|record| record.status.state.requires_workspace_package())
        .filter_map(|record| resolved_workspace_owner(record, resolver))
        .collect();
    let context = ValidationContext {
        resolver,
        known_non_workspace_packages,
        policy_owners: &policy_owners,
        validate_package_policy,
    };
    let mut diagnostics = Vec::new();
    let mut records = Vec::<ValidatedRecord>::new();
    let mut owner_names = BTreeMap::<(WorkspacePackageName, String), String>::new();
    let mut ambiguous_boundary_ids = BTreeSet::new();

    for record in raw.records {
        validate_record(
            record,
            &context,
            &mut owner_names,
            &mut ambiguous_boundary_ids,
            &mut records,
            &mut diagnostics,
        )?;
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
    Ok((
        ValidatedInventory {
            records,
            planning: raw.planning,
        },
        diagnostics,
    ))
}

fn validate_record(
    record: BoundaryRecord,
    context: &ValidationContext<'_>,
    owner_names: &mut BTreeMap<(WorkspacePackageName, String), String>,
    ambiguous_boundary_ids: &mut BTreeSet<String>,
    records: &mut Vec<ValidatedRecord>,
    diagnostics: &mut Vec<InventoryDiagnostic>,
) -> Result<()> {
    let diagnostic_count = diagnostics.len();
    let public_surfaces = validate_public_surfaces(&record, diagnostics);
    validate_implementation_names(&record, diagnostics);
    let owner = validate_owner_name(
        &record,
        context.resolver,
        owner_names,
        ambiguous_boundary_ids,
        records,
        diagnostics,
    );
    validate_trait_constructor(&record, diagnostics);
    let composition_roots = validate_composition_roots(&record, context.resolver, diagnostics);
    validate_forbidden_edges(&record, context.resolver, diagnostics);
    if diagnostics.len() == diagnostic_count {
        let policy = (context.validate_package_policy
            && owner
                .as_ref()
                .is_some_and(|owner| context.policy_owners.contains(owner)))
        .then(|| validate_dependency_policy(&record, context))
        .transpose()?;
        records.push(ValidatedRecord {
            record,
            public_surfaces,
            composition_roots,
            owner,
            policy,
        });
    }
    Ok(())
}

fn validate_dependency_policy(
    record: &BoundaryRecord,
    context: &ValidationContext<'_>,
) -> Result<ValidatedDependencyPolicy> {
    Ok(ValidatedDependencyPolicy {
        owner: resolve_policy_reference(
            context,
            record,
            "owner_package",
            record.owner_package.as_ref(),
        )?
        .expect("enforceable boundary owners must resolve to workspace packages"),
        allowed_dependencies: resolve_policy_patterns(
            context,
            record,
            "dependencies.allowed_dependencies",
            &record.dependencies.allowed_dependencies,
        )?,
        allowed_dev_dependencies: resolve_policy_patterns(
            context,
            record,
            "dependencies.allowed_dev_dependencies",
            &record.dependencies.allowed_dev_dependencies,
        )?,
        allowed_feature_dependencies: resolve_feature_patterns(context, record)?,
        allowed_dependents: resolve_policy_references(
            context,
            record,
            "dependencies.allowed_dependents",
            &record.dependencies.allowed_dependents,
        )?,
        allowed_dev_dependents: record
            .dependencies
            .allowed_dev_dependents
            .as_ref()
            .map(|references| {
                resolve_policy_references(
                    context,
                    record,
                    "dependencies.allowed_dev_dependents",
                    references,
                )
            })
            .transpose()?,
        forbidden_edges: record
            .dependencies
            .forbidden_edges
            .iter()
            .filter_map(|edge| {
                let from = resolve_policy_reference(
                    context,
                    record,
                    "dependencies.forbidden_edges[].from",
                    edge.from.as_ref(),
                );
                let to = resolve_policy_reference(
                    context,
                    record,
                    "dependencies.forbidden_edges[].to",
                    edge.to.as_ref(),
                );
                match (from, to) {
                    (Ok(Some(from)), Ok(Some(to))) => {
                        Some(Ok(ValidatedForbiddenPackageEdge { from, to }))
                    }
                    (Ok(_), Ok(_)) => None,
                    (Err(error), _) | (_, Err(error)) => Some(Err(error)),
                }
            })
            .collect::<Result<BTreeSet<_>>>()?,
    })
}

fn resolve_feature_patterns(
    context: &ValidationContext<'_>,
    record: &BoundaryRecord,
) -> Result<FeatureIndependence<BTreeSet<WorkspacePackageName>>> {
    match &record.dependencies.allowed_feature_dependencies {
        FeatureIndependence::NotChecked => Ok(FeatureIndependence::NotChecked),
        FeatureIndependence::Checked(references) => {
            Ok(FeatureIndependence::Checked(resolve_policy_patterns(
                context,
                record,
                "dependencies.allowed_feature_dependencies",
                references,
            )?))
        }
    }
}

fn resolve_policy_patterns(
    context: &ValidationContext<'_>,
    record: &BoundaryRecord,
    field: &str,
    references: &[PackageRef],
) -> Result<BTreeSet<WorkspacePackageName>> {
    let mut resolved = BTreeSet::new();
    for reference in references {
        match reference {
            PackageRef::Prefix(_) => {
                resolved.extend(context.resolver.packages_matching(reference).cloned())
            }
            PackageRef::Exact(package) => {
                if let Some(package) =
                    resolve_policy_reference(context, record, field, package.as_ref())?
                {
                    resolved.insert(package);
                }
            }
        }
    }
    Ok(resolved)
}

fn resolve_policy_references(
    context: &ValidationContext<'_>,
    record: &BoundaryRecord,
    field: &str,
    references: &BTreeSet<WorkspacePackageName>,
) -> Result<BTreeSet<WorkspacePackageName>> {
    let mut resolved = BTreeSet::new();
    for reference in references {
        if let Some(package) = resolve_policy_reference(context, record, field, reference.as_ref())?
        {
            resolved.insert(package);
        }
    }
    Ok(resolved)
}

fn resolve_policy_reference(
    context: &ValidationContext<'_>,
    record: &BoundaryRecord,
    field: &str,
    reference: &str,
) -> Result<Option<WorkspacePackageName>> {
    let reference_context = format!("`{field}` for boundary `{}`", record.boundary_id);
    match context
        .resolver
        .resolve_if_known(reference, &reference_context)?
    {
        Some(package) => Ok(Some(package)),
        None if context.known_non_workspace_packages.contains(reference) => {
            Ok(Some(WorkspacePackageName::from_package_name(reference)))
        }
        None => context
            .resolver
            .resolve(reference, reference_context)
            .map(Some),
    }
}

fn validate_public_surfaces(
    record: &BoundaryRecord,
    diagnostics: &mut Vec<InventoryDiagnostic>,
) -> Vec<PublicRustPath> {
    let public_value = record
        .public
        .facade
        .as_deref()
        .or(record.public.trait_name.as_deref())
        .expect("structural loader validation requires a public surface");
    match parse_public_surfaces(public_value) {
        Ok(paths) => paths,
        Err(error) => {
            diagnostics.push(path_diagnostic(record, "public", error));
            Vec::new()
        }
    }
}

fn validate_implementation_names(
    record: &BoundaryRecord,
    diagnostics: &mut Vec<InventoryDiagnostic>,
) {
    if let Some(Err(error)) = record
        .implementation
        .implementation_type
        .as_deref()
        .map(RustIdent::parse)
    {
        diagnostics.push(path_diagnostic(record, "implementation.type", error));
    }
    if let Some(Err(error)) = record
        .implementation
        .module
        .as_deref()
        .map(PublicRustPath::parse)
    {
        diagnostics.push(path_diagnostic(record, "implementation.module", error));
    }
}

fn validate_owner_name(
    record: &BoundaryRecord,
    resolver: &WorkspacePackageResolver,
    owner_names: &mut BTreeMap<(WorkspacePackageName, String), String>,
    ambiguous_boundary_ids: &mut BTreeSet<String>,
    records: &[ValidatedRecord],
    diagnostics: &mut Vec<InventoryDiagnostic>,
) -> Option<WorkspacePackageName> {
    let owner = resolved_workspace_owner(record, resolver);
    let owner = owner?;
    let key = (owner.clone(), record.name.clone());
    let Some(previous) = owner_names.get(&key).cloned() else {
        owner_names.insert(key, record.boundary_id.to_string());
        return Some(owner);
    };
    ambiguous_boundary_ids.insert(previous.clone());
    ambiguous_boundary_ids.insert(record.boundary_id.to_string());
    if let Some(previous_record) = records
        .iter()
        .find(|previous_record| previous_record.record.boundary_id.to_string() == previous)
    {
        diagnostics.push(duplicate_owner_name_diagnostic(
            &previous_record.record,
            &owner,
            record.boundary_id.as_ref(),
        ));
    }
    diagnostics.push(InventoryDiagnostic::new(
        record,
        "name",
        InventoryDiagnosticKind::DuplicateOwnerName,
        format!(
            "canonical owner `{owner}` and name `{}` are already declared by boundary `{previous}`",
            record.name
        ),
        "give each boundary for an owner a distinct name",
    ));
    Some(owner)
}

fn resolved_workspace_owner(
    record: &BoundaryRecord,
    resolver: &WorkspacePackageResolver,
) -> Option<WorkspacePackageName> {
    let context = format!("owner_package for boundary `{}`", record.boundary_id);
    resolver
        .resolve_if_known(record.owner_package.as_ref(), &context)
        .ok()
        .flatten()
}

fn duplicate_owner_name_diagnostic(
    record: &BoundaryRecord,
    owner: &WorkspacePackageName,
    duplicate: &str,
) -> InventoryDiagnostic {
    InventoryDiagnostic::new(
        record,
        "name",
        InventoryDiagnosticKind::DuplicateOwnerName,
        format!(
            "canonical owner `{owner}` and name `{}` are also declared by boundary `{duplicate}`",
            record.name
        ),
        "give each boundary for an owner a distinct name",
    )
}

fn validate_trait_constructor(record: &BoundaryRecord, diagnostics: &mut Vec<InventoryDiagnostic>) {
    if record.implementation.visibility == Visibility::TraitOnly
        && matches!(
            record.implementation.constructor,
            Some(Constructor::Public | Constructor::Private | Constructor::PubCrate)
        )
    {
        diagnostics.push(InventoryDiagnostic::new(
            record,
            "implementation.constructor",
            InventoryDiagnosticKind::TraitOnlyConstructor,
            "trait_only boundaries cannot declare a concrete constructor",
            "omit implementation.constructor or set it to `none`",
        ));
    }
}

fn validate_composition_roots(
    record: &BoundaryRecord,
    resolver: &WorkspacePackageResolver,
    diagnostics: &mut Vec<InventoryDiagnostic>,
) -> Vec<PublicRustPath> {
    let mut roots = Vec::new();
    for (index, root) in record.composition.roots.iter().enumerate() {
        match PublicRustPath::parse(root) {
            Ok(path) => {
                validate_external_root(record, &path, resolver, index, diagnostics);
                roots.push(path);
            }
            Err(error) => diagnostics.push(path_diagnostic(
                record,
                format!("composition.roots[{index}]"),
                error,
            )),
        }
    }
    roots
}

fn validate_forbidden_edges(
    record: &BoundaryRecord,
    resolver: &WorkspacePackageResolver,
    diagnostics: &mut Vec<InventoryDiagnostic>,
) {
    let context = format!(
        "dependencies.forbidden_edges for boundary `{}`",
        record.boundary_id
    );
    for edge in &record.dependencies.forbidden_edges {
        let from = resolver.resolve_if_known(edge.from.as_ref(), &context);
        let to = resolver.resolve_if_known(edge.to.as_ref(), &context);
        if matches!((from, to), (Ok(Some(from)), Ok(Some(to))) if from == to) {
            diagnostics.push(InventoryDiagnostic::new(
                record,
                "dependencies.forbidden_edges",
                InventoryDiagnosticKind::SelfForbiddenEdge,
                format!(
                    "forbidden edge `{}` -> `{}` resolves to the same workspace package",
                    edge.from, edge.to
                ),
                "remove the self edge or name two distinct workspace packages",
            ));
        }
    }
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
            .allowed_dependents
            .iter()
            .any(|dependent| dependent == &root_package)
    {
        return;
    }
    diagnostics.push(InventoryDiagnostic::new(
        record,
        format!("composition.roots[{index}]"),
        InventoryDiagnosticKind::UnapprovedCompositionRoot,
        format!(
            "external composition root `{}` resolves to `{root_package}`, which is not in dependencies.allowed_dependents",
            root.as_string()
        ),
        "add the external package to dependencies.allowed_dependents or keep the root owner-local",
    ));
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
