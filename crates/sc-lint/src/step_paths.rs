//! Config-root-relative environment paths. Values are lexical, never filesystem probes.
use crate::step_exec::{StepExecError, StepExecErrorKind, parse_env};
use std::collections::BTreeMap;
use std::path::Path;

type ParsedEnv = (BTreeMap<String, String>, Option<Vec<String>>);

pub(crate) fn execution_env(
    env: Option<&toml::Value>,
    paths: Option<&toml::Value>,
    config: &Path,
) -> Result<ParsedEnv, StepExecError> {
    let mut env = parse_env(env)?;
    let declared = paths.is_some();
    let paths = parse_env(paths).map_err(|mut error| {
        error.field = error.field.replacen("env", "env_paths", 1);
        error.cause = error.cause.replacen("env", "env_paths", 1);
        error
    })?;
    let keys = declared.then(|| paths.keys().cloned().collect());
    for (key, value) in paths {
        if value.is_empty() || value.contains('\0') {
            return Err(StepExecError {
                kind: StepExecErrorKind::Env,
                field: format!("env_paths.{key}"),
                cause: "env_paths values must be non-empty strings without NUL".to_owned(),
            });
        }
        if env.contains_key(&key) {
            return Err(StepExecError {
                kind: StepExecErrorKind::DuplicateEnvKey,
                field: format!("env_paths.{key}"),
                cause: format!("environment key `{key}` is declared in both env and env_paths"),
            });
        }
        env.insert(
            key,
            resolve_path(config.parent().unwrap_or_else(|| Path::new("")), &value),
        );
    }
    Ok((env, keys))
}

pub(crate) fn resolve_path(root: &Path, value: &str) -> String {
    if Path::new(value).is_absolute() {
        return value.to_owned();
    }
    // PathBuf::push replaces prefixes for Windows rooted/drive-relative paths.
    // Concatenation keeps every non-absolute value beneath the config root verbatim.
    let mut path = root.as_os_str().to_os_string();
    path.push(std::path::MAIN_SEPARATOR_STR);
    path.push(value);
    path.to_string_lossy().into_owned()
}
