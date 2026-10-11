use std::collections::BTreeMap;
use std::collections::BTreeSet;

use anyhow::Result;

use crate::Finding;
use crate::OwnerId;
use crate::RuleId;
use crate::inventory::BoundaryInventory;
use crate::inventory::BoundaryRecord;
use crate::inventory::ForbiddenPackageEdge;
use crate::inventory::WorkspacePackageName;

/// A direct workspace-member dependency edge observed in Cargo metadata.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct PackageEdge {
    from: WorkspacePackageName,
    to: WorkspacePackageName,
}

impl From<&ForbiddenPackageEdge> for PackageEdge {
    fn from(edge: &ForbiddenPackageEdge) -> Self {
        Self {
            from: edge.from.clone(),
            to: edge.to.clone(),
        }
    }
}

/// How an observed edge is declared, which decides the allowlists it is
/// checked against (REQ-SCB-016).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EdgeCategory {
    Runtime,
    Dev,
}

impl EdgeCategory {
    fn verb(self) -> &'static str {
        match self {
            Self::Runtime => "depends",
            Self::Dev => "dev-depends",
        }
    }

    fn allowlists(self) -> &'static str {
        match self {
            Self::Runtime => "allowed_dependencies",
            Self::Dev => "allowed_dependencies or allowed_dev_dependencies",
        }
    }
}

pub(crate) struct PackagePolicyReport {
    pub(crate) findings: Vec<Finding>,
    pub(crate) scanned_crates: usize,
}

pub(crate) fn analyze_package_policy(
    metadata: &cargo_metadata::Metadata,
    inventory: &BoundaryInventory,
) -> Result<PackagePolicyReport> {
    let workspace_packages = crate::workspace_packages(metadata);
    let workspace_package_names = workspace_packages
        .iter()
        .map(|package| package.name.to_string())
        .collect::<BTreeSet<_>>();
    let (runtime_edges, dev_edges) =
        collect_direct_workspace_edges(&workspace_packages, &workspace_package_names);

    let findings = collect_policy_findings(
        inventory,
        &workspace_package_names,
        &runtime_edges,
        &dev_edges,
    );

    Ok(PackagePolicyReport {
        findings,
        scanned_crates: workspace_packages.len(),
    })
}

fn collect_direct_workspace_edges(
    workspace_packages: &[&cargo_metadata::Package],
    workspace_package_names: &BTreeSet<String>,
) -> (BTreeSet<PackageEdge>, BTreeSet<PackageEdge>) {
    let mut runtime_edges = BTreeSet::new();
    let mut dev_edges = BTreeSet::new();
    for package in workspace_packages {
        let from = WorkspacePackageName::from_package_name(package.name.to_string());
        for dependency in &package.dependencies {
            let dependency_name = dependency.name.to_string();
            if !workspace_package_names.contains(&dependency_name) {
                continue;
            }
            let edge = PackageEdge {
                from: from.clone(),
                to: WorkspacePackageName::from_package_name(dependency_name),
            };
            if dependency.kind == cargo_metadata::DependencyKind::Development {
                dev_edges.insert(edge);
            } else {
                runtime_edges.insert(edge);
            }
        }
    }
    dev_edges.retain(|edge| !runtime_edges.contains(edge));
    (runtime_edges, dev_edges)
}

fn collect_policy_findings(
    inventory: &BoundaryInventory,
    workspace_package_names: &BTreeSet<String>,
    runtime_edges: &BTreeSet<PackageEdge>,
    dev_edges: &BTreeSet<PackageEdge>,
) -> Vec<Finding> {
    let direct_edges = runtime_edges
        .union(dev_edges)
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut findings = Vec::new();
    let mut forbidden_findings = BTreeSet::new();
    let incoming_edges = index_incoming_edges(&direct_edges);

    for record in &inventory.records {
        if !workspace_package_names.contains(record.owner_package.as_ref()) {
            continue;
        }

        let owner = WorkspacePackageName::from_package_name(record.owner_package.as_ref());

        for edge in runtime_edges.iter().filter(|edge| edge.from == owner) {
            if !record.dependencies.allowed_dependencies.contains(&edge.to) {
                findings.push(disallowed_dependency_finding(
                    record,
                    edge,
                    EdgeCategory::Runtime,
                ));
            }
        }
        for edge in dev_edges.iter().filter(|edge| edge.from == owner) {
            if !record.dependencies.allowed_dependencies.contains(&edge.to)
                && !record
                    .dependencies
                    .allowed_dev_dependencies
                    .contains(&edge.to)
            {
                findings.push(disallowed_dependency_finding(
                    record,
                    edge,
                    EdgeCategory::Dev,
                ));
            }
        }

        for dependent in incoming_edges.get(&owner).into_iter().flatten() {
            if !record.dependencies.allowed_dependents.contains(dependent) {
                findings.push(disallowed_dependent_finding(record, dependent, &owner));
            }
        }

        for edge in &record.dependencies.forbidden_edges {
            let edge = PackageEdge::from(edge);
            if direct_edges.contains(&edge) && forbidden_findings.insert(edge.clone()) {
                findings.push(forbidden_edge_finding(&edge));
            }
        }
    }

    findings.sort_by(|left, right| {
        left.rule_id
            .cmp(&right.rule_id)
            .then_with(|| left.message.cmp(&right.message))
    });
    findings
}

fn index_incoming_edges(
    direct_edges: &BTreeSet<PackageEdge>,
) -> BTreeMap<WorkspacePackageName, BTreeSet<WorkspacePackageName>> {
    let mut incoming = BTreeMap::<WorkspacePackageName, BTreeSet<WorkspacePackageName>>::new();
    for edge in direct_edges {
        incoming
            .entry(edge.to.clone())
            .or_default()
            .insert(edge.from.clone());
    }
    incoming
}

fn disallowed_dependency_finding(
    record: &BoundaryRecord,
    edge: &PackageEdge,
    category: EdgeCategory,
) -> Finding {
    Finding {
        rule_id: RuleId::ScbDependency001,
        kind: "package_dependency_not_allowed".into(),
        message: format!(
            "workspace package `{}` directly {} on `{}` but `{}` is not listed in `{}` {}",
            edge.from,
            category.verb(),
            edge.to,
            edge.to,
            record.boundary_id,
            category.allowlists()
        ),
        owner_ids: vec![
            OwnerId::new(edge.from.as_ref()),
            OwnerId::new(edge.to.as_ref()),
        ],
        node_ids: Vec::new(),
    }
}

fn disallowed_dependent_finding(
    record: &BoundaryRecord,
    dependent: &WorkspacePackageName,
    owner: &WorkspacePackageName,
) -> Finding {
    Finding {
        rule_id: RuleId::ScbDependency002,
        kind: "package_dependent_not_allowed".into(),
        message: format!(
            "workspace package `{}` directly depends on `{}` but `{}` is not listed in `{}` allowed_dependents",
            dependent, owner, dependent, record.boundary_id
        ),
        owner_ids: vec![
            OwnerId::new(dependent.as_ref()),
            OwnerId::new(owner.as_ref()),
        ],
        node_ids: Vec::new(),
    }
}

fn forbidden_edge_finding(edge: &PackageEdge) -> Finding {
    Finding {
        rule_id: RuleId::ScbDependency003,
        kind: "forbidden_package_edge_present".into(),
        message: format!(
            "forbidden workspace dependency edge `{}` is present",
            format_args!("{} -> {}", edge.from, edge.to)
        ),
        owner_ids: vec![
            OwnerId::new(edge.from.as_ref()),
            OwnerId::new(edge.to.as_ref()),
        ],
        node_ids: Vec::new(),
    }
}
