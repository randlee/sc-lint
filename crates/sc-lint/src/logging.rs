use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::Duration;

use sc_observability::ActionName;
use sc_observability::EventError;
use sc_observability::Level;
use sc_observability::LogEvent;
use sc_observability::Logger;
use sc_observability::LoggerConfig;
use sc_observability::OBSERVATION_ENVELOPE_VERSION;
use sc_observability::OutcomeLabel;
use sc_observability::ProcessIdentity;
use sc_observability::Running;
use sc_observability::SchemaVersion;
use sc_observability::ServiceName as ObservabilityServiceName;
use sc_observability::TargetCategory;
use sc_observability::Timestamp;
use serde_json::Map;
use serde_json::Value;
use serde_json::json;

const FIELD_COMMAND: &str = "command";
const FIELD_CONFIG_PATH: &str = "config_path";
const FIELD_ELAPSED_MS: &str = "elapsed_ms";
const FIELD_FINDING_COUNT: &str = "finding_count";
const FIELD_JSON: &str = "json";
const FIELD_LOG_CONSOLE: &str = "log_console";
const FIELD_LOG_ROOT_OVERRIDE: &str = "log_root_override";
const FIELD_MANIFEST_POLICY_MODE: &str = "manifest_policy_mode";
const FIELD_MANIFEST_POLICY_PARITY: &str = "manifest_policy_parity";
const FIELD_PREFLIGHT_MODE: &str = "preflight_mode";
const FIELD_REPO_ROOT: &str = "repo_root";
const FIELD_SUMMARY: &str = "summary";
const FIELD_TARGET_TRIPLE: &str = "target_triple";
use crate::Cli;
use crate::CliError;
use crate::WINDOWS_XWIN_TARGET;
use crate::command::CommandContext;
use crate::command::DispatchTelemetry;
use crate::config::LoadedConfig;
use crate::consts;

#[derive(Debug, Clone)]
pub(crate) struct ObservedCommand<'a> {
    context: &'a CommandContext,
    loaded_config: &'a LoadedConfig,
}

impl<'a> ObservedCommand<'a> {
    pub(crate) fn from_context(
        context: &'a CommandContext,
        loaded_config: &'a LoadedConfig,
    ) -> Self {
        Self {
            context,
            loaded_config,
        }
    }

    fn command_id(&self) -> &str {
        self.context.command_id()
    }

    fn service_name(&self) -> &str {
        self.context.service_name()
    }

    fn observability_service_name(&self) -> Result<ObservabilityServiceName, &'static str> {
        ObservabilityServiceName::new(self.service_name())
            .map_err(|_| "invalid static observability service name")
    }

    fn summary(&self) -> &'static str {
        self.context.summary()
    }

    fn loaded_config(&self) -> &LoadedConfig {
        self.loaded_config
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LogRoot(PathBuf);

impl LogRoot {
    #[expect(
        clippy::result_large_err,
        reason = "Log-root validation failures must stay in the shared top-level CliError contract."
    )]
    fn resolve(override_root: Option<&PathBuf>, service_name: &str) -> Result<Self, CliError> {
        let base = match override_root {
            Some(path) if path.as_os_str().is_empty() => {
                return Err(CliError::config("`--log-root` must not be empty"));
            }
            Some(path) => path.clone(),
            None => default_log_base()?,
        };

        let _ = service_name;
        Ok(Self(base))
    }
}

#[expect(
    clippy::result_large_err,
    reason = "Logger initialization failures are part of the stable top-level CliError contract."
)]
pub(crate) fn initialize_logger(
    observed: &ObservedCommand<'_>,
    cli: &Cli,
) -> Result<Logger, CliError> {
    validate_logging_contract()?;
    let log_root = LogRoot::resolve(
        observed
            .loaded_config()
            .logging_root()
            .or(cli.log_root.as_ref()),
        observed.service_name(),
    )?;
    let mut config = LoggerConfig::default_for(
        observed
            .observability_service_name()
            .map_err(CliError::internal)?,
        log_root.0.clone(),
    );
    config.enable_console_sink = observed.loaded_config().logging_console();
    config.retained_log_policy = sc_observability::RetainedLogPolicy::default();
    Logger::new(config).map_err(classify_logger_init_error)
}

pub(crate) fn log_entry(logger: &Logger, observed: &ObservedCommand<'_>, cli: &Cli) {
    let mut fields = base_fields(observed);
    fields.insert(FIELD_JSON.to_string(), Value::Bool(cli.json));
    fields.insert(
        FIELD_LOG_CONSOLE.to_string(),
        Value::Bool(observed.loaded_config().logging_console()),
    );
    if let Some(log_root) = cli.log_root.as_ref() {
        fields.insert(
            FIELD_LOG_ROOT_OVERRIDE.to_string(),
            Value::String(log_root.display().to_string()),
        );
    }
    if let Some(repo_root) = observed.loaded_config().repo_root() {
        fields.insert(
            FIELD_REPO_ROOT.to_string(),
            Value::String(repo_root.display().to_string()),
        );
    }
    if let Some(config_path) = observed.loaded_config().config_path() {
        fields.insert(
            FIELD_CONFIG_PATH.to_string(),
            Value::String(config_path.display().to_string()),
        );
    }

    dispatch_event(
        logger,
        observed,
        Level::Info,
        started_action().cloned().ok(),
        None,
        Some("command invocation started"),
        fields,
    );
}

pub(crate) fn log_dispatch_start(logger: &Logger, observed: &ObservedCommand<'_>, tool: &str) {
    let mut fields = base_fields(observed);
    fields.insert(
        consts::FIELD_TOOL.to_string(),
        Value::String(tool.to_string()),
    );

    dispatch_event(
        logger,
        observed,
        Level::Info,
        dispatch_started_action().cloned().ok(),
        None,
        Some("backend dispatch started"),
        fields,
    );
}

pub(crate) fn log_dispatch_result(
    logger: &Logger,
    observed: &ObservedCommand<'_>,
    dispatch: &DispatchTelemetry,
) {
    let mut fields = base_fields(observed);
    fields.insert(
        consts::FIELD_TOOL.to_string(),
        Value::String(dispatch.tool().to_string()),
    );
    fields.insert(
        FIELD_FINDING_COUNT.to_string(),
        json!(dispatch.finding_count()),
    );

    dispatch_event(
        logger,
        observed,
        Level::Info,
        dispatch_normalized_action().cloned().ok(),
        success_outcome().cloned().ok(),
        Some("backend result normalized through top-level contract"),
        fields,
    );
}

pub(crate) fn log_completion(
    logger: &Logger,
    observed: &ObservedCommand<'_>,
    ok: bool,
    summary: &str,
    elapsed: Duration,
) {
    let mut fields = base_fields(observed);
    fields.insert(
        FIELD_SUMMARY.to_string(),
        Value::String(summary.to_string()),
    );
    fields.insert(
        FIELD_ELAPSED_MS.to_string(),
        Value::from(elapsed_ms(elapsed)),
    );

    dispatch_event(
        logger,
        observed,
        Level::Info,
        completed_action().cloned().ok(),
        if ok {
            success_outcome().cloned().ok()
        } else {
            failure_outcome().cloned().ok()
        },
        Some("command invocation completed"),
        fields,
    );
}

pub(crate) fn log_error(logger: &Logger, observed: &ObservedCommand<'_>, error: &CliError) {
    let mut fields = base_fields(observed);
    fields.insert(
        consts::FIELD_CODE.to_string(),
        Value::String(error.code().to_string()),
    );
    fields.insert(
        consts::FIELD_KIND.to_string(),
        Value::String(error.kind_label().to_string()),
    );
    fields.insert(
        consts::FIELD_MESSAGE.to_string(),
        Value::String(error.message.clone()),
    );
    if let Some(cause) = error.cause.as_ref() {
        fields.insert(
            consts::FIELD_CAUSE.to_string(),
            Value::String(cause.clone()),
        );
    }
    if !error.details.is_empty() {
        fields.insert(
            consts::FIELD_DETAILS.to_string(),
            Value::Object(error.details.clone()),
        );
    }

    dispatch_event(
        logger,
        observed,
        Level::Error,
        error_action().cloned().ok(),
        failure_outcome().cloned().ok(),
        Some("top-level cli error emitted"),
        fields,
    );
}

pub(crate) fn flush(logger: &Logger) {
    let _ = logger.flush();
}

pub(crate) fn shutdown(logger: Logger) {
    let _ = logger.shutdown();
}

fn dispatch_event(
    logger: &Logger,
    observed: &ObservedCommand<'_>,
    level: Level,
    action: Option<ActionName>,
    outcome: Option<OutcomeLabel>,
    message: Option<&str>,
    fields: Map<String, Value>,
) {
    let Some(action) = action else {
        debug_assert!(false, "failed to resolve static log action");
        #[cfg(debug_assertions)]
        eprintln!("[sc-lint] dispatch_event: missing static action");
        return;
    };
    let event = match build_event(observed, level, action, outcome, message, fields) {
        Ok(event) => event,
        Err(error) => {
            debug_assert!(false, "failed to build log event: {error}");
            #[cfg(debug_assertions)]
            eprintln!("[sc-lint] dispatch_event: build_event error: {:?}", error);
            // telemetry failures are intentionally discarded in release — this is a display-layer tool
            return;
        }
    };
    // C.10 keeps every current CLI event site intentionally non-blocking:
    // telemetry must never stall command completion while the runtime still
    // exposes only the synchronous emit path under the hood.
    let _ = logger.try_log(event);
}

#[expect(
    clippy::result_large_err,
    reason = "Only the top-level logging/event seam needs to lift static-contract failures into the shared CliError contract."
)]
fn build_event(
    observed: &ObservedCommand<'_>,
    level: Level,
    action: ActionName,
    outcome: Option<OutcomeLabel>,
    message: Option<&str>,
    fields: Map<String, Value>,
) -> Result<LogEvent, CliError> {
    let event = LogEvent {
        version: schema_version().map_err(CliError::internal)?.clone(),
        timestamp: Timestamp::now_utc(),
        level,
        service: observed
            .observability_service_name()
            .map_err(CliError::internal)?,
        target: command_target().map_err(CliError::internal)?.clone(),
        action,
        message: message.map(ToString::to_string),
        identity: ProcessIdentity::default(),
        trace: None,
        request_id: None,
        correlation_id: None,
        outcome,
        diagnostic: None,
        state_transition: None,
        fields,
    };
    Ok(event)
}

fn base_fields(observed: &ObservedCommand<'_>) -> Map<String, Value> {
    let mut fields = Map::from_iter([
        (
            FIELD_COMMAND.to_string(),
            Value::String(observed.command_id().to_string()),
        ),
        (FIELD_SUMMARY.to_string(), json!(observed.summary())),
    ]);
    if observed.context.is_xwin_preflight() {
        fields.insert(FIELD_PREFLIGHT_MODE.to_string(), json!("xwin"));
        fields.insert(FIELD_TARGET_TRIPLE.to_string(), json!(WINDOWS_XWIN_TARGET));
    }
    if let Some(adapter_kind) = observed.context.adapter_kind() {
        fields.insert("adapter".to_string(), json!(adapter_kind));
    }
    if let Some(config_scope) = observed.context.adapter_config_scope() {
        fields.insert("config_scope".to_string(), json!(config_scope));
    }
    if let Some(script) = observed.context.adapter_script() {
        fields.insert("script".to_string(), json!(script));
    }
    if observed.command_id() == consts::CMD_BOUNDARY {
        fields.insert(FIELD_MANIFEST_POLICY_MODE.to_string(), json!("rust-native"));
        fields.insert(
            FIELD_MANIFEST_POLICY_PARITY.to_string(),
            json!("python-oracle"),
        );
    }
    fields
}

#[expect(
    clippy::result_large_err,
    reason = "Logger startup must fail through the shared CliError contract when static logging metadata is invalid."
)]
fn validate_logging_contract() -> Result<(), CliError> {
    let _ = (
        consts::TOOL_BOUNDARY,
        consts::CMD_BOUNDARY,
        consts::FIELD_FINDINGS,
        consts::FIELD_STATUS,
        consts::FIELD_CRATE_NAME,
        consts::FIELD_CRATE_VERSION,
        consts::FIELD_SUGGESTED_ACTION,
        consts::FIELD_STEPS,
        consts::FIELD_ROOT,
    );
    let _ = schema_version().map_err(CliError::internal)?;
    let _ = command_target().map_err(CliError::internal)?;
    let _ = started_action().map_err(CliError::internal)?;
    let _ = completed_action().map_err(CliError::internal)?;
    let _ = error_action().map_err(CliError::internal)?;
    let _ = dispatch_started_action().map_err(CliError::internal)?;
    let _ = dispatch_normalized_action().map_err(CliError::internal)?;
    let _ = success_outcome().map_err(CliError::internal)?;
    let _ = failure_outcome().map_err(CliError::internal)?;
    Ok(())
}

fn schema_version() -> Result<&'static SchemaVersion, &'static str> {
    static SCHEMA_VERSION: OnceLock<Result<SchemaVersion, &'static str>> = OnceLock::new();
    match SCHEMA_VERSION.get_or_init(|| {
        SchemaVersion::new(OBSERVATION_ENVELOPE_VERSION)
            .map_err(|_| "invalid static logging schema version")
    }) {
        Ok(value) => Ok(value),
        Err(error) => Err(*error),
    }
}

fn command_target() -> Result<&'static TargetCategory, &'static str> {
    static COMMAND_TARGET: OnceLock<Result<TargetCategory, &'static str>> = OnceLock::new();
    match COMMAND_TARGET.get_or_init(|| {
        TargetCategory::new("cli.command").map_err(|_| "invalid static logging target category")
    }) {
        Ok(value) => Ok(value),
        Err(error) => Err(*error),
    }
}

fn started_action() -> Result<&'static ActionName, &'static str> {
    static STARTED_ACTION: OnceLock<Result<ActionName, &'static str>> = OnceLock::new();
    match STARTED_ACTION.get_or_init(|| {
        ActionName::new("cli.command.started").map_err(|_| "invalid static logging started action")
    }) {
        Ok(value) => Ok(value),
        Err(error) => Err(*error),
    }
}

fn completed_action() -> Result<&'static ActionName, &'static str> {
    static COMPLETED_ACTION: OnceLock<Result<ActionName, &'static str>> = OnceLock::new();
    match COMPLETED_ACTION.get_or_init(|| {
        ActionName::new("cli.command.completed")
            .map_err(|_| "invalid static logging completed action")
    }) {
        Ok(value) => Ok(value),
        Err(error) => Err(*error),
    }
}

fn error_action() -> Result<&'static ActionName, &'static str> {
    static ERROR_ACTION: OnceLock<Result<ActionName, &'static str>> = OnceLock::new();
    match ERROR_ACTION.get_or_init(|| {
        ActionName::new("cli.command.error").map_err(|_| "invalid static logging error action")
    }) {
        Ok(value) => Ok(value),
        Err(error) => Err(*error),
    }
}

fn dispatch_started_action() -> Result<&'static ActionName, &'static str> {
    static DISPATCH_STARTED_ACTION: OnceLock<Result<ActionName, &'static str>> = OnceLock::new();
    match DISPATCH_STARTED_ACTION.get_or_init(|| {
        ActionName::new("cli.dispatch.started")
            .map_err(|_| "invalid static dispatch started action")
    }) {
        Ok(value) => Ok(value),
        Err(error) => Err(*error),
    }
}

fn dispatch_normalized_action() -> Result<&'static ActionName, &'static str> {
    static DISPATCH_NORMALIZED_ACTION: OnceLock<Result<ActionName, &'static str>> = OnceLock::new();
    match DISPATCH_NORMALIZED_ACTION.get_or_init(|| {
        ActionName::new("cli.dispatch.normalized")
            .map_err(|_| "invalid static dispatch normalized action")
    }) {
        Ok(value) => Ok(value),
        Err(error) => Err(*error),
    }
}

fn success_outcome() -> Result<&'static OutcomeLabel, &'static str> {
    static SUCCESS_OUTCOME: OnceLock<Result<OutcomeLabel, &'static str>> = OnceLock::new();
    match SUCCESS_OUTCOME.get_or_init(|| {
        OutcomeLabel::new("success").map_err(|_| "invalid static success outcome label")
    }) {
        Ok(value) => Ok(value),
        Err(error) => Err(*error),
    }
}

fn failure_outcome() -> Result<&'static OutcomeLabel, &'static str> {
    static FAILURE_OUTCOME: OnceLock<Result<OutcomeLabel, &'static str>> = OnceLock::new();
    match FAILURE_OUTCOME.get_or_init(|| {
        OutcomeLabel::new("failure").map_err(|_| "invalid static failure outcome label")
    }) {
        Ok(value) => Ok(value),
        Err(error) => Err(*error),
    }
}

fn elapsed_ms(elapsed: Duration) -> u64 {
    u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX)
}

#[expect(
    clippy::result_large_err,
    reason = "Default log-root discovery returns the same shared CliError contract as other CLI startup failures."
)]
fn default_log_base() -> Result<PathBuf, CliError> {
    home_directory()
        .map(|home| home.join(consts::SERVICE_NAME))
        .ok_or_else(|| {
            CliError::capability("could not resolve the current user's home directory for logging")
                .with_suggested_action(
                    "Pass `--log-root <path>` to choose a writable log location explicitly.",
                )
        })
}

fn home_directory() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        std::env::var_os("USERPROFILE")
            .map(PathBuf::from)
            .or_else(|| {
                let drive = std::env::var_os("HOMEDRIVE")?;
                let path = std::env::var_os("HOMEPATH")?;
                Some(PathBuf::from(format!(
                    "{}{}",
                    drive.to_string_lossy(),
                    path.to_string_lossy()
                )))
            })
    }
    #[cfg(not(windows))]
    {
        std::env::var_os("HOME").map(PathBuf::from)
    }
}

fn classify_logger_init_error<E>(error: E) -> CliError
where
    E: std::fmt::Display,
{
    let message = error.to_string().to_ascii_lowercase();
    if contains_config_signal(&message) {
        return CliError::config("failed to initialize the structured logger")
            .with_source(error)
            .with_suggested_action(
                "Check the logging configuration values and retry the command.",
            );
    }
    if contains_capability_signal(&message) {
        return CliError::capability("failed to initialize the structured logger")
            .with_source(error)
            .with_suggested_action(
                "Verify the configured log root is writable for the current user.",
            );
    }
    CliError::internal("failed to initialize the structured logger")
        .with_source(error)
        .with_suggested_action(
            "Re-run the command; if the failure persists, inspect the logger wiring.",
        )
}

#[expect(
    dead_code,
    reason = "C.10 introduces the blocking compatibility verb now so later queue-backed runtimes can opt into it without exposing emit at call sites."
)]
trait LoggerEventCompat {
    fn try_log(&self, event: LogEvent) -> Result<(), EventError>;
    fn log(&self, event: LogEvent) -> Result<(), EventError>;
}

impl LoggerEventCompat for Logger<Running> {
    fn try_log(&self, event: LogEvent) -> Result<(), EventError> {
        self.emit(event)
    }

    fn log(&self, event: LogEvent) -> Result<(), EventError> {
        self.emit(event)
    }
}

fn contains_config_signal(message: &str) -> bool {
    message.contains("config")
        || message.contains("invalid")
        || message.contains("contradict")
        || message.contains("schema")
}

fn contains_capability_signal(message: &str) -> bool {
    message.contains("permission")
        || message.contains("writable")
        || message.contains("write")
        || message.contains("create")
        || message.contains("path")
        || message.contains("directory")
}
