use serde_json::Value;
use serde_json::json;

use crate::Cli;
use crate::CliError;
use crate::Command;
use crate::config::LoadedConfig;
use crate::consts;
use crate::contract::ServiceName;
use crate::dispatch;
use crate::python_adapter;
use crate::workflow;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DispatchTelemetry {
    tool: &'static str,
    finding_count: usize,
}

impl DispatchTelemetry {
    pub(crate) const fn new(tool: &'static str, finding_count: usize) -> Self {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CommandId {
    Ci,
    CheckNative,
    CheckXwin,
    ClippyNative,
    ClippyXwin,
    LintCi,
    LintFast,
    LintFull,
    LintIdentityLiterals,
    LintLineCounts,
    LintScBoundary,
    LintScPortability,
    LintScRuntime,
    Version,
    ViewFindings,
    ViewGraph,
}

impl CommandId {
    pub fn from_cli_command(command: &Command) -> Self {
        match command {
            Command::Lint { target } => match target {
                crate::LintTarget::ScBoundary => Self::LintScBoundary,
                crate::LintTarget::ScPortability => Self::LintScPortability,
                crate::LintTarget::ScRuntime => Self::LintScRuntime,
                crate::LintTarget::LineCounts => Self::LintLineCounts,
                crate::LintTarget::IdentityLiterals => Self::LintIdentityLiterals,
                crate::LintTarget::Fast => Self::LintFast,
                crate::LintTarget::Full => Self::LintFull,
                crate::LintTarget::Ci => Self::LintCi,
            },
            Command::View { target } => match target {
                crate::ViewTarget::Graph => Self::ViewGraph,
                crate::ViewTarget::Findings => Self::ViewFindings,
            },
            Command::Check { target } => match target {
                crate::CheckTarget::Native => Self::CheckNative,
                crate::CheckTarget::Xwin => Self::CheckXwin,
            },
            Command::Clippy { target } => match target {
                crate::ClippyTarget::Native => Self::ClippyNative,
                crate::ClippyTarget::Xwin => Self::ClippyXwin,
            },
            Command::Version => Self::Version,
            Command::Ci => Self::Ci,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ci => "ci",
            Self::CheckNative => "check.native",
            Self::CheckXwin => "check.xwin",
            Self::ClippyNative => "clippy.native",
            Self::ClippyXwin => "clippy.xwin",
            Self::LintCi => "lint.ci",
            Self::LintFast => "lint.fast",
            Self::LintFull => "lint.full",
            Self::LintIdentityLiterals => "lint.identity-literals",
            Self::LintLineCounts => "lint.line-counts",
            Self::LintScBoundary => consts::CMD_BOUNDARY,
            Self::LintScPortability => consts::CMD_PORTABILITY,
            Self::LintScRuntime => consts::CMD_RUNTIME,
            Self::Version => "version",
            Self::ViewFindings => "view.findings",
            Self::ViewGraph => "view.graph",
        }
    }

    pub const fn service_name(self) -> &'static str {
        match self {
            Self::LintScBoundary => "sc-boundary",
            Self::LintScPortability => "sc-portability",
            Self::LintScRuntime => "sc-runtime",
            Self::Ci
            | Self::CheckNative
            | Self::CheckXwin
            | Self::ClippyNative
            | Self::ClippyXwin
            | Self::LintCi
            | Self::LintFast
            | Self::LintFull
            | Self::LintIdentityLiterals
            | Self::LintLineCounts
            | Self::Version
            | Self::ViewFindings
            | Self::ViewGraph => consts::SERVICE_NAME,
        }
    }

    pub const fn summary(self) -> &'static str {
        match self {
            Self::Ci => "top-level ci orchestration path",
            Self::CheckNative | Self::CheckXwin => "preflight execution path",
            Self::ClippyNative | Self::ClippyXwin => "clippy execution path",
            Self::LintCi | Self::LintFast | Self::LintFull => "lint profile orchestration path",
            Self::LintIdentityLiterals => "python-backed identity literal lint path",
            Self::LintLineCounts => "python-backed line-count lint path",
            Self::LintScBoundary => "boundary analyzer command path",
            Self::LintScPortability => "portability analyzer command path",
            Self::LintScRuntime => "runtime analyzer command path",
            Self::Version => "sc-lint version information",
            Self::ViewFindings => "python-backed findings view path",
            Self::ViewGraph => "reserved view contract surface",
        }
    }

    pub const fn requires_repo_root(self) -> bool {
        !matches!(self, Self::Version)
    }

    pub const fn dispatch_tool(self) -> Option<&'static str> {
        match self {
            Self::LintScBoundary => Some(consts::TOOL_BOUNDARY),
            Self::LintScPortability => Some(consts::TOOL_PORTABILITY),
            Self::LintScRuntime => Some(consts::TOOL_RUNTIME),
            Self::LintLineCounts => Some(python_adapter::PythonTool::LineCounts.tool_name()),
            Self::LintIdentityLiterals => {
                Some(python_adapter::PythonTool::IdentityLiterals.tool_name())
            }
            Self::ViewFindings => Some(python_adapter::PythonTool::ViewFindings.tool_name()),
            _ => None,
        }
    }

    pub fn adapter_kind(self) -> Option<&'static str> {
        python_adapter::adapter_kind_for_command(self.as_str())
    }

    pub fn adapter_config_scope(self) -> Option<&'static str> {
        python_adapter::adapter_config_scope_for_command(self.as_str())
    }

    pub fn adapter_script(self) -> Option<&'static str> {
        python_adapter::adapter_script_for_command(self.as_str())
    }

    pub const fn is_xwin_preflight(self) -> bool {
        matches!(self, Self::CheckXwin | Self::ClippyXwin)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandContext {
    command_id: CommandId,
    service_name: ServiceName,
    summary: &'static str,
    requires_repo_root: bool,
}

impl CommandContext {
    #[expect(
        clippy::result_large_err,
        reason = "Context construction preserves the shared top-level CliError contract before command dispatch starts."
    )]
    pub fn from_cli(cli: &Cli) -> Result<Self, CliError> {
        let command_id = CommandId::from_cli_command(&cli.command);
        let service_name = ServiceName::new(command_id.service_name());

        Ok(Self {
            command_id,
            service_name,
            summary: command_id.summary(),
            requires_repo_root: command_id.requires_repo_root(),
        })
    }

    pub fn command_id(&self) -> &str {
        self.command_id.as_str()
    }

    pub fn service_name(&self) -> &str {
        self.service_name.as_str()
    }

    pub(crate) const fn id(&self) -> CommandId {
        self.command_id
    }

    pub const fn summary(&self) -> &'static str {
        self.summary
    }

    pub(crate) const fn requires_repo_root(&self) -> bool {
        self.requires_repo_root
    }

    pub fn dispatch_tool(&self) -> Option<&'static str> {
        self.command_id.dispatch_tool()
    }

    pub fn adapter_kind(&self) -> Option<&'static str> {
        self.command_id.adapter_kind()
    }

    pub fn adapter_config_scope(&self) -> Option<&'static str> {
        self.command_id.adapter_config_scope()
    }

    pub fn adapter_script(&self) -> Option<&'static str> {
        self.command_id.adapter_script()
    }

    pub const fn is_xwin_preflight(&self) -> bool {
        self.command_id.is_xwin_preflight()
    }
}

#[expect(
    clippy::result_large_err,
    reason = "CliError is the stable top-level contract type for the bootstrap CLI execution seam."
)]
pub(crate) fn execute(
    context: &CommandContext,
    loaded_config: &LoadedConfig,
) -> Result<CommandSuccess, CliError> {
    match context.id() {
        CommandId::Version => Ok(CommandSuccess::direct(json!({
            consts::FIELD_CRATE_NAME: consts::SERVICE_NAME,
            consts::FIELD_CRATE_VERSION: env!("CARGO_PKG_VERSION"),
            "contract_schema": "v1",
            consts::FIELD_STATUS: "dispatch_ready",
        }))),
        CommandId::LintScBoundary => dispatch::run_sc_boundary(context, loaded_config),
        CommandId::LintScPortability => dispatch::run_sc_portability(context, loaded_config),
        CommandId::LintScRuntime => dispatch::run_sc_runtime(context, loaded_config),
        CommandId::LintLineCounts => {
            python_adapter::run_python_tool(loaded_config, python_adapter::PythonTool::LineCounts)
        }
        CommandId::LintIdentityLiterals => python_adapter::run_python_tool(
            loaded_config,
            python_adapter::PythonTool::IdentityLiterals,
        ),
        CommandId::LintFast => {
            workflow::run_lint_profile(loaded_config, crate::cli::LintProfile::Fast)
        }
        CommandId::LintFull => {
            workflow::run_lint_profile(loaded_config, crate::cli::LintProfile::Full)
        }
        CommandId::LintCi => workflow::run_lint_profile(loaded_config, crate::cli::LintProfile::Ci),
        CommandId::ViewGraph => reserved_command(
            context,
            "A later sprint will connect graph-oriented view surfaces once the contract is stable.",
        ),
        CommandId::ViewFindings => {
            python_adapter::run_python_tool(loaded_config, python_adapter::PythonTool::ViewFindings)
        }
        CommandId::CheckNative => workflow::run_check(loaded_config, crate::CheckTarget::Native),
        CommandId::CheckXwin => workflow::run_check(loaded_config, crate::CheckTarget::Xwin),
        CommandId::ClippyNative => workflow::run_clippy(loaded_config, crate::ClippyTarget::Native),
        CommandId::ClippyXwin => workflow::run_clippy(loaded_config, crate::ClippyTarget::Xwin),
        CommandId::Ci => workflow::run_ci(loaded_config),
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
