use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::path::Path;

use anyhow::Context;
use anyhow::Result;
use cargo_metadata::DependencyKind;
use cargo_metadata::Metadata;
use cargo_metadata::Package;

use super::config::DependencyPackageName;
use super::config::ManifestAllowlist;
use crate::Finding;
use crate::OwnerId;
use crate::RuleId;
use crate::inventory::ValidatedInventory;

struct ActualDependencies {
    runtime: BTreeSet<DependencyPackageName>,
    development: BTreeSet<DependencyPackageName>,
    all: BTreeSet<DependencyPackageName>,
}

pub(crate) fn analyze(
    root: &Path,
    metadata: &Metadata,
    inventory: Option<&ValidatedInventory>,
    allowlists: &[ManifestAllowlist],
) -> Result<Vec<Finding>> {
    if allowlists.is_empty() {
        return Ok(Vec::new());
    }
    let inventory =
        inventory.context("manifest dependency allowlists require validated boundary inventory")?;
    let packages = crate::workspace_packages(metadata);
    let manifests = manifest_packages(root, &packages)?;
    let active_owners = active_owner_packages(inventory);
    let mut configured = BTreeSet::new();
    let mut findings = Vec::new();

    for allowlist in allowlists {
        findings.extend(analyze_allowlist(
            root,
            inventory,
            &manifests,
            &active_owners,
            &mut configured,
            allowlist,
        ));
    }
    findings.extend(missing_active_owner_allowlists(
        root,
        &packages,
        &active_owners,
        &configured,
    )?);
    Ok(findings)
}

fn manifest_packages<'a>(
    root: &Path,
    packages: &[&'a Package],
) -> Result<BTreeMap<String, &'a Package>> {
    packages
        .iter()
        .map(|package| {
            Ok((
                relative(root, package.manifest_path.as_std_path())?,
                *package,
            ))
        })
        .collect()
}

fn analyze_allowlist(
    root: &Path,
    inventory: &ValidatedInventory,
    manifests: &BTreeMap<String, &Package>,
    active_owners: &BTreeSet<String>,
    configured: &mut BTreeSet<String>,
    allowlist: &ManifestAllowlist,
) -> Vec<Finding> {
    let owner = allowlist.owner.display();
    if !configured.insert(owner.clone()) {
        return vec![stale(&owner, "duplicate allowlist owner entry")];
    }
    let Some(package) = manifests.get(&owner) else {
        return vec![stale(&owner, "non-workspace allowlist owner")];
    };
    if !active_owners.contains(package.name.as_str()) {
        return vec![stale(&owner, "inactive allowlist owner")];
    }
    let actual = ActualDependencies::from_package(package);
    let mut findings = compare_dependencies(&owner, &actual, allowlist);
    findings.extend(validate_record_locks(
        root, inventory, package, &owner, &actual, allowlist,
    ));
    findings
}

fn active_owner_packages(inventory: &ValidatedInventory) -> BTreeSet<String> {
    inventory
        .records()
        .filter(|record| record.policy().is_some())
        .filter_map(|record| record.owner())
        .map(ToString::to_string)
        .collect()
}

impl ActualDependencies {
    fn from_package(package: &Package) -> Self {
        let runtime = package
            .dependencies
            .iter()
            .filter(|dependency| {
                matches!(
                    dependency.kind,
                    DependencyKind::Normal | DependencyKind::Build
                )
            })
            .map(|dependency| DependencyPackageName(dependency.name.clone()))
            .collect::<BTreeSet<_>>();
        let development = package
            .dependencies
            .iter()
            .filter(|dependency| dependency.kind == DependencyKind::Development)
            .map(|dependency| DependencyPackageName(dependency.name.clone()))
            .collect::<BTreeSet<_>>();
        let all = runtime.union(&development).cloned().collect();
        Self {
            runtime,
            development,
            all,
        }
    }
}

fn compare_dependencies(
    owner: &str,
    actual: &ActualDependencies,
    allowlist: &ManifestAllowlist,
) -> Vec<Finding> {
    let mut findings = Vec::new();
    for dependency in actual.all.difference(&allowlist.allowed) {
        findings.push(Finding {
            rule_id: RuleId::ScbManifest003,
            kind: "manifest_dependency_not_allowlisted".into(),
            message: format!(
                "{owner}: remove dependency `{}` or review/add its permission",
                dependency.0
            ),
            owner_ids: vec![OwnerId::new(owner.to_owned())],
            node_ids: Vec::new(),
        });
    }
    for dependency in allowlist.allowed.difference(&actual.all) {
        findings.push(stale(
            owner,
            &format!("remove unused permission `{}`", dependency.0),
        ));
    }
    findings
}

fn validate_record_locks(
    root: &Path,
    inventory: &ValidatedInventory,
    package: &Package,
    owner: &str,
    actual: &ActualDependencies,
    allowlist: &ManifestAllowlist,
) -> Vec<Finding> {
    let mut findings = Vec::new();
    for record_path in &allowlist.records {
        let record_path = record_path.display();
        let Some(record) = inventory.records().find(|record| {
            relative(root, record.record_path()).ok().as_deref() == Some(record_path.as_str())
        }) else {
            findings.push(stale(
                owner,
                &format!("record lock `{record_path}` does not name a boundary record"),
            ));
            continue;
        };
        let Some(policy) = record.policy() else {
            findings.push(stale(
                owner,
                &format!("record lock `{record_path}` is not an active boundary record"),
            ));
            continue;
        };
        if policy.owner.as_ref() != package.name.as_str() {
            findings.push(stale(
                owner,
                &format!("record lock `{record_path}` belongs to a different owner"),
            ));
            continue;
        }
        let declared_runtime = policy
            .allowed_dependencies
            .iter()
            .map(|value| DependencyPackageName(value.to_string()))
            .collect::<BTreeSet<_>>();
        let declared_development = policy
            .allowed_dev_dependencies
            .iter()
            .map(|value| DependencyPackageName(value.to_string()))
            .collect::<BTreeSet<_>>();
        let declared = declared_runtime
            .union(&declared_development)
            .cloned()
            .collect::<BTreeSet<_>>();
        if declared != allowlist.allowed
            || (!declared_development.is_empty()
                && (declared_runtime != actual.runtime
                    || declared_development != actual.development))
        {
            findings.push(stale(
                owner,
                &format!(
                    "record lock `{record_path}` must align with the allowlist and dependency sections"
                ),
            ));
        }
    }
    findings
}

fn missing_active_owner_allowlists(
    root: &Path,
    packages: &[&Package],
    active_owners: &BTreeSet<String>,
    configured: &BTreeSet<String>,
) -> Result<Vec<Finding>> {
    let mut findings = Vec::new();
    // Multiple active records may share one workspace owner; policy coverage is once per package.
    for package in packages
        .iter()
        .filter(|package| active_owners.contains(package.name.as_str()))
    {
        let path = relative(root, package.manifest_path.as_std_path())?;
        if !configured.contains(&path) {
            findings.push(stale(&path, "missing allowlist entry for active owner"));
        }
    }
    Ok(findings)
}

fn stale(owner: &str, detail: &str) -> Finding {
    Finding {
        rule_id: RuleId::ScbManifest006,
        kind: "manifest_policy_inconsistent".into(),
        message: format!("{owner}: {detail}"),
        owner_ids: vec![OwnerId::new(owner.to_owned())],
        node_ids: Vec::new(),
    }
}

fn relative(root: &Path, path: &Path) -> Result<String> {
    Ok(sc_lint_analyzer_support::relative_slash_path(root, path))
}
