use std::collections::BTreeMap;
use std::collections::BTreeSet;

use anyhow::{Context, Result};

use crate::Finding;
use crate::OwnerId;
use crate::RuleId;
use crate::inventory::ValidatedDependencyPolicy;
use crate::inventory::ValidatedInventory;
use crate::inventory::ValidatedRecord;
use crate::inventory::WorkspacePackageName;

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

pub(crate) fn analyze_package_policy(
    metadata: &cargo_metadata::Metadata,
    inventory: &ValidatedInventory,
) -> Result<PackagePolicyReport> {
    let workspace_packages = crate::workspace_packages(metadata);
    let workspace_package_names = workspace_packages
        .iter()
        .map(|package| package.name.to_string())
        .collect::<BTreeSet<_>>();
    let (runtime_edges, dev_edges) =
        collect_direct_workspace_edges(&workspace_packages, &workspace_package_names);

    let findings =
        collect_policy_findings(inventory, &runtime_edges, &dev_edges, &workspace_packages)?;

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
    inventory: &ValidatedInventory,
    runtime_edges: &BTreeSet<PackageEdge>,
    dev_edges: &BTreeSet<PackageEdge>,
    workspace_packages: &[&cargo_metadata::Package],
) -> Result<Vec<Finding>> {
    let direct_edges = runtime_edges
        .union(dev_edges)
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut findings = Vec::new();
    let mut forbidden_findings = BTreeSet::new();
    let policies = inventory
        .records()
        .filter_map(|record| record.policy().map(|policy| (record, policy)))
        .collect::<Vec<_>>();

    for (record, policy) in policies
        .iter()
        .filter(|(record, _)| record.requires_workspace_package())
    {
        findings.extend(feature_findings(record, policy, workspace_packages)?);
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
            let edge = PackageEdge {
                from: edge.from.clone(),
                to: edge.to.clone(),
            };
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

fn feature_findings(
    record: &ValidatedRecord,
    policy: &ValidatedDependencyPolicy,
    packages: &[&cargo_metadata::Package],
) -> Result<Vec<Finding>> {
    let Some(allowed) = &policy.allowed_feature_dependencies else {
        return Ok(Vec::new());
    };
    let Some(owner) = packages
        .iter()
        .find(|p| p.name.as_str() == policy.owner.as_str())
    else {
        return Ok(Vec::new());
    };
    let text = std::fs::read_to_string(&owner.manifest_path).with_context(|| {
        format!(
            "reading feature declarations from `{}`",
            owner.manifest_path
        )
    })?;
    let manifest: toml::Value = toml::from_str(&text).with_context(|| {
        format!(
            "parsing feature declarations from `{}`",
            owner.manifest_path
        )
    })?;
    let values = manifest
        .get("features")
        .and_then(toml::Value::as_table)
        .into_iter()
        .flat_map(|table| table.values())
        .filter_map(toml::Value::as_array)
        .flatten()
        .filter_map(toml::Value::as_str)
        .collect::<Vec<_>>();
    let mut findings = BTreeMap::new();
    for dependency in &owner.dependencies {
        if dependency.kind == cargo_metadata::DependencyKind::Development {
            continue;
        }
        let Some(target) = dependency.path.as_deref().and_then(|path| {
            packages
                .iter()
                .find(|p| p.manifest_path.parent() == Some(path))
        }) else {
            continue;
        };
        let target_name = WorkspacePackageName::from_package_name(target.name.to_string());
        if allowed.contains(&target_name) {
            continue;
        }
        let key = dependency.rename.as_deref().unwrap_or(&dependency.name);
        let explicit_dep = values.iter().any(|v| v.strip_prefix("dep:") == Some(key));
        let forwards = values.iter().any(|value| {
            references_dependency_with_implicit(value, key, dependency.optional, explicit_dep)
        });
        for (kind, hit) in [
            (
                "package_dependency_sets_features",
                !dependency.uses_default_features || !dependency.features.is_empty(),
            ),
            ("package_feature_forwards_dependency", forwards),
        ] {
            if hit {
                findings.entry((target.name.to_string(), kind)).or_insert_with(|| Finding {
                    rule_id: RuleId::ScbFeature001,
                    kind: kind.into(),
                    message: format!("workspace package `{}` uses features of `{}` ({kind}) but `{}` is not listed in `{}` allowed_feature_dependencies", owner.name, target.name, target.name, record.boundary_id()),
                    owner_ids: vec![OwnerId::new(owner.name.to_string()), OwnerId::new(target.name.to_string())],
                    node_ids: Vec::new(),
                });
            }
        }
    }
    Ok(findings.into_values().collect())
}

pub(crate) fn references_dependency(value: &str, key: &str) -> bool {
    match value.strip_prefix("dep:") {
        Some(name) => name == key,
        None => value
            .split_once('/')
            .is_some_and(|(name, _)| name.strip_suffix('?').unwrap_or(name) == key),
    }
}

pub(crate) fn references_dependency_with_implicit(
    value: &str,
    key: &str,
    optional: bool,
    explicit_dep: bool,
) -> bool {
    references_dependency(value, key) || (optional && !explicit_dep && value == key)
}

fn collect_incoming_policy_findings(
    policies: &[(&ValidatedRecord, &ValidatedDependencyPolicy)],
    runtime_edges: &BTreeSet<PackageEdge>,
    dev_edges: &BTreeSet<PackageEdge>,
) -> Vec<Finding> {
    let active_owners = policies
        .iter()
        .filter(|(record, _)| record.requires_workspace_package())
        .map(|(_, policy)| policy.owner.clone())
        .collect::<BTreeSet<_>>();
    let mut owners = BTreeMap::<
        WorkspacePackageName,
        (
            &ValidatedRecord,
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
    for (owner, (record, runtime_allowed, dev_allowed)) in &owners {
        if !active_owners.contains(owner) {
            continue;
        }
        for dependent in runtime_incoming.get(owner).into_iter().flatten() {
            if !runtime_allowed.contains(dependent) {
                findings.push(disallowed_dependent_finding(record, dependent, owner));
            }
        }
        for dependent in dev_incoming.get(owner).into_iter().flatten() {
            if !dev_allowed.contains(dependent) {
                findings.push(disallowed_dependent_finding(record, dependent, owner));
            }
        }
    }
    findings.extend(collect_stale_dependent_findings(
        policies,
        &owners,
        &active_owners,
        &runtime_incoming,
        &dev_incoming,
    ));
    findings
}

type PermissionKey = (WorkspacePackageName, WorkspacePackageName);

struct PermissionRecordIndex<'a> {
    runtime: BTreeMap<PermissionKey, &'a ValidatedRecord>,
    dev: BTreeMap<PermissionKey, &'a ValidatedRecord>,
    fallback_dev: BTreeSet<PermissionKey>,
    explicit_dev: BTreeSet<PermissionKey>,
}

fn collect_stale_dependent_findings(
    policies: &[(&ValidatedRecord, &ValidatedDependencyPolicy)],
    owners: &BTreeMap<
        WorkspacePackageName,
        (
            &ValidatedRecord,
            BTreeSet<WorkspacePackageName>,
            BTreeSet<WorkspacePackageName>,
        ),
    >,
    active_owners: &BTreeSet<WorkspacePackageName>,
    runtime_incoming: &BTreeMap<WorkspacePackageName, BTreeSet<WorkspacePackageName>>,
    dev_incoming: &BTreeMap<WorkspacePackageName, BTreeSet<WorkspacePackageName>>,
) -> Vec<Finding> {
    let records = index_permission_records(policies);
    collect_stale_findings_from_owners(
        owners,
        active_owners,
        runtime_incoming,
        dev_incoming,
        &records,
    )
}

fn index_permission_records<'a>(
    policies: &[(&'a ValidatedRecord, &ValidatedDependencyPolicy)],
) -> PermissionRecordIndex<'a> {
    let mut records = PermissionRecordIndex {
        runtime: BTreeMap::new(),
        dev: BTreeMap::new(),
        fallback_dev: BTreeSet::new(),
        explicit_dev: BTreeSet::new(),
    };
    for (record, policy) in policies {
        for dependent in &policy.allowed_dependents {
            let key = (policy.owner.clone(), dependent.clone());
            records.runtime.entry(key.clone()).or_insert(*record);
            if policy.allowed_dev_dependents.is_none() {
                records.fallback_dev.insert(key.clone());
            }
            if policy
                .allowed_dev_dependents
                .as_ref()
                .unwrap_or(&policy.allowed_dependents)
                .contains(dependent)
            {
                records.dev.entry(key).or_insert(*record);
            }
        }
        if let Some(dependents) = &policy.allowed_dev_dependents {
            for dependent in dependents {
                let key = (policy.owner.clone(), dependent.clone());
                records.dev.entry(key.clone()).or_insert(*record);
                records.explicit_dev.insert(key);
            }
        }
    }
    records
}

fn collect_stale_findings_from_owners(
    owners: &BTreeMap<
        WorkspacePackageName,
        (
            &ValidatedRecord,
            BTreeSet<WorkspacePackageName>,
            BTreeSet<WorkspacePackageName>,
        ),
    >,
    active_owners: &BTreeSet<WorkspacePackageName>,
    runtime_incoming: &BTreeMap<WorkspacePackageName, BTreeSet<WorkspacePackageName>>,
    dev_incoming: &BTreeMap<WorkspacePackageName, BTreeSet<WorkspacePackageName>>,
    records: &PermissionRecordIndex<'_>,
) -> Vec<Finding> {
    let mut findings = Vec::new();
    let mut stale_permissions = BTreeSet::new();
    for (owner, (_, runtime_allowed, dev_allowed)) in owners {
        if !active_owners.contains(owner) {
            continue;
        }
        for dependent in runtime_allowed.union(dev_allowed) {
            let key = (owner.clone(), dependent.clone());
            let runtime_live = runtime_allowed.contains(dependent)
                && runtime_incoming
                    .get(owner)
                    .is_some_and(|edges| edges.contains(dependent));
            let dev_live = dev_allowed.contains(dependent)
                && dev_incoming
                    .get(owner)
                    .is_some_and(|edges| edges.contains(dependent));
            if runtime_live || dev_live || !stale_permissions.insert(key.clone()) {
                continue;
            }

            let (record, field) = if let Some(record) = records.runtime.get(&key) {
                (*record, "dependencies.allowed_dependents")
            } else {
                (
                    *records
                        .dev
                        .get(&key)
                        .expect("aggregated dev permission has a source record"),
                    "dependencies.allowed_dev_dependents",
                )
            };
            let has_runtime = runtime_allowed.contains(dependent);
            let has_dev = dev_allowed.contains(dependent);
            let categories = match (has_runtime, has_dev) {
                (true, true)
                    if records.fallback_dev.contains(&key)
                        && !records.explicit_dev.contains(&key) =>
                {
                    "runtime and fallback dev"
                }
                (true, true) => "runtime and dev",
                (true, false) => "runtime",
                (false, true) => "dev",
                (false, false) => continue,
            };
            findings.push(stale_allowed_dependent_finding(
                record, dependent, owner, field, categories,
            ));
        }
    }
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
    record: &ValidatedRecord,
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
            record.boundary_id(),
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
    record: &ValidatedRecord,
    dependent: &WorkspacePackageName,
    owner: &WorkspacePackageName,
) -> Finding {
    Finding {
        rule_id: RuleId::ScbDependency002,
        kind: "package_dependent_not_allowed".into(),
        message: format!(
            "workspace package `{}` directly depends on `{}` but `{}` is not listed in `{}` allowed_dependents",
            dependent,
            owner,
            dependent,
            record.boundary_id()
        ),
        owner_ids: vec![
            OwnerId::new(dependent.as_ref()),
            OwnerId::new(owner.as_ref()),
        ],
        node_ids: Vec::new(),
    }
}

fn stale_allowed_dependent_finding(
    record: &ValidatedRecord,
    dependent: &WorkspacePackageName,
    owner: &WorkspacePackageName,
    field: &str,
    categories: &str,
) -> Finding {
    Finding {
        rule_id: RuleId::ScbDependency004,
        kind: "stale_allowed_dependent".into(),
        message: format!(
            "boundary record `{}` (`{}`) field `{field}` grants `{dependent}` permission to depend on `{owner}` for {categories} categories, but no live direct workspace edge exists in any granted category; remove the obsolete permission or restore the intended direct workspace edge",
            record.record_path().display(),
            record.boundary_id(),
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
