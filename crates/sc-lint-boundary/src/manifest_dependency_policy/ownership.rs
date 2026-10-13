use std::fs;
use std::path::Path;

use anyhow::Result;
use cargo_metadata::Metadata;

use super::config::DependencyOwnership;
use crate::Finding;
use crate::OwnerId;
use crate::RuleId;

pub(crate) fn analyze(
    root: &Path,
    metadata: &Metadata,
    rules: &[DependencyOwnership],
) -> Result<Vec<Finding>> {
    let mut findings = Vec::new();
    let packages = crate::workspace_packages(metadata);
    for rule in rules {
        for package in &packages {
            let manifest = sc_lint_analyzer_support::relative_slash_path(
                root,
                package.manifest_path.as_std_path(),
            );
            if package
                .dependencies
                .iter()
                .any(|dependency| dependency.name == rule.dependency.0)
                && !rule.manifests.iter().any(|path| path.display() == manifest)
            {
                findings.push(Finding {
                    rule_id: RuleId::ScbManifest004,
                    kind: "manifest_dependency_owner_violation".into(),
                    message: format!(
                        "{manifest}: {} (allowed manifests: {})",
                        rule.manifest_message,
                        displays(&rule.manifests)
                    ),
                    owner_ids: vec![OwnerId::new(manifest)],
                    node_ids: Vec::new(),
                });
            }
        }
        let crate_spelling = rule.dependency.0.replace('-', "_");
        for file in sc_lint_analyzer_support::discover_source_files(root)? {
            let relative = sc_lint_analyzer_support::relative_slash_path(root, &file.source_path);
            if rule.roots.iter().any(|path| {
                relative == path.display() || relative.starts_with(&format!("{}/", path.display()))
            }) {
                continue;
            }
            let source = fs::read_to_string(&file.source_path)?;
            if source_mentions_dependency(&source, &crate_spelling) {
                findings.push(Finding {
                    rule_id: RuleId::ScbManifest004,
                    kind: "source_dependency_owner_violation".into(),
                    message: format!(
                        "{relative}: {} (allowed source roots: {})",
                        rule.source_message,
                        displays(&rule.roots)
                    ),
                    owner_ids: vec![OwnerId::new(relative)],
                    node_ids: Vec::new(),
                });
            }
        }
    }
    Ok(findings)
}

fn source_mentions_dependency(source: &str, crate_name: &str) -> bool {
    source.lines().any(|line| {
        let line = line.trim_start();
        !line.starts_with("//")
            && (line.contains(&format!("{crate_name}::"))
                || line.contains(&format!("use {crate_name}"))
                || line.contains(&format!("extern crate {crate_name}")))
    })
}

fn displays(paths: &[super::config::PolicyPath]) -> String {
    paths
        .iter()
        .map(super::config::PolicyPath::display)
        .collect::<Vec<_>>()
        .join(", ")
}
