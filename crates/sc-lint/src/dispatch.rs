use std::error::Error;
use std::path::Path;

use sc_lint_boundary::AnalyzeOptions;
use sc_lint_boundary::OutputFormat;
use sc_lint_boundary::analyze_workspace;
use serde_json::Error as JsonError;
use serde_json::Value;
use serde_json::json;

use crate::CliError;
use crate::command::CommandContext;
use crate::command::CommandSuccess;
use crate::command::DispatchTelemetry;
use crate::config::LoadedConfig;
use crate::consts;

#[derive(Debug)]
enum BoundaryDispatchError {
    Analysis(Box<dyn Error + Send + Sync>),
    Serialize(JsonError),
    Normalize(JsonError),
}

impl BoundaryDispatchError {
    fn into_cli_error(self, tool: &str, repo_root: Option<&Path>) -> CliError {
        match self {
            Self::Analysis(error) => CliError::backend_failure(format!(
                "{tool} failed to analyze `{}`",
                repo_root.map_or_else(
                    || "<unknown>".to_string(),
                    |root| root.display().to_string()
                )
            ))
            .with_source(error)
            .with_detail(consts::FIELD_TOOL, json!(tool))
            .with_detail(
                consts::FIELD_ROOT,
                json!(repo_root.map(|root| root.display().to_string())),
            ),
            Self::Serialize(error) => CliError::backend_protocol(format!(
                "{tool} produced a report that could not be encoded as machine JSON"
            ))
            .with_source(error)
            .with_detail(consts::FIELD_TOOL, json!(tool)),
            Self::Normalize(error) => {
                CliError::backend_protocol(format!("{tool} returned malformed machine output"))
                    .with_source(error)
                    .with_detail(consts::FIELD_TOOL, json!(tool))
            }
        }
    }
}

#[expect(
    clippy::result_large_err,
    reason = "Dispatch failures must remain in the shared top-level CliError contract."
)]
pub fn run_sc_boundary(
    _context: &CommandContext,
    loaded_config: &LoadedConfig,
) -> Result<CommandSuccess, CliError> {
    let repo_root = loaded_config.require_repo_root()?;
    let tool = consts::TOOL_BOUNDARY;
    let report = analyze_workspace(&AnalyzeOptions {
        root: repo_root.to_path_buf(),
        format: OutputFormat::Json,
        rule: None,
    })
    .map_err(|error| BoundaryDispatchError::Analysis(Box::new(error)))
    .map_err(|error| error.into_cli_error(tool, Some(repo_root)))?;

    let raw = serde_json::to_string(&report)
        .map_err(BoundaryDispatchError::Serialize)
        .map_err(|error| error.into_cli_error(tool, Some(repo_root)))?;
    let normalized = normalize_backend_json(tool, &raw)?;
    let finding_count = normalized
        .get(consts::FIELD_FINDINGS)
        .and_then(Value::as_array)
        .map_or(0, std::vec::Vec::len);

    Ok(CommandSuccess::with_dispatch(
        normalized,
        DispatchTelemetry::new(tool, finding_count),
    ))
}

#[expect(
    clippy::result_large_err,
    reason = "Backend protocol violations must remain in the shared top-level CliError contract."
)]
pub fn normalize_backend_json(tool: &str, raw: &str) -> Result<Value, CliError> {
    serde_json::from_str(raw)
        .map_err(BoundaryDispatchError::Normalize)
        .map_err(|error| error.into_cli_error(tool, None))
}
