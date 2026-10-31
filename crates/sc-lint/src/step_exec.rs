//! Shared parsing for per-step child environment and working directory.

use std::collections::BTreeMap;
use std::path::Component;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StepExecError {
    pub(crate) field: String,
    pub(crate) message: String,
}

pub(crate) fn parse_env(
    name: &str,
    value: Option<&toml::Value>,
) -> Result<BTreeMap<String, String>, StepExecError> {
    let Some(value) = value else {
        return Ok(BTreeMap::new());
    };
    let invalid = || StepExecError {
        field: "env".to_owned(),
        message: format!("selector `{name}` env must be a table of strings"),
    };
    let toml::Value::Table(table) = value else {
        return Err(invalid());
    };
    let mut env = BTreeMap::new();
    for (key, value) in table {
        if key.is_empty() || key.contains('=') || key.contains('\0') {
            return Err(StepExecError {
                field: format!("env.{key}"),
                message: format!(
                    "selector `{name}` env key `{key}` is empty or contains `=` or NUL"
                ),
            });
        }
        let toml::Value::String(value) = value else {
            return Err(StepExecError {
                field: format!("env.{key}"),
                message: format!("selector `{name}` env must be a table of strings"),
            });
        };
        env.insert(key.clone(), value.clone());
    }
    Ok(env)
}

pub(crate) fn parse_cwd(
    name: &str,
    value: Option<&toml::Value>,
) -> Result<Option<PathBuf>, StepExecError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let relative = value.as_str().map(PathBuf::from).filter(|path| {
        !path.as_os_str().is_empty()
            && path
                .components()
                .all(|component| matches!(component, Component::Normal(_) | Component::CurDir))
    });
    relative.map(Some).ok_or_else(|| StepExecError {
        field: "cwd".to_owned(),
        message: format!(
            "selector `{name}` cwd must be a non-empty repo-root-relative path without `..`"
        ),
    })
}
