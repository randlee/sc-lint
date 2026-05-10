use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CommandEnvelope<T>
where
    T: Serialize,
{
    pub ok: bool,
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
