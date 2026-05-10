use std::ffi::OsString;
use std::process::Command as ProcessCommand;

use serde_json::Map;
use serde_json::Value;
use serde_json::json;

use crate::CliError;
use crate::CliErrorKind;
use crate::command::CommandSuccess;
use crate::command::DispatchTelemetry;
use crate::config::LoadedConfig;

pub(crate) const ADAPTER_SCHEMA: &str = "sc-lint-python-v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PythonTool {
    LineCounts,
    IdentityLiterals,
    ViewFindings,
}

impl PythonTool {
    pub const fn tool_name(self) -> &'static str {
        match self {
            Self::LineCounts => "sc-lint-line-counts",
            Self::IdentityLiterals => "sc-lint-identity-literals",
            Self::ViewFindings => "sc-lint-view-findings",
        }
    }

    pub const fn script_relative_path(self) -> &'static str {
        match self {
            Self::LineCounts => ".just/lint_line_counts.py",
            Self::IdentityLiterals => ".just/lint_identity_literals.py",
            Self::ViewFindings => ".just/view_findings.py",
        }
    }

    pub const fn config_scope(self) -> &'static str {
        match self {
            Self::LineCounts => "line_counts",
            Self::IdentityLiterals => "identities",
            Self::ViewFindings => "view.findings",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AdapterResult {
    summary: String,
    data: Option<Value>,
    error: Option<CliError>,
}

#[expect(
    clippy::result_large_err,
    reason = "Python utility failures must remain normalized through the shared top-level CliError contract."
)]
pub(crate) fn run_python_tool(
    loaded_config: &LoadedConfig,
    tool: PythonTool,
) -> Result<CommandSuccess, CliError> {
    let repo_root = loaded_config.require_repo_root()?;
    let script_path = repo_root.join(tool.script_relative_path());
    let output = ProcessCommand::new(python_command())
        .current_dir(repo_root)
        .arg(&script_path)
        .arg("--root")
        .arg(repo_root)
        .args(
            loaded_config
                .config_path()
                .into_iter()
                .flat_map(|path| [OsString::from("--config"), path.as_os_str().to_os_string()]),
        )
        .arg("--json")
        .output()
        .map_err(|error| {
            CliError::backend_failure(format!("{} failed to start", tool.tool_name()))
                .with_source(error)
                .with_detail("tool", json!(tool.tool_name()))
                .with_detail("script", json!(tool.script_relative_path()))
        })?;

    let parsed = parse_adapter_output(tool, &output.stdout)?;
    match parsed.error {
        Some(error) => Err(error),
        None => {
            let mut data = parsed.data.unwrap_or_else(|| json!({}));
            if let Some(object) = data.as_object_mut() {
                object
                    .entry("adapter".to_string())
                    .or_insert_with(|| json!(ADAPTER_SCHEMA));
                object
                    .entry("config_scope".to_string())
                    .or_insert_with(|| json!(tool.config_scope()));
                object
                    .entry("script".to_string())
                    .or_insert_with(|| json!(tool.script_relative_path()));
                object
                    .entry("summary".to_string())
                    .or_insert_with(|| json!(parsed.summary));
            }
            let finding_count = data
                .get("findings")
                .and_then(Value::as_array)
                .map_or(0, std::vec::Vec::len);
            Ok(CommandSuccess::with_dispatch(
                data,
                DispatchTelemetry::new(tool.tool_name(), finding_count),
            ))
        }
    }
}

#[expect(
    clippy::result_large_err,
    reason = "Adapter normalization failures must use the shared top-level CliError contract."
)]
fn parse_adapter_output(tool: PythonTool, raw: &[u8]) -> Result<AdapterResult, CliError> {
    let text = std::str::from_utf8(raw).map_err(|error| {
        CliError::backend_protocol(format!(
            "{} returned non-utf8 adapter output",
            tool.tool_name()
        ))
        .with_source(error)
        .with_detail("tool", json!(tool.tool_name()))
    })?;
    let payload: Value = serde_json::from_str(text).map_err(|error| {
        CliError::backend_protocol(format!(
            "{} returned malformed adapter json",
            tool.tool_name()
        ))
        .with_source(error)
        .with_detail("tool", json!(tool.tool_name()))
    })?;
    let object = payload.as_object().ok_or_else(|| {
        CliError::backend_protocol(format!(
            "{} returned a non-object adapter payload",
            tool.tool_name()
        ))
        .with_detail("tool", json!(tool.tool_name()))
    })?;
    if object.get("adapter_schema").and_then(Value::as_str) != Some(ADAPTER_SCHEMA) {
        return Err(CliError::backend_protocol(format!(
            "{} returned an unknown adapter schema",
            tool.tool_name()
        ))
        .with_detail("tool", json!(tool.tool_name()))
        .with_detail("expected_schema", json!(ADAPTER_SCHEMA)));
    }
    let summary = object
        .get("summary")
        .and_then(Value::as_str)
        .unwrap_or("python adapter completed")
        .to_string();

    if object.get("ok").and_then(Value::as_bool) == Some(true) {
        return Ok(AdapterResult {
            summary,
            data: object.get("data").cloned(),
            error: None,
        });
    }

    let error_object = object
        .get("error")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            CliError::backend_protocol(format!(
                "{} returned an adapter failure without an error object",
                tool.tool_name()
            ))
            .with_detail("tool", json!(tool.tool_name()))
        })?;
    let kind = parse_error_kind(tool, error_object.get("kind"))?;
    let message = error_object
        .get("message")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            CliError::backend_protocol(format!(
                "{} returned an adapter failure without a message",
                tool.tool_name()
            ))
            .with_detail("tool", json!(tool.tool_name()))
        })?
        .to_string();
    let mut error = CliError::new(kind, message)
        .with_detail("tool", json!(tool.tool_name()))
        .with_detail("script", json!(tool.script_relative_path()));
    if let Some(details) = error_object.get("details").and_then(Value::as_object) {
        error = merge_details(error, details);
    }
    if let Some(action) = error_object.get("suggested_action").and_then(Value::as_str) {
        error = error.with_suggested_action(action);
    }
    Ok(AdapterResult {
        summary,
        data: None,
        error: Some(error),
    })
}

fn merge_details(mut error: CliError, details: &Map<String, Value>) -> CliError {
    for (key, value) in details {
        error = error.with_detail(key.clone(), value.clone());
    }
    error
}

#[expect(
    clippy::result_large_err,
    reason = "Adapter error-kind validation failures must use the shared top-level CliError contract."
)]
fn parse_error_kind(tool: PythonTool, value: Option<&Value>) -> Result<CliErrorKind, CliError> {
    let kind = value.and_then(Value::as_str).ok_or_else(|| {
        CliError::backend_protocol(format!(
            "{} returned an adapter failure without an error kind",
            tool.tool_name()
        ))
        .with_detail("tool", json!(tool.tool_name()))
    })?;
    match kind {
        "usage" => Ok(CliErrorKind::Usage),
        "config" => Ok(CliErrorKind::Config),
        "capability" => Ok(CliErrorKind::Capability),
        "backend_failure" => Ok(CliErrorKind::BackendFailure),
        "backend_protocol" => Ok(CliErrorKind::BackendProtocol),
        "internal" => Ok(CliErrorKind::Internal),
        _ => Err(CliError::backend_protocol(format!(
            "{} returned an unknown adapter error kind `{kind}`",
            tool.tool_name()
        ))
        .with_detail("tool", json!(tool.tool_name()))),
    }
}

fn python_command() -> OsString {
    if cfg!(windows) {
        OsString::from("python")
    } else {
        OsString::from("python3")
    }
}

pub(crate) fn adapter_kind_for_command(command_id: &str) -> Option<&'static str> {
    python_tool_for_command(command_id).map(|_| ADAPTER_SCHEMA)
}

pub(crate) fn adapter_config_scope_for_command(command_id: &str) -> Option<&'static str> {
    python_tool_for_command(command_id).map(PythonTool::config_scope)
}

pub(crate) fn adapter_script_for_command(command_id: &str) -> Option<&'static str> {
    python_tool_for_command(command_id).map(PythonTool::script_relative_path)
}

fn python_tool_for_command(command_id: &str) -> Option<PythonTool> {
    match command_id {
        "lint.line-counts" => Some(PythonTool::LineCounts),
        "lint.identity-literals" => Some(PythonTool::IdentityLiterals),
        "view.findings" => Some(PythonTool::ViewFindings),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adapter_success_payload_normalizes_data() {
        let payload = format!(
            r#"{{
              "adapter_schema": "{ADAPTER_SCHEMA}",
              "ok": true,
              "summary": "source file size limits satisfied",
              "data": {{"status": "pass", "findings": []}},
              "diagnostics": []
            }}"#
        );
        let parsed = parse_adapter_output(PythonTool::LineCounts, payload.as_bytes())
            .expect("payload parses");

        assert_eq!(parsed.summary, "source file size limits satisfied");
        assert_eq!(parsed.data.expect("data")["status"], "pass");
    }

    #[test]
    fn adapter_error_payload_maps_to_cli_error() {
        let payload = format!(
            r#"{{
              "adapter_schema": "{ADAPTER_SCHEMA}",
              "ok": false,
              "summary": "config error",
              "error": {{
                "kind": "config",
                "message": "bad identities config",
                "details": {{"key": "identities"}},
                "suggested_action": "fix the config"
              }},
              "diagnostics": []
            }}"#
        );
        let parsed = parse_adapter_output(PythonTool::IdentityLiterals, payload.as_bytes())
            .expect("payload parses");

        let error = parsed.error.expect("error");
        assert_eq!(error.kind, CliErrorKind::Config);
        assert_eq!(error.details["key"], "identities");
        assert_eq!(error.suggested_action.as_deref(), Some("fix the config"));
    }
}
