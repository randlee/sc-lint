use serde_json::Value;
use serde_json::json;

use crate::Cli;
use crate::CliError;
use crate::Command;
use crate::config::LoadedConfig;
use crate::dispatch;
use crate::python_adapter;
use crate::workflow;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DispatchTelemetry {
    tool: &'static str,
    finding_count: usize,
}

impl DispatchTelemetry {
    pub const fn new(tool: &'static str, finding_count: usize) -> Self {
        Self {
            tool,
            finding_count,
        }
    }

    pub const fn tool(&self) -> &'static str {
        self.tool
    }

    pub const fn finding_count(&self) -> usize {
        self.finding_count
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CommandSuccess {
    pub data: Value,
    pub dispatch: Option<DispatchTelemetry>,
}

impl CommandSuccess {
    pub fn direct(data: Value) -> Self {
        Self {
            data,
            dispatch: None,
        }
    }

    pub fn with_dispatch(data: Value, dispatch: DispatchTelemetry) -> Self {
        Self {
            data,
            dispatch: Some(dispatch),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandContext {
    command_id: String,
    service_name: &'static str,
    summary: &'static str,
    requires_repo_root: bool,
}

impl CommandContext {
    pub fn from_cli(cli: &Cli) -> Self {
        match &cli.command {
            Command::Lint { target } => Self {
                command_id: format!("lint.{}", target.command_suffix()),
                service_name: match target {
                    crate::LintTarget::ScBoundary => "sc-boundary",
                    crate::LintTarget::ScPortability => "sc-portability",
                    crate::LintTarget::ScRuntime => "sc-runtime",
                    crate::LintTarget::LineCounts | crate::LintTarget::IdentityLiterals => {
                        "sc-lint"
                    }
                    crate::LintTarget::Fast | crate::LintTarget::Full | crate::LintTarget::Ci => {
                        "sc-lint"
                    }
                },
                summary: match target {
                    crate::LintTarget::ScBoundary => "boundary analyzer command path",
                    crate::LintTarget::ScPortability => {
                        "reserved portability analyzer contract surface"
                    }
                    crate::LintTarget::ScRuntime => "reserved runtime analyzer contract surface",
                    crate::LintTarget::LineCounts => "python-backed line-count lint path",
                    crate::LintTarget::IdentityLiterals => {
                        "python-backed identity literal lint path"
                    }
                    crate::LintTarget::Fast | crate::LintTarget::Full | crate::LintTarget::Ci => {
                        "lint profile orchestration path"
                    }
                },
                requires_repo_root: true,
            },
            Command::View { target } => Self {
                command_id: format!("view.{}", target.command_suffix()),
                service_name: "sc-lint",
                summary: match target {
                    crate::ViewTarget::Graph => "reserved view contract surface",
                    crate::ViewTarget::Findings => "python-backed findings view path",
                },
                requires_repo_root: true,
            },
            Command::Check { target } => Self {
                command_id: format!("check.{}", target.command_suffix()),
                service_name: "sc-lint",
                summary: "preflight execution path",
                requires_repo_root: true,
            },
            Command::Clippy { target } => Self {
                command_id: format!("clippy.{}", target.command_suffix()),
                service_name: "sc-lint",
                summary: "clippy execution path",
                requires_repo_root: true,
            },
            Command::Version => Self {
                command_id: "version".to_string(),
                service_name: "sc-lint",
                summary: "sc-lint version information",
                requires_repo_root: false,
            },
            Command::Ci => Self {
                command_id: "ci".to_string(),
                service_name: "sc-lint",
                summary: "top-level ci orchestration path",
                requires_repo_root: true,
            },
        }
    }

    pub fn command_id(&self) -> &str {
        &self.command_id
    }

    pub const fn service_name(&self) -> &'static str {
        self.service_name
    }

    pub const fn summary(&self) -> &'static str {
        self.summary
    }

    pub const fn requires_repo_root(&self) -> bool {
        self.requires_repo_root
    }

    pub fn dispatch_tool(&self) -> Option<&'static str> {
        match self.command_id.as_str() {
            "lint.sc-boundary" => Some("sc-lint-boundary"),
            "lint.line-counts" => Some("sc-lint-line-counts"),
            "lint.identity-literals" => Some("sc-lint-identity-literals"),
            "view.findings" => Some("sc-lint-view-findings"),
            _ => None,
        }
    }
}

#[expect(
    clippy::result_large_err,
    reason = "CliError is the stable top-level contract type for the bootstrap CLI execution seam."
)]
pub fn execute(
    context: &CommandContext,
    loaded_config: &LoadedConfig,
) -> Result<CommandSuccess, CliError> {
    match context.command_id() {
        "version" => Ok(CommandSuccess::direct(json!({
            "crate_name": "sc-lint",
            "crate_version": env!("CARGO_PKG_VERSION"),
            "contract_schema": "v1",
            "status": "dispatch_ready",
        }))),
        "lint.sc-boundary" => dispatch::run_sc_boundary(context, loaded_config),
        "lint.sc-portability" => reserved_command(
            context,
            "A.4 will add the portability analyzer backend path.",
        ),
        "lint.sc-runtime" => {
            reserved_command(context, "A.5 will add the runtime analyzer backend path.")
        }
        "lint.line-counts" => {
            python_adapter::run_python_tool(loaded_config, python_adapter::PythonTool::LineCounts)
        }
        "lint.identity-literals" => python_adapter::run_python_tool(
            loaded_config,
            python_adapter::PythonTool::IdentityLiterals,
        ),
        "lint.fast" => workflow::run_lint_profile(loaded_config, crate::LintProfile::Fast),
        "lint.full" => workflow::run_lint_profile(loaded_config, crate::LintProfile::Full),
        "lint.ci" => workflow::run_lint_profile(loaded_config, crate::LintProfile::Ci),
        "view.graph" => reserved_command(
            context,
            "A later sprint will connect graph-oriented view surfaces once the contract is stable.",
        ),
        "view.findings" => {
            python_adapter::run_python_tool(loaded_config, python_adapter::PythonTool::ViewFindings)
        }
        "check.native" => workflow::run_check(loaded_config, crate::CheckTarget::Native),
        "check.xwin" => workflow::run_check(loaded_config, crate::CheckTarget::Xwin),
        "clippy.native" => workflow::run_clippy(loaded_config, crate::ClippyTarget::Native),
        "clippy.xwin" => workflow::run_clippy(loaded_config, crate::ClippyTarget::Xwin),
        "ci" => workflow::run_ci(loaded_config),
        _ => Err(CliError::internal(format!(
            "unknown command identity `{}` reached the execution layer",
            context.command_id()
        ))),
    }
}

#[expect(
    clippy::result_large_err,
    reason = "Reserved bootstrap commands must return the same top-level CliError contract as real command paths."
)]
fn reserved_command(context: &CommandContext, follow_up: &str) -> Result<CommandSuccess, CliError> {
    Err(CliError::capability(format!(
        "{} is a reserved contract surface. {follow_up}",
        context.command_id()
    )))
}
