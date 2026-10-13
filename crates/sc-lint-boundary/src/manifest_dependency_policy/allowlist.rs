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
    let mut configured = BTreeSet::new();
    let mut findings = Vec::new();

    for allowlist in allowlists {
        findings.extend(analyze_allowlist(
            root,
            inventory,
            &manifests,
            &mut configured,
            allowlist,
        ));
    }
    findings.extend(missing_active_owner_allowlists(
        root,
        inventory,
        &packages,
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
    configured: &mut BTreeSet<String>,
    allowlist: &ManifestAllowlist,
) -> Vec<Finding> {
    let owner = allowlist.owner.display();
    if !configured.insert(owner.clone()) {
        return vec![stale(&owner, "duplicate allowlist owner")];
    }
    let Some(package) = manifests.get(&owner) else {
        return vec![stale(&owner, "owner manifest is not a workspace member")];
    };
    if !has_exactly_one_active_record(inventory, package) {
        return vec![stale(
            &owner,
            "owner must have exactly one active boundary record",
        )];
    }
    let actual = ActualDependencies::from_package(package);
    let mut findings = compare_dependencies(&owner, &actual, allowlist);
    findings.extend(validate_record_locks(
        root, inventory, package, &owner, &actual, allowlist,
    ));
    findings
}

fn has_exactly_one_active_record(inventory: &ValidatedInventory, package: &Package) -> bool {
    inventory
        .records()
        .filter(|record| record.status.state.requires_workspace_package())
        .filter(|record| record.owner_package.as_ref() == package.name.as_str())
        .count()
        == 1
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
            relative(root, &record.record_path).ok().as_deref() == Some(record_path.as_str())
        }) else {
            findings.push(stale(
                owner,
                &format!("record lock `{record_path}` does not name a boundary record"),
            ));
            continue;
        };
        if record.owner_package.as_ref() != package.name.as_str() {
            findings.push(stale(
                owner,
                &format!("record lock `{record_path}` belongs to a different owner"),
            ));
            continue;
        }
        let declared_runtime = record
            .dependencies
            .allowed_dependencies
            .iter()
            .map(|value| DependencyPackageName(value.to_string()))
            .collect::<BTreeSet<_>>();
        let declared_development = record
            .dependencies
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
    inventory: &ValidatedInventory,
    packages: &[&Package],
    configured: &BTreeSet<String>,
) -> Result<Vec<Finding>> {
    let mut findings = Vec::new();
    for record in inventory
        .records()
        .filter(|record| record.status.state.requires_workspace_package())
    {
        let Some(package) = packages
            .iter()
            .find(|package| package.name.as_str() == record.owner_package.as_ref())
        else {
            continue;
        };
        let path = relative(root, package.manifest_path.as_std_path())?;
        if !configured.contains(&path) {
            findings.push(stale(&path, "missing active-owner allowlist entry"));
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
