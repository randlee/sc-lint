use serde::Serialize;
use serde_json::Map;
use serde_json::Value;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CliErrorKind {
    Usage,
    Config,
    Capability,
    BackendFailure,
    BackendProtocol,
    Internal,
}

impl CliErrorKind {
    pub const fn code(self) -> &'static str {
        match self {
            Self::Usage => "CLI.USAGE_ERROR",
            Self::Config => "CLI.CONFIG_ERROR",
            Self::Capability => "CLI.CAPABILITY_ERROR",
            Self::BackendFailure => "CLI.BACKEND_EXEC_FAILURE",
            Self::BackendProtocol => "CLI.BACKEND_PROTOCOL_ERROR",
            Self::Internal => "CLI.INTERNAL_ERROR",
        }
    }

    pub const fn exit_code(self) -> u8 {
        match self {
            Self::Usage => 2,
            Self::Config => 3,
            Self::Capability => 4,
            Self::BackendFailure => 5,
            Self::BackendProtocol => 6,
            Self::Internal => 1,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Error)]
#[error("{message}")]
pub struct CliError {
    pub kind: CliErrorKind,
    pub code: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Map::is_empty")]
    pub details: Map<String, Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggested_action: Option<String>,
}

impl CliError {
    pub fn usage(message: impl Into<String>) -> Self {
        Self::new(CliErrorKind::Usage, message)
    }

    pub fn config(message: impl Into<String>) -> Self {
        Self::new(CliErrorKind::Config, message)
    }

    pub fn capability(message: impl Into<String>) -> Self {
        Self::new(CliErrorKind::Capability, message)
    }

    pub fn backend_failure(message: impl Into<String>) -> Self {
        Self::new(CliErrorKind::BackendFailure, message)
    }

    pub fn backend_protocol(message: impl Into<String>) -> Self {
        Self::new(CliErrorKind::BackendProtocol, message)
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(CliErrorKind::Internal, message)
    }

    pub fn new(kind: CliErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            code: kind.code().to_string(),
            message: message.into(),
            details: Map::new(),
            suggested_action: None,
        }
    }

    pub fn with_detail(mut self, key: impl Into<String>, value: Value) -> Self {
        self.details.insert(key.into(), value);
        self
    }

    pub fn with_suggested_action(mut self, suggested_action: impl Into<String>) -> Self {
        self.suggested_action = Some(suggested_action.into());
        self
    }

    pub const fn exit_code(&self) -> u8 {
        self.kind.exit_code()
    }
}
