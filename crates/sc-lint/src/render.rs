use crate::CliError;
use crate::CommandEnvelope;
use crate::command::CommandContext;
use crate::command::CommandId;
use crate::consts;
use serde::Serialize;
use serde_json::Value;
use serde_json::json;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct RenderedOutput {
    pub stdout: Option<String>,
    pub stderr: Option<String>,
}

impl RenderedOutput {
    pub(crate) fn stdout(value: String) -> Self {
        Self {
            stdout: Some(value),
            stderr: None,
        }
    }

    pub(crate) fn stderr(value: String) -> Self {
        Self {
            stdout: None,
            stderr: Some(value),
        }
    }
}

pub(crate) fn render_success_json<T>(envelope: &CommandEnvelope<T>) -> String
where
    T: Serialize,
{
    match serde_json::to_string_pretty(envelope) {
        Ok(rendered) => rendered,
        Err(error) => fallback_render_error(
            &envelope.command,
            &CliError::internal("failed to serialize success envelope").with_source(error),
        ),
    }
}

pub(crate) fn render_error_json(command_id: &str, error: &CliError) -> String {
    let envelope = CommandEnvelope::<Value>::failure(command_id, error.clone());
    match serde_json::to_string_pretty(&envelope) {
        Ok(rendered) => rendered,
        Err(_) => fallback_render_error(command_id, error),
    }
}

pub(crate) fn render_configure_error_json(command_id: &str, error: &CliError) -> String {
    let payload = json!({
        "ok": false,
        "command": command_id,
        "error": {
            "code": error.code(),
            "message": error.message,
            "cause": error.cause,
            "pointer": error.details.get("pointer").cloned().unwrap_or(Value::Null),
            "recovery": error.details.get("recovery").cloned().unwrap_or(Value::Null),
            "recovery_description": error.suggested_action,
            "docs_ref": error.documentation,
        },
    });
    match serde_json::to_string_pretty(&payload) {
        Ok(rendered) => rendered,
        Err(_) => fallback_render_error(command_id, error),
    }
}

pub(crate) fn render_success_human(
    context: &CommandContext,
    envelope: &CommandEnvelope<Value>,
) -> String {
    match context.id() {
        CommandId::Version => {
            let version = envelope
                .data
                .as_ref()
                .and_then(|value| value.get(consts::FIELD_VERSION))
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            format!("sc-lint {version}")
        }
        CommandId::LintScBoundary => {
            let status = envelope
                .data
                .as_ref()
                .and_then(|value| value.get(consts::FIELD_STATUS))
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            let finding_count = envelope
                .data
                .as_ref()
                .and_then(|value| value.get(consts::FIELD_FINDINGS))
                .and_then(Value::as_array)
                .map_or(0, std::vec::Vec::len);
            format!(
                "{}: {status} ({finding_count} findings)",
                consts::TOOL_BOUNDARY
            )
        }
        CommandId::LintFast
        | CommandId::LintFull
        | CommandId::LintCi
        | CommandId::Ci
        | CommandId::CheckNative
        | CommandId::CheckXwin
        | CommandId::ClippyNative
        | CommandId::ClippyXwin => {
            let status = envelope
                .data
                .as_ref()
                .and_then(|value| value.get(consts::FIELD_STATUS))
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            let step_count = envelope
                .data
                .as_ref()
                .and_then(|value| value.get(consts::FIELD_STEPS))
                .and_then(Value::as_array)
                .map_or(0, std::vec::Vec::len);
            format!("{}: {status} ({step_count} steps)", context.command_id())
        }
        CommandId::Setup | CommandId::Upgrade => envelope
            .data
            .as_ref()
            .and_then(|value| value.get("summary"))
            .and_then(Value::as_str)
            .map_or_else(
                || format!("{}: ok", context.command_id()),
                ToString::to_string,
            ),
        CommandId::Docs => {
            let data = envelope.data.as_ref();
            let path = data
                .and_then(|value| value.get("path"))
                .and_then(Value::as_str);
            let content = data
                .and_then(|value| value.get("content"))
                .and_then(Value::as_str);
            match (path, content) {
                (Some(path), Some(content)) => format!("Documentation root: {path}\n\n{content}"),
                (Some(path), None) => format!("Documentation root: {path}"),
                (None, Some(content)) => content.to_string(),
                (None, None) => format!("{}: ok", context.command_id()),
            }
        }
        CommandId::LintLineCounts | CommandId::LintIdentityLiterals | CommandId::ViewFindings => {
            envelope
                .data
                .as_ref()
                .and_then(|value| value.get("summary"))
                .and_then(Value::as_str)
                .map_or_else(
                    || format!("{}: ok", context.command_id()),
                    |summary| format!("{}: {summary}", context.command_id()),
                )
        }
        _ => format!("{}: ok", context.command_id()),
    }
}

pub(crate) fn render_error_human(command_id: &str, error: &CliError) -> String {
    let mut rendered = format!("{command_id}: {} ({})", error.message, error.code());
    for (label, key) in [
        ("Required version", "minimum_version"),
        ("Observed version", "installed_version"),
        ("Reported version", "reported_version"),
        ("Binary path", "binary_path"),
        ("Configuration path", "config_path"),
        ("Bundle path", "bundle_path"),
        ("Required field", "required_field"),
        ("Exit code", "exit_code"),
        ("Standard output", "stdout"),
        ("Standard error", "stderr"),
    ] {
        if let Some(value) = error.details.get(key) {
            rendered.push('\n');
            rendered.push_str(label);
            rendered.push_str(": ");
            rendered.push_str(&render_detail_value(value));
        }
    }
    if let Some(cause) = error.cause.as_deref() {
        rendered.push('\n');
        rendered.push_str("Cause: ");
        rendered.push_str(cause);
    }
    if let Some(suggested_action) = error.suggested_action.as_deref() {
        rendered.push('\n');
        rendered.push_str(suggested_action);
    }
    if let Some(documentation) = error.documentation.as_deref() {
        rendered.push('\n');
        rendered.push_str("Docs: ");
        rendered.push_str(documentation);
    }
    rendered
}

fn render_detail_value(value: &Value) -> String {
    value
        .as_str()
        .map_or_else(|| value.to_string(), ToString::to_string)
}

fn fallback_render_error(command_id: &str, error: &CliError) -> String {
    let fallback = json!({
        "ok": false,
        "command": command_id,
        "error": error,
        "diagnostics": [],
    });
    match serde_json::to_string_pretty(&fallback) {
        Ok(rendered) => rendered,
        Err(_) => format!(
            "{{\"ok\":false,\"command\":\"{command_id}\",\"error\":{{\"kind\":\"internal\",\"code\":\"CLI.INTERNAL_ERROR\",\"message\":\"failed to serialize CLI output\"}},\"diagnostics\":[]}}"
        ),
    }
}
