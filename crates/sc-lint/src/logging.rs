use std::path::PathBuf;
use std::sync::OnceLock;

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
use crate::command::CommandContext;

#[derive(Debug, Clone, PartialEq, Eq)]
struct LogRoot(PathBuf);

impl LogRoot {
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

pub fn initialize_logger(context: &CommandContext, cli: &Cli) -> Result<Logger, CliError> {
    let service_name = ServiceName::new(context.service_name()).map_err(|error| {
        CliError::internal(format!(
            "invalid service name `{}`: {error}",
            context.service_name()
        ))
    })?;
    let log_root = LogRoot::resolve(cli.log_root.as_ref(), context.service_name())?;
    let mut config =
        LoggerConfig::default_for(service_name.clone(), log_root.service_root().clone());
    let rotation = config.rotation;
    let retention = config.retention;
    config.enable_file_sink = false;
    config.enable_console_sink = cli.log_console;

    let mut builder = LoggerBuilder::new(config).map_err(|error| {
        CliError::config(format!(
            "failed to initialize the structured logger: {error}"
        ))
        .with_suggested_action("Verify the configured log root is writable for the current user.")
    })?;
    builder.register_sink(SinkRegistration::new(std::sync::Arc::new(
        JsonlFileSink::new(log_root.active_log_path(&service_name), rotation, retention),
    )));

    Ok(builder.build())
}

pub fn emit_entry(logger: &Logger, context: &CommandContext, cli: &Cli) {
    let mut fields = base_fields(context);
    fields.insert("json".to_string(), Value::Bool(cli.json));
    fields.insert("log_console".to_string(), Value::Bool(cli.log_console));
    if let Some(log_root) = cli.log_root.as_ref() {
        fields.insert(
            "log_root_override".to_string(),
            Value::String(log_root.display().to_string()),
        );
    }
    emit(
        logger,
        context,
        Level::Info,
        "cli.command.started",
        None,
        Some("command invocation started"),
        fields,
    );
}

pub fn emit_completion(logger: &Logger, context: &CommandContext, ok: bool, summary: &str) {
    let mut fields = base_fields(context);
    fields.insert("summary".to_string(), Value::String(summary.to_string()));
    emit(
        logger,
        context,
        Level::Info,
        "cli.command.completed",
        Some(if ok { "success" } else { "failure" }),
        Some("command invocation completed"),
        fields,
    );
}

pub fn emit_error(logger: &Logger, context: &CommandContext, error: &CliError) {
    let mut fields = base_fields(context);
    fields.insert("code".to_string(), Value::String(error.code.clone()));
    fields.insert(
        "kind".to_string(),
        Value::String(format!("{:?}", error.kind).to_lowercase()),
    );
    fields.insert("message".to_string(), Value::String(error.message.clone()));
    if !error.details.is_empty() {
        fields.insert("details".to_string(), Value::Object(error.details.clone()));
    }
    emit(
        logger,
        context,
        Level::Error,
        "cli.command.error",
        Some("failure"),
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

fn emit(
    logger: &Logger,
    context: &CommandContext,
    level: Level,
    action: &str,
    outcome: Option<&str>,
    message: Option<&str>,
    fields: Map<String, Value>,
) {
    let action = ActionName::new(action).expect("static action names are valid");
    let target = TargetCategory::new("cli.command").expect("static target category is valid");
    let outcome =
        outcome.map(|value| OutcomeLabel::new(value).expect("static outcome labels are valid"));
    let event = LogEvent {
        version: schema_version().clone(),
        timestamp: Timestamp::now_utc(),
        level,
        service: ServiceName::new(context.service_name()).expect("context service name is valid"),
        target,
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

fn base_fields(context: &CommandContext) -> Map<String, Value> {
    Map::from_iter([
        (
            "command".to_string(),
            Value::String(context.command_id().to_string()),
        ),
        ("summary".to_string(), json!(context.summary())),
    ])
}

fn schema_version() -> &'static SchemaVersion {
    static SCHEMA_VERSION: OnceLock<SchemaVersion> = OnceLock::new();
    SCHEMA_VERSION.get_or_init(|| {
        SchemaVersion::new(OBSERVATION_ENVELOPE_VERSION).expect("shared schema version is valid")
    })
}

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
