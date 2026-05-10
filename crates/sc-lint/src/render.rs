use crate::CliError;
use crate::CommandEnvelope;
use crate::command::CommandContext;
use crate::command::CommandId;
use crate::consts;
use serde::Serialize;
use serde_json::Value;
use serde_json::json;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct RenderedOutput {
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

pub(crate) fn render_success_human(
    context: &CommandContext,
    envelope: &CommandEnvelope<Value>,
) -> String {
    match context.id() {
        CommandId::Version => {
            let version = envelope
                .data
                .as_ref()
                .and_then(|value| value.get(consts::FIELD_CRATE_VERSION))
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
                .and_then(|value| value.get("steps"))
                .and_then(Value::as_array)
                .map_or(0, std::vec::Vec::len);
            format!("{}: {status} ({step_count} steps)", context.command_id())
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
    if let Some(suggested_action) = error.suggested_action.as_deref() {
        rendered.push('\n');
        rendered.push_str(suggested_action);
    }
    rendered
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
        Err(_) => "{\"ok\":false,\"command\":\"render.failure\",\"error\":{\"kind\":\"internal\",\"code\":\"CLI.INTERNAL_ERROR\",\"message\":\"failed to serialize CLI output\"},\"diagnostics\":[]}".to_string(),
    }
}
