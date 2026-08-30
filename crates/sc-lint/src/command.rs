use serde_json::Value;
use serde_json::json;
use std::path::Path;

use crate::Cli;
use crate::CliError;
use crate::Command;
use crate::DocsGuide;
use crate::config::LoadedConfig;
use crate::consts;
use crate::contract::ServiceName;
use crate::dispatch;
use crate::installer;
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
    CompatibilityCheck,
    Docs,
    Init,
    ConsumerTest,
    Setup,
    Upgrade,
    LintCi,
    ConsumerLintCi,
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

/// Parsed consumer-integration flags remain a single validated request rather
/// than three unrelated booleans at the execution boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ConsumerInitRequest {
    pub(crate) just: bool,
    pub(crate) check: bool,
    pub(crate) dry_run: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DocsRequest {
    pub(crate) guide: Option<DocsGuide>,
    pub(crate) path: bool,
}

/// Command-specific payloads stay coupled to the command variant that owns
/// them. This prevents an unrelated command context from carrying an
/// impossible `None` payload that would only fail during dispatch.
#[derive(Debug, Clone, PartialEq, Eq)]
enum CommandRequest {
    Setup {
        dry_run: bool,
    },
    Upgrade {
        check: bool,
        dry_run: bool,
    },
    Init(ConsumerInitRequest),
    Docs(DocsRequest),
    /// Optional lint profile step or test layer selector for consumer profile commands.
    ConsumerSelector(Option<String>),
}

impl CommandId {
    pub fn from_cli_command(command: &Command) -> Self {
        match command {
            Command::Lint {
                target, consumer, ..
            } => match target {
                crate::LintTarget::ScBoundary => Self::LintScBoundary,
                crate::LintTarget::ScPortability => Self::LintScPortability,
                crate::LintTarget::ScRuntime => Self::LintScRuntime,
                crate::LintTarget::LineCounts => Self::LintLineCounts,
                crate::LintTarget::IdentityLiterals => Self::LintIdentityLiterals,
                crate::LintTarget::Fast => Self::LintFast,
                crate::LintTarget::Full => Self::LintFull,
                crate::LintTarget::Ci if *consumer => Self::ConsumerLintCi,
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
            Command::Compatibility { command } => match command {
                crate::CompatibilityCommand::Check { .. } => Self::CompatibilityCheck,
            },
            Command::Docs { .. } => Self::Docs,
            Command::Setup { .. } => Self::Setup,
            Command::Upgrade { .. } => Self::Upgrade,
            Command::Init { .. } => Self::Init,
            Command::Test { .. } => Self::ConsumerTest,
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
            Self::CompatibilityCheck => "compatibility.check",
            Self::Docs => "docs",
            Self::Init => "init",
            Self::ConsumerTest => "test",
            Self::Setup => "setup",
            Self::Upgrade => "upgrade",
            Self::LintCi => "lint.ci",
            Self::ConsumerLintCi => "lint.ci.consumer",
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
            | Self::CompatibilityCheck
            | Self::Docs
            | Self::Init
            | Self::ConsumerTest
            | Self::Setup
            | Self::Upgrade
            | Self::LintCi
            | Self::ConsumerLintCi
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
            Self::CompatibilityCheck => "installed sc-lint compatibility preflight",
            Self::Docs => "offline documentation discovery",
            Self::Init => "consumer integration generation",
            Self::ConsumerTest => "consumer test profile orchestration",
            Self::Setup => "managed sc-lint installation and repair",
            Self::Upgrade => "managed sc-lint upgrade inspection and activation",
            Self::LintCi | Self::LintFast | Self::LintFull => "lint profile orchestration path",
            Self::ConsumerLintCi => "consumer lint profile orchestration",
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
        !matches!(
            self,
            Self::Version
                | Self::CompatibilityCheck
                | Self::Docs
                | Self::Setup
                | Self::Upgrade
                | Self::Init
                | Self::ConsumerTest
                | Self::ConsumerLintCi
        )
    }

    pub const fn dispatch_tool(self) -> Option<&'static str> {
        match self {
            Self::LintScBoundary => Some(consts::TOOL_BOUNDARY),
            Self::LintScPortability => Some(consts::TOOL_PORTABILITY),
            Self::LintScRuntime => Some(consts::TOOL_RUNTIME),
            _ => match self.python_tool() {
                Some(tool) => Some(tool.tool_name()),
                None => None,
            },
        }
    }

    /// Python-backed tool behind this command, if any.
    pub const fn python_tool(self) -> Option<python_adapter::PythonTool> {
        match self {
            Self::LintLineCounts => Some(python_adapter::PythonTool::LineCounts),
            Self::LintIdentityLiterals => Some(python_adapter::PythonTool::IdentityLiterals),
            Self::ViewFindings => Some(python_adapter::PythonTool::ViewFindings),
            _ => None,
        }
    }

    pub fn adapter_kind(self) -> Option<&'static str> {
        self.python_tool().map(|_| python_adapter::adapter_kind())
    }

    pub fn adapter_config_scope(self) -> Option<&'static str> {
        self.python_tool()
            .map(python_adapter::PythonTool::config_scope)
    }

    pub fn adapter_script(self) -> Option<&'static str> {
        self.python_tool()
            .map(python_adapter::PythonTool::script_relative_path)
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
    compatibility_binary: Option<std::path::PathBuf>,
    request: Option<CommandRequest>,
}

impl CommandContext {
    /// Constructs a command context from parsed CLI arguments.
    ///
    /// This constructor is fallible: it returns a usage error when `--version`
    /// is combined with a subcommand or when neither a subcommand nor
    /// `--version` was provided.
    #[expect(
        clippy::result_large_err,
        reason = "Context construction preserves the shared top-level CliError contract before command dispatch starts."
    )]
    pub fn from_cli(cli: &Cli) -> Result<Self, CliError> {
        let (command_id, compatibility_binary, request) = match (&cli.command, cli.version) {
            (Some(command), false) => {
                if matches!(command, Command::Init { .. })
                    && (cli.config.is_some() || cli.root.is_some())
                {
                    return Err(CliError::usage(
                        "`sc-lint init --just` does not accept `--config` or `--root`",
                    )
                    .with_suggested_action(
                        "Run it from the consumer repository root; it always manages `sc-lint.toml` there.",
                    ));
                }
                if matches!(command, Command::Lint { consumer: true, target, .. } if !matches!(target, crate::LintTarget::Ci))
                {
                    return Err(CliError::usage(
                            "`--consumer` is only supported by `sc-lint lint ci`",
                        )
                        .with_suggested_action(
                            "Use `sc-lint lint --consumer --config sc-lint.toml ci` for a consumer repository.",
                        ));
                }
                let compatibility_binary = match command {
                    Command::Compatibility {
                        command: crate::CompatibilityCommand::Check { binary },
                    } => binary.clone(),
                    _ => None,
                };
                let request = match command {
                    Command::Setup { dry_run } => Some(CommandRequest::Setup { dry_run: *dry_run }),
                    Command::Upgrade { check, dry_run } => Some(CommandRequest::Upgrade {
                        check: *check,
                        dry_run: *dry_run,
                    }),
                    Command::Init {
                        just,
                        check,
                        dry_run,
                    } => Some(CommandRequest::Init(ConsumerInitRequest {
                        just: *just,
                        check: *check,
                        dry_run: *dry_run,
                    })),
                    Command::Docs { guide, path } => Some(CommandRequest::Docs(DocsRequest {
                        guide: *guide,
                        path: *path,
                    })),
                    Command::Lint {
                        consumer: true,
                        profile,
                        ..
                    } => Some(CommandRequest::ConsumerSelector(profile.clone())),
                    Command::Test { layer } => {
                        Some(CommandRequest::ConsumerSelector(layer.clone()))
                    }
                    _ => None,
                };
                (
                    CommandId::from_cli_command(command),
                    compatibility_binary,
                    request,
                )
            }
            (None, true) => (CommandId::Version, None, None),
            (Some(_), true) => {
                return Err(CliError::usage(
                        "`--version` cannot be combined with a subcommand",
                    )
                    .with_suggested_action(
                        "Use either `sc-lint --version` or a subcommand such as `sc-lint version`.",
                    ));
            }
            (None, false) => {
                return Err(
                    CliError::usage("a command is required").with_suggested_action(
                        "Run `sc-lint --help` to inspect the supported command surface.",
                    ),
                );
            }
        };
        let service_name = ServiceName::new(command_id.service_name());

        Ok(Self {
            command_id,
            service_name,
            summary: command_id.summary(),
            requires_repo_root: command_id.requires_repo_root(),
            compatibility_binary,
            request,
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

    pub(crate) const fn requires_compatibility_config(&self) -> bool {
        matches!(
            self.command_id,
            CommandId::CompatibilityCheck
                | CommandId::Setup
                | CommandId::Upgrade
                | CommandId::ConsumerLintCi
                | CommandId::ConsumerTest
        )
    }

    pub(crate) fn compatibility_binary(&self) -> Option<&Path> {
        self.compatibility_binary.as_deref()
    }

    pub(crate) const fn setup_dry_run(&self) -> bool {
        matches!(self.request, Some(CommandRequest::Setup { dry_run: true }))
    }

    pub(crate) const fn upgrade_check(&self) -> bool {
        matches!(
            self.request,
            Some(CommandRequest::Upgrade { check: true, .. })
        )
    }

    pub(crate) const fn upgrade_dry_run(&self) -> bool {
        matches!(
            self.request,
            Some(CommandRequest::Upgrade { dry_run: true, .. })
        )
    }

    pub(crate) fn consumer_init_request(&self) -> ConsumerInitRequest {
        match self.request {
            Some(CommandRequest::Init(request)) => request,
            _ => unreachable!("consumer initialization request is tied to the init command"),
        }
    }

    /// Selector for consumer profile commands: `None` and `all` run every configured step.
    pub(crate) fn consumer_selector(&self) -> Option<&str> {
        match &self.request {
            Some(CommandRequest::ConsumerSelector(selector)) => selector.as_deref(),
            _ => None,
        }
    }

    pub(crate) fn docs_request(&self) -> DocsRequest {
        match self.request {
            Some(CommandRequest::Docs(request)) => request,
            _ => unreachable!("documentation request is tied to the docs command"),
        }
    }

    /// Standalone consumer probes must never initialize repository logging.
    pub const fn skips_logging(&self) -> bool {
        matches!(
            self.command_id,
            CommandId::Version
                | CommandId::CompatibilityCheck
                | CommandId::Setup
                | CommandId::Upgrade
                | CommandId::Init
                | CommandId::Docs
                | CommandId::ConsumerLintCi
                | CommandId::ConsumerTest
        )
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
        CommandId::Version => Ok(CommandSuccess::direct(version_payload())),
        CommandId::CompatibilityCheck => Ok(CommandSuccess::direct(
            loaded_config.evaluate_compatibility(context.compatibility_binary())?,
        )),
        CommandId::Docs => Ok(CommandSuccess::direct(crate::docs::run(
            context.docs_request(),
        )?)),
        CommandId::Setup => Ok(CommandSuccess::direct(installer::run_setup(
            loaded_config,
            context.setup_dry_run(),
        )?)),
        CommandId::Upgrade => Ok(CommandSuccess::direct(installer::run_upgrade(
            loaded_config,
            context.upgrade_check(),
            context.upgrade_dry_run(),
        )?)),
        CommandId::Init => Ok(CommandSuccess::direct(crate::config::run_consumer_init(
            context.consumer_init_request(),
        )?)),
        CommandId::ConsumerLintCi => {
            workflow::run_consumer_lint_profile(loaded_config, context.consumer_selector())
        }
        CommandId::ConsumerTest => {
            workflow::run_consumer_test_profile(loaded_config, context.consumer_selector())
        }
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

pub(crate) fn version_payload() -> Value {
    json!({
        consts::FIELD_TOOL: consts::SERVICE_NAME,
        consts::FIELD_VERSION: env!("CARGO_PKG_VERSION"),
        "contract_schema": crate::config::VERSION_PROBE_SCHEMA,
        "self_contained": dispatch::self_contained_layout(),
        consts::FIELD_STATUS: "pass",
    })
}
