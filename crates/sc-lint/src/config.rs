use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::str::FromStr;

use semver::Version;
use serde::Deserialize;
use serde_json::Value;
use serde_json::json;

use crate::Cli;
use crate::CliError;
use crate::command::CommandContext;

pub(crate) const CONFIG_FILENAME: &str = "sc-lint.toml";
pub(crate) const VERSION_PROBE_SCHEMA: &str = "sc-lint-version-v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedConfig {
    repo_root: Option<RepoRoot>,
    config_path: Option<PathBuf>,
    logging_root: Option<PathBuf>,
    logging_console: bool,
    mode: LoadedConfigMode,
}

/// `LoadedConfig` has one of these two valid modes; an ordinary command can
/// never accidentally carry a consumer compatibility requirement.
#[derive(Debug, Clone, PartialEq, Eq)]
enum LoadedConfigMode {
    Standard,
    Compatibility(CompatibilityRequirement),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CompatibilityErrorCode {
    ConfigMissing,
    ConfigMalformed,
    BinaryNotFound,
    BinaryExecutionFailed,
    ProbeMalformed,
    VersionUnparsable,
    VersionTooOld,
}

impl CompatibilityErrorCode {
    const fn as_str(self) -> &'static str {
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
struct CompatibilityRequirement {
    minimum_version: MinimumVersion,
    config_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RepoRoot(PathBuf);

impl RepoRoot {
    #[expect(
        clippy::result_large_err,
        reason = "Repo-root discovery failures are part of the stable top-level CliError contract."
    )]
    fn discover(start: &Path) -> Result<Self, CliError> {
        let mut current = if start.is_dir() {
            start.to_path_buf()
        } else {
            start
                .parent()
                .map_or_else(|| start.to_path_buf(), Path::to_path_buf)
        };
        current = dunce::canonicalize(&current).map_err(|error| {
            CliError::config(format!(
                "failed to canonicalize repo-root discovery start `{}`",
                start.display()
            ))
            .with_source(error)
        })?;

        loop {
            if current.join("Cargo.toml").is_file() && current.join("boundaries").is_dir() {
                return Ok(Self(current));
            }
            let inspected = current.clone();
            if !current.pop() {
                return Err(CliError::config(format!(
                    "could not discover the sc-lint repo root from `{}`; last inspected `{}` was missing required sentinels `Cargo.toml` and/or `boundaries`",
                    start.display(),
                    inspected.display()
                ))
                .with_detail("discovery_start", json!(start.display().to_string()))
                .with_detail("last_inspected", json!(inspected.display().to_string()))
                .with_detail("required_sentinels", json!(["Cargo.toml", "boundaries"]))
                .with_suggested_action(
                    "Run the command inside the repo or pass `--root <path>` to the workspace root.",
                ));
            }
        }
    }

    pub(crate) fn as_path(&self) -> &Path {
        &self.0
    }
}

impl LoadedConfig {
    #[expect(
        clippy::result_large_err,
        reason = "Config loading failures are part of the stable top-level CliError contract."
    )]
    pub fn load(cli: &Cli, context: &CommandContext) -> Result<Self, CliError> {
        if context.requires_compatibility_config() {
            return Self::load_compatibility(cli);
        }
        if !context.requires_repo_root() {
            return Ok(Self {
                repo_root: None,
                config_path: None,
                logging_root: cli.log_root.clone(),
                logging_console: cli.log_console,
                mode: LoadedConfigMode::Standard,
            });
        }

        let discovery_base = if let Some(root) = cli.root.as_ref() {
            root.clone()
        } else {
            std::env::current_dir().map_err(|error| {
                CliError::config("failed to read current directory").with_source(error)
            })?
        };
        let repo_root = RepoRoot::discover(&discovery_base)?;
        let config_path = find_repo_config(repo_root.as_path(), cli.config.as_deref());
        let file_config = if let Some(path) = config_path.as_ref() {
            parse_repo_config(path)?
        } else {
            RepoConfigFile::default()
        };

        let logging_root = cli.log_root.clone().or_else(|| {
            file_config
                .logging
                .as_ref()
                .and_then(|logging| logging.root.as_ref())
                .map(|path| resolve_repo_relative_path(repo_root.as_path(), path))
        });
        let logging_console = if cli.log_console {
            true
        } else {
            file_config
                .logging
                .as_ref()
                .and_then(|logging| logging.console)
                .unwrap_or(false)
        };

        Ok(Self {
            repo_root: Some(repo_root),
            config_path,
            logging_root,
            logging_console,
            mode: LoadedConfigMode::Standard,
        })
    }

    #[expect(
        clippy::result_large_err,
        reason = "The installer shares the established stable CliError contract at the consumer configuration boundary."
    )]
    pub(crate) fn compatibility_requirement(&self) -> Result<(&MinimumVersion, &Path), CliError> {
        match &self.mode {
            LoadedConfigMode::Compatibility(requirement) => {
                Ok((&requirement.minimum_version, &requirement.config_path))
            }
            LoadedConfigMode::Standard => Err(CliError::internal(
                "installation commands require a loaded consumer compatibility configuration",
            )),
        }
    }

    #[expect(
        clippy::result_large_err,
        reason = "Compatibility configuration errors are deliberately surfaced through CliError."
    )]
    fn load_compatibility(cli: &Cli) -> Result<Self, CliError> {
        let current_dir = std::env::current_dir().map_err(|error| {
            compatibility_config_error(
                CompatibilityErrorCode::ConfigMissing,
                format!("failed to read the current directory while locating `{CONFIG_FILENAME}`"),
                None,
                None,
            )
            .with_source(error)
        })?;
        let config_path = cli.config.as_ref().map_or_else(
            || current_dir.join(CONFIG_FILENAME),
            |path| resolve_current_dir_relative_path(&current_dir, path),
        );
        let requirement = load_compatibility_requirement(&config_path)?;

        Ok(Self {
            repo_root: None,
            config_path: Some(config_path),
            logging_root: cli.log_root.clone(),
            logging_console: cli.log_console,
            mode: LoadedConfigMode::Compatibility(requirement),
        })
    }

    pub fn repo_root(&self) -> Option<&Path> {
        self.repo_root.as_ref().map(RepoRoot::as_path)
    }

    #[expect(
        clippy::result_large_err,
        reason = "Commands that require a repo root must surface failures through the shared CliError contract."
    )]
    pub fn require_repo_root(&self) -> Result<&Path, CliError> {
        self.repo_root
            .as_ref()
            .map(RepoRoot::as_path)
            .ok_or_else(|| {
                CliError::internal("repo root required but configuration did not resolve one")
            })
    }

    pub fn config_path(&self) -> Option<&Path> {
        self.config_path.as_deref()
    }

    pub fn logging_root(&self) -> Option<&PathBuf> {
        self.logging_root.as_ref()
    }

    pub const fn logging_console(&self) -> bool {
        self.logging_console
    }

    #[expect(
        clippy::result_large_err,
        reason = "The compatibility check must return one stable CliError envelope for each recoverable failure."
    )]
    pub(crate) fn evaluate_compatibility(
        &self,
        binary_override: Option<&Path>,
    ) -> Result<Value, CliError> {
        let LoadedConfigMode::Compatibility(requirement) = &self.mode else {
            return Err(CliError::internal(
                "compatibility evaluation requires a compatibility configuration",
            ));
        };
        let binary = binary_override.unwrap_or_else(|| Path::new(crate::consts::SERVICE_NAME));
        let output = Command::new(binary)
            .args(["--json", "version"])
            .output()
            .map_err(|error| {
                let (code, message) = if error.kind() == std::io::ErrorKind::NotFound {
                    (
                        CompatibilityErrorCode::BinaryNotFound,
                        format!(
                            "could not find sc-lint binary `{}` required at least `{}` by `{}`",
                            binary.display(),
                            requirement.minimum_version,
                            requirement.config_path.display()
                        ),
                    )
                } else {
                    (
                        CompatibilityErrorCode::BinaryExecutionFailed,
                        format!(
                            "could not execute sc-lint binary `{}` required at least `{}` by `{}`",
                            binary.display(),
                            requirement.minimum_version,
                            requirement.config_path.display()
                        ),
                    )
                };
                compatibility_runtime_error(
                    code,
                    message,
                    requirement,
                    binary,
                    None,
                    error.to_string(),
                )
            })?;

        if !output.status.success() {
            return Err(compatibility_runtime_error(
                CompatibilityErrorCode::BinaryExecutionFailed,
                format!(
                    "sc-lint binary `{}` required at least `{}` by `{}` exited unsuccessfully",
                    binary.display(),
                    requirement.minimum_version,
                    requirement.config_path.display()
                ),
                requirement,
                binary,
                None,
                stderr_or_status(&output),
            ));
        }

        let probe =
            serde_json::from_slice::<VersionProbeEnvelope>(&output.stdout).map_err(|error| {
                compatibility_runtime_error(
                    CompatibilityErrorCode::ProbeMalformed,
                    format!(
                        "sc-lint binary `{}` required at least `{}` by `{}` did not emit a valid version probe",
                        binary.display(), requirement.minimum_version, requirement.config_path.display()
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
}

#[derive(Debug, Clone, Default, Deserialize)]
struct RepoConfigFile {
    logging: Option<LoggingConfigFile>,
    tool: Option<ToolConfigFile>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct ToolConfigFile {
    #[serde(rename = "sc-lint")]
    sc_lint: Option<ScLintConfigFile>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct ScLintConfigFile {
    minimum_version: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct LoggingConfigFile {
    root: Option<PathBuf>,
    console: Option<bool>,
}

fn find_repo_config(repo_root: &Path, override_path: Option<&Path>) -> Option<PathBuf> {
    if let Some(path) = override_path {
        return Some(resolve_repo_relative_path(repo_root, path));
    }
    [CONFIG_FILENAME, ".just/lint-config.toml"]
        .into_iter()
        .map(|relative| repo_root.join(relative))
        .find(|path| path.exists())
}

#[expect(
    clippy::result_large_err,
    reason = "Repo config parse failures are part of the stable top-level CliError contract."
)]
fn parse_repo_config(path: &Path) -> Result<RepoConfigFile, CliError> {
    let text = fs::read_to_string(path).map_err(|error| {
        CliError::config(format!("failed to read repo config `{}`", path.display()))
            .with_source(error)
    })?;
    toml::from_str(&text).map_err(|error| {
        CliError::config(format!("failed to parse repo config `{}`", path.display()))
            .with_source(error)
            .with_detail("config_path", Value::String(path.display().to_string()))
    })
}

fn resolve_repo_relative_path(repo_root: &Path, candidate: &Path) -> PathBuf {
    if candidate.is_absolute() {
        candidate.to_path_buf()
    } else {
        repo_root.join(candidate)
    }
}

fn resolve_current_dir_relative_path(current_dir: &Path, candidate: &Path) -> PathBuf {
    if candidate.is_absolute() {
        candidate.to_path_buf()
    } else {
        current_dir.join(candidate)
    }
}

#[expect(
    clippy::result_large_err,
    reason = "Consumer configuration is validated before any lint or test command can begin."
)]
fn load_compatibility_requirement(path: &Path) -> Result<CompatibilityRequirement, CliError> {
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
        compatibility_config_error(
            CompatibilityErrorCode::ConfigMalformed,
            format!("failed to parse sc-lint configuration `{}`", path.display()),
            Some(path),
            Some(error.to_string()),
        )
    })?;
    let raw_minimum = file_config
        .tool
        .and_then(|tool| tool.sc_lint)
        .and_then(|sc_lint| sc_lint.minimum_version)
        .ok_or_else(|| {
            compatibility_config_error(
                CompatibilityErrorCode::ConfigMalformed,
                format!(
                    "configuration `{}` is missing required field `{}`",
                    path.display(),
                    MinimumVersion::FIELD_PATH
                ),
                Some(path),
                None,
            )
        })?;
    let minimum_version = MinimumVersion::from_str(&raw_minimum).map_err(|error| {
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

fn compatibility_config_error(
    code: CompatibilityErrorCode,
    message: impl Into<String>,
    path: Option<&Path>,
    cause: Option<String>,
) -> CliError {
    let mut error = CliError::config(message)
        .with_code(code.as_str())
        .with_suggested_action(
            "Run `just setup` (or your repository's sc-lint installer) to create or repair the compatible installation.",
        )
        .with_documentation("sc-lint docs setup")
        .with_detail("required_field", json!(MinimumVersion::FIELD_PATH));
    if let Some(path) = path {
        error = error.with_detail("config_path", json!(path.display().to_string()));
    }
    error = error.with_cause(cause.unwrap_or_else(|| {
        "the canonical sc-lint compatibility requirement was absent".to_string()
    }));
    error
}

fn compatibility_runtime_error(
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
        .with_documentation("sc-lint docs setup")
        .with_detail("minimum_version", json!(requirement.minimum_version.to_string()))
        .with_detail("binary_path", json!(binary.display().to_string()))
        .with_detail("config_path", json!(requirement.config_path.display().to_string()));
    if let Some(observed) = observed {
        error = error.with_detail("installed_version", json!(observed.to_string()));
    }
    error
}

#[derive(Debug, Deserialize)]
struct VersionProbeEnvelope {
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
fn validate_version_probe(
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

fn stderr_or_status(output: &std::process::Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if stderr.is_empty() {
        format!("process exited with status {}", output.status)
    } else {
        stderr
    }
}
