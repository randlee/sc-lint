use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ServiceName(&'static str);

impl ServiceName {
    // Static command/service identifiers are fixed at compile time, so Cow
    // would add generality without any real construction-site benefit here.
    pub(crate) const fn new(value: &'static str) -> Self {
        debug_assert!(!value.is_empty(), "service names must not be empty");
        Self(value)
    }

    pub(crate) const fn as_str(&self) -> &str {
        self.0
    }
}

impl AsRef<str> for ServiceName {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl std::fmt::Display for ServiceName {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CommandEnvelope<T>
where
    T: Serialize,
{
    pub ok: bool,
    /// Commands are normalized to stable dotted identifiers so logs and machine envelopes share the same contract key.
    pub command: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<crate::CliError>,
    #[serde(default)]
    pub diagnostics: Vec<String>,
}

impl<T> CommandEnvelope<T>
where
    T: Serialize,
{
    pub fn success(command: &str, data: T) -> Self {
        Self {
            ok: true,
            command: command.to_string(),
            data: Some(data),
            error: None,
            diagnostics: Vec::new(),
        }
    }
}

impl CommandEnvelope<serde_json::Value> {
    pub fn failure(command: &str, error: crate::CliError) -> Self {
        Self {
            ok: false,
            command: command.to_string(),
            data: None,
            error: Some(error),
            diagnostics: Vec::new(),
        }
    }
}
