use std::error::Error;
use std::fmt;

use serde_json::Map;
use serde_json::Value;

/// A shape mismatch in the `sc-lint-python-v1` transport payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PayloadDrift {
    pub key_path: String,
    pub problem: String,
}

impl PayloadDrift {
    fn new(key_path: impl Into<String>, problem: &'static str) -> Self {
        Self {
            key_path: key_path.into(),
            problem: problem.to_string(),
        }
    }
}

impl fmt::Display for PayloadDrift {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "`{}`: {}", self.key_path, self.problem)
    }
}

impl Error for PayloadDrift {}

/// Validates the stable shape of a Python adapter payload.
///
/// Unknown keys are allowed so the transport can grow additively. Required
/// fields and conditional success/failure fields are checked in a fixed order,
/// making the first reported drift deterministic.
pub fn validate_adapter_payload(value: &Value) -> Result<(), PayloadDrift> {
    let object = value
        .as_object()
        .ok_or_else(|| PayloadDrift::new("$", "expected object"))?;
    required_string(object, "adapter_schema")?;
    let ok = required_boolean(object, "ok")?;
    validate_string_array(object, "diagnostics")?;

    if ok {
        required_object(object, "data")?;
        required_string(object, "summary")?;
        if object.contains_key("error") {
            return Err(PayloadDrift::new("error", "must be absent when ok is true"));
        }
    } else {
        let error = required_object(object, "error")?;
        required_string(error, "error.kind")?;
        required_string(error, "error.message")?;
        if let Some(details) = error.get("details")
            && !details.is_object()
        {
            return Err(PayloadDrift::new("error.details", "expected object"));
        }
        if let Some(suggested_action) = error.get("suggested_action")
            && !suggested_action.is_string()
        {
            return Err(PayloadDrift::new(
                "error.suggested_action",
                "expected string",
            ));
        }
        if let Some(summary) = object.get("summary")
            && !summary.is_string()
        {
            return Err(PayloadDrift::new("summary", "expected string"));
        }
        if object.contains_key("data") {
            return Err(PayloadDrift::new("data", "must be absent when ok is false"));
        }
    }

    Ok(())
}

fn required_string(object: &Map<String, Value>, key_path: &str) -> Result<(), PayloadDrift> {
    let key = key_path.rsplit('.').next().unwrap_or(key_path);
    let value = object
        .get(key)
        .ok_or_else(|| PayloadDrift::new(key_path, "missing required key"))?;
    if !value.is_string() {
        return Err(PayloadDrift::new(key_path, "expected string"));
    }
    Ok(())
}

fn required_boolean(object: &Map<String, Value>, key_path: &str) -> Result<bool, PayloadDrift> {
    let value = object
        .get(key_path)
        .ok_or_else(|| PayloadDrift::new(key_path, "missing required key"))?;
    value
        .as_bool()
        .ok_or_else(|| PayloadDrift::new(key_path, "expected boolean"))
}

fn required_object<'a>(
    object: &'a Map<String, Value>,
    key_path: &str,
) -> Result<&'a Map<String, Value>, PayloadDrift> {
    let key = key_path.rsplit('.').next().unwrap_or(key_path);
    let value = object
        .get(key)
        .ok_or_else(|| PayloadDrift::new(key_path, "missing required key"))?;
    value
        .as_object()
        .ok_or_else(|| PayloadDrift::new(key_path, "expected object"))
}

fn validate_string_array(object: &Map<String, Value>, key_path: &str) -> Result<(), PayloadDrift> {
    let values = object
        .get(key_path)
        .ok_or_else(|| PayloadDrift::new(key_path, "missing required key"))?
        .as_array()
        .ok_or_else(|| PayloadDrift::new(key_path, "expected array"))?;
    for (index, value) in values.iter().enumerate() {
        if !value.is_string() {
            return Err(PayloadDrift::new(
                format!("{key_path}[{index}]"),
                "expected string",
            ));
        }
    }
    Ok(())
}
