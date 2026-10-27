use std::path::Path;

use semver::Version;
use serde::Deserialize;
use serde_json::json;

use super::CompatibilityErrorCode;
use super::CompatibilityRequirement;
use super::VERSION_PROBE_SCHEMA;
use crate::CliError;
use crate::consumer_integration::DOCS_SETUP_REFERENCE;
use crate::error::ErrorCode;

pub(super) fn compatibility_runtime_error(
    code: CompatibilityErrorCode,
    message: impl Into<String>,
    requirement: &CompatibilityRequirement,
    binary: &Path,
    observed: Option<&Version>,
    cause: String,
) -> CliError {
    let mut error = CliError::backend_failure(message)
        .with_code(code.as_str())
        .with_cause(cause)
        .with_suggested_action("Run `just setup` (or your repository's sc-lint installer) to install or upgrade sc-lint, then retry.")
        .with_documentation(DOCS_SETUP_REFERENCE)
        .with_detail("minimum_version", json!(requirement.minimum_version.to_string()))
        .with_detail("binary_path", json!(binary.display().to_string()))
        .with_detail("config_path", json!(requirement.config_path.display().to_string()));
    if let Some(observed) = observed {
        error = error.with_detail("installed_version", json!(observed.to_string()));
    }
    error
}

#[derive(Debug, Deserialize)]
pub(super) struct VersionProbeEnvelope {
    ok: bool,
    command: String,
    data: Option<VersionProbeData>,
}

#[derive(Debug, Deserialize)]
struct VersionProbeData {
    tool: String,
    version: String,
    contract_schema: String,
}

pub(super) fn validate_version_probe(
    probe: VersionProbeEnvelope,
    requirement: &CompatibilityRequirement,
    binary: &Path,
) -> Result<Version, CliError> {
    let invalid = |cause| {
        compatibility_runtime_error(
            CompatibilityErrorCode::ProbeMalformed,
            format!(
                "sc-lint binary `{}` emitted an unsupported version probe for required minimum `{}` from `{}`",
                binary.display(),
                requirement.minimum_version,
                requirement.config_path.display()
            ),
            requirement,
            binary,
            None,
            cause,
        )
    };
    if !probe.ok || probe.command != "version" {
        return Err(invalid(
            "the version probe envelope was not a successful `version` result".to_string(),
        ));
    }
    let data = probe
        .data
        .ok_or_else(|| invalid("the version probe envelope did not include data".to_string()))?;
    if data.tool != crate::consts::SERVICE_NAME || data.contract_schema != VERSION_PROBE_SCHEMA {
        return Err(invalid(
            "the version probe contract schema or tool name was unsupported".to_string(),
        ));
    }
    Version::parse(&data.version).map_err(|error| compatibility_runtime_error(CompatibilityErrorCode::VersionUnparsable, format!("sc-lint binary `{}` reported unparsable version `{}`; required minimum `{}` comes from `{}`", binary.display(), data.version, requirement.minimum_version, requirement.config_path.display()), requirement, binary, None, error.to_string()).with_detail("reported_version", json!(data.version)))
}

pub(super) fn stderr_or_status(output: &std::process::Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if stderr.is_empty() {
        format!("process exited with status {}", output.status)
    } else {
        stderr
    }
}
