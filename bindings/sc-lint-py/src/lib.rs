//! Python extension module for sc-lint.
//!
//! The binding surface is intentionally minimal (ADR-016): the extension
//! exposes the version probe, the CLI entry point, and one TOML parser shared
//! with the CLI (`toml_loads`). All other helper logic lives in the pure-Python
//! `sc_lint` package or in the Rust CLI itself.

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

/// Returns the `sc-lint version` JSON payload.
#[pyfunction]
fn version_json() -> PyResult<String> {
    Ok(sc_lint::version_json())
}

/// Runs the sc-lint CLI in-process with `argv` (argv[0] is the program name)
/// and returns the exit code instead of terminating the interpreter.
#[pyfunction]
fn run(argv: Vec<String>) -> PyResult<i32> {
    Ok(i32::from(sc_lint::run_code(argv)))
}

/// Parses TOML with the workspace `toml` crate and returns the document as a
/// JSON string. Datetimes map to their RFC 3339 text, and non-finite floats map
/// to `null`. Invalid input raises `ValueError` naming the line and column.
#[pyfunction]
fn toml_loads(text: &str) -> PyResult<String> {
    let table: toml::Table = toml::from_str(text).map_err(|error| {
        let location = error
            .span()
            .map_or_else(String::new, |span| toml_location(text, span.start));
        PyValueError::new_err(format!("invalid TOML{location}: {}", error.message()))
    })?;
    serde_json::to_string(&json_object(table))
        .map_err(|error| PyValueError::new_err(error.to_string()))
}

/// Formats a byte offset as ` at line N, column M` (1-based, column in chars).
fn toml_location(text: &str, offset: usize) -> String {
    let prefix = text.get(..offset).unwrap_or(text);
    let line = prefix.matches('\n').count() + 1;
    let line_start = prefix.rfind('\n').map_or(0, |index| index + 1);
    let column = prefix[line_start..].chars().count() + 1;
    format!(" at line {line}, column {column}")
}

fn json_object(table: toml::Table) -> serde_json::Map<String, serde_json::Value> {
    table
        .into_iter()
        .map(|(key, value)| (key, json_value(value)))
        .collect()
}

fn json_value(value: toml::Value) -> serde_json::Value {
    match value {
        toml::Value::String(text) => serde_json::Value::String(text),
        toml::Value::Integer(number) => serde_json::Value::from(number),
        toml::Value::Float(number) => serde_json::Number::from_f64(number)
            .map_or(serde_json::Value::Null, serde_json::Value::Number),
        toml::Value::Boolean(flag) => serde_json::Value::Bool(flag),
        toml::Value::Datetime(datetime) => serde_json::Value::String(datetime.to_string()),
        toml::Value::Array(items) => {
            serde_json::Value::Array(items.into_iter().map(json_value).collect())
        }
        toml::Value::Table(table) => serde_json::Value::Object(json_object(table)),
    }
}

#[pymodule]
#[pyo3(name = "_native")]
fn native(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(version_json, module)?)?;
    module.add_function(wrap_pyfunction!(run, module)?)?;
    module.add_function(wrap_pyfunction!(toml_loads, module)?)?;
    Ok(())
}
