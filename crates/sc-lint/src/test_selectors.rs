//! Consumer test selectors (ADR-020 decision 7).
//!
//! `[tool.sc-lint.test]` is exactly one of two forms per file: the existing
//! `[[tool.sc-lint.test]]` row array, where each row is a command selector
//! named after the row, or a table with an optional `default` and
//! `selectors.<name>` entries (a `command` argv or a `group` of selector names,
//! plus `env`, `cwd`, `platforms` and `after_all`). Table entries iterate in
//! sorted name order (`toml::Table` is sorted); group members keep their listed
//! order; rows keep declaration order.

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::path::Component;
use std::path::Path;
use std::path::PathBuf;

use serde_json::Value;
use serde_json::json;

use crate::CliError;
use crate::config::CONSUMER_SELECTOR_ALL;
use crate::config::consumer_profile_error;
use crate::profile_plan::ConsumerProfileStep;
use crate::profile_plan::Platform;
use crate::scheduler::ProfileCode;
use crate::workflow::Lane;
use crate::workflow::StepPlan;

const TEST_PROFILE: &str = "test";
const TABLE_KEYS: [&str; 2] = ["default", "selectors"];
const SELECTOR_KEYS: [&str; 6] = ["command", "group", "env", "cwd", "platforms", "after_all"];
const GROUP_FORBIDDEN_KEYS: [&str; 3] = ["env", "cwd", "platforms"];

/// A non-blank selector name.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct SelectorName(String);

impl SelectorName {
    /// Parses a selector name; blank names are rejected.
    pub(crate) fn parse(value: String) -> Option<Self> {
        (!value.trim().is_empty()).then_some(Self(value))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

/// What a selector runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SelectorBody {
    /// One command argv.
    Command(Vec<String>),
    /// Other selectors, in listed order.
    Group(Vec<SelectorName>),
}

/// One declared test selector.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TestSelector {
    name: SelectorName,
    body: SelectorBody,
    env: BTreeMap<String, String>,
    cwd: Option<PathBuf>,
    platforms: Vec<Platform>,
    after_all: bool,
}

impl TestSelector {
    pub(crate) fn name(&self) -> &str {
        self.name.as_str()
    }

    /// Whether this selector runs on `host` (no `platforms` means every host).
    pub(crate) fn runs_on(&self, host: Platform) -> bool {
        self.platforms.is_empty() || self.platforms.contains(&host)
    }

    /// The `data.steps[]` row of a selector skipped on `host`.
    pub(crate) fn skipped_row(&self, host: Platform) -> Value {
        json!({
            "name": self.name(),
            "status": "skipped",
            "reason": "platform_mismatch",
            "host": host.as_str(),
            "platforms": self.platform_names(),
        })
    }

    /// The executable step of a command selector with `args` appended.
    pub(crate) fn step_plan(&self, args: &[String]) -> StepPlan {
        let SelectorBody::Command(argv) = &self.body else {
            unreachable!("only command selectors are expanded into steps")
        };
        let (program, rest) = argv.split_first().expect("validated non-empty command");
        let mut plan = StepPlan::new(
            self.name(),
            TEST_PROFILE,
            Lane::for_program(OsStr::new(program)),
            program,
            rest.iter().chain(args),
        )
        .with_after_all(self.after_all);
        for (key, value) in &self.env {
            plan = plan.with_env(key, value);
        }
        match &self.cwd {
            Some(cwd) => plan.with_cwd(cwd.clone()),
            None => plan,
        }
    }

    fn platform_names(&self) -> Vec<&'static str> {
        self.platforms
            .iter()
            .map(|platform| platform.as_str())
            .collect()
    }

    fn list_json(&self) -> Value {
        let mut row = json!({
            "name": self.name(),
            "kind": match self.body {
                SelectorBody::Command(_) => "command",
                SelectorBody::Group(_) => "group",
            },
            "platforms": self.platform_names(),
            "cwd": self.cwd.as_ref().map(|cwd| cwd.display().to_string()),
            "env_keys": self.env.keys().collect::<Vec<_>>(),
            "after_all": self.after_all,
        });
        let (key, value) = match &self.body {
            SelectorBody::Command(argv) => ("command", json!(argv)),
            SelectorBody::Group(members) => (
                "group",
                json!(members.iter().map(SelectorName::as_str).collect::<Vec<_>>()),
            ),
        };
        if let Some(fields) = row.as_object_mut() {
            fields.insert(key.to_string(), value);
        }
        row
    }
}

/// The validated `[tool.sc-lint.test]` declaration of one config file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TestSelectors {
    entries: Vec<TestSelector>,
    default: Option<SelectorName>,
    legacy: bool,
    source: PathBuf,
}

/// What `sc-lint test` was asked to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TestRequest {
    selector: Option<String>,
    args: Vec<String>,
    list: bool,
}

impl TestRequest {
    pub(crate) const fn new(selector: Option<String>, args: Vec<String>, list: bool) -> Self {
        Self {
            selector,
            args,
            list,
        }
    }

    pub(crate) fn selector(&self) -> Option<&str> {
        self.selector.as_deref()
    }

    pub(crate) fn args(&self) -> &[String] {
        &self.args
    }

    pub(crate) const fn list(&self) -> bool {
        self.list
    }

    /// A bare `sc-lint test`.
    #[cfg(test)]
    pub(crate) const fn all() -> Self {
        Self::new(None, Vec::new(), false)
    }

    /// `sc-lint test <name>`.
    #[cfg(test)]
    pub(crate) fn named(name: &str) -> Self {
        Self::new(Some(name.to_string()), Vec::new(), false)
    }
}

impl TestSelectors {
    /// Normalizes `[[tool.sc-lint.test]]` rows to command selectors in
    /// declaration order.
    pub(crate) fn from_rows(steps: Vec<ConsumerProfileStep>, source: PathBuf) -> Self {
        let entries = steps
            .into_iter()
            .map(|step| TestSelector {
                name: SelectorName(step.name().to_string()),
                body: SelectorBody::Command(step.command().to_vec()),
                env: BTreeMap::new(),
                cwd: None,
                platforms: Vec::new(),
                after_all: step.after_all(),
            })
            .collect();
        Self {
            entries,
            default: None,
            legacy: true,
            source,
        }
    }

    /// Validates the table form; the first error wins, in this order: unknown
    /// keys, per-selector shape, undeclared names, group cycles.
    #[expect(
        clippy::result_large_err,
        reason = "Selector validation returns the shared top-level CliError contract."
    )]
    pub(crate) fn parse_table(table: &toml::Table, source: &Path) -> Result<Self, CliError> {
        let malformed = |message: String| consumer_profile_error(source, TEST_PROFILE, message);
        let raw = raw_selectors(table, &malformed)?;
        let mut entries = Vec::with_capacity(raw.len());
        for (name, fields) in raw {
            entries.push(parse_selector(name, fields, source, &malformed)?);
        }
        let default = match table.get("default") {
            None => None,
            Some(toml::Value::String(name)) => Some(SelectorName(name.clone())),
            Some(_) => {
                return Err(malformed(
                    "`tool.sc-lint.test.default` must be a selector name string".to_string(),
                ));
            }
        };
        let selectors = Self {
            entries,
            default,
            legacy: false,
            source: source.to_path_buf(),
        };
        selectors.check_references()?;
        selectors.check_cycles()?;
        Ok(selectors)
    }

    /// The command selectors `requested` runs, in order, each at most once.
    ///
    /// Rows form: `None` or `all` selects every row. Table form: `None`
    /// selects `default`; `all` is an ordinary name.
    #[expect(
        clippy::result_large_err,
        reason = "Selector resolution returns the shared top-level CliError contract."
    )]
    pub(crate) fn expand(&self, requested: Option<&str>) -> Result<Vec<&TestSelector>, CliError> {
        if self.legacy && requested.is_none_or(|name| name == CONSUMER_SELECTOR_ALL) {
            return Ok(self.entries.iter().collect());
        }
        let name = match (requested, &self.default) {
            (Some(name), _) => name,
            (None, Some(default)) => default.as_str(),
            (None, None) => {
                return Err(self.unknown(
                    format!(
                        "no selector was named and {} declares no default",
                        self.source.display()
                    ),
                    None,
                ));
            }
        };
        let selector = self.find(name).ok_or_else(|| {
            self.unknown(
                format!(
                    "selector '{name}' is not declared in {}",
                    self.source.display()
                ),
                Some(name),
            )
        })?;
        let mut expanded = Vec::new();
        self.push_expanded(selector, &mut expanded, &mut BTreeSet::new());
        Ok(expanded)
    }

    /// Rejects forwarded arguments unless `request` names a command selector.
    #[expect(
        clippy::result_large_err,
        reason = "Usage errors use the shared top-level CliError contract."
    )]
    pub(crate) fn check_forwarded_args(&self, request: &TestRequest) -> Result<(), CliError> {
        if request.args().is_empty() {
            return Ok(());
        }
        let name = self.selected_name(request.selector());
        let is_group = self
            .find(&name)
            .is_none_or(|selector| matches!(selector.body, SelectorBody::Group(_)));
        if is_group {
            return Err(CliError::usage(format!(
                "group selector '{name}' does not accept forwarded arguments"
            ))
            .with_suggested_action("name a command selector to forward arguments"));
        }
        Ok(())
    }

    /// The selector name a run reports: the requested name, else `default`
    /// (table form) or `all` (rows form).
    pub(crate) fn selected_name(&self, requested: Option<&str>) -> String {
        requested
            .or_else(|| self.default.as_ref().map(SelectorName::as_str))
            .unwrap_or(CONSUMER_SELECTOR_ALL)
            .to_string()
    }

    /// `data` of `sc-lint test --list`.
    pub(crate) fn list_json(&self) -> Value {
        json!({
            "status": "pass",
            "profile": TEST_PROFILE,
            "default": self.default.as_ref().map(SelectorName::as_str),
            "selectors": self.entries.iter().map(TestSelector::list_json).collect::<Vec<_>>(),
        })
    }

    fn find(&self, name: &str) -> Option<&TestSelector> {
        self.entries.iter().find(|selector| selector.name() == name)
    }

    fn names(&self) -> Vec<&str> {
        self.entries.iter().map(TestSelector::name).collect()
    }

    fn push_expanded<'a>(
        &'a self,
        selector: &'a TestSelector,
        expanded: &mut Vec<&'a TestSelector>,
        seen: &mut BTreeSet<&'a str>,
    ) {
        match &selector.body {
            SelectorBody::Command(_) => {
                if seen.insert(selector.name()) {
                    expanded.push(selector);
                }
            }
            SelectorBody::Group(members) => {
                for member in members {
                    if let Some(next) = self.find(member.as_str()) {
                        self.push_expanded(next, expanded, seen);
                    }
                }
            }
        }
    }

    fn unknown(&self, cause: String, selector: Option<&str>) -> CliError {
        ProfileCode::UnknownSelector
            .config_error(cause, &self.source)
            .with_detail("selector", json!(selector))
            .with_detail("available", json!(self.names()))
    }

    #[expect(
        clippy::result_large_err,
        reason = "Selector validation returns the shared top-level CliError contract."
    )]
    fn check_references(&self) -> Result<(), CliError> {
        let path = self.source.display();
        for selector in &self.entries {
            let SelectorBody::Group(members) = &selector.body else {
                continue;
            };
            if let Some(member) = members
                .iter()
                .find(|member| self.find(member.as_str()).is_none())
            {
                return Err(self.unknown(
                    format!(
                        "selector '{}' listed by group '{}' is not declared in {path}",
                        member.as_str(),
                        selector.name()
                    ),
                    Some(member.as_str()),
                ));
            }
        }
        match &self.default {
            Some(default) if self.find(default.as_str()).is_none() => Err(self.unknown(
                format!(
                    "default names selector '{}', which is not declared in {path}",
                    default.as_str()
                ),
                Some(default.as_str()),
            )),
            _ => Ok(()),
        }
    }

    /// Depth-first over selectors in sorted order, members in listed order;
    /// the first cycle found is reported from its first repeated selector.
    #[expect(
        clippy::result_large_err,
        reason = "Selector validation returns the shared top-level CliError contract."
    )]
    fn check_cycles(&self) -> Result<(), CliError> {
        let mut finished = BTreeSet::new();
        for selector in &self.entries {
            if let Some(cycle) = self.find_cycle(selector, &mut Vec::new(), &mut finished) {
                return Err(ProfileCode::GroupCycle.config_error(
                    format!(
                        "selector group cycle: {} in {}",
                        cycle.join(" -> "),
                        self.source.display()
                    ),
                    &self.source,
                ));
            }
        }
        Ok(())
    }

    fn find_cycle<'a>(
        &'a self,
        selector: &'a TestSelector,
        path: &mut Vec<&'a str>,
        finished: &mut BTreeSet<&'a str>,
    ) -> Option<Vec<&'a str>> {
        if let Some(start) = path.iter().position(|name| *name == selector.name()) {
            let mut cycle = path[start..].to_vec();
            cycle.push(selector.name());
            return Some(cycle);
        }
        if finished.contains(selector.name()) {
            return None;
        }
        path.push(selector.name());
        if let SelectorBody::Group(members) = &selector.body {
            for member in members {
                let next = self.find(member.as_str())?;
                if let Some(cycle) = self.find_cycle(next, path, finished) {
                    return Some(cycle);
                }
            }
        }
        path.pop();
        finished.insert(selector.name());
        None
    }
}

/// Step 1 of table validation: exact keys of the table and of every selector.
#[expect(
    clippy::result_large_err,
    reason = "Selector validation returns the shared top-level CliError contract."
)]
fn raw_selectors<'a>(
    table: &'a toml::Table,
    malformed: &dyn Fn(String) -> CliError,
) -> Result<Vec<(&'a String, &'a toml::Table)>, CliError> {
    if let Some(key) = table.keys().find(|key| !TABLE_KEYS.contains(&key.as_str())) {
        return Err(malformed(format!(
            "`tool.sc-lint.test` has unknown key `{key}`; use `default` and `selectors`"
        )));
    }
    let selectors = match table.get("selectors") {
        Some(toml::Value::Table(selectors)) if !selectors.is_empty() => selectors,
        None | Some(toml::Value::Table(_)) => {
            return Err(malformed(
                "consumer test profile must contain at least one selector".to_string(),
            ));
        }
        Some(_) => {
            return Err(malformed(
                "`tool.sc-lint.test.selectors` must be a table of selectors".to_string(),
            ));
        }
    };
    let mut raw = Vec::with_capacity(selectors.len());
    for (name, value) in selectors {
        let toml::Value::Table(fields) = value else {
            return Err(malformed(format!("selector `{name}` must be a table")));
        };
        if let Some(key) = fields
            .keys()
            .find(|key| !SELECTOR_KEYS.contains(&key.as_str()))
        {
            return Err(malformed(format!(
                "selector `{name}` has unknown key `{key}`; selectors accept {}",
                SELECTOR_KEYS.join(", ")
            )));
        }
        raw.push((name, fields));
    }
    Ok(raw)
}

/// Step 2 of table validation for one selector.
#[expect(
    clippy::result_large_err,
    reason = "Selector validation returns the shared top-level CliError contract."
)]
fn parse_selector(
    name: &str,
    fields: &toml::Table,
    source: &Path,
    malformed: &dyn Fn(String) -> CliError,
) -> Result<TestSelector, CliError> {
    let selector_name = SelectorName::parse(name.to_string())
        .ok_or_else(|| malformed("selector names must not be blank".to_string()))?;
    let body = match (fields.get("command"), fields.get("group")) {
        (Some(command), None) => SelectorBody::Command(parse_command(name, command, malformed)?),
        (None, Some(group)) => SelectorBody::Group(parse_group(name, group, fields, malformed)?),
        _ => {
            return Err(malformed(format!(
                "selector `{name}` must declare exactly one of `command` or `group`"
            )));
        }
    };
    let env = parse_env(name, fields.get("env"), malformed)?;
    let cwd = parse_cwd(name, fields.get("cwd"), malformed)?;
    let platforms = parse_platforms(name, fields.get("platforms"), malformed)?;
    let after_all = parse_after_all(name, &body, fields.get("after_all"), source)?;
    Ok(TestSelector {
        name: selector_name,
        body,
        env,
        cwd,
        platforms,
        after_all,
    })
}

#[expect(
    clippy::result_large_err,
    reason = "Selector validation returns the shared top-level CliError contract."
)]
fn parse_command(
    name: &str,
    value: &toml::Value,
    malformed: &dyn Fn(String) -> CliError,
) -> Result<Vec<String>, CliError> {
    let argv = string_array(value).ok_or_else(|| {
        malformed(format!(
            "selector `{name}` command must be an array of strings"
        ))
    })?;
    if argv.is_empty() {
        return Err(malformed(format!("selector `{name}` has an empty command")));
    }
    if argv.iter().any(|argument| argument.trim().is_empty()) {
        return Err(malformed(format!(
            "selector `{name}` has a blank command argument"
        )));
    }
    Ok(argv)
}

#[expect(
    clippy::result_large_err,
    reason = "Selector validation returns the shared top-level CliError contract."
)]
fn parse_group(
    name: &str,
    value: &toml::Value,
    fields: &toml::Table,
    malformed: &dyn Fn(String) -> CliError,
) -> Result<Vec<SelectorName>, CliError> {
    let members = string_array(value).ok_or_else(|| {
        malformed(format!(
            "selector `{name}` group must be an array of selector names"
        ))
    })?;
    if members.is_empty() {
        return Err(malformed(format!(
            "group selector `{name}` lists no selectors"
        )));
    }
    if let Some(key) = GROUP_FORBIDDEN_KEYS
        .iter()
        .find(|key| fields.contains_key(**key))
    {
        return Err(malformed(format!(
            "group selector `{name}` declares `{key}`; only command selectors accept env, cwd and platforms"
        )));
    }
    members
        .into_iter()
        .map(|member| {
            SelectorName::parse(member)
                .ok_or_else(|| malformed(format!("group selector `{name}` lists a blank name")))
        })
        .collect()
}

#[expect(
    clippy::result_large_err,
    reason = "Selector validation returns the shared top-level CliError contract."
)]
fn parse_env(
    name: &str,
    value: Option<&toml::Value>,
    malformed: &dyn Fn(String) -> CliError,
) -> Result<BTreeMap<String, String>, CliError> {
    let Some(value) = value else {
        return Ok(BTreeMap::new());
    };
    let invalid = || malformed(format!("selector `{name}` env must be a table of strings"));
    let toml::Value::Table(table) = value else {
        return Err(invalid());
    };
    let mut env = BTreeMap::new();
    for (key, value) in table {
        if key.is_empty() || key.contains('=') || key.contains('\0') {
            return Err(malformed(format!(
                "selector `{name}` env key `{key}` is empty or contains `=` or NUL"
            )));
        }
        let toml::Value::String(value) = value else {
            return Err(invalid());
        };
        env.insert(key.clone(), value.clone());
    }
    Ok(env)
}

#[expect(
    clippy::result_large_err,
    reason = "Selector validation returns the shared top-level CliError contract."
)]
fn parse_cwd(
    name: &str,
    value: Option<&toml::Value>,
    malformed: &dyn Fn(String) -> CliError,
) -> Result<Option<PathBuf>, CliError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let relative = value.as_str().map(PathBuf::from).filter(|path| {
        !path.as_os_str().is_empty()
            && path
                .components()
                .all(|component| matches!(component, Component::Normal(_) | Component::CurDir))
    });
    relative.map(Some).ok_or_else(|| {
        malformed(format!(
            "selector `{name}` cwd must be a non-empty repo-root-relative path without `..`"
        ))
    })
}

#[expect(
    clippy::result_large_err,
    reason = "Selector validation returns the shared top-level CliError contract."
)]
fn parse_platforms(
    name: &str,
    value: Option<&toml::Value>,
    malformed: &dyn Fn(String) -> CliError,
) -> Result<Vec<Platform>, CliError> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    let invalid = || {
        malformed(format!(
            "selector `{name}` platforms must list only linux, macos or windows"
        ))
    };
    let names = string_array(value).ok_or_else(invalid)?;
    names
        .iter()
        .map(|platform| Platform::parse(platform).ok_or_else(invalid))
        .collect()
}

#[expect(
    clippy::result_large_err,
    reason = "Selector validation returns the shared top-level CliError contract."
)]
fn parse_after_all(
    name: &str,
    body: &SelectorBody,
    value: Option<&toml::Value>,
    source: &Path,
) -> Result<bool, CliError> {
    let path = source.display();
    match (body, value) {
        (_, None) => Ok(false),
        (SelectorBody::Group(_), Some(_)) => Err(ProfileCode::InvalidAfterAll.config_error(
            format!("group selector '{name}' in {path} declares after_all"),
            source,
        )),
        (SelectorBody::Command(_), Some(toml::Value::Boolean(after_all))) => Ok(*after_all),
        (SelectorBody::Command(_), Some(_)) => Err(ProfileCode::InvalidAfterAll.config_error(
            format!("selector '{name}' in {path} declares a non-boolean after_all"),
            source,
        )),
    }
}

/// The elements of an array of strings, `None` for any other shape.
fn string_array(value: &toml::Value) -> Option<Vec<String>> {
    value
        .as_array()?
        .iter()
        .map(|item| item.as_str().map(str::to_string))
        .collect()
}
