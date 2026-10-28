use std::ffi::OsString;
use std::process::Command as ProcessCommand;

use serde_json::Map;
use serde_json::Value;
use serde_json::json;

use crate::CliError;
use crate::CliErrorKind;
use crate::command::CommandSuccess;
use crate::command::DispatchTelemetry;
use crate::config::ConfigCode;
use crate::config::LoadedConfig;
use crate::consts;
use crate::error::ErrorCode;

pub(crate) const ADAPTER_SCHEMA: &str = "sc-lint-python-v1";
pub(crate) const BACKEND_PAYLOAD_DRIFT_CODE: &str = "backend_payload_drift";
pub(crate) const BACKEND_PAYLOAD_DRIFT_RECOVERY: &str =
    "upgrade or reinstall the analyzer backend so its payload matches the published schema";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PythonTool {
    FunctionLength,
    LineCounts,
    IdentityLiterals,
    ViewFindings,
}

impl PythonTool {
    pub const fn tool_name(self) -> &'static str {
        match self {
            Self::FunctionLength => "sc-lint-function-length",
            Self::LineCounts => "sc-lint-line-counts",
            Self::IdentityLiterals => "sc-lint-identity-literals",
            Self::ViewFindings => "sc-lint-view-findings",
        }
    }

    /// Python module (inside the `sc_lint` wheel) that implements this tool.
    pub const fn script_relative_path(self) -> &'static str {
        match self {
            Self::FunctionLength => "sc_lint.lint_function_length",
            Self::LineCounts => "sc_lint.lint_line_counts",
            Self::IdentityLiterals => "sc_lint.lint_identity_literals",
            Self::ViewFindings => "sc_lint.view_findings",
        }
    }

    pub const fn config_scope(self) -> &'static str {
        match self {
            Self::FunctionLength => "function_length",
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
    let output = python_module_command(repo_root)
        .arg(tool.script_relative_path())
        .arg("--root")
        .arg(repo_root)
        .args(
            loaded_config
                .explicit_config()
                .into_iter()
                .flat_map(|path| [OsString::from("--config"), path.as_os_str().to_os_string()]),
        )
        .arg("--json")
        .output()
        .map_err(|error| {
            CliError::backend_failure(format!("{} failed to start", tool.tool_name()))
                .with_source(error)
                .with_detail(consts::FIELD_TOOL, json!(tool.tool_name()))
                .with_detail(consts::FIELD_SCRIPT, json!(tool.script_relative_path()))
        })?;

    adapter_outcome(tool, output.status.code(), &output.stdout)
}

/// Items shown per list in a failure envelope; the full counts are always reported.
const FAILURE_REPORT_LIMIT: usize = 10;

/// Name the offending items and the findings artifact in a failed verdict's details.
fn with_failure_report(mut error: CliError, data: &Value) -> CliError {
    for key in ["violations", "stale_baseline", consts::FIELD_FINDINGS] {
        if let Some(items) = data.get(key).and_then(Value::as_array) {
            let shown: Vec<Value> = items.iter().take(FAILURE_REPORT_LIMIT).cloned().collect();
            error = error
                .with_detail(format!("{key}_count"), json!(items.len()))
                .with_detail(key, Value::Array(shown));
        }
    }
    if let Some(artifact_dir) = data.get("artifact_dir") {
        error = error.with_detail("artifact_dir", artifact_dir.clone());
    }
    error
}

/// A non-zero exit or a `fail` status is a failed command, not a successful envelope.
#[expect(
    clippy::result_large_err,
    reason = "Adapter verdicts must use the shared top-level CliError contract."
)]
fn adapter_outcome(
    tool: PythonTool,
    exit_code: Option<i32>,
    stdout: &[u8],
) -> Result<CommandSuccess, CliError> {
    let parsed = parse_adapter_output(tool, stdout)?;
    if let Some(error) = parsed.error {
        return Err(error);
    }
    let mut data = parsed.data.unwrap_or_else(|| json!({}));
    let status = data.get(consts::FIELD_STATUS).and_then(Value::as_str);
    if exit_code != Some(0) || status == Some(consts::STATUS_FAIL) {
        let error = CliError::backend_failure(parsed.summary)
            .with_detail(consts::FIELD_TOOL, json!(tool.tool_name()))
            .with_detail(consts::FIELD_SCRIPT, json!(tool.script_relative_path()))
            .with_detail(consts::FIELD_STATUS, json!(status))
            .with_detail(consts::FIELD_EXIT_CODE, json!(exit_code));
        return Err(with_failure_report(error, &data));
    }
    if let Some(object) = data.as_object_mut() {
        object
            .entry(consts::FIELD_ADAPTER.to_string())
            .or_insert_with(|| json!(ADAPTER_SCHEMA));
        object
            .entry(consts::FIELD_CONFIG_SCOPE.to_string())
            .or_insert_with(|| json!(tool.config_scope()));
        object
            .entry(consts::FIELD_SCRIPT.to_string())
            .or_insert_with(|| json!(tool.script_relative_path()));
        object
            .entry(consts::FIELD_SUMMARY.to_string())
            .or_insert_with(|| json!(parsed.summary));
    }
    let finding_count = data
        .get(consts::FIELD_FINDINGS)
        .and_then(Value::as_array)
        .map_or(0, std::vec::Vec::len);
    Ok(CommandSuccess::with_dispatch(
        data,
        DispatchTelemetry::new(tool.tool_name(), finding_count),
    ))
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
        .with_detail(consts::FIELD_TOOL, json!(tool.tool_name()))
    })?;
    let payload: Value = serde_json::from_str(text).map_err(|error| {
        CliError::backend_protocol(format!(
            "{} returned malformed adapter json",
            tool.tool_name()
        ))
        .with_source(error)
        .with_detail(consts::FIELD_TOOL, json!(tool.tool_name()))
    })?;
    let object = adapter_object(tool, &payload)?;
    validate_adapter_payload(tool, &payload)?;
    let summary = object
        .get(consts::FIELD_SUMMARY)
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
            .with_detail(consts::FIELD_TOOL, json!(tool.tool_name()))
        })?;
    let kind = parse_error_kind(tool, error_object.get(consts::FIELD_KIND))?;
    let message = error_object
        .get(consts::FIELD_MESSAGE)
        .and_then(Value::as_str)
        .ok_or_else(|| {
            CliError::backend_protocol(format!(
                "{} returned an adapter failure without a message",
                tool.tool_name()
            ))
            .with_detail(consts::FIELD_TOOL, json!(tool.tool_name()))
        })?
        .to_string();
    let mut error = CliError::new(kind, message.clone())
        .with_detail(consts::FIELD_TOOL, json!(tool.tool_name()))
        .with_detail(consts::FIELD_SCRIPT, json!(tool.script_relative_path()));
    if let Some(details) = error_object
        .get(consts::FIELD_DETAILS)
        .and_then(Value::as_object)
    {
        error = merge_details(error, details, kind, message);
    }
    if let Some(action) = error_object
        .get(consts::FIELD_SUGGESTED_ACTION)
        .and_then(Value::as_str)
    {
        error = error.with_suggested_action(action);
    }
    Ok(AdapterResult {
        summary,
        data: None,
        error: Some(error),
    })
}

#[expect(
    clippy::result_large_err,
    reason = "Adapter normalization failures must use the shared top-level CliError contract."
)]
fn adapter_object(tool: PythonTool, payload: &Value) -> Result<&Map<String, Value>, CliError> {
    let object = payload.as_object().ok_or_else(|| {
        CliError::backend_protocol(format!(
            "{} returned a non-object adapter payload",
            tool.tool_name()
        ))
        .with_detail(consts::FIELD_TOOL, json!(tool.tool_name()))
    })?;
    if object.get("adapter_schema").and_then(Value::as_str) != Some(ADAPTER_SCHEMA) {
        return Err(CliError::backend_protocol(format!(
            "{} returned an unknown adapter schema",
            tool.tool_name()
        ))
        .with_detail(consts::FIELD_TOOL, json!(tool.tool_name()))
        .with_detail("expected_schema", json!(ADAPTER_SCHEMA)));
    }
    Ok(object)
}

#[expect(
    clippy::result_large_err,
    reason = "Adapter schema drift uses the shared top-level CliError contract."
)]
fn validate_adapter_payload(tool: PythonTool, payload: &Value) -> Result<(), CliError> {
    if let Err(drift) = sc_lint_schema::validate_adapter_payload(payload) {
        return Err(CliError::backend_protocol(format!(
            "{} returned a payload that drifted from the sc-lint-python-v1 schema",
            tool.tool_name()
        ))
        .with_code(BACKEND_PAYLOAD_DRIFT_CODE)
        .with_cause(drift.to_string())
        .with_suggested_action(BACKEND_PAYLOAD_DRIFT_RECOVERY)
        .with_detail(consts::FIELD_TOOL, json!(tool.tool_name()))
        .with_detail("key_path", json!(drift.key_path)));
    }
    Ok(())
}

fn merge_details(
    mut error: CliError,
    details: &Map<String, Value>,
    kind: CliErrorKind,
    message: String,
) -> CliError {
    for (key, value) in details {
        error = error.with_detail(key.clone(), value.clone());
    }
    let code = (kind == CliErrorKind::Config)
        .then(|| error.details.get("code").and_then(Value::as_str))
        .flatten()
        .and_then(ConfigCode::from_wire);
    if let Some(code) = code {
        error = error.with_code(code.as_str()).with_cause(message);
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
        .with_detail(consts::FIELD_TOOL, json!(tool.tool_name()))
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
        .with_detail(consts::FIELD_TOOL, json!(tool.tool_name()))),
    }
}

fn python_command() -> OsString {
    if cfg!(windows) {
        OsString::from("python")
    } else {
        OsString::from("python3")
    }
}

/// Builds `python -m` using the repo-local `.sc-lint/venv` interpreter when it
/// exists; otherwise the host interpreter, with the source checkout's package
/// directory on `PYTHONPATH` so a source repository works before `just setup`.
/// Interpreter used for `python -m sc_lint.<module>` helpers: the repo venv when
/// provisioned, otherwise the host interpreter with the source package on `PYTHONPATH`.
pub(crate) struct PythonInterpreter {
    pub(crate) program: std::ffi::OsString,
    pub(crate) python_path: Option<std::ffi::OsString>,
}

impl PythonInterpreter {
    pub(crate) fn resolve(repo_root: &std::path::Path) -> Self {
        let venv_python = repo_root
            .join(consts::VENV_RELATIVE_DIR)
            .join(if cfg!(windows) {
                "Scripts/python.exe"
            } else {
                "bin/python3"
            });
        if venv_python.is_file() {
            return Self {
                program: venv_python.into_os_string(),
                python_path: None,
            };
        }
        let source_package_dir = repo_root.join(consts::SOURCE_PYTHON_PACKAGE_DIR);
        let python_path = source_package_dir.is_dir().then(|| {
            let mut python_path = source_package_dir.into_os_string();
            if let Some(existing) = std::env::var_os("PYTHONPATH") {
                python_path.push(if cfg!(windows) { ";" } else { ":" });
                python_path.push(existing);
            }
            python_path
        });
        Self {
            program: python_command(),
            python_path,
        }
    }
}

fn python_module_command(repo_root: &std::path::Path) -> ProcessCommand {
    let interpreter = PythonInterpreter::resolve(repo_root);
    let mut command = ProcessCommand::new(interpreter.program);
    if let Some(python_path) = interpreter.python_path {
        command.env("PYTHONPATH", python_path);
    }
    // Parity with `workflow::python_step`: a delegated utility that resolves
    // the product binary must reach this running executable, never a stale
    // PATH or managed installation.
    command.env("SC_LINT_BIN", crate::dispatch::product_binary());
    command.current_dir(repo_root).arg("-m");
    command
}

/// Adapter schema identifier shared by every Python-backed tool.
pub(crate) const fn adapter_kind() -> &'static str {
    ADAPTER_SCHEMA
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

    fn adapter_payload(status: &str, summary: &str) -> String {
        format!(
            r#"{{
              "adapter_schema": "{ADAPTER_SCHEMA}",
              "ok": true,
              "summary": "{summary}",
              "data": {{"status": "{status}", "violations": [{{"name": "big"}}]}},
              "diagnostics": []
            }}"#
        )
    }

    #[test]
    fn function_length_violation_fails_the_command() {
        let payload = adapter_payload(
            consts::STATUS_FAIL,
            "function-length gate failed: 1 new or grown function(s)",
        );
        let error = adapter_outcome(PythonTool::FunctionLength, Some(1), payload.as_bytes())
            .expect_err("a failing verdict must fail the command");

        assert_eq!(error.kind, CliErrorKind::BackendFailure);
        assert_eq!(error.exit_code(), 5);
        assert_eq!(
            error.message,
            "function-length gate failed: 1 new or grown function(s)"
        );
        assert_eq!(error.details[consts::FIELD_STATUS], consts::STATUS_FAIL);
        assert_eq!(error.details[consts::FIELD_EXIT_CODE], 1);
    }

    #[test]
    fn non_zero_exit_fails_even_when_the_payload_reports_pass() {
        let payload = adapter_payload("pass", "line counts within limits");
        let error = adapter_outcome(PythonTool::LineCounts, Some(1), payload.as_bytes())
            .expect_err("a non-zero exit must fail the command");

        assert_eq!(error.kind, CliErrorKind::BackendFailure);
        assert_eq!(error.details[consts::FIELD_EXIT_CODE], 1);
    }

    #[test]
    fn failing_verdict_names_the_violations_with_a_cap() {
        let violations: Vec<Value> = (0..12)
            .map(|index| {
                json!({
                    "path": "crates/demo/src/lib.rs",
                    "name": format!("f{index}"),
                    "code_lines": 90,
                    "recorded_lines": 85,
                    "reason": "grown",
                })
            })
            .collect();
        let payload = json!({
            "adapter_schema": ADAPTER_SCHEMA,
            "ok": true,
            "summary": "function-length gate failed: 12 new or grown function(s), 0 stale allowance(s)",
            "data": {
                consts::FIELD_STATUS: consts::STATUS_FAIL,
                "violations": violations,
                "stale_baseline": [],
                "artifact_dir": "artifacts/findings/function-length",
            },
            "diagnostics": [],
        })
        .to_string();
        let error = adapter_outcome(PythonTool::FunctionLength, Some(1), payload.as_bytes())
            .expect_err("a failing verdict must fail the command");

        assert_eq!(error.details["violations_count"], 12);
        let shown = error.details["violations"]
            .as_array()
            .expect("violations list");
        assert_eq!(shown.len(), FAILURE_REPORT_LIMIT);
        assert_eq!(shown[0]["name"], "f0");
        assert_eq!(shown[0]["recorded_lines"], 85);
        assert_eq!(error.details["stale_baseline_count"], 0);
        assert_eq!(
            error.details["artifact_dir"],
            "artifacts/findings/function-length"
        );
    }

    #[test]
    fn passing_verdict_keeps_the_success_envelope() {
        let payload = adapter_payload("pass", "function-length gate passed");
        let success = adapter_outcome(PythonTool::FunctionLength, Some(0), payload.as_bytes())
            .expect("a passing verdict succeeds");

        assert_eq!(success.data["adapter"], ADAPTER_SCHEMA);
        assert_eq!(success.data["status"], "pass");
    }

    fn malformed_success_payload() -> String {
        let mut payload = json!({
            "adapter_schema": ADAPTER_SCHEMA,
            "ok": true,
            "summary": "line counts within limits",
            "data": {"status": "pass"},
            "diagnostics": []
        });
        payload.as_object_mut().unwrap().remove("data");
        payload.to_string()
    }

    #[test]
    fn backend_payload_drift_rejected() {
        let payload = malformed_success_payload();
        let error = adapter_outcome(PythonTool::LineCounts, Some(0), payload.as_bytes())
            .expect_err("schema drift is a protocol error");

        assert_eq!(error.kind, CliErrorKind::BackendProtocol);
        assert_eq!(error.code(), BACKEND_PAYLOAD_DRIFT_CODE);
        assert_eq!(error.cause.as_deref(), Some("`data`: missing required key"));
        assert_eq!(
            error.suggested_action.as_deref(),
            Some(BACKEND_PAYLOAD_DRIFT_RECOVERY)
        );
        assert_eq!(error.details["key_path"], "data");
        assert_eq!(error.exit_code(), consts::EXIT_BACKEND_PROTOCOL);
    }

    #[test]
    fn backend_payload_drift_precedes_exit_code_check() {
        let payload = malformed_success_payload();
        let error = adapter_outcome(PythonTool::LineCounts, Some(1), payload.as_bytes())
            .expect_err("payload drift takes precedence over process exit status");

        assert_eq!(error.code(), BACKEND_PAYLOAD_DRIFT_CODE);
        assert_ne!(error.code(), "CLI.BACKEND_EXEC_FAILURE");
    }

    #[test]
    fn unknown_adapter_schema_is_not_payload_drift() {
        let payload = json!({
            "adapter_schema": "sc-lint-python-v2",
            "ok": true,
            "summary": "line counts within limits",
            "data": {"status": "pass"},
            "diagnostics": []
        })
        .to_string();
        let error = adapter_outcome(PythonTool::LineCounts, Some(0), payload.as_bytes())
            .expect_err("foreign schema remains an unknown schema protocol error");

        assert_eq!(error.kind, CliErrorKind::BackendProtocol);
        assert_eq!(error.code(), "CLI.BACKEND_PROTOCOL_ERROR");
        assert_ne!(error.code(), BACKEND_PAYLOAD_DRIFT_CODE);
    }

    #[test]
    fn python_module_command_delegates_to_the_running_product_binary() {
        let command = python_module_command(std::path::Path::new("."));
        let product_binary = crate::dispatch::product_binary().into_os_string();

        let exported = command
            .get_envs()
            .find(|(key, _)| *key == "SC_LINT_BIN")
            .and_then(|(_, value)| value)
            .expect("python_module_command exports SC_LINT_BIN");
        assert_eq!(exported, product_binary.as_os_str());
    }
}
