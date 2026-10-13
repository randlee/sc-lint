use std::collections::BTreeMap;
use std::collections::BTreeSet;

use anyhow::Result;

use crate::Finding;
use crate::OwnerId;
use crate::RuleId;
use crate::inventory::BoundaryInventory;
use crate::inventory::BoundaryRecord;
use crate::inventory::WorkspacePackageName;
use crate::inventory::WorkspacePackageResolver;

/// A direct workspace-member dependency edge observed in Cargo metadata.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct PackageEdge {
    from: WorkspacePackageName,
    to: WorkspacePackageName,
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

struct ResolvedDependencyPolicy {
    owner: WorkspacePackageName,
    allowed_dependencies: BTreeSet<WorkspacePackageName>,
    allowed_dev_dependencies: BTreeSet<WorkspacePackageName>,
    allowed_dependents: BTreeSet<WorkspacePackageName>,
    allowed_dev_dependents: Option<BTreeSet<WorkspacePackageName>>,
    forbidden_edges: BTreeSet<PackageEdge>,
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
    let known_non_workspace_packages = metadata
        .packages
        .iter()
        .map(|package| package.name.to_string())
        .filter(|package| !workspace_package_names.contains(package))
        .chain(
            inventory
                .records
                .iter()
                .filter(|record| !record.status.state.requires_workspace_package())
                .map(|record| record.owner_package.to_string()),
        )
        .collect::<BTreeSet<_>>();
    let resolver = WorkspacePackageResolver::from_workspace_packages(&workspace_packages);
    let (runtime_edges, dev_edges) =
        collect_direct_workspace_edges(&workspace_packages, &workspace_package_names);

    let findings = collect_policy_findings(
        inventory,
        &resolver,
        &known_non_workspace_packages,
        &runtime_edges,
        &dev_edges,
    )?;

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
    resolver: &WorkspacePackageResolver,
    known_non_workspace_packages: &BTreeSet<String>,
    runtime_edges: &BTreeSet<PackageEdge>,
    dev_edges: &BTreeSet<PackageEdge>,
) -> Result<Vec<Finding>> {
    let direct_edges = runtime_edges
        .union(dev_edges)
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut findings = Vec::new();
    let mut forbidden_findings = BTreeSet::new();
    let policies = inventory
        .records
        .iter()
        .filter(|record| record.status.state.requires_workspace_package())
        .map(|record| {
            resolve_dependency_policy(resolver, known_non_workspace_packages, record)
                .map(|policy| (record, policy))
        })
        .collect::<Result<Vec<_>>>()?;

    for (record, policy) in &policies {
        for edge in runtime_edges
            .iter()
            .filter(|edge| edge.from == policy.owner)
        {
            if !policy.allowed_dependencies.contains(&edge.to) {
                findings.push(disallowed_dependency_finding(
                    record,
                    edge,
                    EdgeCategory::Runtime,
                ));
            }
        }
        for edge in dev_edges.iter().filter(|edge| edge.from == policy.owner) {
            if !policy.allowed_dependencies.contains(&edge.to)
                && !policy.allowed_dev_dependencies.contains(&edge.to)
            {
                findings.push(disallowed_dependency_finding(
                    record,
                    edge,
                    EdgeCategory::Dev,
                ));
            }
        }

        for edge in &policy.forbidden_edges {
            if direct_edges.contains(&edge) && forbidden_findings.insert(edge.clone()) {
                findings.push(forbidden_edge_finding(&edge));
            }
        }
    }

    findings.extend(collect_incoming_policy_findings(
        &policies,
        runtime_edges,
        dev_edges,
    ));

    findings.sort_by(|left, right| {
        left.rule_id
            .cmp(&right.rule_id)
            .then_with(|| left.message.cmp(&right.message))
    });
    Ok(findings)
}

fn resolve_dependency_policy(
    resolver: &WorkspacePackageResolver,
    known_non_workspace_packages: &BTreeSet<String>,
    record: &BoundaryRecord,
) -> Result<ResolvedDependencyPolicy> {
    Ok(ResolvedDependencyPolicy {
        owner: resolve_policy_reference(
            resolver,
            known_non_workspace_packages,
            record,
            "owner_package",
            record.owner_package.as_ref(),
        )?
        .expect("enforceable boundary owners must resolve to workspace packages"),
        allowed_dependencies: resolve_policy_references(
            resolver,
            known_non_workspace_packages,
            record,
            "dependencies.allowed_dependencies",
            &record.dependencies.allowed_dependencies,
        )?,
        allowed_dev_dependencies: resolve_policy_references(
            resolver,
            known_non_workspace_packages,
            record,
            "dependencies.allowed_dev_dependencies",
            &record.dependencies.allowed_dev_dependencies,
        )?,
        allowed_dependents: resolve_policy_references(
            resolver,
            known_non_workspace_packages,
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
                    resolver,
                    known_non_workspace_packages,
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
                    resolver,
                    known_non_workspace_packages,
                    record,
                    "dependencies.forbidden_edges[].from",
                    edge.from.as_ref(),
                );
                let to = resolve_policy_reference(
                    resolver,
                    known_non_workspace_packages,
                    record,
                    "dependencies.forbidden_edges[].to",
                    edge.to.as_ref(),
                );
                match (from, to) {
                    (Ok(Some(from)), Ok(Some(to))) => Some(Ok(PackageEdge { from, to })),
                    (Ok(_), Ok(_)) => None,
                    (Err(error), _) | (_, Err(error)) => Some(Err(error)),
                }
            })
            .collect::<Result<BTreeSet<_>>>()?,
    })
}

fn collect_incoming_policy_findings(
    policies: &[(&BoundaryRecord, ResolvedDependencyPolicy)],
    runtime_edges: &BTreeSet<PackageEdge>,
    dev_edges: &BTreeSet<PackageEdge>,
) -> Vec<Finding> {
    let mut owners = BTreeMap::<
        WorkspacePackageName,
        (
            &BoundaryRecord,
            BTreeSet<WorkspacePackageName>,
            BTreeSet<WorkspacePackageName>,
        ),
    >::new();
    for (record, policy) in policies {
        let entry = owners
            .entry(policy.owner.clone())
            .or_insert_with(|| (*record, BTreeSet::new(), BTreeSet::new()));
        entry.1.extend(policy.allowed_dependents.iter().cloned());
        entry.2.extend(
            policy
                .allowed_dev_dependents
                .as_ref()
                .unwrap_or(&policy.allowed_dependents)
                .iter()
                .cloned(),
        );
    }

    let runtime_incoming = index_incoming_edges(runtime_edges);
    let dev_incoming = index_incoming_edges(dev_edges);
    let mut findings = Vec::new();
    for (owner, (record, runtime_allowed, dev_allowed)) in owners {
        for dependent in runtime_incoming.get(&owner).into_iter().flatten() {
            if !runtime_allowed.contains(dependent) {
                findings.push(disallowed_dependent_finding(record, dependent, &owner));
            }
        }
        for dependent in dev_incoming.get(&owner).into_iter().flatten() {
            if !dev_allowed.contains(dependent) {
                findings.push(disallowed_dependent_finding(record, dependent, &owner));
            }
        }
    }
    findings
}

fn resolve_policy_references(
    resolver: &WorkspacePackageResolver,
    known_non_workspace_packages: &BTreeSet<String>,
    record: &BoundaryRecord,
    field: &str,
    references: &BTreeSet<WorkspacePackageName>,
) -> Result<BTreeSet<WorkspacePackageName>> {
    let mut resolved = BTreeSet::new();
    for reference in references {
        if let Some(package) = resolve_policy_reference(
            resolver,
            known_non_workspace_packages,
            record,
            field,
            reference.as_ref(),
        )? {
            resolved.insert(package);
        }
    }
    Ok(resolved)
}

fn resolve_policy_reference(
    resolver: &WorkspacePackageResolver,
    known_non_workspace_packages: &BTreeSet<String>,
    record: &BoundaryRecord,
    field: &str,
    reference: &str,
) -> Result<Option<WorkspacePackageName>> {
    let context = format!("`{field}` for boundary `{}`", record.boundary_id);
    match resolver.resolve_if_known(reference, &context)? {
        Some(package) => Ok(Some(package)),
        None if known_non_workspace_packages.contains(reference) => Ok(None),
        None => resolver.resolve(reference, context).map(Some),
    }
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
