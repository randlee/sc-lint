use sc_observability::Logger;
use serde_json::Value;
use serde_json::json;

use crate::Cli;
use crate::CliError;
use crate::Command;
use crate::config::LoadedConfig;
use crate::dispatch;

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
                    crate::LintTarget::Fast | crate::LintTarget::Full | crate::LintTarget::Ci => {
                        "sc-lint"
                    }
                },
                summary: match target {
                    crate::LintTarget::ScBoundary => "boundary analyzer command path",
                    crate::LintTarget::ScPortability
                    | crate::LintTarget::ScRuntime
                    | crate::LintTarget::Fast
                    | crate::LintTarget::Full
                    | crate::LintTarget::Ci => "reserved lint contract surface",
                },
                requires_repo_root: true,
            },
            Command::View { target } => Self {
                command_id: format!("view.{}", target.command_suffix()),
                service_name: "sc-lint",
                summary: "reserved view contract surface",
                requires_repo_root: true,
            },
            Command::Check { target } => Self {
                command_id: format!("check.{}", target.command_suffix()),
                service_name: "sc-lint",
                summary: "reserved preflight contract surface",
                requires_repo_root: true,
            },
            Command::Clippy { target } => Self {
                command_id: format!("clippy.{}", target.command_suffix()),
                service_name: "sc-lint",
                summary: "reserved clippy contract surface",
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
                summary: "reserved ci contract surface",
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
}

pub fn execute(
    context: &CommandContext,
    loaded_config: &LoadedConfig,
    logger: Option<&Logger>,
) -> Result<Value, CliError> {
    match context.command_id() {
        "version" => Ok(json!({
            "crate_name": "sc-lint",
            "crate_version": env!("CARGO_PKG_VERSION"),
            "contract_schema": "v1",
            "status": "dispatch_ready",
        })),
        "lint.sc-boundary" => dispatch::run_sc_boundary(context, loaded_config, logger),
        "lint.sc-portability" => reserved_command(
            context,
            "A.4 will add the portability analyzer backend path.",
        ),
        "lint.sc-runtime" => {
            reserved_command(context, "A.5 will add the runtime analyzer backend path.")
        }
        "lint.fast" | "lint.full" | "lint.ci" => reserved_command(
            context,
            "A.2 will materialize named lint profiles behind the stable contract.",
        ),
        "view.graph" | "view.findings" => reserved_command(
            context,
            "A.3 will connect the reserved view surfaces to extracted utility paths.",
        ),
        "check.native" | "check.xwin" | "clippy.native" | "clippy.xwin" => reserved_command(
            context,
            "A.2 will add the preflight execution strategy and capability-aware dispatch.",
        ),
        "ci" => reserved_command(
            context,
            "A.2 will compose lint profiles and test execution into the top-level CI flow.",
        ),
        _ => Err(CliError::internal(format!(
            "unknown command identity `{}` reached the execution layer",
            context.command_id()
        ))),
    }
}

fn reserved_command(context: &CommandContext, follow_up: &str) -> Result<Value, CliError> {
    Err(CliError::capability(format!(
        "{} is a reserved contract surface in Sprint A.1b. {follow_up}",
        context.command_id()
    )))
}
