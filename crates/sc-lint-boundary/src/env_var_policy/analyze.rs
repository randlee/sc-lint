use std::collections::BTreeSet;
use std::path::Path;

use anyhow::{Context, Result};
use cargo_metadata::Metadata;
use sc_lint_analyzer_support::{discover_source_files, relative_slash_path};

use super::collect::{self, Value};
use super::{EnvVarBoundary, EnvVarBoundaryConfig};
use crate::{Finding, OwnerId, RuleId};

pub(super) fn analyze(
    root: &Path,
    metadata: &Metadata,
    config: &EnvVarBoundaryConfig,
) -> Result<Vec<Finding>> {
    if config.entries.is_empty() {
        return Ok(Vec::new());
    }
    let packages = crate::workspace_packages(metadata);
    let mut findings = Vec::new();
    for file in discover_source_files(root)? {
        if file.is_test_file {
            continue;
        }
        let Some(package) = packages
            .iter()
            .find(|package| package.name.as_str() == file.package.as_str())
        else {
            continue;
        };
        let Some(package_root) = package.manifest_path.as_std_path().parent() else {
            continue;
        };
        let package_relative = relative_slash_path(root, package_root);
        let rules: Vec<_> = config
            .entries
            .iter()
            .filter(|entry| restricted(entry, &package_relative))
            .collect();
        if rules.is_empty() {
            continue;
        }
        let source = std::fs::read_to_string(&file.source_path)
            .with_context(|| format!("read {}", file.source_path.display()))?;
        let parsed = syn::parse_file(&source)
            .with_context(|| format!("parse {}", file.source_path.display()))?;
        let relative = relative_slash_path(root, &file.source_path);
        let prefix = module_prefix(package.name.as_str(), package_root, &file.source_path);
        let functions = collect::functions(&parsed, prefix);
        for function in &functions {
            for rule in &rules {
                if rule
                    .boundary_reader_functions
                    .iter()
                    .any(|reader| reader == &function.path)
                {
                    continue;
                }
                let summaries = collect::summaries(&functions, &rule.boundary_reader_functions);
                let mut reads = BTreeSet::new();
                for (value, line) in &function.direct {
                    if let Value::Literal(name) = value {
                        reads.insert((name.clone(), *line));
                    }
                }
                for call in &function.calls {
                    for index in
                        collect::matching_functions(&functions, &function.path, &call.target)
                    {
                        if rule
                            .boundary_reader_functions
                            .contains(&functions[index].path)
                        {
                            continue;
                        }
                        for effect in &summaries[index] {
                            if let Some(Value::Literal(name)) =
                                collect::substitute(effect, &call.args)
                            {
                                reads.insert((name, call.line));
                            }
                        }
                    }
                }
                for (name, line) in reads {
                    if rule.forbidden_env_vars.contains(&name) {
                        findings.push(Finding {
                            rule_id: RuleId::ScbEnv001,
                            kind: "env_var_boundary_violation".into(),
                            message: format!("{relative}:{line}: forbidden environment variable `{name}` read outside a boundary reader"),
                            owner_ids: vec![OwnerId::new(relative.clone())],
                            node_ids: Vec::new(),
                        });
                    }
                }
            }
        }
    }
    Ok(findings)
}

fn restricted(rule: &EnvVarBoundary, package: &str) -> bool {
    rule.restricted_crate_roots.iter().any(|root| {
        let root = root.trim_end_matches('/');
        root == "."
            || package == root
            || package
                .strip_prefix(root)
                .is_some_and(|suffix| suffix.starts_with('/'))
    })
}

fn module_prefix(package: &str, package_root: &Path, source: &Path) -> Vec<String> {
    let mut result = vec![package.replace('-', "_")];
    let Ok(relative) = source.strip_prefix(package_root.join("src")) else {
        return result;
    };
    let mut components: Vec<_> = relative
        .iter()
        .map(|s| s.to_string_lossy().into_owned())
        .collect();
    if let Some(last) = components.last_mut() {
        if matches!(last.as_str(), "lib.rs" | "main.rs" | "mod.rs") {
            components.pop();
        } else if let Some(stem) = last.strip_suffix(".rs") {
            *last = stem.to_owned();
        }
    }
    result.extend(components);
    result
}
