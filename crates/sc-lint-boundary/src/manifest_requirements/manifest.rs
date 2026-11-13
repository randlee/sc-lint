use anyhow::Context;
use anyhow::Result;
use toml_edit::ImDocument;
use toml_edit::Item;
use toml_edit::Table;

use crate::Finding;

use super::finding;
use super::version;

pub(super) fn findings(label: &str, text: &str) -> Result<Vec<Finding>> {
    let document = text
        .parse::<ImDocument<String>>()
        .with_context(|| format!("failed to parse manifest `{label}`"))?;
    let mut findings = Vec::new();

    for section in ["dependencies", "dev-dependencies", "build-dependencies"] {
        if let Some(table) = document.get(section).and_then(Item::as_table) {
            inspect_dependencies(label, text, section, table, &mut findings)?;
        }
    }

    if let Some(targets) = document.get("target").and_then(Item::as_table) {
        let mut target_names = targets.iter().map(|(name, _)| name).collect::<Vec<_>>();
        target_names.sort_unstable();
        for target_name in target_names {
            let Some(target) = targets.get(target_name).and_then(Item::as_table) else {
                continue;
            };
            for section in ["dependencies", "dev-dependencies", "build-dependencies"] {
                if let Some(table) = target.get(section).and_then(Item::as_table) {
                    inspect_dependencies(
                        label,
                        text,
                        &format!("target.{target_name}.{section}"),
                        table,
                        &mut findings,
                    )?;
                }
            }
        }
    }

    if let Some(workspace) = document.get("workspace").and_then(Item::as_table)
        && let Some(table) = workspace.get("dependencies").and_then(Item::as_table)
    {
        inspect_dependencies(label, text, "workspace.dependencies", table, &mut findings)?;
    }

    Ok(findings)
}

fn inspect_dependencies(
    label: &str,
    text: &str,
    section: &str,
    dependencies: &Table,
    findings: &mut Vec<Finding>,
) -> Result<()> {
    for (name, item) in dependencies.iter() {
        let Some((raw_requirement, span)) = registry_requirement(item) else {
            continue;
        };
        let line = line_number(text, span.start);
        let bound_comment = version::bound_comment_on(text, line);
        for violation in
            version::violations(raw_requirement, bound_comment.as_ref()).with_context(|| {
                format!("{label}:{line} [{section}.{name}]: invalid version requirement")
            })?
        {
            findings.push(finding(
                violation.rule_id,
                violation.kind,
                format!("{label}:{line} [{section}.{name}]: {}", violation.detail),
                label.to_owned(),
            ));
        }
    }
    Ok(())
}

fn registry_requirement(item: &Item) -> Option<(&str, std::ops::Range<usize>)> {
    if let Some(value) = item.as_value()
        && let Some(requirement) = value.as_str()
    {
        return Some((requirement, value.span()?));
    }
    let table = item.as_table_like()?;
    if table.contains_key("path") || table.contains_key("git") {
        return None;
    }
    let version = table.get("version")?.as_value()?;
    Some((version.as_str()?, version.span()?))
}

fn line_number(text: &str, offset: usize) -> usize {
    text.as_bytes()[..offset.min(text.len())]
        .iter()
        .filter(|byte| **byte == b'\n')
        .count()
        + 1
}

#[cfg(test)]
mod tests {
    use super::findings;
    use crate::RuleId;

    #[test]
    fn reports_workspace_and_target_dependencies_with_source_lines() {
        let text = r#"[workspace.dependencies]
pin = "=1.2.3"

[target.'cfg(unix)'.dev-dependencies]
tilde = "~1.2.3"
"#;
        let findings = findings("Cargo.toml", text).expect("manifest scans");
        assert_eq!(findings.len(), 2);
        assert!(
            findings
                .iter()
                .all(|finding| finding.rule_id == RuleId::ScbManifest007)
        );
        assert!(
            findings
                .iter()
                .any(|finding| finding.message.starts_with("Cargo.toml:2 "))
        );
        assert!(
            findings
                .iter()
                .any(|finding| finding.message.starts_with("Cargo.toml:5 "))
        );
    }

    #[test]
    fn path_and_git_requirements_are_not_registry_dependencies() {
        let text = r#"[dependencies]
local = { version = "=1.2.3", path = "../local" }
git_dep = { version = "~1.2.3", git = "https://example.invalid/repo" }
registry = { version = "=1.2.3" }
"#;
        let findings = findings("Cargo.toml", text).expect("manifest scans");
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule_id, RuleId::ScbManifest007);
        assert!(findings[0].message.contains("registry"));
    }
}
