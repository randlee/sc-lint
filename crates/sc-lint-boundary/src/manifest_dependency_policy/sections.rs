use std::collections::BTreeMap;
use std::path::Path;

use anyhow::Context;
use anyhow::Result;
use cargo_metadata::Metadata;
use toml::Table;
use toml::Value;

use super::config::SectionRule;
use crate::Finding;
use crate::OwnerId;
use crate::RuleId;

pub(crate) fn analyze(
    root: &Path,
    metadata: &Metadata,
    rules: &[SectionRule],
) -> Result<Vec<Finding>> {
    let mut findings = Vec::new();
    for package in crate::workspace_packages(metadata) {
        let manifest_path = sc_lint_analyzer_support::relative_slash_path(
            root,
            package.manifest_path.as_std_path(),
        );
        let matching = rules
            .iter()
            .filter(|rule| rule.owner.display() == manifest_path)
            .collect::<Vec<_>>();
        if matching.is_empty() {
            continue;
        }
        let manifest = load_manifest(package.manifest_path.as_std_path())?;
        let aliases = package
            .dependencies
            .iter()
            .map(|dependency| {
                (
                    dependency
                        .rename
                        .as_deref()
                        .unwrap_or(&dependency.name)
                        .to_owned(),
                    dependency.name.as_str(),
                )
            })
            .collect::<BTreeMap<_, _>>();
        for (section, dependencies) in dependency_sections(&manifest) {
            for dependency_key in dependencies.keys() {
                let package_name = aliases
                    .get(dependency_key)
                    .copied()
                    .unwrap_or(dependency_key.as_str());
                for rule in matching
                    .iter()
                    .filter(|rule| rule.dependency.0 == package_name)
                {
                    if !rule.sections.iter().any(|allowed| allowed.0 == section) {
                        findings.push(Finding {
                            rule_id: RuleId::ScbManifest005,
                            kind: "manifest_section_restriction".into(),
                            message: format!("{manifest_path} [{section}.{dependency_key}]: {} (allowed sections: {})", rule.message, rule.sections.iter().map(|value| value.0.as_str()).collect::<Vec<_>>().join(", ")),
                            owner_ids: vec![OwnerId::new(manifest_path.clone())],
                            node_ids: Vec::new(),
                        });
                    }
                }
            }
        }
    }
    Ok(findings)
}

fn load_manifest(path: &Path) -> Result<Table> {
    toml::from_str(&std::fs::read_to_string(path)?)
        .with_context(|| format!("failed to parse manifest `{}`", path.display()))
}

fn dependency_sections(manifest: &Table) -> Vec<(String, &Table)> {
    let mut sections = Vec::new();
    for name in ["dependencies", "dev-dependencies", "build-dependencies"] {
        if let Some(table) = manifest.get(name).and_then(Value::as_table) {
            sections.push((name.to_owned(), table));
        }
    }
    if let Some(targets) = manifest.get("target").and_then(Value::as_table) {
        for (target, target_table) in targets {
            let Some(target_table) = target_table.as_table() else {
                continue;
            };
            for name in ["dependencies", "dev-dependencies", "build-dependencies"] {
                if let Some(table) = target_table.get(name).and_then(Value::as_table) {
                    sections.push((format!("target.{target}.{name}"), table));
                }
            }
        }
    }
    sections
}
