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
use crate::command::ConsumerInitRequest;
use crate::error::ErrorCode;
use crate::installer::CONSUMER_BOOTSTRAP_ASSET;

pub(crate) const CONFIG_FILENAME: &str = "sc-lint.toml";
pub(crate) const VERSION_PROBE_SCHEMA: &str = "sc-lint-version-v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ConsumerProfile {
    Lint,
    Test,
}

impl ConsumerProfile {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Lint => "lint",
            Self::Test => "test",
        }
    }
}

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
    Consumer(ConsumerRequirement),
}

/// A command is decoded and validated at the TOML boundary, so profile
/// orchestration never needs to split shell strings or infer consumer policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ConsumerProfileStep {
    name: String,
    command: Vec<String>,
}

impl ConsumerProfileStep {
    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn command(&self) -> &[String] {
        &self.command
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ConsumerRequirement {
    compatibility: CompatibilityRequirement,
    root: PathBuf,
    lint: Vec<ConsumerProfileStep>,
    test: Vec<ConsumerProfileStep>,
}

const GENERATED_FILE_HEADER: &str =
    "# Managed by sc-lint; regenerate with `sc-lint init --just`.\n";
const CONSUMER_JUSTFILE_ASSET: &str = include_str!("../assets/consumer-Justfile");

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
        if matches!(
            context.id(),
            crate::command::CommandId::ConsumerLintCi | crate::command::CommandId::ConsumerTest
        ) {
            return Self::load_consumer(cli);
        }
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
            LoadedConfigMode::Consumer(requirement) => Ok((
                &requirement.compatibility.minimum_version,
                &requirement.compatibility.config_path,
            )),
            LoadedConfigMode::Standard => Err(CliError::internal(
                "installation commands require a loaded consumer compatibility configuration",
            )),
        }
    }

    #[expect(
        clippy::result_large_err,
        reason = "Consumer profile lookup retains the shared top-level CliError contract."
    )]
    pub(crate) fn consumer_profile(
        &self,
        profile: ConsumerProfile,
    ) -> Result<(&Path, &[ConsumerProfileStep]), CliError> {
        let LoadedConfigMode::Consumer(requirement) = &self.mode else {
            return Err(CliError::internal(
                "consumer profile execution requires a consumer configuration",
            ));
        };
        let steps = match profile {
            ConsumerProfile::Lint => &requirement.lint,
            ConsumerProfile::Test => &requirement.test,
        };
        Ok((&requirement.root, steps))
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

    #[expect(
        clippy::result_large_err,
        reason = "Consumer profiles are validated before a product command can launch any configured backend."
    )]
    fn load_consumer(cli: &Cli) -> Result<Self, CliError> {
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
        let file_config = load_consumer_config_file(&config_path)?;
        let sc_lint = file_config
            .tool
            .as_ref()
            .and_then(|tool| tool.sc_lint.as_ref())
            .ok_or_else(|| missing_consumer_field(&config_path, "[tool.sc-lint]"))?;
        let compatibility = compatibility_requirement_from_file(sc_lint, &config_path)?;
        let lint = validate_consumer_profile("lint", &sc_lint.lint, &config_path)?;
        let test = validate_consumer_profile("test", &sc_lint.test, &config_path)?;
        let root = config_path
            .parent()
            .ok_or_else(|| CliError::internal("consumer configuration has no parent directory"))?
            .to_path_buf();

        Ok(Self {
            repo_root: None,
            config_path: Some(config_path),
            logging_root: cli.log_root.clone(),
            logging_console: cli.log_console,
            mode: LoadedConfigMode::Consumer(ConsumerRequirement {
                compatibility,
                root,
                lint,
                test,
            }),
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
    #[serde(default)]
    lint: Vec<ConsumerProfileStepFile>,
    #[serde(default)]
    test: Vec<ConsumerProfileStepFile>,
}

#[derive(Debug, Clone, Deserialize)]
struct ConsumerProfileStepFile {
    name: Option<String>,
    command: Option<Vec<String>>,
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
fn compatibility_requirement_from_file(
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

fn missing_consumer_field(path: &Path, field: &str) -> CliError {
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

#[expect(
    clippy::result_large_err,
    reason = "Consumer configuration loading returns the shared top-level CliError contract."
)]
fn load_consumer_config_file(path: &Path) -> Result<RepoConfigFile, CliError> {
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
    parse_repo_config(path).map_err(|error| {
        compatibility_config_error(
            CompatibilityErrorCode::ConfigMalformed,
            format!("failed to parse sc-lint configuration `{}`", path.display()),
            Some(path),
            Some(error.to_string()),
        )
    })
}

#[expect(
    clippy::result_large_err,
    reason = "Consumer profile validation returns the shared top-level CliError contract."
)]
fn validate_consumer_profile(
    profile: &str,
    raw_steps: &[ConsumerProfileStepFile],
    path: &Path,
) -> Result<Vec<ConsumerProfileStep>, CliError> {
    if raw_steps.is_empty() {
        return Err(consumer_profile_error(
            path,
            profile,
            format!("consumer `{profile}` profile must contain at least one command"),
        ));
    }
    let mut names = std::collections::BTreeSet::new();
    let mut steps = Vec::with_capacity(raw_steps.len());
    for (index, step) in raw_steps.iter().enumerate() {
        let name = step
            .name
            .as_deref()
            .filter(|name| !name.trim().is_empty())
            .ok_or_else(|| {
                consumer_profile_error(
                    path,
                    profile,
                    format!(
                        "consumer `{profile}` profile step {} is missing a non-empty `name`",
                        index + 1
                    ),
                )
            })?;
        if !names.insert(name.to_string()) {
            return Err(consumer_profile_error(
                path,
                profile,
                format!("consumer `{profile}` profile repeats step name `{name}`"),
            ));
        }
        let command = step
            .command
            .as_ref()
            .filter(|command| !command.is_empty())
            .ok_or_else(|| {
                consumer_profile_error(
                    path,
                    profile,
                    format!(
                        "consumer `{profile}` profile step `{name}` is missing a command argv array"
                    ),
                )
            })?;
        if command.iter().any(|argument| argument.is_empty()) {
            return Err(consumer_profile_error(
                path,
                profile,
                format!("consumer `{profile}` profile step `{name}` has an empty command argument"),
            ));
        }
        steps.push(ConsumerProfileStep {
            name: name.to_string(),
            command: command.clone(),
        });
    }
    Ok(steps)
}

fn consumer_profile_error(path: &Path, profile: &str, message: String) -> CliError {
    compatibility_config_error(
        CompatibilityErrorCode::ConfigMalformed,
        message,
        Some(path),
        None,
    )
    .with_detail("profile", json!(profile))
}

/// Render the canonical consumer integration into the current directory. The
/// caller deliberately has no source-repository root: consumer mode is an
/// explicit command contract rather than a directory-name heuristic.
#[expect(
    clippy::result_large_err,
    reason = "Consumer initialization preserves the shared top-level CliError contract."
)]
pub(crate) fn run_consumer_init(request: ConsumerInitRequest) -> Result<Value, CliError> {
    let root = std::env::current_dir().map_err(|error| {
        CliError::config("failed to read current directory for consumer initialization")
            .with_source(error)
    })?;
    run_consumer_init_at(&root, request)
}

#[expect(
    clippy::result_large_err,
    reason = "Consumer integration file ownership errors use the shared top-level CliError contract."
)]
pub(crate) fn run_consumer_init_at(
    root: &Path,
    request: ConsumerInitRequest,
) -> Result<Value, CliError> {
    if !request.just {
        return Err(
            CliError::usage("`sc-lint init` requires `--just`").with_suggested_action(
                "Run `sc-lint init --just` to create the canonical consumer integration.",
            ),
        );
    }
    if request.check && request.dry_run {
        return Err(CliError::usage(
            "`--check` and `--dry-run` cannot be combined",
        ));
    }

    let files = consumer_integration_files(root);
    let mut missing = Vec::new();
    let mut conflicts = Vec::new();
    let mut current = Vec::new();
    for file in &files {
        match fs::read_to_string(&file.path) {
            Ok(actual) if actual == file.contents => current.push(file.path.clone()),
            Ok(_) => conflicts.push(file.path.clone()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                missing.push(file.path.clone());
            }
            Err(error) => {
                return Err(CliError::config(format!(
                    "failed to inspect consumer integration file `{}`",
                    file.path.display()
                ))
                .with_source(error));
            }
        }
    }
    if !conflicts.is_empty() {
        return Err(CliError::config("consumer integration contains user-owned file conflicts")
            .with_code("CLI.SC_LINT_INTEGRATION_CONFLICT")
            .with_detail("conflicts", paths_to_json(&conflicts))
            .with_suggested_action(
                "Move or remove the conflicting file, then rerun `sc-lint init --just`; sc-lint will not overwrite it.",
            )
            .with_documentation("sc-lint docs setup"));
    }
    if request.check && !missing.is_empty() {
        return Err(CliError::config("consumer integration is not current")
            .with_code("CLI.SC_LINT_INTEGRATION_OUTDATED")
            .with_detail("missing", paths_to_json(&missing))
            .with_suggested_action("Run `sc-lint init --just` to create the missing managed files.")
            .with_documentation("sc-lint docs setup"));
    }
    if !request.check && !request.dry_run {
        for file in &files {
            if !file.path.exists() {
                if let Some(parent) = file.path.parent() {
                    fs::create_dir_all(parent).map_err(|error| {
                        CliError::config(format!(
                            "failed to create consumer integration directory `{}`",
                            parent.display()
                        ))
                        .with_source(error)
                    })?;
                }
                fs::write(&file.path, &file.contents).map_err(|error| {
                    CliError::config(format!(
                        "failed to write consumer integration file `{}`",
                        file.path.display()
                    ))
                    .with_source(error)
                })?;
                set_bootstrap_permissions(&file.path)?;
            }
        }
    }

    Ok(json!({
        "status": if missing.is_empty() { "current" } else if request.dry_run { "would_create" } else { "created" },
        "managed_files": files.iter().map(|file| file.path.display().to_string()).collect::<Vec<_>>(),
        "current_files": paths_to_strings(&current),
        "changed_files": paths_to_strings(&missing),
        "check": request.check,
        "dry_run": request.dry_run,
        "summary": "consumer Just integration is managed by sc-lint",
    }))
}

struct ConsumerIntegrationFile {
    path: PathBuf,
    contents: String,
}

fn consumer_integration_files(root: &Path) -> Vec<ConsumerIntegrationFile> {
    vec![
        ConsumerIntegrationFile {
            path: root.join(CONFIG_FILENAME),
            contents: canonical_consumer_config(),
        },
        ConsumerIntegrationFile {
            path: root.join("Justfile"),
            contents: format!("{GENERATED_FILE_HEADER}{CONSUMER_JUSTFILE_ASSET}"),
        },
        ConsumerIntegrationFile {
            path: root.join(".sc-lint/bootstrap"),
            contents: format!("{GENERATED_FILE_HEADER}{CONSUMER_BOOTSTRAP_ASSET}"),
        },
    ]
}

fn canonical_consumer_config() -> String {
    format!(
        "{GENERATED_FILE_HEADER}\n[tool.sc-lint]\nminimum_version = \"{}\"\n\n[[tool.sc-lint.lint]]\nname = \"fmt\"\ncommand = [\"cargo\", \"fmt\", \"--all\", \"--check\"]\n\n[[tool.sc-lint.lint]]\nname = \"clippy\"\ncommand = [\"cargo\", \"clippy\", \"--workspace\", \"--all-targets\", \"--\", \"-D\", \"warnings\"]\n\n[[tool.sc-lint.test]]\nname = \"workspace\"\ncommand = [\"cargo\", \"test\", \"--workspace\"]\n",
        env!("CARGO_PKG_VERSION")
    )
}

fn paths_to_strings(paths: &[PathBuf]) -> Vec<String> {
    paths
        .iter()
        .map(|path| path.display().to_string())
        .collect()
}

fn paths_to_json(paths: &[PathBuf]) -> Value {
    json!(paths_to_strings(paths))
}

#[expect(
    clippy::result_large_err,
    reason = "Generated bootstrap permission errors use the shared top-level CliError contract."
)]
fn set_bootstrap_permissions(path: &Path) -> Result<(), CliError> {
    if path.file_name().is_some_and(|name| name == "bootstrap") {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut permissions = fs::metadata(path)
                .map_err(|error| {
                    CliError::config("failed to inspect generated bootstrap").with_source(error)
                })?
                .permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(path, permissions).map_err(|error| {
                CliError::config("failed to make generated bootstrap executable").with_source(error)
            })?;
        }
    }
    Ok(())
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
