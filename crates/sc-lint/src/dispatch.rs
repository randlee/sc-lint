use sc_lint_boundary::AnalyzeOptions;
use sc_lint_boundary::OutputFormat;
use sc_lint_boundary::analyze_workspace;
use sc_observability::Logger;
use serde_json::Value;
use serde_json::json;

use crate::CliError;
use crate::CliErrorKind;
use crate::command::CommandContext;
use crate::config::LoadedConfig;
use crate::logging;

pub fn run_sc_boundary(
    context: &CommandContext,
    loaded_config: &LoadedConfig,
    logger: Option<&Logger>,
) -> Result<Value, CliError> {
    if let Some(logger) = logger {
        logging::emit_dispatch_start(logger, context, "sc-lint-boundary");
    }

    let repo_root = loaded_config.require_repo_root()?;
    let report = analyze_workspace(&AnalyzeOptions {
        root: repo_root.to_path_buf(),
        format: OutputFormat::Json,
        rule: None,
    })
    .map_err(|error| {
        CliError::new(
            CliErrorKind::BackendFailure,
            format!(
                "sc-lint-boundary failed to analyze `{}`: {error}",
                repo_root.display()
            ),
        )
        .with_detail("tool", json!("sc-lint-boundary"))
        .with_detail("root", json!(repo_root.display().to_string()))
    })?;

    let raw = serde_json::to_string(&report).map_err(|error| {
        CliError::new(
            CliErrorKind::BackendProtocol,
            format!(
                "sc-lint-boundary produced a report that could not be encoded as machine JSON: {error}"
            ),
        )
        .with_detail("tool", json!("sc-lint-boundary"))
    })?;
    let normalized = normalize_backend_json("sc-lint-boundary", &raw)?;
    if let Some(logger) = logger {
        let finding_count = normalized
            .get("findings")
            .and_then(Value::as_array)
            .map_or(0, std::vec::Vec::len);
        logging::emit_dispatch_result(logger, context, "sc-lint-boundary", finding_count);
    }

    Ok(normalized)
}

pub fn normalize_backend_json(tool: &str, raw: &str) -> Result<Value, CliError> {
    serde_json::from_str(raw).map_err(|error| {
        CliError::new(
            CliErrorKind::BackendProtocol,
            format!("{tool} returned malformed machine output: {error}"),
        )
        .with_detail("tool", json!(tool))
    })
}
