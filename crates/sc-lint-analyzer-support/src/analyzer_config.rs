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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct AnalyzerName(&'static str);

impl AnalyzerName {
    pub const fn new(value: &'static str) -> Self {
        assert!(!value.is_empty(), "analyzer name must not be empty");
        Self(value)
    }

    pub const fn as_str(self) -> &'static str {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct CatalogRuleId(&'static str);

impl CatalogRuleId {
    pub const fn new(value: &'static str) -> Self {
        assert!(!value.is_empty(), "rule id must not be empty");
        Self(value)
    }

    pub const fn as_str(self) -> &'static str {
        self.0
    }
}

/// Rule ids, rule families, and analyzer-specific keys published by an analyzer.
#[derive(Debug, Clone, Copy)]
pub struct RuleCatalog {
    pub analyzer: AnalyzerName,
    pub rules: &'static [CatalogRuleId],
    pub families: &'static [(&'static str, &'static [CatalogRuleId])],
    pub default_fail: &'static [CatalogRuleId],
    pub own_keys: &'static [&'static str],
}

#[derive(Debug, Clone, PartialEq)]
pub struct AnalyzerSettings {
    analyzer: AnalyzerName,
    pub enabled: bool,
    pub exclude_rules: BTreeSet<CatalogRuleId>,
    pub fail_rules: BTreeSet<CatalogRuleId>,
    pub exclude_paths: Vec<String>,
    pub rule_filter: Option<BTreeSet<CatalogRuleId>>,
    own: toml::Table,
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
    #[error("{analyzer}: [{table}] must be a table")]
    NotATable {
        analyzer: &'static str,
        table: String,
    },
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
    /// Whether this error came from an invalid rule selector supplied by the caller.
    pub const fn is_selector(&self) -> bool {
        matches!(self, Self::Selector { .. })
    }

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
    let document = read_config_document(catalog.analyzer.as_str(), root, config)?;
    analyzer_settings_from(catalog, &document, rule_filter)
}

pub fn analyzer_settings_from(
    catalog: &RuleCatalog,
    document: &toml::Table,
    rule_filter: Option<&str>,
) -> Result<AnalyzerSettings, AnalyzerConfigError> {
    let analyzer = catalog.analyzer.as_str();
    let empty = toml::Table::new();
    let table = analyzer_table(document, analyzer)?.unwrap_or(&empty);
    validate_keys(table, catalog)?;
    let enabled = read_enabled(table, analyzer)?;
    let _reason = read_string(table, "reason", analyzer)?;

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
        analyzer: catalog.analyzer,
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
    let mut current = document;
    let mut path = String::new();
    for segment in ["tool", "sc-lint", "analyzers", analyzer] {
        if !path.is_empty() {
            path.push('.');
        }
        path.push_str(segment);
        match current.get(segment) {
            None => return Ok(None),
            Some(toml::Value::Table(table)) => current = table,
            Some(_) => {
                return Err(AnalyzerConfigError::NotATable {
                    analyzer,
                    table: path,
                });
            }
        }
    }
    Ok(Some(current))
}

fn validate_keys(table: &toml::Table, catalog: &RuleCatalog) -> Result<(), AnalyzerConfigError> {
    let analyzer = catalog.analyzer.as_str();
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

fn read_string<'a>(
    table: &'a toml::Table,
    key: &str,
    analyzer: &'static str,
) -> Result<Option<&'a str>, AnalyzerConfigError> {
    match table.get(key) {
        None => Ok(None),
        Some(toml::Value::String(value)) => Ok(Some(value.as_str())),
        Some(_) => Err(AnalyzerConfigError::WrongType {
            analyzer,
            key: key.to_owned(),
            expected: "a string",
        }),
    }
}

fn parse_rule_filter(
    catalog: &RuleCatalog,
    rule_filter: Option<&str>,
) -> Result<Option<BTreeSet<CatalogRuleId>>, AnalyzerConfigError> {
    let Some(raw) = rule_filter else {
        return Ok(None);
    };
    let selectors: Vec<String> = raw.split(',').map(|item| item.trim().to_owned()).collect();
    if selectors.iter().any(String::is_empty) {
        return Err(AnalyzerConfigError::Selector {
            analyzer: catalog.analyzer.as_str(),
            selector: raw.to_owned(),
        });
    }
    resolve(catalog, &selectors)
        .map(Some)
        .map_err(|selector| AnalyzerConfigError::Selector {
            analyzer: catalog.analyzer.as_str(),
            selector,
        })
}

fn read_strings(
    table: &toml::Table,
    key: &str,
    analyzer: &'static str,
) -> Result<Option<Vec<String>>, AnalyzerConfigError> {
    read_string_values(table.get(key), key, analyzer)
}

fn read_string_value(
    value: Option<&toml::Value>,
    key: &str,
    analyzer: &'static str,
) -> Result<Option<String>, AnalyzerConfigError> {
    match value {
        None => Ok(None),
        Some(toml::Value::String(value)) => Ok(Some(value.clone())),
        Some(_) => Err(AnalyzerConfigError::WrongType {
            analyzer,
            key: key.to_owned(),
            expected: "a string",
        }),
    }
}

fn read_string_values(
    value: Option<&toml::Value>,
    key: &str,
    analyzer: &'static str,
) -> Result<Option<Vec<String>>, AnalyzerConfigError> {
    let Some(value) = value else {
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

fn resolve(catalog: &RuleCatalog, selectors: &[String]) -> Result<BTreeSet<CatalogRuleId>, String> {
    let mut rules = BTreeSet::new();
    for selector in selectors {
        if let Some(rule) = catalog
            .rules
            .iter()
            .copied()
            .find(|rule| rule.as_str() == selector.as_str())
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
    pub fn own_string(&self, key: &str) -> Result<Option<String>, AnalyzerConfigError> {
        read_string_value(self.own.get(key), key, self.analyzer.as_str())
    }

    pub fn own_strings(&self, key: &str) -> Result<Option<Vec<String>>, AnalyzerConfigError> {
        read_string_values(self.own.get(key), key, self.analyzer.as_str())
    }

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
                Some(selected) => selected.contains(&CatalogRuleId::new(id(&finding.rule_id))),
                None => !self
                    .exclude_rules
                    .contains(&CatalogRuleId::new(id(&finding.rule_id))),
            })
            .collect();
        let status = if kept.iter().any(|finding| {
            self.fail_rules
                .contains(&CatalogRuleId::new(id(&finding.rule_id)))
        }) {
            ReportStatus::Fail
        } else {
            ReportStatus::Pass
        };
        (kept, status)
    }
}

fn glob_match(pattern: &str, path: &str) -> bool {
    let pattern: Vec<&str> = pattern.split('/').collect();
    let path: Vec<&str> = path.split('/').collect();
    // `**` matches any run of path segments; inside a segment `*` matches any
    // run of bytes and `?` matches one byte.
    wildcard_match(
        &pattern,
        &path,
        |token| *token == "**",
        |token, segment| {
            wildcard_match(
                token.as_bytes(),
                segment.as_bytes(),
                |byte| *byte == b'*',
                |byte, actual| *byte == b'?' || byte == actual,
            )
        },
    )
}

/// Wildcard matching by dynamic programming over `text` positions, so the cost
/// is `O(pattern.len() * text.len())` steps with no backtracking. `is_star`
/// tokens match any run of `text` items, including none; every other token
/// consumes one item that `matches_one` accepts.
fn wildcard_match<P, T>(
    pattern: &[P],
    text: &[T],
    is_star: impl Fn(&P) -> bool,
    matches_one: impl Fn(&P, &T) -> bool,
) -> bool {
    // previous[index] is true when the tokens seen so far match text[..index].
    let mut previous = vec![false; text.len() + 1];
    previous[0] = true;
    for token in pattern {
        let mut current = vec![false; text.len() + 1];
        if is_star(token) {
            current[0] = previous[0];
            for index in 1..=text.len() {
                current[index] = previous[index] || current[index - 1];
            }
        } else {
            for index in 1..=text.len() {
                current[index] = previous[index - 1] && matches_one(token, &text[index - 1]);
            }
        }
        previous = current;
    }
    previous[text.len()]
}
