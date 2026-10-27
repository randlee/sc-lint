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

pub(crate) fn render_success_human(
    context: &CommandContext,
    envelope: &CommandEnvelope<Value>,
) -> String {
    let data = envelope.data.as_ref();
    match context.id() {
        CommandId::Version => {
            let version = data_str(data, consts::FIELD_VERSION).unwrap_or("unknown");
            format!("sc-lint {version}")
        }
        CommandId::LintScBoundary | CommandId::LintScPortability | CommandId::LintScRuntime => {
            let status = data_str(data, consts::FIELD_STATUS).unwrap_or("unknown");
            let finding_count = data_len(data, consts::FIELD_FINDINGS);
            format!(
                "{}: {status} ({finding_count} findings)",
                context.id().dispatch_tool().unwrap_or(context.command_id())
            )
        }
        CommandId::ConsumerTest if data.is_some_and(|value| value.get("selectors").is_some()) => {
            render_selector_list(data)
        }
        CommandId::ConsumerTest
        | CommandId::ConsumerLintCi
        | CommandId::LintFast
        | CommandId::LintFull
        | CommandId::LintCi
        | CommandId::Ci
        | CommandId::CheckNative
        | CommandId::CheckXwin
        | CommandId::ClippyNative
        | CommandId::ClippyXwin => {
            let status = data_str(data, consts::FIELD_STATUS).unwrap_or("unknown");
            let step_count = data_len(data, consts::FIELD_STEPS);
            format!("{}: {status} ({step_count} steps)", context.command_id())
        }
        CommandId::Setup | CommandId::Upgrade => data_str(data, "summary").map_or_else(
            || format!("{}: ok", context.command_id()),
            ToString::to_string,
        ),
        CommandId::Docs => match (data_str(data, "path"), data_str(data, "content")) {
            (Some(path), Some(content)) => format!("Documentation root: {path}\n\n{content}"),
            (Some(path), None) => format!("Documentation root: {path}"),
            (None, Some(content)) => content.to_string(),
            (None, None) => format!("{}: ok", context.command_id()),
        },
        CommandId::LintFunctionLength
        | CommandId::LintLineCounts
        | CommandId::LintIdentityLiterals
        | CommandId::ViewFindings => data_str(data, "summary").map_or_else(
            || format!("{}: ok", context.command_id()),
            |summary| format!("{}: {summary}", context.command_id()),
        ),
        _ => format!("{}: ok", context.command_id()),
    }
}

/// The string at `key` of a success envelope's `data`.
fn data_str<'a>(data: Option<&'a Value>, key: &str) -> Option<&'a str> {
    data.and_then(|value| value.get(key))
        .and_then(Value::as_str)
}

/// The length of the array at `key` of a success envelope's `data` (0 when absent).
fn data_len(data: Option<&Value>, key: &str) -> usize {
    data.and_then(|value| value.get(key))
        .and_then(Value::as_array)
        .map_or(0, Vec::len)
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
        ("Exit code", consts::FIELD_EXIT_CODE),
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
    render_step_failures(&mut rendered, error);
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

/// `Failed steps:` and `Skipped steps (fail-fast):` sections of a
/// `profile.steps_failed` error.
fn render_step_failures(rendered: &mut String, error: &CliError) {
    if let Some(failed) = error.details.get("failed_steps").and_then(Value::as_array) {
        rendered.push_str("\nFailed steps:");
        for step in failed {
            let field = |key: &str| step.get(key).map(render_detail_value).unwrap_or_default();
            let exit = step
                .get("exit_code")
                .and_then(Value::as_i64)
                .map_or_else(|| "n/a".to_string(), |code| code.to_string());
            rendered.push_str(&format!(
                "\n  [{}] {} (exit {exit}): {}",
                field("index"),
                field("name"),
                field("message")
            ));
        }
    }
    let skipped = error
        .details
        .get("skipped_steps")
        .and_then(Value::as_array)
        .filter(|skipped| !skipped.is_empty());
    if let Some(skipped) = skipped {
        let names = skipped
            .iter()
            .filter_map(|step| step.get("name").and_then(Value::as_str))
            .collect::<Vec<_>>();
        rendered.push_str("\nSkipped steps (fail-fast): ");
        rendered.push_str(&names.join(", "));
    }
}

/// One line per selector of `sc-lint test --list`:
/// `<name>[ (default)]: <argv | group [a, b]>[ [platforms]]`.
fn render_selector_list(data: Option<&Value>) -> String {
    let default = data_str(data, "default");
    data.and_then(|value| value.get("selectors"))
        .and_then(Value::as_array)
        .map(|selectors| {
            selectors
                .iter()
                .map(|selector| {
                    let name = selector
                        .get("name")
                        .and_then(Value::as_str)
                        .unwrap_or_default();
                    let marker = if Some(name) == default {
                        " (default)"
                    } else {
                        ""
                    };
                    let body = match selector.get("group") {
                        Some(group) => format!("group [{}]", string_items(Some(group)).join(", ")),
                        None => string_items(selector.get("command")).join(" "),
                    };
                    let platforms = string_items(selector.get("platforms"));
                    let platforms = if platforms.is_empty() {
                        String::new()
                    } else {
                        format!(" [{}]", platforms.join(","))
                    };
                    format!("{name}{marker}: {body}{platforms}")
                })
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default()
}

fn string_items(value: Option<&Value>) -> Vec<&str> {
    value
        .and_then(Value::as_array)
        .map(|items| items.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default()
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
