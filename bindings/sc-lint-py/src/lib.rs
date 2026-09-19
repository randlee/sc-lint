//! Python extension module for sc-lint.
//!
//! The binding surface is intentionally minimal (ADR-016): the extension
//! exposes the version probe and the CLI entry point only. All helper logic
//! lives in the pure-Python `sc_lint` package or in the Rust CLI itself.

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

#[pymodule]
#[pyo3(name = "_native")]
fn native(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(version_json, module)?)?;
    module.add_function(wrap_pyfunction!(run, module)?)?;
    Ok(())
}
