use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

use serde::Deserialize;
use serde_json::Value;
use serde_json::json;

use crate::Cli;
use crate::CliError;
use crate::command::CommandContext;
pub(crate) use crate::compatibility::CompatibilityErrorCode;
pub(crate) use crate::compatibility::CompatibilityRequirement;
pub use crate::compatibility::MinimumVersion;
pub(crate) use crate::compatibility::VERSION_PROBE_SCHEMA;
pub(crate) use crate::compatibility::VersionProbeEnvelope;
pub(crate) use crate::compatibility::compatibility_config_error;
pub(crate) use crate::compatibility::compatibility_requirement_from_file;
pub(crate) use crate::compatibility::compatibility_runtime_error;
pub(crate) use crate::compatibility::load_compatibility_requirement;
pub(crate) use crate::compatibility::missing_consumer_field;
pub(crate) use crate::compatibility::stderr_or_status;
pub(crate) use crate::compatibility::validate_version_probe;
pub(crate) use crate::consumer_integration::run_consumer_init;
#[cfg(test)]
pub(crate) use crate::consumer_integration::run_consumer_init_at;
use crate::error::ErrorCode;
pub(crate) use crate::profile_plan::ConsumerProfileStep;
use crate::profile_plan::LintPlanInput;
use crate::profile_plan::LintProfiles;
use crate::profile_plan::ProfileName;
use crate::profile_plan::decode_lint_step_fields;
use crate::profile_plan::profile_plan_config_error;
use crate::profile_plan::reject_test_step_fields;
use crate::profile_plan::validate_lint_step;

pub(crate) const CONFIG_FILENAME: &str = "sc-lint.toml";

/// Selector value that runs every configured consumer profile step in declaration order.
pub(crate) const CONSUMER_SELECTOR_ALL: &str = "all";

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
    explicit_config: Option<PathBuf>,
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct ConsumerRequirement {
    compatibility: CompatibilityRequirement,
    root: PathBuf,
    lint: Vec<ConsumerProfileStep>,
    lint_profiles: LintProfiles,
    test: Vec<ConsumerProfileStep>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ConfigCode {
    BoundariesDirMissing,
    TableMissing,
}

impl ConfigCode {
    pub(crate) fn from_wire(value: &str) -> Option<Self> {
        match value {
            "config.table_missing" => Some(Self::TableMissing),
            _ => None,
        }
    }
}

impl ErrorCode for ConfigCode {
    fn as_str(&self) -> &'static str {
        match self {
            Self::BoundariesDirMissing => "config.boundaries_dir_missing",
            Self::TableMissing => "config.table_missing",
        }
    }
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
            if current.join(CONFIG_FILENAME).is_file() || workspace_manifest(&current) {
                return Ok(Self(current));
            }
            let inspected = current.clone();
            if !current.pop() {
                return Err(CliError::config(format!(
                    "could not discover the sc-lint repo root from `{}`; last inspected `{}` was missing required sentinels sc-lint.toml or Cargo.toml [workspace]",
                    start.display(),
                    inspected.display()
                ))
                .with_detail("discovery_start", json!(start.display().to_string()))
                .with_detail("last_inspected", json!(inspected.display().to_string()))
                .with_detail("required_sentinels", json!(["sc-lint.toml", "Cargo.toml [workspace]"]))
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

fn workspace_manifest(path: &Path) -> bool {
    let manifest = path.join("Cargo.toml");
    manifest.is_file()
        && fs::read_to_string(manifest)
            .ok()
            .and_then(|contents| toml::from_str::<toml::Value>(&contents).ok())
            .and_then(|value| value.get("workspace").cloned())
            .is_some_and(|value| value.is_table())
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
                explicit_config: None,
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
        let explicit_config = cli
            .config
            .as_ref()
            .map(|path| resolve_repo_relative_path(repo_root.as_path(), path));
        let config_path = find_repo_config(repo_root.as_path(), explicit_config.as_deref());
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
            explicit_config,
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
    /// Resolves the consumer profile steps selected by `selector`.
    ///
    /// `None` and [`CONSUMER_SELECTOR_ALL`] select every configured step in
    /// declaration order; any other value selects the single step with that name.
    pub(crate) fn consumer_profile(
        &self,
        profile: ConsumerProfile,
        selector: Option<&str>,
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
        let Some(name) = selector.filter(|name| *name != CONSUMER_SELECTOR_ALL) else {
            return Ok((&requirement.root, steps));
        };
        let step =
            steps
                .iter()
                .find(|step| step.name() == name)
                .ok_or_else(|| {
                    let available = steps
                        .iter()
                        .map(ConsumerProfileStep::name)
                        .collect::<Vec<_>>();
                    consumer_profile_error(
                self.config_path.as_deref().unwrap_or_else(|| Path::new(CONFIG_FILENAME)),
                profile.as_str(),
                format!(
                    "consumer `{}` profile has no step named `{name}`; configured steps: {}",
                    profile.as_str(),
                    available.join(", ")
                ),
            )
            .with_detail("selector", json!(name))
            .with_detail("available", json!(available))
            .with_suggested_action(format!(
                "Pass one of the configured step names or `{CONSUMER_SELECTOR_ALL}`."
            ))
                })?;
        Ok((&requirement.root, std::slice::from_ref(step)))
    }

    /// Resolves a consumer lint selector to planner input.
    ///
    /// `None` and `all` select every declared step. Otherwise the selector names
    /// a step first, then a `[tool.sc-lint.lint_profiles]` profile (collisions
    /// are rejected at load, so a name is never both).
    #[expect(
        clippy::result_large_err,
        reason = "Consumer lint selection retains the shared top-level CliError contract."
    )]
    pub(crate) fn consumer_lint_inputs(
        &self,
        selector: Option<&str>,
    ) -> Result<LintPlanInput<'_>, CliError> {
        let LoadedConfigMode::Consumer(requirement) = &self.mode else {
            return Err(CliError::internal(
                "consumer profile execution requires a consumer configuration",
            ));
        };
        let steps = &requirement.lint;
        let root = requirement.root.as_path();
        let Some(name) = selector.filter(|name| *name != CONSUMER_SELECTOR_ALL) else {
            return Ok(LintPlanInput {
                root,
                steps: steps.clone(),
                selector: None,
            });
        };
        if steps.iter().any(|step| step.name() == name) {
            let selector = Some(ProfileName::parse(name.to_string())?);
            return Ok(LintPlanInput {
                root,
                steps: steps.clone(),
                selector,
            });
        }
        if let Some(members) = requirement.lint_profiles.resolve(name, steps) {
            return Ok(LintPlanInput {
                root,
                steps: members,
                selector: None,
            });
        }
        let mut available: Vec<&str> = steps.iter().map(ConsumerProfileStep::name).collect();
        available.extend(requirement.lint_profiles.names());
        Err(consumer_profile_error(
            self.config_path
                .as_deref()
                .unwrap_or_else(|| Path::new(CONFIG_FILENAME)),
            ConsumerProfile::Lint.as_str(),
            format!(
                "consumer `lint` profile has no step or profile named `{name}`; configured: {}",
                available.join(", ")
            ),
        )
        .with_detail("selector", json!(name))
        .with_detail("available", json!(available))
        .with_suggested_action(format!(
            "Pass one of the configured step or profile names or `{CONSUMER_SELECTOR_ALL}`."
        )))
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
            explicit_config: None,
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
        let lint_profiles = LintProfiles::parse(&sc_lint.lint_profiles, &lint)
            .map_err(|error| profile_plan_config_error(error, &config_path))?;
        let test = validate_consumer_profile("test", &sc_lint.test, &config_path)?;
        let root = config_path
            .parent()
            .ok_or_else(|| CliError::internal("consumer configuration has no parent directory"))?
            .to_path_buf();

        Ok(Self {
            repo_root: None,
            config_path: Some(config_path),
            explicit_config: None,
            logging_root: cli.log_root.clone(),
            logging_console: cli.log_console,
            mode: LoadedConfigMode::Consumer(ConsumerRequirement {
                compatibility,
                root,
                lint,
                lint_profiles,
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

    pub(crate) fn explicit_config(&self) -> Option<&Path> {
        self.explicit_config.as_deref()
    }

    #[expect(
        clippy::result_large_err,
        reason = "Boundary configuration failures are part of the stable top-level CliError contract."
    )]
    pub(crate) fn require_boundaries_dir(&self) -> Result<(), CliError> {
        let root = self.require_repo_root()?;
        if root.join("boundaries").is_dir() {
            return Ok(());
        }
        Err(
            CliError::config("the boundary family requires a boundaries/ directory")
                .with_code(ConfigCode::BoundariesDirMissing.as_str())
                .with_cause(format!(
                    "no boundaries/directory under repo root{}",
                    root.display()
                ))
                .with_detail("repo_root", json!(root.display().to_string()))
                .with_suggested_action("create boundaries/ or do not run the boundary family"),
        )
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
pub(crate) struct RepoConfigFile {
    logging: Option<LoggingConfigFile>,
    pub(crate) tool: Option<ToolConfigFile>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub(crate) struct ToolConfigFile {
    #[serde(rename = "sc-lint")]
    pub(crate) sc_lint: Option<ScLintConfigFile>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub(crate) struct ScLintConfigFile {
    pub(crate) minimum_version: Option<String>,
    #[serde(default)]
    lint: Vec<ConsumerProfileStepFile>,
    /// Named lint profiles: an ordered list of declared lint step names.
    #[serde(default)]
    lint_profiles: std::collections::BTreeMap<String, Vec<String>>,
    #[serde(default)]
    test: Vec<ConsumerProfileStepFile>,
}

/// One raw `[[tool.sc-lint.lint|test]]` row. `platform`, `target` and any other
/// key stay raw so Rust alone decides their meaning (see `profile_plan`).
#[derive(Debug, Clone, Deserialize)]
struct ConsumerProfileStepFile {
    name: Option<String>,
    command: Option<Vec<String>>,
    platform: Option<toml::Value>,
    target: Option<toml::Value>,
    #[serde(flatten)]
    extra: std::collections::BTreeMap<String, toml::Value>,
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
    Some(repo_root.join(CONFIG_FILENAME)).filter(|path| path.exists())
}

#[expect(
    clippy::result_large_err,
    reason = "Repo config parse failures are part of the stable top-level CliError contract."
)]
pub(crate) fn parse_repo_config(path: &Path) -> Result<RepoConfigFile, CliError> {
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
        let (platform, target) = profile_step_fields(profile, name, step, path)?;
        let declared = ConsumerProfileStep::new(name, command.clone(), platform, target);
        if profile == ConsumerProfile::Lint.as_str() {
            validate_lint_step(&declared)
                .map_err(|error| profile_plan_config_error(error, path))?;
        }
        steps.push(declared);
    }
    Ok(steps)
}

#[expect(
    clippy::result_large_err,
    reason = "Profile field validation preserves structured CLI configuration errors."
)]
fn profile_step_fields(
    profile: &str,
    name: &str,
    step: &ConsumerProfileStepFile,
    path: &Path,
) -> Result<
    (
        Option<crate::profile_plan::Platform>,
        Option<crate::profile_plan::Target>,
    ),
    CliError,
> {
    match profile {
        "lint" => decode_lint_step_fields(
            name,
            step.platform.as_ref(),
            step.target.as_ref(),
            &step.extra,
        )
        .map_err(|error| profile_plan_config_error(error, path)),
        "test" => {
            reject_test_step_fields(
                name,
                step.platform.as_ref(),
                step.target.as_ref(),
                &step.extra,
            )
            .map_err(|error| profile_plan_config_error(error, path))?;
            Ok((None, None))
        }
        _ => unreachable!("consumer profiles are lint or test"),
    }
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
