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

pub(crate) struct PackagePolicyReport {
    pub(crate) findings: Vec<Finding>,
    pub(crate) scanned_crates: usize,
}

pub(crate) fn analyze_package_policy(
    metadata: &cargo_metadata::Metadata,
    inventory: &BoundaryInventory,
) -> Result<PackagePolicyReport> {
    let workspace_members = metadata.workspace_members.iter().collect::<BTreeSet<_>>();
    let workspace_packages = metadata
        .packages
        .iter()
        .filter(|package| workspace_members.contains(&package.id))
        .collect::<Vec<_>>();
    let workspace_package_names = workspace_packages
        .iter()
        .map(|package| package.name.to_string())
        .collect::<BTreeSet<_>>();
    let direct_edges =
        collect_direct_workspace_edges(&workspace_packages, &workspace_package_names);

    let findings = collect_policy_findings(inventory, &workspace_package_names, &direct_edges);

    Ok(PackagePolicyReport {
        findings,
        scanned_crates: workspace_packages.len(),
    })
}

fn collect_direct_workspace_edges(
    workspace_packages: &[&cargo_metadata::Package],
    workspace_package_names: &BTreeSet<String>,
) -> BTreeSet<ForbiddenPackageEdge> {
    let mut direct_edges = BTreeSet::new();
    for package in workspace_packages {
        let from = WorkspacePackageName::from_package_name(package.name.to_string());
        for dependency in &package.dependencies {
            let dependency_name = dependency.name.to_string();
            if !workspace_package_names.contains(&dependency_name) {
                continue;
            }
            direct_edges.insert(ForbiddenPackageEdge {
                from: from.clone(),
                to: WorkspacePackageName::from_package_name(dependency_name),
            });
        }
    }
    direct_edges
}

fn collect_policy_findings(
    inventory: &BoundaryInventory,
    workspace_package_names: &BTreeSet<String>,
    direct_edges: &BTreeSet<ForbiddenPackageEdge>,
) -> Vec<Finding> {
    let mut findings = Vec::new();
    let mut forbidden_findings = BTreeSet::new();
    let incoming_edges = index_incoming_edges(direct_edges);

    for record in &inventory.records {
        if !workspace_package_names.contains(record.owner_package.as_ref()) {
            continue;
        }

        let owner = WorkspacePackageName::from_package_name(record.owner_package.as_ref());

        for edge in direct_edges.iter().filter(|edge| edge.from == owner) {
            if !record.dependencies.allowed_dependencies.contains(&edge.to) {
                findings.push(disallowed_dependency_finding(record, edge));
            }
        }

        for dependent in incoming_edges.get(&owner).into_iter().flatten() {
            if !record.dependencies.allowed_dependents.contains(dependent) {
                findings.push(disallowed_dependent_finding(record, dependent, &owner));
            }
        }

        for edge in &record.dependencies.forbidden_edges {
            if direct_edges.contains(edge) && forbidden_findings.insert(edge.clone()) {
                findings.push(forbidden_edge_finding(edge));
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
    direct_edges: &BTreeSet<ForbiddenPackageEdge>,
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

fn disallowed_dependency_finding(record: &BoundaryRecord, edge: &ForbiddenPackageEdge) -> Finding {
    Finding {
        rule_id: RuleId::ScbDependency001,
        kind: "package_dependency_not_allowed".into(),
        message: format!(
            "workspace package `{}` directly depends on `{}` but `{}` is not listed in `{}` allowed_dependencies",
            edge.from, edge.to, edge.to, record.boundary_id
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

fn forbidden_edge_finding(edge: &ForbiddenPackageEdge) -> Finding {
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
