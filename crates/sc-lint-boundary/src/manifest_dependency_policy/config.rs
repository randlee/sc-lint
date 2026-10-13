use std::collections::BTreeSet;
use std::path::PathBuf;

use anyhow::Context;
use anyhow::Result;
use sc_lint_analyzer_support::AnalyzerSettings;
use toml::Table;
use toml::Value;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct DependencyPackageName(pub(crate) String);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ManifestSection(pub(crate) String);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct PolicyPath(pub(crate) PathBuf);

impl PolicyPath {
    pub(crate) fn parse(value: &str) -> Result<Self> {
        let mut normalized = PathBuf::new();
        for component in PathBuf::from(value.replace('\\', "/")).components() {
            match component {
                std::path::Component::CurDir => {}
                component => normalized.push(component.as_os_str()),
            }
        }
        Ok(Self(normalized))
    }

    pub(crate) fn display(&self) -> String {
        self.0.to_string_lossy().replace('\\', "/")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ManifestAllowlist {
    pub(crate) owner: PolicyPath,
    pub(crate) allowed: BTreeSet<DependencyPackageName>,
    pub(crate) records: Vec<PolicyPath>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DependencyOwnership {
    pub(crate) dependency: DependencyPackageName,
    pub(crate) manifests: Vec<PolicyPath>,
    pub(crate) roots: Vec<PolicyPath>,
    pub(crate) manifest_message: String,
    pub(crate) source_message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SectionRule {
    pub(crate) owner: PolicyPath,
    pub(crate) dependency: DependencyPackageName,
    pub(crate) sections: Vec<ManifestSection>,
    pub(crate) message: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ManifestPolicyConfig {
    pub(crate) allowlists: Vec<ManifestAllowlist>,
    pub(crate) ownership: Vec<DependencyOwnership>,
    pub(crate) sections: Vec<SectionRule>,
}

impl ManifestPolicyConfig {
    pub(crate) fn is_empty(&self) -> bool {
        self.allowlists.is_empty() && self.ownership.is_empty() && self.sections.is_empty()
    }

    pub(crate) fn requires_inventory(&self) -> bool {
        !self.allowlists.is_empty()
    }
}

pub(crate) fn parse_policy(settings: &AnalyzerSettings) -> Result<ManifestPolicyConfig> {
    Ok(ManifestPolicyConfig {
        allowlists: settings
            .own_tables("manifest_dependency_allowlists")?
            .unwrap_or_default()
            .into_iter()
            .enumerate()
            .map(|(index, table)| parse_allowlist(table, index))
            .collect::<Result<_>>()?,
        ownership: settings
            .own_tables("global_dependency_ownership")?
            .unwrap_or_default()
            .into_iter()
            .enumerate()
            .map(|(index, table)| parse_ownership(table, index))
            .collect::<Result<_>>()?,
        sections: settings
            .own_tables("manifest_section_rules")?
            .unwrap_or_default()
            .into_iter()
            .enumerate()
            .map(|(index, table)| parse_section_rule(table, index))
            .collect::<Result<_>>()?,
    })
}

fn parse_allowlist(table: &Table, index: usize) -> Result<ManifestAllowlist> {
    let prefix = format!("manifest_dependency_allowlists[{index}]");
    Ok(ManifestAllowlist {
        owner: PolicyPath::parse(required_string(table, "owner_manifest_path", &prefix)?)?,
        allowed: required_strings(table, "allowed_dependencies", &prefix)?
            .into_iter()
            .map(|value| Ok(DependencyPackageName(value)))
            .collect::<Result<_>>()?,
        records: optional_paths(table, "boundary_record_path", &prefix, true)?,
    })
}

fn parse_ownership(table: &Table, index: usize) -> Result<DependencyOwnership> {
    let prefix = format!("global_dependency_ownership[{index}]");
    Ok(DependencyOwnership {
        dependency: DependencyPackageName(
            required_string(table, "dependency", &prefix)?.to_owned(),
        ),
        manifests: optional_paths(table, "allowed_manifest_paths", &prefix, false)?,
        roots: optional_paths(table, "allowed_source_roots", &prefix, false)?,
        manifest_message: required_string(table, "manifest_message", &prefix)?.to_owned(),
        source_message: required_string(table, "source_message", &prefix)?.to_owned(),
    })
}

fn parse_section_rule(table: &Table, index: usize) -> Result<SectionRule> {
    let prefix = format!("manifest_section_rules[{index}]");
    Ok(SectionRule {
        owner: PolicyPath::parse(required_string(table, "owner_manifest_path", &prefix)?)?,
        dependency: DependencyPackageName(
            required_string(table, "dependency_package", &prefix)?.to_owned(),
        ),
        sections: optional_strings(table, "allowed_sections", &prefix, false)?
            .into_iter()
            .map(ManifestSection)
            .collect(),
        message: required_string(table, "message", &prefix)?.to_owned(),
    })
}

fn required_string<'a>(table: &'a Table, key: &str, prefix: &str) -> Result<&'a str> {
    table
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .with_context(|| format!("{prefix}.{key} must be a non-empty string"))
}

fn required_strings(table: &Table, key: &str, prefix: &str) -> Result<Vec<String>> {
    optional_strings(table, key, prefix, true)
}

fn optional_strings(table: &Table, key: &str, prefix: &str, required: bool) -> Result<Vec<String>> {
    let Some(value) = table.get(key) else {
        if required {
            anyhow::bail!("{prefix}.{key} must be an array of non-empty strings");
        }
        return Ok(Vec::new());
    };
    let array = value
        .as_array()
        .with_context(|| format!("{prefix}.{key} must be an array of strings"))?;
    array
        .iter()
        .enumerate()
        .map(|(index, value)| {
            value
                .as_str()
                .map(ToOwned::to_owned)
                .with_context(|| format!("{prefix}.{key}[{index}] must be a string"))
        })
        .collect()
}

fn optional_paths(
    table: &Table,
    key: &str,
    prefix: &str,
    single_allowed: bool,
) -> Result<Vec<PolicyPath>> {
    let Some(value) = table.get(key) else {
        return Ok(Vec::new());
    };
    match value {
        Value::String(value) if single_allowed && !value.trim().is_empty() => {
            Ok(vec![PolicyPath::parse(value)?])
        }
        Value::Array(_) => optional_strings(table, key, prefix, false)?
            .into_iter()
            .map(|value| PolicyPath::parse(&value))
            .collect(),
        _ => anyhow::bail!(
            "{prefix}.{key} must be {}",
            if single_allowed {
                "a non-empty string or an array of non-empty strings"
            } else {
                "an array of strings"
            }
        ),
    }
}
