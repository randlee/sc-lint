use serde_json::Value;
use serde_json::json;
use std::ffi::OsString;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command as ProcessCommand;

use crate::Cli;
use crate::CliError;
use crate::Command;
use crate::DocsGuide;
use crate::config::LoadedConfig;
use crate::configure::reviewed_removals::{
    add_just_artifacts, add_managed_creation, add_reviewed_removals, configure_apply_error,
    ensure_applyable_plan, ensure_reviewed_plan_matches, plan_proposes, read_configure_request,
};
use crate::consts;
use crate::consts::CLI_CONFIGURE_UNMANAGED_COLLISION;
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
    ConfigurePlan,
    ConfigureApply,
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

/// Configure owns an explicit request and root so no standard repository
/// discovery or configuration loading can broaden the F.2 observation scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ConfigureRequest {
    request_path: PathBuf,
    root: PathBuf,
    dry_run: bool,
    reviewed_plan_path: Option<PathBuf>,
}

/// Command-specific payloads stay coupled to the command variant that owns
/// them. This prevents an unrelated command context from carrying an
/// impossible `None` payload that would only fail during dispatch.
#[derive(Debug, Clone, PartialEq, Eq)]
enum CommandRequest {
    Setup { dry_run: bool },
    Upgrade { check: bool, dry_run: bool },
    Init(ConsumerInitRequest),
    Docs(DocsRequest),
    Configure(ConfigureRequest),
}

impl CommandId {
    pub fn from_cli_command(command: &Command) -> Self {
        match command {
            Command::Lint { target, consumer } => match target {
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
            Command::Configure { apply, .. } if *apply => Self::ConfigureApply,
            Command::Configure { .. } => Self::ConfigurePlan,
            Command::Setup { .. } => Self::Setup,
            Command::Upgrade { .. } => Self::Upgrade,
            Command::Init { .. } => Self::Init,
            Command::Test => Self::ConsumerTest,
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
            Self::ConfigurePlan => "configure.plan",
            Self::ConfigureApply => "configure.apply",
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
            | Self::ConfigurePlan
            | Self::ConfigureApply
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
            Self::ConfigurePlan => "no-write consumer configuration planning",
            Self::ConfigureApply => "reviewed consumer configuration apply",
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
                | Self::ConfigurePlan
                | Self::ConfigureApply
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
                if matches!(command, Command::Configure { .. }) && cli.config.is_some() {
                    return Err(CliError::usage(
                        "`sc-lint configure` does not accept `--config`",
                    )
                    .with_suggested_action(
                        "Put the requested minimum version in the JSON request and pass the consumer path with `--root`.",
                    ));
                }
                if matches!(command, Command::Lint { consumer: true, target } if !matches!(target, crate::LintTarget::Ci))
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
                    Command::Configure {
                        request,
                        dry_run,
                        apply,
                        plan,
                    } => {
                        let root = cli.root.clone().ok_or_else(|| {
                            CliError::usage("`sc-lint configure` requires `--root <path>`")
                                .with_suggested_action(
                                    "Pass the consumer repository directory with `--root <path>`.",
                                )
                        })?;
                        Some(CommandRequest::Configure(ConfigureRequest {
                            request_path: request.clone(),
                            root,
                            dry_run: *dry_run,
                            reviewed_plan_path: if *apply { plan.clone() } else { None },
                        }))
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
        match self.request.as_ref() {
            Some(CommandRequest::Init(request)) => *request,
            _ => unreachable!("consumer initialization request is tied to the init command"),
        }
    }

    pub(crate) fn docs_request(&self) -> DocsRequest {
        match self.request.as_ref() {
            Some(CommandRequest::Docs(request)) => *request,
            _ => unreachable!("documentation request is tied to the docs command"),
        }
    }

    pub(crate) fn configure_request(&self) -> &ConfigureRequest {
        match &self.request {
            Some(CommandRequest::Configure(request)) => request,
            _ => unreachable!("configure request is tied to the configure command"),
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
                | CommandId::ConfigurePlan
                | CommandId::ConfigureApply
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
        CommandId::Version => Ok(CommandSuccess::direct(json!({
            consts::FIELD_TOOL: consts::SERVICE_NAME,
            consts::FIELD_VERSION: env!("CARGO_PKG_VERSION"),
            "contract_schema": crate::config::VERSION_PROBE_SCHEMA,
            consts::FIELD_STATUS: "pass",
        }))),
        CommandId::CompatibilityCheck => Ok(CommandSuccess::direct(
            loaded_config.evaluate_compatibility(context.compatibility_binary())?,
        )),
        CommandId::Docs => Ok(CommandSuccess::direct(crate::docs::run(
            context.docs_request(),
        )?)),
        CommandId::ConfigurePlan => run_configure(context.configure_request()),
        CommandId::ConfigureApply => run_configure_apply(context.configure_request()),
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
        CommandId::ConsumerLintCi => workflow::run_consumer_lint_profile(loaded_config),
        CommandId::ConsumerTest => workflow::run_consumer_test_profile(loaded_config),
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
    reason = "The thin configure dispatcher must retain stable recovery data while it normalizes the Python planner result."
)]
fn run_configure(request: &ConfigureRequest) -> Result<CommandSuccess, CliError> {
    let script_path = configure_script_path()?;
    let mut command = ProcessCommand::new(python_command());
    command
        .arg(&script_path)
        .arg("--request")
        .arg(&request.request_path)
        .arg("--root")
        .arg(&request.root)
        .arg("--json");
    if request.dry_run {
        command.arg("--dry-run");
    }
    let output = command.output().map_err(|error| {
        CliError::backend_failure("sc-lint configure planner failed to start")
            .with_source(error)
            .with_detail(consts::FIELD_SCRIPT, json!("scripts/sc_lint_configure.py"))
    })?;
    let payload: Value = serde_json::from_slice(&output.stdout).map_err(|error| {
        CliError::backend_protocol("sc-lint configure planner returned malformed JSON")
            .with_source(error)
            .with_detail(consts::FIELD_SCRIPT, json!("scripts/sc_lint_configure.py"))
    })?;
    let object = payload.as_object().ok_or_else(|| {
        CliError::backend_protocol("sc-lint configure planner returned a non-object payload")
            .with_detail(consts::FIELD_SCRIPT, json!("scripts/sc_lint_configure.py"))
    })?;
    match object.get("ok").and_then(Value::as_bool) {
        Some(true) => {
            let data = object
                .get("data")
                .cloned()
                .filter(Value::is_object)
                .ok_or_else(|| {
                    CliError::backend_protocol("sc-lint configure planner returned no plan data")
                        .with_detail(consts::FIELD_SCRIPT, json!("scripts/sc_lint_configure.py"))
                })?;
            Ok(CommandSuccess::direct(data))
        }
        Some(false) => Err(normalize_configure_failure(object)?),
        None => Err(CliError::backend_protocol(
            "sc-lint configure planner omitted the result status",
        )
        .with_detail(consts::FIELD_SCRIPT, json!("scripts/sc_lint_configure.py"))),
    }
}

/// Apply never trusts a serialized plan on its own.  It first invokes the
/// bounded F.2 planner again, then requires both its identifier and its
/// per-path source-digest preconditions to be byte-for-byte unchanged.
#[expect(
    clippy::result_large_err,
    reason = "Reviewed-plan rejections retain the F.1 configure error envelope."
)]
fn run_configure_apply(request: &ConfigureRequest) -> Result<CommandSuccess, CliError> {
    let reviewed_path = request
        .reviewed_plan_path
        .as_ref()
        .expect("apply requires --plan at CLI parsing");
    let reviewed_raw = std::fs::read(reviewed_path).map_err(|error| {
        configure_apply_error(
            "CLI.CONFIGURE_STALE_PLAN",
            "the reviewed configure plan could not be read",
            error.to_string(),
            "Supply the JSON plan you reviewed, then rerun configure --apply.",
        )
    })?;
    let reviewed_json: Value = serde_json::from_slice(&reviewed_raw).map_err(|error| {
        configure_apply_error(
            "CLI.CONFIGURE_STALE_PLAN",
            "the reviewed configure plan is not valid JSON",
            error.to_string(),
            "Regenerate, review, and save the configure plan before applying it.",
        )
    })?;
    let reviewed = reviewed_json.get("data").cloned().unwrap_or(reviewed_json);
    let fresh = run_configure(request)?.data;
    ensure_reviewed_plan_matches(&reviewed, &fresh)?;
    ensure_applyable_plan(&fresh)?;

    let request_json = read_configure_request(&request.request_path)?;
    let minimum_version = request_json
        .pointer("/request/minimum_version")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            configure_apply_error(
                "CLI.CONFIGURE_STALE_PLAN",
                "the reviewed configure request has no minimum version",
                "the request no longer satisfies the v1 configure schema",
                "Regenerate and review the configure plan from a valid request.",
            )
        })?;
    let just_mode = request_json
        .pointer("/request/just/mode")
        .and_then(Value::as_str)
        .unwrap_or("disabled");
    let mut artifacts: Vec<Box<dyn crate::configure::artifact::ManagedArtifact>> = Vec::new();
    if plan_proposes(&fresh, "sc-lint.toml") {
        artifacts.push(Box::new(crate::configure::artifact::BytesArtifact::new(
            crate::configure::artifact::ArtifactKind::Toml,
            request.root.join("sc-lint.toml"),
            crate::consumer_integration::canonical_consumer_config_for(minimum_version)
                .into_bytes(),
        )));
    }
    if plan_proposes(&fresh, ".sc-lint/bootstrap") {
        add_managed_creation(
            &mut artifacts,
            crate::configure::artifact::ArtifactKind::Shell,
            request.root.join(".sc-lint/bootstrap"),
            crate::consumer_integration::generated_bootstrap_asset().into_bytes(),
        )?;
    }
    if plan_proposes(&fresh, ".sc-lint/bootstrap.ps1") {
        add_managed_creation(
            &mut artifacts,
            crate::configure::artifact::ArtifactKind::Shell,
            request.root.join(".sc-lint/bootstrap.ps1"),
            crate::consumer_integration::generated_powershell_bootstrap_asset().into_bytes(),
        )?;
    }
    if just_mode == "generate_managed_import" {
        add_just_artifacts(&request.root, &fresh, &mut artifacts)?;
    }
    crate::configure::workflow::add_workflow_artifact(&request.root, &fresh, &mut artifacts)?;
    add_reviewed_removals(&request.root, &fresh, &mut artifacts)?;
    if artifacts.is_empty() {
        return Ok(CommandSuccess::direct(serde_json::json!({
            "status": "current",
            "plan_id": fresh.get("plan_id"),
            "changed_paths": [],
            "summary": "reviewed configure plan required no writes",
        })));
    }
    if request.dry_run {
        return Ok(CommandSuccess::direct(serde_json::json!({
            "status": "would_apply",
            "plan_id": fresh.get("plan_id"),
            "changed_paths": artifacts.iter().map(|artifact| artifact.target().display().to_string()).collect::<Vec<_>>(),
            "summary": "reviewed configure plan would apply without writing files",
        })));
    }
    let changed_paths = artifacts
        .iter()
        .map(|artifact| artifact.target().display().to_string())
        .collect::<Vec<_>>();
    let changed_artifacts = artifacts
        .iter()
        .map(|artifact| format!("{:?}", artifact.kind()))
        .collect::<Vec<_>>();
    crate::configure::apply::commit(artifacts)?;
    Ok(CommandSuccess::direct(serde_json::json!({
        "status": "applied",
        "plan_id": fresh.get("plan_id"),
        "changed_paths": changed_paths,
        "changed_artifacts": changed_artifacts,
        "summary": "reviewed configure plan applied transactionally",
    })))
}

fn configure_script_candidates_for_executable(executable: &Path) -> Vec<PathBuf> {
    let Some(bin_dir) = executable.parent() else {
        return Vec::new();
    };
    let mut candidates = vec![
        bin_dir.join("sc_lint_configure.py"),
        bin_dir.join("scripts/sc_lint_configure.py"),
    ];
    if let Some(prefix) = bin_dir.parent() {
        candidates.push(prefix.join("scripts/sc_lint_configure.py"));
        candidates.push(prefix.join("share/sc-lint/scripts/sc_lint_configure.py"));
    }
    candidates
}

fn configure_script_candidates() -> Vec<PathBuf> {
    let mut candidates = std::env::current_exe()
        .map(|executable| configure_script_candidates_for_executable(&executable))
        .unwrap_or_default();
    // Keep source-checkout tests and `cargo run` usable only after every
    // installed/release/Homebrew layout has had priority.
    candidates.push(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("sc-lint crate has a workspace root")
            .join("scripts/sc_lint_configure.py"),
    );
    candidates
}

#[expect(
    clippy::result_large_err,
    reason = "Missing configure assets must retain the shared CLI recovery contract."
)]
fn configure_script_path() -> Result<PathBuf, CliError> {
    configure_script_path_from_candidates(&configure_script_candidates())
}

#[expect(
    clippy::result_large_err,
    reason = "Installed-layout resolution reports a structured failure when neither planner asset is present."
)]
fn configure_script_path_from_candidates(candidates: &[PathBuf]) -> Result<PathBuf, CliError> {
    if let Some(path) = candidates.iter().find(|candidate| {
        candidate.is_file()
            && candidate
                .parent()
                .is_some_and(|directory| directory.join("sc_lint_configure_schema.py").is_file())
    }) {
        return Ok(path.clone());
    }
    let searched = candidates
        .first()
        .cloned()
        .unwrap_or_else(|| PathBuf::from("scripts/sc_lint_configure.py"));
    Err(CliError::backend_failure("sc-lint configure planner asset is unavailable")
        .with_cause("the planner script or its schema helper was not found in a supported install layout")
        .with_detail(consts::FIELD_SCRIPT, json!(searched.display().to_string()))
        .with_suggested_action(
            "Install a matching sc-lint release that includes the configure planner assets, then retry.",
        )
        .with_documentation("sc-lint docs installation"))
}

#[expect(
    clippy::result_large_err,
    reason = "Protocol validation must return the fully structured CliError that explains malformed configure planner output."
)]
fn normalize_configure_failure(
    object: &serde_json::Map<String, Value>,
) -> Result<CliError, CliError> {
    let error = object
        .get("error")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            CliError::backend_protocol("sc-lint configure planner returned no error object")
                .with_detail(consts::FIELD_SCRIPT, json!("scripts/sc_lint_configure.py"))
        })?;
    let message = error
        .get("message")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            CliError::backend_protocol("sc-lint configure planner error omitted message")
                .with_detail(consts::FIELD_SCRIPT, json!("scripts/sc_lint_configure.py"))
        })?;
    let code = error.get("code").and_then(Value::as_str).ok_or_else(|| {
        CliError::backend_protocol("sc-lint configure planner error omitted code")
            .with_detail(consts::FIELD_SCRIPT, json!("scripts/sc_lint_configure.py"))
    })?;
    let stable_code = match code {
        "CLI.CONFIGURE_UNSUPPORTED_SCHEMA" => "CLI.CONFIGURE_UNSUPPORTED_SCHEMA",
        "CLI.CONFIGURE_UI_UNAVAILABLE" => "CLI.CONFIGURE_UI_UNAVAILABLE",
        CLI_CONFIGURE_UNMANAGED_COLLISION => CLI_CONFIGURE_UNMANAGED_COLLISION,
        "CLI.CONFIGURE_STALE_PLAN" => "CLI.CONFIGURE_STALE_PLAN",
        "CLI.CONFIGURE_ROLLBACK_FAILED" => "CLI.CONFIGURE_ROLLBACK_FAILED",
        _ => {
            return Err(CliError::backend_protocol(
                "sc-lint configure planner returned an unknown stable error code",
            )
            .with_detail("reported_code", json!(code)));
        }
    };
    let cause = required_configure_string(error, "cause")?;
    let pointer = required_configure_pointer(error)?;
    let recovery = required_configure_string(error, "recovery")?;
    let recovery_description = required_configure_string(error, "recovery_description")?;
    let docs_ref = required_configure_string(error, "docs_ref")?;
    let normalized = CliError::config(message)
        .with_code(stable_code)
        .with_detail(consts::FIELD_SCRIPT, json!("scripts/sc_lint_configure.py"))
        .with_cause(cause)
        .with_detail("pointer", pointer)
        .with_detail("recovery", json!(recovery))
        .with_suggested_action(recovery_description)
        .with_documentation(docs_ref);
    Ok(normalized)
}

#[expect(
    clippy::result_large_err,
    reason = "A malformed configure error payload must report a structured protocol failure."
)]
fn required_configure_string(
    error: &serde_json::Map<String, Value>,
    field: &'static str,
) -> Result<String, CliError> {
    error
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| {
            CliError::backend_protocol(format!("sc-lint configure planner error omitted {field}"))
                .with_detail(consts::FIELD_SCRIPT, json!("scripts/sc_lint_configure.py"))
        })
        .map(ToOwned::to_owned)
}

#[expect(
    clippy::result_large_err,
    reason = "A malformed configure error payload must report a structured protocol failure."
)]
fn required_configure_pointer(error: &serde_json::Map<String, Value>) -> Result<Value, CliError> {
    let pointer = error.get("pointer").ok_or_else(|| {
        CliError::backend_protocol("sc-lint configure planner error omitted pointer")
            .with_detail(consts::FIELD_SCRIPT, json!("scripts/sc_lint_configure.py"))
    })?;
    if pointer.is_null() || pointer.is_string() {
        return Ok(pointer.clone());
    }
    Err(
        CliError::backend_protocol("sc-lint configure planner error returned a non-string pointer")
            .with_detail(consts::FIELD_SCRIPT, json!("scripts/sc_lint_configure.py")),
    )
}

fn python_command() -> OsString {
    if cfg!(windows) {
        OsString::from("python")
    } else {
        OsString::from("python3")
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

#[cfg(test)]
mod configure_script_tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn installed_layout_script_candidate_wins_over_source_fallback() {
        let fixture = TempDir::new().expect("fixture");
        let bin = fixture.path().join("bin");
        let scripts = fixture.path().join("share/sc-lint/scripts");
        fs::create_dir_all(&bin).expect("bin directory");
        fs::create_dir_all(&scripts).expect("installed scripts directory");
        let executable = bin.join("sc-lint");
        let planner = scripts.join("sc_lint_configure.py");
        fs::write(&planner, "# planner\n").expect("planner asset");
        fs::write(
            scripts.join("sc_lint_configure_schema.py"),
            "# schema helper\n",
        )
        .expect("schema helper asset");

        let resolved = configure_script_path_from_candidates(
            &configure_script_candidates_for_executable(&executable),
        )
        .expect("installed candidate resolves");
        assert_eq!(resolved, planner);
    }
}
