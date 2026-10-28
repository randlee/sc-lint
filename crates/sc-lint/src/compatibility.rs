//! Version-compatibility contract: the consumer `minimum_version` floor, its
//! configuration errors, and validation of the installed binary's version probe.

use std::path::Path;
use std::path::PathBuf;
use std::process::ExitStatus;
use std::str::FromStr;
use std::time::Duration;

use semver::Version;
use serde::Deserialize;
use serde_json::Value;
use serde_json::json;

use crate::CliError;
use crate::config::ConfigCode;
use crate::config::ScLintConfigFile;
use crate::config::parse_repo_config;
use crate::consumer_integration::BINARY_NOT_FOUND_RECOVERY;
use crate::consumer_integration::DOCS_SETUP_REFERENCE;
use crate::error::ErrorCode;
use crate::installer::BACKEND_PROBE_TIMEOUT;
use crate::installer::ProbeFailure;
use crate::installer::run_version_command;

pub(crate) const VERSION_PROBE_SCHEMA: &str = "sc-lint-version-v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CompatibilityErrorCode {
    ConfigMissing,
    ConfigMalformed,
    BinaryNotFound,
    BinaryExecutionFailed,
    ProbeMalformed,
    VersionUnparsable,
    VersionTooOld,
}

impl ErrorCode for CompatibilityErrorCode {
    fn as_str(&self) -> &'static str {
        match self {
            Self::ConfigMissing => "CLI.SC_LINT_CONFIG_MISSING",
            Self::ConfigMalformed => "CLI.SC_LINT_CONFIG_MALFORMED",
            Self::BinaryNotFound => "CLI.SC_LINT_BINARY_NOT_FOUND",
            Self::BinaryExecutionFailed => "CLI.SC_LINT_BINARY_EXECUTION_FAILED",
            Self::ProbeMalformed => "CLI.SC_LINT_VERSION_PROBE_MALFORMED",
            Self::VersionUnparsable => "CLI.SC_LINT_VERSION_UNPARSABLE",
            Self::VersionTooOld => "CLI.SC_LINT_VERSION_TOO_OLD",
        }
    }
}

/// A validated semantic-version floor for a repository's sc-lint installation.
///
/// The string is parsed at configuration load time; callers receive this type
/// rather than a raw string, which prevents lexical version comparisons.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MinimumVersion(Version);

impl MinimumVersion {
    pub const FIELD_PATH: &'static str = "[tool.sc-lint].minimum_version";

    pub fn as_semver(&self) -> &Version {
        &self.0
    }
}

impl FromStr for MinimumVersion {
    type Err = semver::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Version::parse(value).map(Self)
    }
}

impl std::fmt::Display for MinimumVersion {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CompatibilityRequirement {
    pub(crate) minimum_version: MinimumVersion,
    pub(crate) config_path: PathBuf,
}

#[expect(
    clippy::result_large_err,
    reason = "Consumer configuration is validated before any lint or test command can begin."
)]
pub(crate) fn load_compatibility_requirement(
    path: &Path,
) -> Result<CompatibilityRequirement, CliError> {
    if !path.is_file() {
        return Err(compatibility_config_error(
            CompatibilityErrorCode::ConfigMissing,
            format!(
                "required sc-lint configuration `{}` was not found",
                path.display()
            ),
            Some(path),
            None,
        ));
    }
    let file_config = parse_repo_config(path).map_err(|error| {
        if error.code() == ConfigCode::UnknownTable.as_str() {
            return error;
        }
        compatibility_config_error(
            CompatibilityErrorCode::ConfigMalformed,
            format!("failed to parse sc-lint configuration `{}`", path.display()),
            Some(path),
            Some(error.to_string()),
        )
    })?;
    let sc_lint = file_config
        .tool
        .as_ref()
        .and_then(|tool| tool.sc_lint.as_ref())
        .ok_or_else(|| missing_consumer_field(path, "[tool.sc-lint]"))?;
    compatibility_requirement_from_file(sc_lint, path)
}

#[expect(
    clippy::result_large_err,
    reason = "Consumer configuration validation returns the shared top-level CliError contract."
)]
pub(crate) fn compatibility_requirement_from_file(
    sc_lint: &ScLintConfigFile,
    path: &Path,
) -> Result<CompatibilityRequirement, CliError> {
    let raw_minimum = sc_lint
        .minimum_version
        .as_ref()
        .ok_or_else(|| missing_consumer_field(path, MinimumVersion::FIELD_PATH))?;
    let minimum_version = MinimumVersion::from_str(raw_minimum).map_err(|error| {
        compatibility_config_error(
            CompatibilityErrorCode::ConfigMalformed,
            format!(
                "configuration `{}` has malformed `{}` value `{raw_minimum}`",
                path.display(),
                MinimumVersion::FIELD_PATH
            ),
            Some(path),
            Some(error.to_string()),
        )
    })?;
    Ok(CompatibilityRequirement {
        minimum_version,
        config_path: path.to_path_buf(),
    })
}

pub(crate) fn missing_consumer_field(path: &Path, field: &str) -> CliError {
    compatibility_config_error(
        CompatibilityErrorCode::ConfigMalformed,
        format!(
            "configuration `{}` is missing required field `{field}`",
            path.display()
        ),
        Some(path),
        None,
    )
}

pub(crate) fn compatibility_config_error(
    code: CompatibilityErrorCode,
    message: impl Into<String>,
    path: Option<&Path>,
    cause: Option<String>,
) -> CliError {
    let mut error = CliError::config(message)
        .with_code(code.as_str())
        .with_suggested_action(BINARY_NOT_FOUND_RECOVERY)
        .with_documentation(DOCS_SETUP_REFERENCE)
        .with_detail("required_field", json!(MinimumVersion::FIELD_PATH));
    if let Some(path) = path {
        error = error.with_detail("config_path", json!(path.display().to_string()));
    }
    error = error.with_cause(cause.unwrap_or_else(|| {
        "the canonical sc-lint compatibility requirement was absent".to_string()
    }));
    error
}

pub(crate) fn compatibility_runtime_error(
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
        .with_suggested_action(
            "Run `just setup` (or your repository's sc-lint installer) to install or upgrade sc-lint, then retry.",
        )
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
pub(crate) struct VersionProbeEnvelope {
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

#[expect(
    clippy::result_large_err,
    reason = "An invalid external version probe is exposed as a structured recovery error."
)]
pub(crate) fn validate_version_probe(
    probe: VersionProbeEnvelope,
    requirement: &CompatibilityRequirement,
    binary: &Path,
) -> Result<Version, CliError> {
    let invalid_probe = |cause: String| {
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
        return Err(invalid_probe(
            "the version probe envelope was not a successful `version` result".to_string(),
        ));
    }
    let data = probe.data.ok_or_else(|| {
        invalid_probe("the version probe envelope did not include data".to_string())
    })?;
    if data.tool != crate::consts::SERVICE_NAME || data.contract_schema != VERSION_PROBE_SCHEMA {
        return Err(invalid_probe(
            "the version probe contract schema or tool name was unsupported".to_string(),
        ));
    }
    Version::parse(&data.version).map_err(|error| {
        compatibility_runtime_error(
            CompatibilityErrorCode::VersionUnparsable,
            format!(
                "sc-lint binary `{}` reported unparsable version `{}`; required minimum `{}` comes from `{}`",
                binary.display(),
                data.version,
                requirement.minimum_version,
                requirement.config_path.display()
            ),
            requirement,
            binary,
            None,
            error.to_string(),
        )
        .with_detail("reported_version", json!(data.version))
    })
}

/// Probes the installed sc-lint binary with `--json version` and checks the
/// reported version against `requirement`.
#[expect(
    clippy::result_large_err,
    reason = "The compatibility check must return one stable CliError envelope for each recoverable failure."
)]
pub(crate) fn evaluate_compatibility(
    requirement: &CompatibilityRequirement,
    binary_override: Option<&Path>,
) -> Result<Value, CliError> {
    let binary = binary_override.unwrap_or_else(|| Path::new(crate::consts::SERVICE_NAME));
    let stdout = run_version_probe(requirement, binary)?;
    let probe = serde_json::from_slice::<VersionProbeEnvelope>(&stdout).map_err(|error| {
        compatibility_runtime_error(
            CompatibilityErrorCode::ProbeMalformed,
            format!(
                "sc-lint binary {} did not emit a valid version probe",
                required_by(requirement, binary)
            ),
            requirement,
            binary,
            None,
            error.to_string(),
        )
    })?;
    let observed = validate_version_probe(probe, requirement, binary)?;
    if observed < *requirement.minimum_version.as_semver() {
        return Err(compatibility_runtime_error(
            CompatibilityErrorCode::VersionTooOld,
            format!(
                "installed sc-lint version `{observed}` at `{}` does not satisfy required minimum `{}` from `{}`",
                binary.display(),
                requirement.minimum_version,
                requirement.config_path.display()
            ),
            requirement,
            binary,
            Some(&observed),
            "the installed release is below the repository minimum version".to_string(),
        ));
    }

    Ok(json!({
        "status": "pass",
        "minimum_version": requirement.minimum_version.to_string(),
        "installed_version": observed.to_string(),
        "binary_path": binary.display().to_string(),
        "config_path": requirement.config_path.display().to_string(),
    }))
}

/// `` `<binary>` required at least `<minimum>` by `<config>` ``, the subject of
/// every probe failure message.
fn required_by(requirement: &CompatibilityRequirement, binary: &Path) -> String {
    format!(
        "`{}` required at least `{}` by `{}`",
        binary.display(),
        requirement.minimum_version,
        requirement.config_path.display()
    )
}

/// Runs `<binary> --json version` bounded by the installer's probe timeout; a
/// binary that cannot start, times out or exits unsuccessfully is a
/// compatibility runtime error. Returns the probe's stdout.
#[expect(
    clippy::result_large_err,
    reason = "The compatibility check must return one stable CliError envelope for each recoverable failure."
)]
fn run_version_probe(
    requirement: &CompatibilityRequirement,
    binary: &Path,
) -> Result<Vec<u8>, CliError> {
    run_version_probe_with(requirement, binary, run_version_command)
}

/// [`run_version_probe`] with the bounded probe as a seam: production passes
/// [`run_version_command`], tests a controlled fake. `run` always receives
/// [`BACKEND_PROBE_TIMEOUT`], the bound `probe_version` uses.
#[expect(
    clippy::result_large_err,
    reason = "The compatibility check must return one stable CliError envelope for each recoverable failure."
)]
fn run_version_probe_with(
    requirement: &CompatibilityRequirement,
    binary: &Path,
    run: impl FnOnce(&Path, Duration) -> Result<(ExitStatus, Vec<u8>), ProbeFailure>,
) -> Result<Vec<u8>, CliError> {
    let (status, stdout) = run(binary, BACKEND_PROBE_TIMEOUT).map_err(|failure| {
        let (code, verb, cause) = match failure {
            ProbeFailure::Start(error) if error.kind() == std::io::ErrorKind::NotFound => (
                CompatibilityErrorCode::BinaryNotFound,
                "find",
                error.to_string(),
            ),
            ProbeFailure::Start(error) => (
                CompatibilityErrorCode::BinaryExecutionFailed,
                "execute",
                error.to_string(),
            ),
            ProbeFailure::Probe(message) => (
                CompatibilityErrorCode::BinaryExecutionFailed,
                "complete the version probe of",
                message,
            ),
        };
        compatibility_runtime_error(
            code,
            format!(
                "could not {verb} sc-lint binary {}",
                required_by(requirement, binary)
            ),
            requirement,
            binary,
            None,
            cause,
        )
    })?;
    if !status.success() {
        return Err(compatibility_runtime_error(
            CompatibilityErrorCode::BinaryExecutionFailed,
            format!(
                "sc-lint binary {} exited unsuccessfully",
                required_by(requirement, binary)
            ),
            requirement,
            binary,
            None,
            format!("process exited with status {status}"),
        ));
    }
    Ok(stdout)
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    #[cfg(unix)]
    use std::os::unix::process::ExitStatusExt;
    #[cfg(windows)]
    use std::os::windows::process::ExitStatusExt;

    use super::*;

    fn requirement() -> CompatibilityRequirement {
        CompatibilityRequirement {
            minimum_version: MinimumVersion::from_str("0.4.1").expect("minimum version"),
            config_path: PathBuf::from("sc-lint.toml"),
        }
    }

    #[cfg(unix)]
    fn exit_status(code: i32) -> ExitStatus {
        ExitStatus::from_raw(code << 8)
    }

    #[cfg(windows)]
    fn exit_status(code: u32) -> ExitStatus {
        ExitStatus::from_raw(code)
    }

    /// Runs the compatibility probe against a fake that records the timeout it
    /// was given and returns `result`; no process is started.
    fn probe_with(
        result: Result<(ExitStatus, Vec<u8>), ProbeFailure>,
    ) -> (Result<Vec<u8>, CliError>, Option<Duration>) {
        let seen = Cell::new(None);
        let outcome = run_version_probe_with(&requirement(), Path::new("sc-lint"), |_, timeout| {
            seen.set(Some(timeout));
            result
        });
        (outcome, seen.get())
    }

    #[test]
    fn version_probe_is_bounded_by_the_installer_probe_timeout() {
        let (outcome, timeout) = probe_with(Ok((exit_status(0), b"{}".to_vec())));
        assert_eq!(timeout, Some(BACKEND_PROBE_TIMEOUT));
        assert_eq!(outcome.expect("successful probe"), b"{}");
    }

    #[test]
    fn version_probe_timeout_is_a_binary_execution_failure() {
        let timed_out = "backend companion `sc-lint` at `sc-lint` timed out after 15000 ms during its `--json version` probe; process-tree cleanup was attempted";
        let (outcome, _) = probe_with(Err(ProbeFailure::Probe(timed_out.to_string())));
        let error = outcome.expect_err("a timed-out probe fails");
        assert_eq!(error.code(), "CLI.SC_LINT_BINARY_EXECUTION_FAILED");
        assert_eq!(error.cause.as_deref(), Some(timed_out));
        assert_eq!(
            error.message,
            "could not complete the version probe of sc-lint binary `sc-lint` required at least `0.4.1` by `sc-lint.toml`"
        );
    }

    #[test]
    fn version_probe_start_failures_keep_their_codes() {
        let (missing, _) = probe_with(Err(ProbeFailure::Start(std::io::Error::from(
            std::io::ErrorKind::NotFound,
        ))));
        assert_eq!(
            missing.expect_err("missing binary").code(),
            "CLI.SC_LINT_BINARY_NOT_FOUND"
        );
        let (denied, _) = probe_with(Err(ProbeFailure::Start(std::io::Error::from(
            std::io::ErrorKind::PermissionDenied,
        ))));
        assert_eq!(
            denied.expect_err("unexecutable binary").code(),
            "CLI.SC_LINT_BINARY_EXECUTION_FAILED"
        );
    }

    #[test]
    fn version_probe_unsuccessful_exit_is_a_binary_execution_failure() {
        let (outcome, _) = probe_with(Ok((exit_status(3), Vec::new())));
        let error = outcome.expect_err("non-zero exit fails");
        assert_eq!(error.code(), "CLI.SC_LINT_BINARY_EXECUTION_FAILED");
        assert!(
            error
                .cause
                .as_deref()
                .is_some_and(|cause| cause.starts_with("process exited with status ")),
            "{error:?}"
        );
    }
}
