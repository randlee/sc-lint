use crate::CliError;
use crate::CommandEnvelope;
use crate::command::CommandContext;
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct RenderedOutput {
    pub stdout: Option<String>,
    pub stderr: Option<String>,
}

impl RenderedOutput {
    pub fn stdout(value: String) -> Self {
        Self {
            stdout: Some(value),
            stderr: None,
        }
    }

    pub fn stderr(value: String) -> Self {
        Self {
            stdout: None,
            stderr: Some(value),
        }
    }
}

pub fn render_success_json<T>(envelope: &CommandEnvelope<T>) -> String
where
    T: Serialize,
{
    serde_json::to_string_pretty(envelope).expect("success envelopes always serialize")
}

pub fn render_error_json(command_id: &str, error: &CliError) -> String {
    let envelope = CommandEnvelope::<Value>::failure(command_id, error.clone());
    serde_json::to_string_pretty(&envelope).expect("error envelopes always serialize")
}

pub fn render_success_human(context: &CommandContext, envelope: &CommandEnvelope<Value>) -> String {
    match context.command_id() {
        "version" => {
            let version = envelope
                .data
                .as_ref()
                .and_then(|value| value.get("crate_version"))
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            format!("sc-lint {version}")
        }
        _ => format!("{}: ok", context.command_id()),
    }
}

pub fn render_error_human(command_id: &str, error: &CliError) -> String {
    let mut rendered = format!("{command_id}: {} ({})", error.message, error.code);
    if let Some(suggested_action) = error.suggested_action.as_deref() {
        rendered.push('\n');
        rendered.push_str(suggested_action);
    }
    rendered
}
