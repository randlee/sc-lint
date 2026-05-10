use serde_json::Value;
use serde_json::json;

use crate::Cli;
use crate::CliError;
use crate::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandContext {
    command_id: String,
    service_name: &'static str,
    summary: &'static str,
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
                summary: "reserved lint contract surface",
            },
            Command::View { target } => Self {
                command_id: format!("view.{}", target.command_suffix()),
                service_name: "sc-lint",
                summary: "reserved view contract surface",
            },
            Command::Check { target } => Self {
                command_id: format!("check.{}", target.command_suffix()),
                service_name: "sc-lint",
                summary: "reserved preflight contract surface",
            },
            Command::Clippy { target } => Self {
                command_id: format!("clippy.{}", target.command_suffix()),
                service_name: "sc-lint",
                summary: "reserved clippy contract surface",
            },
            Command::Version => Self {
                command_id: "version".to_string(),
                service_name: "sc-lint",
                summary: "sc-lint version information",
            },
            Command::Ci => Self {
                command_id: "ci".to_string(),
                service_name: "sc-lint",
                summary: "reserved ci contract surface",
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
}

#[expect(
    clippy::result_large_err,
    reason = "CliError is the stable top-level contract type for the bootstrap CLI execution seam."
)]
pub fn execute(context: &CommandContext) -> Result<Value, CliError> {
    match context.command_id() {
        "version" => Ok(json!({
            "crate_name": "sc-lint",
            "crate_version": env!("CARGO_PKG_VERSION"),
            "contract_schema": "v1",
            "status": "bootstrap_ready",
        })),
        "lint.sc-boundary" => reserved_command(
            context,
            "A.1b will normalize the first real delegated backend path.",
        ),
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

#[expect(
    clippy::result_large_err,
    reason = "Reserved bootstrap commands must return the same top-level CliError contract as real command paths."
)]
fn reserved_command(context: &CommandContext, follow_up: &str) -> Result<Value, CliError> {
    Err(CliError::capability(format!(
        "{} is a reserved contract surface in Sprint A.1a. {follow_up}",
        context.command_id()
    )))
}
