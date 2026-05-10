use crate::consts;
use std::error::Error;
use std::fmt;

use serde::Serialize;
use serde::ser::SerializeStruct;
use serde_json::Map;
use serde_json::Value;

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliError {
    pub kind: CliErrorKind,
    pub message: String,
    pub details: Map<String, Value>,
    pub cause: Option<String>,
    pub suggested_action: Option<String>,
    source: Option<CliErrorSource>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CliErrorSource(String);

impl fmt::Display for CliErrorSource {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Error for CliErrorSource {}

impl Serialize for CliError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let field_count = 3
            + usize::from(!self.details.is_empty())
            + usize::from(self.cause.is_some())
            + usize::from(self.suggested_action.is_some());
        let mut state = serializer.serialize_struct("CliError", field_count)?;
        state.serialize_field(consts::FIELD_KIND, &self.kind)?;
        state.serialize_field(consts::FIELD_CODE, self.code())?;
        state.serialize_field(consts::FIELD_MESSAGE, &self.message)?;
        if !self.details.is_empty() {
            state.serialize_field(consts::FIELD_DETAILS, &self.details)?;
        }
        if let Some(cause) = self.cause.as_ref() {
            state.serialize_field(consts::FIELD_CAUSE, cause)?;
        }
        if let Some(suggested_action) = self.suggested_action.as_ref() {
            state.serialize_field("suggested_action", suggested_action)?;
        }
        state.end()
    }
}

impl fmt::Display for CliError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for CliError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.source
            .as_ref()
            .map(|source| source as &(dyn Error + 'static))
    }
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
            message: message.into(),
            details: Map::new(),
            cause: None,
            suggested_action: None,
            source: None,
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

    pub fn with_cause(mut self, cause: impl Into<String>) -> Self {
        let cause = cause.into();
        self.cause = Some(cause.clone());
        self.source = Some(CliErrorSource(cause));
        self
    }

    pub fn with_source<E>(mut self, source: E) -> Self
    where
        E: fmt::Display,
    {
        let cause = source.to_string();
        self.cause = Some(cause.clone());
        self.source = Some(CliErrorSource(cause));
        self
    }

    pub const fn exit_code(&self) -> u8 {
        self.kind.exit_code()
    }

    pub const fn code(&self) -> &'static str {
        self.kind.code()
    }
}
