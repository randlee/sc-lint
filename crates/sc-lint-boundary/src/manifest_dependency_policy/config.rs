use std::collections::BTreeSet;
use std::path::PathBuf;

use sc_lint_analyzer_support::AnalyzerConfigError;
use sc_lint_analyzer_support::AnalyzerSettings;
use sc_lint_analyzer_support::CatalogRuleId;
use toml::Table;
use toml::Value;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct DependencyPackageName(pub(crate) String);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ManifestSection(pub(crate) String);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct PolicyPath(pub(crate) PathBuf);

impl PolicyPath {
    pub(crate) fn parse(value: &str) -> Self {
        let mut normalized = PathBuf::new();
        for component in PathBuf::from(value.replace('\\', "/")).components() {
            match component {
                std::path::Component::CurDir => {}
                component => normalized.push(component.as_os_str()),
            }
        }
        Self(normalized)
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

pub(crate) fn parse_policy(
    settings: &AnalyzerSettings,
    selected: Option<&BTreeSet<CatalogRuleId>>,
) -> std::result::Result<ManifestPolicyConfig, AnalyzerConfigError> {
    let parse_all = selected.is_none();
    let parse_allowlists = parse_all
        || selected.is_some_and(|selected| {
            ["SCB-MANIFEST-003", "SCB-MANIFEST-006"]
                .iter()
                .any(|id| selected.contains(&CatalogRuleId::new(id)))
        });
    let parse_ownership_rules = parse_all
        || selected
            .is_some_and(|selected| selected.contains(&CatalogRuleId::new("SCB-MANIFEST-004")));
    let parse_sections = parse_all
        || selected
            .is_some_and(|selected| selected.contains(&CatalogRuleId::new("SCB-MANIFEST-005")));
    Ok(ManifestPolicyConfig {
        allowlists: if parse_allowlists {
            policy_tables(settings, "manifest_dependency_allowlists")?
                .into_iter()
                .enumerate()
                .map(|(index, table)| parse_allowlist(table, index))
                .collect::<std::result::Result<_, _>>()?
        } else {
            Vec::new()
        },
        ownership: if parse_ownership_rules {
            policy_tables(settings, "global_dependency_ownership")?
                .into_iter()
                .enumerate()
                .map(|(index, table)| parse_ownership(table, index))
                .collect::<std::result::Result<_, _>>()?
        } else {
            Vec::new()
        },
        sections: if parse_sections {
            policy_tables(settings, "manifest_section_rules")?
                .into_iter()
                .enumerate()
                .map(|(index, table)| parse_section_rule(table, index))
                .collect::<std::result::Result<_, _>>()?
        } else {
            Vec::new()
        },
    })
}

fn policy_tables<'a>(
    settings: &'a AnalyzerSettings,
    key: &str,
) -> std::result::Result<Vec<&'a Table>, AnalyzerConfigError> {
    settings
        .own_tables(key)
        .map_err(|error| match error {
            AnalyzerConfigError::WrongType { key: path, .. } if path == key => wrong_type(
                path,
                "an array of tables (the policy field has the wrong TOML type; use array-of-table entries)",
            ),
            AnalyzerConfigError::WrongType { key: path, .. } => wrong_type(
                path,
                "a table (the indexed policy item has the wrong TOML type; replace it with a table entry)",
            ),
            other => other,
        })
        .map(Option::unwrap_or_default)
}

fn parse_allowlist(
    table: &Table,
    index: usize,
) -> std::result::Result<ManifestAllowlist, AnalyzerConfigError> {
    let prefix = format!("manifest_dependency_allowlists[{index}]");
    Ok(ManifestAllowlist {
        owner: PolicyPath::parse(required_string(table, "owner_manifest_path", &prefix)?),
        allowed: required_strings(table, "allowed_dependencies", &prefix)?
            .into_iter()
            .map(DependencyPackageName)
            .collect(),
        records: optional_nonempty_paths(table, "boundary_record_path", &prefix, true)?,
    })
}

fn parse_ownership(
    table: &Table,
    index: usize,
) -> std::result::Result<DependencyOwnership, AnalyzerConfigError> {
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

fn parse_section_rule(
    table: &Table,
    index: usize,
) -> std::result::Result<SectionRule, AnalyzerConfigError> {
    let prefix = format!("manifest_section_rules[{index}]");
    Ok(SectionRule {
        owner: PolicyPath::parse(required_string(table, "owner_manifest_path", &prefix)?),
        dependency: DependencyPackageName(
            required_string(table, "dependency_package", &prefix)?.to_owned(),
        ),
        sections: optional_strings(table, "allowed_sections", &prefix, false, false)?
            .into_iter()
            .map(ManifestSection)
            .collect(),
        message: required_string(table, "message", &prefix)?.to_owned(),
    })
}

fn required_string<'a>(
    table: &'a Table,
    key: &str,
    prefix: &str,
) -> std::result::Result<&'a str, AnalyzerConfigError> {
    let path = format!("{prefix}.{key}");
    match table.get(key) {
        None => Err(wrong_type(
            path,
            "a non-empty string (the required field is missing; add a quoted string value)",
        )),
        Some(Value::String(value)) if !value.trim().is_empty() => Ok(value),
        Some(Value::String(_)) => Err(wrong_type(
            path,
            "a non-empty string (the value is blank; provide a non-empty quoted string)",
        )),
        Some(_) => Err(wrong_type(
            path,
            "a non-empty string (the value has the wrong TOML type; provide a quoted string)",
        )),
    }
}

fn required_strings(
    table: &Table,
    key: &str,
    prefix: &str,
) -> std::result::Result<Vec<String>, AnalyzerConfigError> {
    optional_strings(table, key, prefix, true, true)
}

fn optional_strings(
    table: &Table,
    key: &str,
    prefix: &str,
    required: bool,
    nonempty_items: bool,
) -> std::result::Result<Vec<String>, AnalyzerConfigError> {
    let Some(value) = table.get(key) else {
        if required {
            return Err(wrong_type(
                format!("{prefix}.{key}"),
                "an array of non-empty strings (the required field is missing; add quoted string values in an array)",
            ));
        }
        return Ok(Vec::new());
    };
    let array = value.as_array().ok_or_else(|| {
        wrong_type(
            format!("{prefix}.{key}"),
            "an array of strings (the value has the wrong TOML type; use an array of quoted strings)",
        )
    })?;
    array
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let path = format!("{prefix}.{key}[{index}]");
            match value {
                Value::String(value) if !nonempty_items || !value.trim().is_empty() => {
                    Ok(value.clone())
                }
                Value::String(_) => Err(wrong_type(
                    path,
                    "a non-empty string (the array item is blank; replace it with a non-empty quoted string)",
                )),
                _ => Err(wrong_type(
                    path,
                    "a string (the array item has the wrong TOML type; replace it with a quoted string)",
                )),
            }
        })
        .collect()
}

fn optional_paths(
    table: &Table,
    key: &str,
    prefix: &str,
    single_allowed: bool,
) -> std::result::Result<Vec<PolicyPath>, AnalyzerConfigError> {
    let Some(value) = table.get(key) else {
        return Ok(Vec::new());
    };
    match value {
        Value::String(value) if single_allowed && !value.trim().is_empty() => {
            Ok(vec![PolicyPath::parse(value)])
        }
        Value::Array(_) => {
            optional_strings(table, key, prefix, false, single_allowed).map(|values| {
                values
                    .iter()
                    .map(|value| PolicyPath::parse(value))
                    .collect()
            })
        }
        Value::String(_) if single_allowed => Err(wrong_type(
            format!("{prefix}.{key}"),
            "a non-empty string or an array of non-empty strings (the path is blank; provide a repository-relative path)",
        )),
        _ if single_allowed => Err(wrong_type(
            format!("{prefix}.{key}"),
            "a non-empty string or an array of non-empty strings (the value has the wrong TOML type; provide a repository-relative path or an array of paths)",
        )),
        _ => Err(wrong_type(
            format!("{prefix}.{key}"),
            "an array of non-empty strings (the value has the wrong TOML type; use an array of repository-relative paths)",
        )),
    }
}

fn wrong_type(key: String, expected: &'static str) -> AnalyzerConfigError {
    AnalyzerConfigError::WrongType {
        analyzer: "boundary",
        key,
        expected,
    }
}

fn optional_nonempty_paths(
    table: &Table,
    key: &str,
    prefix: &str,
    single_allowed: bool,
) -> std::result::Result<Vec<PolicyPath>, AnalyzerConfigError> {
    if matches!(table.get(key), Some(Value::Array(values)) if values.is_empty()) {
        return Err(wrong_type(
            format!("{prefix}.{key}"),
            "a non-empty array of non-empty strings (the path array is empty; provide at least one repository-relative path)",
        ));
    }
    optional_paths(table, key, prefix, single_allowed)
}
