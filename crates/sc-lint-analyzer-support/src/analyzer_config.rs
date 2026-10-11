use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use sc_lint_schema::Finding;
use sc_lint_schema::ReportStatus;
use serde::Serialize;
use thiserror::Error;

const COMMON_KEYS: &[&str] = &[
    "enabled",
    "reason",
    "exclude_rules",
    "fail_rules",
    "exclude_paths",
];

/// Rule ids, rule families, and analyzer-specific keys published by an analyzer.
#[derive(Debug, Clone, Copy)]
pub struct RuleCatalog {
    pub analyzer: &'static str,
    pub rules: &'static [&'static str],
    pub families: &'static [(&'static str, &'static [&'static str])],
    pub default_fail: &'static [&'static str],
    pub own_keys: &'static [&'static str],
}

#[derive(Debug, Clone, PartialEq)]
pub struct AnalyzerSettings {
    pub enabled: bool,
    pub exclude_rules: BTreeSet<&'static str>,
    pub fail_rules: BTreeSet<&'static str>,
    pub exclude_paths: Vec<String>,
    pub rule_filter: Option<BTreeSet<&'static str>>,
    own: toml::Table,
}

impl AnalyzerSettings {
    pub fn own_value(&self, key: &str) -> Option<&toml::Value> {
        self.own.get(key)
    }
}

#[derive(Debug, Error)]
pub enum AnalyzerConfigError {
    #[error("{analyzer}: unknown rule selector `{selector}`")]
    Selector {
        analyzer: &'static str,
        selector: String,
    },
    #[error("{analyzer}: failed to read {path}: {source}")]
    Read {
        analyzer: &'static str,
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("{analyzer}: failed to parse {path}: {source}")]
    Parse {
        analyzer: &'static str,
        path: String,
        #[source]
        source: Box<toml::de::Error>,
    },
    #[error("{analyzer}: [tool.sc-lint.analyzers.{analyzer}] must be a table")]
    NotATable { analyzer: &'static str },
    #[error("{analyzer}: unknown key `{key}` in [tool.sc-lint.analyzers.{analyzer}]")]
    UnknownKey { analyzer: &'static str, key: String },
    #[error("{analyzer}: `{key}` in [tool.sc-lint.analyzers.{analyzer}] must be {expected}")]
    WrongType {
        analyzer: &'static str,
        key: String,
        expected: &'static str,
    },
    #[error("{analyzer}: unknown rule `{rule}` in {key}")]
    UnknownRule {
        analyzer: &'static str,
        key: &'static str,
        rule: String,
    },
}

impl AnalyzerConfigError {
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Selector { .. } => "ANALYZER_CONFIG.SELECTOR",
            Self::Read { .. } => "ANALYZER_CONFIG.READ",
            Self::Parse { .. } => "ANALYZER_CONFIG.PARSE",
            Self::NotATable { .. } => "ANALYZER_CONFIG.NOT_A_TABLE",
            Self::UnknownKey { .. } => "ANALYZER_CONFIG.UNKNOWN_KEY",
            Self::WrongType { .. } => "ANALYZER_CONFIG.WRONG_TYPE",
            Self::UnknownRule { .. } => "ANALYZER_CONFIG.UNKNOWN_RULE",
        }
    }
}

pub fn read_config_document(
    analyzer: &'static str,
    root: &Path,
    config: Option<&Path>,
) -> Result<toml::Table, AnalyzerConfigError> {
    let path = match config {
        Some(path) => path.to_path_buf(),
        None => match Some(root.join("sc-lint.toml")).filter(|path| path.exists()) {
            Some(path) => path,
            None => return Ok(toml::Table::new()),
        },
    };
    let text = fs::read_to_string(&path).map_err(|error| AnalyzerConfigError::Read {
        analyzer,
        path: path.display().to_string(),
        source: error,
    })?;
    toml::from_str::<toml::Table>(&text).map_err(|error| AnalyzerConfigError::Parse {
        analyzer,
        path: path.display().to_string(),
        source: Box::new(error),
    })
}

pub fn load_analyzer_settings(
    catalog: &RuleCatalog,
    root: &Path,
    config: Option<&Path>,
    rule_filter: Option<&str>,
) -> Result<AnalyzerSettings, AnalyzerConfigError> {
    let document = read_config_document(catalog.analyzer, root, config)?;
    analyzer_settings_from(catalog, &document, rule_filter)
}

pub fn analyzer_settings_from(
    catalog: &RuleCatalog,
    document: &toml::Table,
    rule_filter: Option<&str>,
) -> Result<AnalyzerSettings, AnalyzerConfigError> {
    let analyzer = catalog.analyzer;
    let empty = toml::Table::new();
    let table = analyzer_table(document, analyzer)?.unwrap_or(&empty);
    validate_keys(table, catalog)?;
    let enabled = read_enabled(table, analyzer)?;

    let exclude_rules = resolve(
        catalog,
        &read_strings(table, "exclude_rules", analyzer)?.unwrap_or_default(),
    )
    .map_err(|rule| AnalyzerConfigError::UnknownRule {
        analyzer,
        key: "exclude_rules",
        rule,
    })?;
    let fail_rules = match read_strings(table, "fail_rules", analyzer)? {
        Some(list) => resolve(catalog, &list).map_err(|rule| AnalyzerConfigError::UnknownRule {
            analyzer,
            key: "fail_rules",
            rule,
        })?,
        None => catalog.default_fail.iter().copied().collect(),
    };
    let exclude_paths = read_strings(table, "exclude_paths", analyzer)?.unwrap_or_default();
    let rule_filter = parse_rule_filter(catalog, rule_filter)?;

    Ok(AnalyzerSettings {
        enabled,
        exclude_rules,
        fail_rules,
        exclude_paths,
        rule_filter,
        own: table
            .iter()
            .filter(|(key, _)| !COMMON_KEYS.contains(&key.as_str()))
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect(),
    })
}

fn analyzer_table<'a>(
    document: &'a toml::Table,
    analyzer: &'static str,
) -> Result<Option<&'a toml::Table>, AnalyzerConfigError> {
    match document
        .get("tool")
        .and_then(|tool| tool.get("sc-lint"))
        .and_then(|sc_lint| sc_lint.get("analyzers"))
        .and_then(|analyzers| analyzers.get(analyzer))
    {
        None => Ok(None),
        Some(toml::Value::Table(table)) => Ok(Some(table)),
        Some(_) => Err(AnalyzerConfigError::NotATable { analyzer }),
    }
}

fn validate_keys(table: &toml::Table, catalog: &RuleCatalog) -> Result<(), AnalyzerConfigError> {
    let analyzer = catalog.analyzer;
    if let Some(key) = table.keys().find(|key| {
        !COMMON_KEYS.contains(&key.as_str()) && !catalog.own_keys.contains(&key.as_str())
    }) {
        return Err(AnalyzerConfigError::UnknownKey {
            analyzer,
            key: key.clone(),
        });
    }
    Ok(())
}

fn read_enabled(table: &toml::Table, analyzer: &'static str) -> Result<bool, AnalyzerConfigError> {
    match table.get("enabled") {
        None => Ok(true),
        Some(toml::Value::Boolean(enabled)) => Ok(*enabled),
        Some(_) => Err(AnalyzerConfigError::WrongType {
            analyzer,
            key: "enabled".to_owned(),
            expected: "a boolean",
        }),
    }
}

fn parse_rule_filter(
    catalog: &RuleCatalog,
    rule_filter: Option<&str>,
) -> Result<Option<BTreeSet<&'static str>>, AnalyzerConfigError> {
    let Some(raw) = rule_filter else {
        return Ok(None);
    };
    let selectors: Vec<String> = raw.split(',').map(|item| item.trim().to_owned()).collect();
    if selectors.iter().any(String::is_empty) {
        return Err(AnalyzerConfigError::Selector {
            analyzer: catalog.analyzer,
            selector: raw.to_owned(),
        });
    }
    resolve(catalog, &selectors)
        .map(Some)
        .map_err(|selector| AnalyzerConfigError::Selector {
            analyzer: catalog.analyzer,
            selector,
        })
}

fn read_strings(
    table: &toml::Table,
    key: &str,
    analyzer: &'static str,
) -> Result<Option<Vec<String>>, AnalyzerConfigError> {
    let Some(value) = table.get(key) else {
        return Ok(None);
    };
    let invalid = || AnalyzerConfigError::WrongType {
        analyzer,
        key: key.to_owned(),
        expected: "an array of strings",
    };
    let toml::Value::Array(items) = value else {
        return Err(invalid());
    };
    items
        .iter()
        .map(|item| match item {
            toml::Value::String(text) => Ok(text.clone()),
            _ => Err(invalid()),
        })
        .collect::<Result<Vec<_>, _>>()
        .map(Some)
}

fn resolve(catalog: &RuleCatalog, selectors: &[String]) -> Result<BTreeSet<&'static str>, String> {
    let mut rules = BTreeSet::new();
    for selector in selectors {
        if let Some(rule) = catalog
            .rules
            .iter()
            .copied()
            .find(|rule| *rule == selector.as_str())
        {
            rules.insert(rule);
        } else if let Some((_, members)) = catalog
            .families
            .iter()
            .find(|(family, _)| *family == selector.as_str())
        {
            rules.extend(members.iter().copied());
        } else {
            return Err(selector.clone());
        }
    }
    Ok(rules)
}

impl AnalyzerSettings {
    pub fn path_excluded(&self, relative: &str) -> bool {
        self.exclude_paths
            .iter()
            .any(|pattern| glob_match(pattern, relative))
    }

    pub fn apply<R: Serialize>(
        &self,
        findings: Vec<Finding<R>>,
        id: impl Fn(&R) -> &'static str,
    ) -> (Vec<Finding<R>>, ReportStatus) {
        let kept: Vec<Finding<R>> = findings
            .into_iter()
            .filter(|finding| match &self.rule_filter {
                Some(selected) => selected.contains(id(&finding.rule_id)),
                None => !self.exclude_rules.contains(id(&finding.rule_id)),
            })
            .collect();
        let status = if kept
            .iter()
            .any(|finding| self.fail_rules.contains(id(&finding.rule_id)))
        {
            ReportStatus::Fail
        } else {
            ReportStatus::Pass
        };
        (kept, status)
    }
}

fn glob_match(pattern: &str, path: &str) -> bool {
    fn matches_segments(pattern: &[&str], path: &[&str]) -> bool {
        match pattern.split_first() {
            None => path.is_empty(),
            Some((&"**", rest)) => {
                (0..=path.len()).any(|skip| matches_segments(rest, &path[skip..]))
            }
            Some((head, rest)) => path.split_first().is_some_and(|(first, tail)| {
                matches_segment(head.as_bytes(), first.as_bytes()) && matches_segments(rest, tail)
            }),
        }
    }
    fn matches_segment(pattern: &[u8], text: &[u8]) -> bool {
        match (pattern.split_first(), text.split_first()) {
            (None, None) => true,
            (Some((b'*', rest)), _) => {
                matches_segment(rest, text)
                    || (!text.is_empty() && matches_segment(pattern, &text[1..]))
            }
            (Some((b'?', rest)), Some((_, text_rest))) => matches_segment(rest, text_rest),
            (Some((expected, rest)), Some((actual, text_rest))) => {
                expected == actual && matches_segment(rest, text_rest)
            }
            _ => false,
        }
    }

    matches_segments(
        &pattern.split('/').collect::<Vec<_>>(),
        &path.split('/').collect::<Vec<_>>(),
    )
}
