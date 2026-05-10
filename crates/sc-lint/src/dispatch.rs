use sc_lint_boundary::AnalyzeOptions;
use sc_lint_boundary::OutputFormat;
use sc_lint_boundary::analyze_workspace;
use serde_json::Value;
use serde_json::json;

use crate::CliError;
use crate::command::CommandContext;
use crate::command::CommandSuccess;
use crate::command::DispatchTelemetry;
use crate::config::LoadedConfig;

#[expect(
    clippy::result_large_err,
    reason = "Dispatch failures must remain in the shared top-level CliError contract."
)]
pub fn run_sc_boundary(
    _context: &CommandContext,
    loaded_config: &LoadedConfig,
) -> Result<CommandSuccess, CliError> {
    let repo_root = loaded_config.require_repo_root()?;
    let report = analyze_workspace(&AnalyzeOptions {
        root: repo_root.to_path_buf(),
        format: OutputFormat::Json,
        rule: None,
    })
    .map_err(|error| {
        CliError::backend_failure(format!(
            "sc-lint-boundary failed to analyze `{}`",
            repo_root.display()
        ))
        .with_source(error)
        .with_detail("tool", json!("sc-lint-boundary"))
        .with_detail("root", json!(repo_root.display().to_string()))
    })?;

    let raw = serde_json::to_string(&report).map_err(|error| {
        CliError::backend_protocol(
            "sc-lint-boundary produced a report that could not be encoded as machine JSON",
        )
        .with_source(error)
        .with_detail("tool", json!("sc-lint-boundary"))
    })?;
    let normalized = normalize_backend_json("sc-lint-boundary", &raw)?;
    let finding_count = normalized
        .get("findings")
        .and_then(Value::as_array)
        .map_or(0, std::vec::Vec::len);

    Ok(CommandSuccess::with_dispatch(
        normalized,
        DispatchTelemetry::new("sc-lint-boundary", finding_count),
    ))
}

#[expect(
    clippy::result_large_err,
    reason = "Backend protocol violations must remain in the shared top-level CliError contract."
)]
pub fn normalize_backend_json(tool: &str, raw: &str) -> Result<Value, CliError> {
    serde_json::from_str(raw).map_err(|error| {
        CliError::backend_protocol(format!("{tool} returned malformed machine output"))
            .with_source(error)
            .with_detail("tool", json!(tool))
    })
}
