use std::collections::BTreeSet;
use std::path::PathBuf;

use sc_lint_analyzer_support::AnalyzerConfigError;
use sc_lint_analyzer_support::AnalyzerSettings;
use sc_lint_analyzer_support::CatalogRuleId;
use toml::Table;
use toml::Value;

use crate::env_var_policy::EnvVarBoundary;
use crate::env_var_policy::EnvVarBoundaryConfig;
use crate::manifest_requirements::DependencyRequirements;
use crate::manifest_requirements::DependencyRequirementsConfig;

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
    pub(crate) env_var_boundary: EnvVarBoundaryConfig,
    pub(crate) dependency_requirements: DependencyRequirementsConfig,
}

impl ManifestPolicyConfig {
    pub(crate) fn is_empty(&self) -> bool {
        self.allowlists.is_empty()
            && self.ownership.is_empty()
            && self.sections.is_empty()
            && self.env_var_boundary.entries.is_empty()
            && self.dependency_requirements.entries.is_empty()
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
    let parse_env_var_boundary = parse_all
        || selected.is_some_and(|selected| selected.contains(&CatalogRuleId::new("SCB-ENV-001")));
    let parse_dependency_requirements = parse_all
        || selected.is_some_and(|selected| {
            ["SCB-MANIFEST-007", "SCB-MANIFEST-008"]
                .iter()
                .any(|id| selected.contains(&CatalogRuleId::new(id)))
        });
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
        env_var_boundary: if parse_env_var_boundary {
            EnvVarBoundaryConfig {
                entries: policy_tables(settings, "env_var_boundary")?
                    .into_iter()
                    .enumerate()
                    .map(|(index, table)| parse_env_var_boundary_entry(table, index))
                    .collect::<std::result::Result<_, _>>()?,
            }
        } else {
            EnvVarBoundaryConfig::default()
        },
        dependency_requirements: if parse_dependency_requirements {
            DependencyRequirementsConfig {
                entries: policy_tables(settings, "dependency_requirements")?
                    .into_iter()
                    .enumerate()
                    .map(|(index, table)| parse_dependency_requirements_entry(table, index))
                    .collect::<std::result::Result<_, _>>()?,
            }
        } else {
            DependencyRequirementsConfig::default()
        },
    })
}

fn parse_env_var_boundary_entry(
    table: &Table,
    index: usize,
) -> std::result::Result<EnvVarBoundary, AnalyzerConfigError> {
    const FIELDS: &[&str] = &[
        "forbidden_env_vars",
        "restricted_crate_roots",
        "boundary_reader_functions",
        "reason",
    ];
    let prefix = format!("env_var_boundary[{index}]");
    reject_unknown_fields(table, FIELDS, &prefix)?;
    Ok(EnvVarBoundary {
        forbidden_env_vars: optional_strings(table, "forbidden_env_vars", &prefix, false, true)?,
        restricted_crate_roots: optional_strings(
            table,
            "restricted_crate_roots",
            &prefix,
            false,
            true,
        )?,
        boundary_reader_functions: optional_strings(
            table,
            "boundary_reader_functions",
            &prefix,
            false,
            true,
        )?,
        reason: optional_string(table, "reason", &prefix)?,
    })
}

fn parse_dependency_requirements_entry(
    table: &Table,
    index: usize,
) -> std::result::Result<DependencyRequirements, AnalyzerConfigError> {
    const FIELDS: &[&str] = &["template_manifests", "package_archives", "test_manifests"];
    let prefix = format!("dependency_requirements[{index}]");
    reject_unknown_fields(table, FIELDS, &prefix)?;
    Ok(DependencyRequirements {
        template_manifests: optional_strings(table, "template_manifests", &prefix, false, true)?,
        package_archives: optional_strings(table, "package_archives", &prefix, false, true)?,
        test_manifests: optional_strings(table, "test_manifests", &prefix, false, true)?,
    })
}

fn reject_unknown_fields(
    table: &Table,
    fields: &[&str],
    prefix: &str,
) -> std::result::Result<(), AnalyzerConfigError> {
    if let Some(key) = table.keys().find(|key| !fields.contains(&key.as_str())) {
        return Err(AnalyzerConfigError::UnknownKey {
            analyzer: "boundary",
            key: format!("{prefix}.{key}"),
        });
    }
    Ok(())
}

fn optional_string(
    table: &Table,
    key: &str,
    prefix: &str,
) -> std::result::Result<Option<String>, AnalyzerConfigError> {
    match table.get(key) {
        None => Ok(None),
        Some(Value::String(value)) if !value.trim().is_empty() => Ok(Some(value.clone())),
        Some(Value::String(_)) => Err(wrong_type(
            format!("{prefix}.{key}"),
            "a non-empty string (provide a non-empty quoted string)",
        )),
        Some(_) => Err(wrong_type(
            format!("{prefix}.{key}"),
            "a string (provide a quoted string)",
        )),
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::BOUNDARY_CATALOG;
    use sc_lint_analyzer_support::load_analyzer_settings;
    use std::fs;
    use tempfile::TempDir;

    fn parse_config(config: &str) -> ManifestPolicyConfig {
        let fixture = TempDir::new().expect("config fixture");
        fs::write(fixture.path().join("sc-lint.toml"), config).expect("write config");
        let settings = load_analyzer_settings(&BOUNDARY_CATALOG, fixture.path(), None, None)
            .expect("load analyzer settings");
        parse_policy(&settings, None).expect("parse boundary policies")
    }

    #[test]
    fn parses_new_policy_families_into_typed_configs() {
        let config = parse_config(
            r#"
                [[tool.sc-lint.analyzers.boundary.env_var_boundary]]
                forbidden_env_vars = ["ATM_HOME", "SC_LINT_HOME"]
                restricted_crate_roots = ["crates/atm-core"]
                boundary_reader_functions = ["atm_core::config::read_home"]
                reason = "settings belong to the host application"

                [[tool.sc-lint.analyzers.boundary.dependency_requirements]]
                template_manifests = ["templates/**/Cargo.toml"]
                package_archives = ["target/package/*.crate"]
                test_manifests = ["tests/fixtures/**/Cargo.toml"]
            "#,
        );

        assert_eq!(
            config.env_var_boundary.entries,
            vec![EnvVarBoundary {
                forbidden_env_vars: vec!["ATM_HOME".to_owned(), "SC_LINT_HOME".to_owned()],
                restricted_crate_roots: vec!["crates/atm-core".to_owned()],
                boundary_reader_functions: vec!["atm_core::config::read_home".to_owned()],
                reason: Some("settings belong to the host application".to_owned()),
            }]
        );
        assert_eq!(
            config.dependency_requirements.entries,
            vec![DependencyRequirements {
                template_manifests: vec!["templates/**/Cargo.toml".to_owned()],
                package_archives: vec!["target/package/*.crate".to_owned()],
                test_manifests: vec!["tests/fixtures/**/Cargo.toml".to_owned()],
            }]
        );
    }

    #[test]
    fn absent_policy_families_are_inactive() {
        let config = parse_config("[tool.sc-lint.analyzers.boundary]\n");

        assert!(config.env_var_boundary.entries.is_empty());
        assert!(config.dependency_requirements.entries.is_empty());
        assert!(config.is_empty());
    }
}
