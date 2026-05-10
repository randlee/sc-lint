use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::Duration;

use sc_observability::ActionName;
use sc_observability::JsonlFileSink;
use sc_observability::Level;
use sc_observability::LogEvent;
use sc_observability::Logger;
use sc_observability::LoggerBuilder;
use sc_observability::LoggerConfig;
use sc_observability::OBSERVATION_ENVELOPE_VERSION;
use sc_observability::OutcomeLabel;
use sc_observability::ProcessIdentity;
use sc_observability::SchemaVersion;
use sc_observability::ServiceName;
use sc_observability::SinkRegistration;
use sc_observability::TargetCategory;
use sc_observability::Timestamp;
use serde_json::Map;
use serde_json::Value;
use serde_json::json;

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
    #[expect(
        clippy::result_large_err,
        reason = "The binary logging seam preserves the same top-level CliError contract as the library execution path."
    )]
    pub(crate) fn from_context(
        context: &'a CommandContext,
        loaded_config: &'a LoadedConfig,
    ) -> Result<Self, CliError> {
        Ok(Self {
            context,
            loaded_config,
        })
    }

    fn command_id(&self) -> &str {
        self.context.command_id()
    }

    fn service_name(&self) -> &ServiceName {
        self.context.service_name()
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

        Ok(Self(base.join(service_name)))
    }

    fn service_root(&self) -> &PathBuf {
        &self.0
    }

    fn active_log_path(&self, service_name: &ServiceName) -> PathBuf {
        self.0.join(format!("{}.log.jsonl", service_name.as_str()))
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
        observed.service_name().as_str(),
    )?;
    let mut config = LoggerConfig::default_for(
        observed.service_name().clone(),
        log_root.service_root().clone(),
    );
    let rotation = config.rotation;
    let retention = config.retention;
    config.enable_file_sink = false;
    config.enable_console_sink = observed.loaded_config().logging_console();

    let mut builder = LoggerBuilder::new(config).map_err(classify_logger_init_error)?;
    builder.register_sink(SinkRegistration::new(std::sync::Arc::new(
        JsonlFileSink::new(
            log_root.active_log_path(observed.service_name()),
            rotation,
            retention,
        ),
    )));

    Ok(builder.build())
}

pub(crate) fn log_entry(logger: &Logger, observed: &ObservedCommand<'_>, cli: &Cli) {
    let mut fields = base_fields(observed);
    fields.insert("json".to_string(), Value::Bool(cli.json));
    fields.insert(
        "log_console".to_string(),
        Value::Bool(observed.loaded_config().logging_console()),
    );
    if let Some(log_root) = cli.log_root.as_ref() {
        fields.insert(
            "log_root_override".to_string(),
            Value::String(log_root.display().to_string()),
        );
    }
    if let Some(repo_root) = observed.loaded_config().repo_root() {
        fields.insert(
            "repo_root".to_string(),
            Value::String(repo_root.display().to_string()),
        );
    }
    if let Some(config_path) = observed.loaded_config().config_path() {
        fields.insert(
            consts::FIELD_CONFIG_PATH.to_string(),
            Value::String(config_path.display().to_string()),
        );
    }

    emit_event(
        logger,
        observed,
        Level::Info,
        started_action().cloned(),
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

    emit_event(
        logger,
        observed,
        Level::Info,
        dispatch_started_action().cloned(),
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
    fields.insert("finding_count".to_string(), json!(dispatch.finding_count()));

    emit_event(
        logger,
        observed,
        Level::Info,
        dispatch_normalized_action().cloned(),
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
    fields.insert("summary".to_string(), Value::String(summary.to_string()));
    fields.insert("elapsed_ms".to_string(), Value::from(elapsed_ms(elapsed)));

    emit_event(
        logger,
        observed,
        Level::Info,
        completed_action().cloned(),
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
        Value::String(format!("{:?}", error.kind).to_lowercase()),
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

    emit_event(
        logger,
        observed,
        Level::Error,
        error_action().cloned(),
        failure_outcome().cloned().ok(),
        Some("top-level cli error emitted"),
        fields,
    );
}

pub(crate) fn flush(logger: &Logger) {
    let _ = logger.flush();
}

pub(crate) fn shutdown(logger: &Logger) {
    let _ = logger.shutdown();
}

fn emit_event(
    logger: &Logger,
    observed: &ObservedCommand<'_>,
    level: Level,
    action: Result<ActionName, &'static str>,
    outcome: Option<OutcomeLabel>,
    message: Option<&str>,
    fields: Map<String, Value>,
) {
    let Ok(event) = build_event(observed, level, action, outcome, message, fields) else {
        return;
    };
    let _ = logger.emit(event);
}

#[expect(
    clippy::result_large_err,
    reason = "Only the top-level logging/event seam needs to lift static-contract failures into the shared CliError contract."
)]
fn build_event(
    observed: &ObservedCommand<'_>,
    level: Level,
    action: Result<ActionName, &'static str>,
    outcome: Option<OutcomeLabel>,
    message: Option<&str>,
    fields: Map<String, Value>,
) -> Result<LogEvent, CliError> {
    let event = LogEvent {
        version: schema_version().map_err(CliError::internal)?.clone(),
        timestamp: Timestamp::now_utc(),
        level,
        service: observed.service_name().clone(),
        target: command_target().map_err(CliError::internal)?.clone(),
        action: action.map_err(CliError::internal)?,
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
            "command".to_string(),
            Value::String(observed.command_id().to_string()),
        ),
        ("summary".to_string(), json!(observed.summary())),
    ]);
    if observed.context.is_xwin_preflight() {
        fields.insert("preflight_mode".to_string(), json!("xwin"));
        fields.insert("target_triple".to_string(), json!(WINDOWS_XWIN_TARGET));
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
    fields
}

#[expect(
    clippy::result_large_err,
    reason = "Logger startup must fail through the shared CliError contract when static logging metadata is invalid."
)]
fn validate_logging_contract() -> Result<(), CliError> {
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
        .map(|home| home.join(consts::SERVICE_NAME).join("logs"))
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
