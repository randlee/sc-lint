use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::Duration;

use sc_lint::Cli;
use sc_lint::CliError;
use sc_lint::CommandContext;
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

#[derive(Debug, Clone)]
pub struct ObservedCommand<'a> {
    context: &'a CommandContext,
    service_name: ServiceName,
}

impl<'a> ObservedCommand<'a> {
    #[expect(
        clippy::result_large_err,
        reason = "The binary logging seam preserves the same top-level CliError contract as the library execution path."
    )]
    pub fn from_context(context: &'a CommandContext) -> Result<Self, CliError> {
        let service_name = ServiceName::new(context.service_name()).map_err(|error| {
            CliError::internal(format!("invalid service name `{}`", context.service_name()))
                .with_source(error)
        })?;

        Ok(Self {
            context,
            service_name,
        })
    }

    fn command_id(&self) -> &str {
        self.context.command_id()
    }

    fn service_name(&self) -> &ServiceName {
        &self.service_name
    }

    fn summary(&self) -> &'static str {
        self.context.summary()
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
pub fn initialize_logger(observed: &ObservedCommand<'_>, cli: &Cli) -> Result<Logger, CliError> {
    let log_root = LogRoot::resolve(cli.log_root.as_ref(), observed.service_name().as_str())?;
    let mut config = LoggerConfig::default_for(
        observed.service_name().clone(),
        log_root.service_root().clone(),
    );
    let rotation = config.rotation;
    let retention = config.retention;
    config.enable_file_sink = false;
    config.enable_console_sink = cli.log_console;

    let mut builder = LoggerBuilder::new(config).map_err(|error| {
        CliError::config("failed to initialize the structured logger")
            .with_source(error)
            .with_suggested_action(
                "Verify the configured log root is writable for the current user.",
            )
    })?;
    builder.register_sink(SinkRegistration::new(std::sync::Arc::new(
        JsonlFileSink::new(
            log_root.active_log_path(observed.service_name()),
            rotation,
            retention,
        ),
    )));

    Ok(builder.build())
}

pub fn log_entry(logger: &Logger, observed: &ObservedCommand<'_>, cli: &Cli) {
    let mut fields = base_fields(observed);
    fields.insert("json".to_string(), Value::Bool(cli.json));
    fields.insert("log_console".to_string(), Value::Bool(cli.log_console));
    if let Some(log_root) = cli.log_root.as_ref() {
        fields.insert(
            "log_root_override".to_string(),
            Value::String(log_root.display().to_string()),
        );
    }

    log_event(
        logger,
        observed,
        Level::Info,
        started_action().clone(),
        None,
        Some("command invocation started"),
        fields,
    );
}

pub fn log_completion(
    logger: &Logger,
    observed: &ObservedCommand<'_>,
    ok: bool,
    summary: &str,
    elapsed: Duration,
) {
    let mut fields = base_fields(observed);
    fields.insert("summary".to_string(), Value::String(summary.to_string()));
    fields.insert("elapsed_ms".to_string(), Value::from(elapsed_ms(elapsed)));

    log_event(
        logger,
        observed,
        Level::Info,
        completed_action().clone(),
        Some(if ok {
            success_outcome().clone()
        } else {
            failure_outcome().clone()
        }),
        Some("command invocation completed"),
        fields,
    );
}

pub fn log_error(logger: &Logger, observed: &ObservedCommand<'_>, error: &CliError) {
    let mut fields = base_fields(observed);
    fields.insert("code".to_string(), Value::String(error.code().to_string()));
    fields.insert(
        "kind".to_string(),
        Value::String(format!("{:?}", error.kind).to_lowercase()),
    );
    fields.insert("message".to_string(), Value::String(error.message.clone()));
    if let Some(cause) = error.cause.as_ref() {
        fields.insert("cause".to_string(), Value::String(cause.clone()));
    }
    if !error.details.is_empty() {
        fields.insert("details".to_string(), Value::Object(error.details.clone()));
    }

    log_event(
        logger,
        observed,
        Level::Error,
        error_action().clone(),
        Some(failure_outcome().clone()),
        Some("top-level cli error emitted"),
        fields,
    );
}

pub fn flush(logger: &Logger) {
    let _ = logger.flush();
}

pub fn shutdown(logger: &Logger) {
    let _ = logger.shutdown();
}

fn log_event(
    logger: &Logger,
    observed: &ObservedCommand<'_>,
    level: Level,
    action: ActionName,
    outcome: Option<OutcomeLabel>,
    message: Option<&str>,
    fields: Map<String, Value>,
) {
    let event = LogEvent {
        version: schema_version().clone(),
        timestamp: Timestamp::now_utc(),
        level,
        service: observed.service_name().clone(),
        target: command_target().clone(),
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
    let _ = logger.emit(event);
}

fn base_fields(observed: &ObservedCommand<'_>) -> Map<String, Value> {
    Map::from_iter([
        (
            "command".to_string(),
            Value::String(observed.command_id().to_string()),
        ),
        ("summary".to_string(), json!(observed.summary())),
    ])
}

fn schema_version() -> &'static SchemaVersion {
    static SCHEMA_VERSION: OnceLock<SchemaVersion> = OnceLock::new();
    SCHEMA_VERSION.get_or_init(|| {
        SchemaVersion::new(OBSERVATION_ENVELOPE_VERSION).expect("static schema version is valid")
    })
}

fn command_target() -> &'static TargetCategory {
    static COMMAND_TARGET: OnceLock<TargetCategory> = OnceLock::new();
    COMMAND_TARGET.get_or_init(|| {
        TargetCategory::new("cli.command").expect("static target category is valid")
    })
}

fn started_action() -> &'static ActionName {
    static STARTED_ACTION: OnceLock<ActionName> = OnceLock::new();
    STARTED_ACTION.get_or_init(|| {
        ActionName::new("cli.command.started").expect("static started action is valid")
    })
}

fn completed_action() -> &'static ActionName {
    static COMPLETED_ACTION: OnceLock<ActionName> = OnceLock::new();
    COMPLETED_ACTION.get_or_init(|| {
        ActionName::new("cli.command.completed").expect("static completed action is valid")
    })
}

fn error_action() -> &'static ActionName {
    static ERROR_ACTION: OnceLock<ActionName> = OnceLock::new();
    ERROR_ACTION
        .get_or_init(|| ActionName::new("cli.command.error").expect("static error action is valid"))
}

fn success_outcome() -> &'static OutcomeLabel {
    static SUCCESS_OUTCOME: OnceLock<OutcomeLabel> = OnceLock::new();
    SUCCESS_OUTCOME
        .get_or_init(|| OutcomeLabel::new("success").expect("static success outcome is valid"))
}

fn failure_outcome() -> &'static OutcomeLabel {
    static FAILURE_OUTCOME: OnceLock<OutcomeLabel> = OnceLock::new();
    FAILURE_OUTCOME
        .get_or_init(|| OutcomeLabel::new("failure").expect("static failure outcome is valid"))
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
        .map(|home| home.join("sc-lint").join("logs"))
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
