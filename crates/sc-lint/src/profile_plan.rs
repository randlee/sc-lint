//! Consumer lint-profile semantics: validated names, platform/target
//! declarations, ordered lint profiles, and the pure selection planner.
//!
//! Rust is the only place that decides what a consumer's `[[tool.sc-lint.lint]]`
//! rows and `[tool.sc-lint.lint_profiles]` mean (ADR-017 change 4). The installer
//! only checks JSON shape. This module owns three things:
//!
//! - validation of step `platform`/`target` fields and of profile membership,
//!   reported as [`ProfilePlanError`] with a stable detail code;
//! - [`plan_consumer_lint`], a pure function from declared steps, a selector, a
//!   [`PlatformScope`] and an injected host [`Platform`] to ordered
//!   [`PlanDecision`]s (run a planned command, or skip with a reason);
//! - the execution matrix that turns a platform-tagged cargo check/clippy step
//!   into the host-appropriate argv (plain cargo, or `cargo xwin` for
//!   Linux/macOS to `x86_64-pc-windows-msvc`).
//!
//! Nothing here spawns a process; `workflow` executes the decisions.
#![expect(
    clippy::result_large_err,
    reason = "ProfilePlanError carries the full structured error contract (code, detail code, step, field, cause, recovery, docs) by value."
)]

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::ffi::OsString;
use std::fmt;
use std::path::Path;

use serde_json::json;

use crate::CliError;
use crate::config::CONSUMER_SELECTOR_ALL;
use crate::scheduler::ProfileCode;
use crate::workflow::WINDOWS_XWIN_TARGET;

/// Decodes an optional `after_all` value of a lint or test row.
///
/// # Errors
/// Returns `profile.invalid_after_all` for a non-boolean value.
#[expect(
    clippy::result_large_err,
    reason = "Config validation stays within the top-level CliError contract."
)]
pub(crate) fn decode_after_all(
    value: Option<&toml::Value>,
    step: &str,
    config_path: &Path,
) -> Result<bool, CliError> {
    match value {
        None => Ok(false),
        Some(toml::Value::Boolean(after_all)) => Ok(*after_all),
        Some(_) => Err(ProfileCode::InvalidAfterAll.config_error(
            format!(
                "step '{step}' in {} declares a non-boolean after_all",
                config_path.display()
            ),
            config_path,
        )),
    }
}

/// Stable top-level code for every invalid consumer profile declaration.
pub const PROFILE_PLAN_ERROR_CODE: &str = "CLI.SC_LINT_PROFILE_CONFIG_INVALID";

/// Documentation reference attached to profile configuration errors.
const PROFILE_DOCS_REFERENCE: &str = "sc-lint docs configuration";

/// Machine-readable reason a structured profile error was raised.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfilePlanDetailCode {
    /// A named profile lists no steps.
    EmptyMembers,
    /// A named profile lists the same step twice.
    DuplicateMember,
    /// A selector or profile member names no declared step.
    UnknownMember,
    /// A profile name is empty, reserved, or equals a step name.
    AmbiguousName,
    /// A lint row has a key other than `name`, `command`, `platform`, `target`,
    /// or an unusable `platform` value.
    UnknownStepField,
    /// A step declares `target` without `platform`.
    TargetRequiresPlatform,
    /// A step's `target` is malformed or names a different OS than `platform`.
    TargetOsMismatch,
    /// A step with `target` is not a literal `cargo check|clippy` command.
    UnsupportedCommandShape,
}

impl ProfilePlanDetailCode {
    /// Returns the stable `PROFILE.*` string for this detail code.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::EmptyMembers => "PROFILE.EMPTY_MEMBERS",
            Self::DuplicateMember => "PROFILE.DUPLICATE_MEMBER",
            Self::UnknownMember => "PROFILE.UNKNOWN_MEMBER",
            Self::AmbiguousName => "PROFILE.AMBIGUOUS_NAME",
            Self::UnknownStepField => "PROFILE.UNKNOWN_STEP_FIELD",
            Self::TargetRequiresPlatform => "PROFILE.TARGET_REQUIRES_PLATFORM",
            Self::TargetOsMismatch => "PROFILE.TARGET_OS_MISMATCH",
            Self::UnsupportedCommandShape => "PROFILE.UNSUPPORTED_COMMAND_SHAPE",
        }
    }
}

/// Structured invalid-profile error; converts to a [`CliError`] without loss.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfilePlanError {
    /// Always [`PROFILE_PLAN_ERROR_CODE`].
    pub code: &'static str,
    /// The single reason this declaration was rejected.
    pub detail_code: ProfilePlanDetailCode,
    /// The step or profile the error is about, when one applies.
    pub step: Option<String>,
    /// The offending configuration field, when one applies.
    pub field: Option<String>,
    /// Why the declaration is invalid.
    pub cause: String,
    /// What the operator should change.
    pub recovery: String,
    /// Documentation reference, when one exists.
    pub docs: Option<String>,
}

impl ProfilePlanError {
    fn new(
        detail_code: ProfilePlanDetailCode,
        step: Option<&str>,
        field: Option<&str>,
        cause: impl Into<String>,
        recovery: impl Into<String>,
    ) -> Self {
        Self {
            code: PROFILE_PLAN_ERROR_CODE,
            detail_code,
            step: step.map(str::to_string),
            field: field.map(str::to_string),
            cause: cause.into(),
            recovery: recovery.into(),
            docs: Some(PROFILE_DOCS_REFERENCE.to_string()),
        }
    }
}

impl fmt::Display for ProfilePlanError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} ({}): {}",
            self.code,
            self.detail_code.as_str(),
            self.cause
        )
    }
}

impl std::error::Error for ProfilePlanError {}

impl From<ProfilePlanError> for CliError {
    fn from(error: ProfilePlanError) -> Self {
        let mut converted = Self::config(format!(
            "consumer lint profile configuration is invalid: {}",
            error.cause
        ))
        .with_code(error.code)
        .with_detail("detail_code", json!(error.detail_code.as_str()))
        .with_cause(error.cause)
        .with_suggested_action(error.recovery);
        if let Some(step) = error.step {
            converted = converted.with_detail("step", json!(step));
        }
        if let Some(field) = error.field {
            converted = converted.with_detail("field", json!(field));
        }
        if let Some(docs) = error.docs {
            converted = converted.with_documentation(docs);
        }
        converted
    }
}

/// A validated, non-empty semantic profile or selector name.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProfileName(String);

impl ProfileName {
    /// Parses a profile or selector name.
    ///
    /// # Errors
    /// Returns [`ProfilePlanDetailCode::AmbiguousName`] when the name is empty
    /// or only whitespace, because it could not select anything.
    pub fn parse(value: String) -> Result<Self, ProfilePlanError> {
        if value.trim().is_empty() {
            return Err(ProfilePlanError::new(
                ProfilePlanDetailCode::AmbiguousName,
                None,
                Some("name"),
                "a lint profile or selector name must not be empty",
                "Use a non-empty step or profile name.",
            ));
        }
        Ok(Self(value))
    }

    /// Returns the name as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ProfileName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// An operating-system family a lint step is intended for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Platform {
    /// Linux hosts and `*-linux-*` targets.
    Linux,
    /// macOS hosts and `*-apple-darwin` targets.
    MacOS,
    /// Windows hosts and `*-windows-*` targets.
    Windows,
}

impl Platform {
    /// Every platform, in the order the planner matrix is exercised.
    pub const ALL: [Self; 3] = [Self::Linux, Self::MacOS, Self::Windows];

    /// The platform this binary was compiled for. Other Unix hosts plan as Linux.
    pub const fn host() -> Self {
        if cfg!(windows) {
            Self::Windows
        } else if cfg!(target_os = "macos") {
            Self::MacOS
        } else {
            Self::Linux
        }
    }

    /// Returns the lowercase configuration and JSON spelling.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Linux => "linux",
            Self::MacOS => "macos",
            Self::Windows => "windows",
        }
    }

    /// Parses the exact lowercase configuration spelling.
    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|platform| platform.as_str() == value)
    }
}

impl fmt::Display for Platform {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Which declared steps a consumer lint run selects beyond the unannotated ones.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlatformScope {
    /// Unannotated steps plus steps tagged for the host (the default).
    Host,
    /// Every declared step; unsupported cross combinations are reported as skips.
    All,
    /// Host and unannotated steps plus steps tagged for the named platform.
    Named(Platform),
}

/// A validated Rust target triple such as `x86_64-pc-windows-msvc`.
///
/// Validation is syntactic: this type never claims a triple is installed or
/// known to rustc. A missing target fails at execution time, never here.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Target(String);

impl Target {
    /// Parses a target triple.
    ///
    /// # Errors
    /// Returns [`ProfilePlanDetailCode::TargetOsMismatch`] for a value that is
    /// not a lowercase, hyphen-separated triple with at least two components.
    pub fn parse(value: String) -> Result<Self, ProfilePlanError> {
        let well_formed = value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"_.-".contains(&byte)
        }) && value.split('-').count() >= 2
            && value.split('-').all(|part| !part.is_empty())
            && value.starts_with(|first: char| first.is_ascii_lowercase());
        if !well_formed {
            return Err(ProfilePlanError::new(
                ProfilePlanDetailCode::TargetOsMismatch,
                None,
                Some("target"),
                format!("`{value}` is not a valid Rust target triple"),
                "Declare a target such as `x86_64-pc-windows-msvc`.",
            ));
        }
        Ok(Self(value))
    }

    /// Returns the triple as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The OS family the triple names, or `None` for an OS sc-lint does not plan.
    pub fn os(&self) -> Option<Platform> {
        if self.0.contains("windows") {
            Some(Platform::Windows)
        } else if self.0.contains("apple-darwin") {
            Some(Platform::MacOS)
        } else if self.0.contains("linux") {
            Some(Platform::Linux)
        } else {
            None
        }
    }
}

impl fmt::Display for Target {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// The cargo verb of a literal `cargo check` or `cargo clippy` command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CargoVerb {
    /// `cargo check`
    Check,
    /// `cargo clippy`
    Clippy,
}

impl CargoVerb {
    /// Derives the verb only from a literal `cargo check|clippy ...` argv.
    ///
    /// Wrappers, toolchain overrides (`cargo +nightly`) and other verbs yield
    /// `None`.
    pub fn from_argv(argv: &[String]) -> Option<Self> {
        match argv {
            [program, verb, ..] if program == "cargo" => match verb.as_str() {
                "check" => Some(Self::Check),
                "clippy" => Some(Self::Clippy),
                _ => None,
            },
            _ => None,
        }
    }

    /// Returns the cargo subcommand spelling.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Check => "check",
            Self::Clippy => "clippy",
        }
    }
}

/// A command the planner decided to run on this host.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedStep {
    name: String,
    command: OsString,
    args: Vec<OsString>,
    platform: Option<Platform>,
    target: Option<Target>,
    cargo_verb: Option<CargoVerb>,
    after_all: bool,
}

impl PlannedStep {
    /// Step name as declared.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Program to execute.
    pub fn command(&self) -> &OsString {
        &self.command
    }

    /// Arguments passed to the program, already adapted for the host.
    pub fn args(&self) -> &[OsString] {
        &self.args
    }

    /// Declared platform tag, if any.
    pub const fn platform(&self) -> Option<Platform> {
        self.platform
    }

    /// Declared target triple, if any.
    pub const fn target(&self) -> Option<&Target> {
        self.target.as_ref()
    }

    /// Cargo verb of the declared literal cargo check/clippy argv, if any.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "Planner API accessor; production execution consumes the already-adapted argv."
        )
    )]
    pub const fn cargo_verb(&self) -> Option<CargoVerb> {
        self.cargo_verb
    }

    /// Whether the step starts only after every other step has finished.
    pub const fn after_all(&self) -> bool {
        self.after_all
    }

    /// Whether this step runs through the `cargo xwin` subcommand.
    pub fn uses_cargo_xwin(&self) -> bool {
        self.command == "cargo" && self.args.first().is_some_and(|arg| arg == "xwin")
    }
}

/// Why a selected step is reported instead of run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkipReason {
    /// A platform-tagged command without a target was selected on another host.
    UnsupportedCrossCommand,
    /// A targeted cargo step has no supported toolchain path from this host.
    UnsupportedCrossTarget,
}

impl SkipReason {
    /// Returns the stable JSON enum value.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::UnsupportedCrossCommand => "unsupported_cross_command",
            Self::UnsupportedCrossTarget => "unsupported_cross_target",
        }
    }
}

/// A selected step that was skipped on this host, with the evidence for why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkipRecord {
    /// Step name as declared.
    pub name: String,
    /// The host the plan was made for.
    pub host: Platform,
    /// The step's declared platform.
    pub platform: Platform,
    /// The step's declared target, when it has one.
    pub target: Option<Target>,
    /// Why the step was skipped.
    pub reason: SkipReason,
}

/// One ordered planning outcome for a selected step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlanDecision {
    /// Execute this command.
    Run(PlannedStep),
    /// Report this step as skipped.
    Skip(SkipRecord),
}

fn invalid_shape(step: &str, cause: String, recovery: &str) -> ProfilePlanError {
    ProfilePlanError::new(
        ProfilePlanDetailCode::UnsupportedCommandShape,
        Some(step),
        Some("command"),
        cause,
        recovery,
    )
}

/// True when `argv` embeds its own `--target` flag.
fn embeds_target(argv: &[String]) -> bool {
    argv.iter()
        .any(|argument| argument == "--target" || argument.starts_with("--target="))
}

/// Validates the `platform`/`target` declaration of one lint step.
///
/// # Errors
/// Returns the first violated rule: `TARGET_REQUIRES_PLATFORM`,
/// `TARGET_OS_MISMATCH`, or `UNSUPPORTED_COMMAND_SHAPE` (a target needs a
/// literal `cargo check|clippy` command with no embedded `--target`).
pub(crate) fn validate_lint_step(step: &ConsumerProfileStep) -> Result<(), ProfilePlanError> {
    let Some(target) = step.target() else {
        return Ok(());
    };
    let name = step.name();
    let Some(platform) = step.platform() else {
        return Err(ProfilePlanError::new(
            ProfilePlanDetailCode::TargetRequiresPlatform,
            Some(name),
            Some("target"),
            format!("step `{name}` declares target `{target}` without a `platform`"),
            "Add `platform = \"linux\" | \"macos\" | \"windows\"` matching the target.",
        ));
    };
    if target.os() != Some(platform) {
        return Err(ProfilePlanError::new(
            ProfilePlanDetailCode::TargetOsMismatch,
            Some(name),
            Some("target"),
            format!("step `{name}` target `{target}` does not name a {platform} target"),
            "Use a target triple whose OS matches `platform`.",
        ));
    }
    if CargoVerb::from_argv(step.command()).is_none() {
        return Err(invalid_shape(
            name,
            format!(
                "step `{name}` declares a target but its command is not a literal `cargo check` or `cargo clippy`"
            ),
            "Targets apply only to `[\"cargo\", \"check\"|\"clippy\", ...]`; remove the target or use that command.",
        ));
    }
    if embeds_target(step.command()) {
        return Err(invalid_shape(
            name,
            format!("step `{name}` command already contains `--target`"),
            "Remove `--target` from the command; the `target` field supplies it.",
        ));
    }
    Ok(())
}

/// Decodes the optional `platform`/`target` values of one lint row after an
/// exact-key and value-shape check.
///
/// # Errors
/// Returns `UNKNOWN_STEP_FIELD` for any key outside `name`, `command`,
/// `platform`, `target`, `after_all` (decoded by the loader), or a `platform` that is not `linux|macos|windows`;
/// returns `TARGET_OS_MISMATCH` for a malformed `target` triple.
pub(crate) fn decode_lint_step_fields(
    step: &str,
    platform: Option<&toml::Value>,
    target: Option<&toml::Value>,
    extra_keys: &BTreeMap<String, toml::Value>,
) -> Result<(Option<Platform>, Option<Target>), ProfilePlanError> {
    if let Some(key) = extra_keys.keys().next() {
        return Err(ProfilePlanError::new(
            ProfilePlanDetailCode::UnknownStepField,
            Some(step),
            Some(key),
            format!("lint step `{step}` has unknown field `{key}`"),
            "Lint steps accept only `name`, `command`, `platform`, `target` and `after_all`.",
        ));
    }
    let platform = platform
        .map(|value| {
            value.as_str().and_then(Platform::parse).ok_or_else(|| {
                ProfilePlanError::new(
                    ProfilePlanDetailCode::UnknownStepField,
                    Some(step),
                    Some("platform"),
                    format!("lint step `{step}` has an unsupported `platform` value `{value}`"),
                    "Use `platform = \"linux\"`, `\"macos\"` or `\"windows\"`.",
                )
            })
        })
        .transpose()?;
    let target = target
        .map(|value| {
            let text = value.as_str().map(str::to_string).ok_or_else(|| {
                ProfilePlanError::new(
                    ProfilePlanDetailCode::TargetOsMismatch,
                    Some(step),
                    Some("target"),
                    format!("lint step `{step}` has a non-string `target` value `{value}`"),
                    "Declare the target as a triple string such as `x86_64-pc-windows-msvc`.",
                )
            })?;
            Target::parse(text).map_err(|mut error| {
                error.step = Some(step.to_string());
                error
            })
        })
        .transpose()?;
    Ok((platform, target))
}

/// Rejects profile-only fields on a consumer test row.
///
/// # Errors
/// Returns `UNKNOWN_STEP_FIELD` because test rows support only `name` and
/// `command`; platform profile selection applies exclusively to lint rows.
pub(crate) fn reject_test_step_fields(
    step: &str,
    platform: Option<&toml::Value>,
    target: Option<&toml::Value>,
    extra_keys: &BTreeMap<String, toml::Value>,
) -> Result<(), ProfilePlanError> {
    let field = platform
        .is_some()
        .then_some("platform")
        .or_else(|| target.is_some().then_some("target"))
        .or_else(|| extra_keys.keys().next().map(String::as_str));
    let Some(field) = field else {
        return Ok(());
    };
    Err(ProfilePlanError::new(
        ProfilePlanDetailCode::UnknownStepField,
        Some(step),
        Some(field),
        format!("test step `{step}` has unsupported field `{field}`"),
        "Test steps accept only `name` and `command`; move platform selection to a lint step.",
    ))
}

/// Validated ordered lint profiles: a profile name maps to existing step names.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct LintProfiles(BTreeMap<ProfileName, Vec<String>>);

impl LintProfiles {
    /// Validates `[tool.sc-lint.lint_profiles]` against the declared lint steps.
    ///
    /// # Errors
    /// Rejects an empty profile name or one equal to `all` or a step name
    /// (`AMBIGUOUS_NAME`), an empty list (`EMPTY_MEMBERS`), a repeated member
    /// (`DUPLICATE_MEMBER`) and a member that is not a declared step
    /// (`UNKNOWN_MEMBER`). Profiles are not nested: members are step names only.
    pub(crate) fn parse(
        raw: &BTreeMap<String, Vec<String>>,
        steps: &[ConsumerProfileStep],
    ) -> Result<Self, ProfilePlanError> {
        let declared: BTreeSet<&str> = steps.iter().map(ConsumerProfileStep::name).collect();
        let mut profiles = BTreeMap::new();
        for (raw_name, members) in raw {
            let name = ProfileName::parse(raw_name.clone())?;
            validate_profile_name(&name, &declared)?;
            validate_members(&name, members, &declared)?;
            profiles.insert(name, members.clone());
        }
        Ok(Self(profiles))
    }

    /// Declared profile names in sorted order.
    pub(crate) fn names(&self) -> Vec<&str> {
        self.0.keys().map(ProfileName::as_str).collect()
    }

    /// Expands a profile into its member steps in profile order.
    pub(crate) fn resolve(
        &self,
        name: &str,
        steps: &[ConsumerProfileStep],
    ) -> Option<Vec<ConsumerProfileStep>> {
        let members = self.0.iter().find(|(key, _)| key.as_str() == name)?.1;
        Some(
            members
                .iter()
                .filter_map(|member| steps.iter().find(|step| step.name() == member))
                .cloned()
                .collect(),
        )
    }
}

fn validate_profile_name(
    name: &ProfileName,
    declared: &BTreeSet<&str>,
) -> Result<(), ProfilePlanError> {
    if name.as_str() == CONSUMER_SELECTOR_ALL || declared.contains(name.as_str()) {
        return Err(ProfilePlanError::new(
            ProfilePlanDetailCode::AmbiguousName,
            Some(name.as_str()),
            Some("lint_profiles"),
            format!(
                "profile `{name}` collides with {}",
                if name.as_str() == CONSUMER_SELECTOR_ALL {
                    "the reserved selector `all`"
                } else {
                    "a lint step of the same name"
                }
            ),
            "Rename the profile so it differs from `all` and from every step name.",
        ));
    }
    Ok(())
}

fn validate_members(
    name: &ProfileName,
    members: &[String],
    declared: &BTreeSet<&str>,
) -> Result<(), ProfilePlanError> {
    if members.is_empty() {
        return Err(ProfilePlanError::new(
            ProfilePlanDetailCode::EmptyMembers,
            Some(name.as_str()),
            Some("lint_profiles"),
            format!("profile `{name}` lists no steps"),
            "List at least one declared lint step name.",
        ));
    }
    let mut seen = BTreeSet::new();
    for member in members {
        if !declared.contains(member.as_str()) {
            return Err(ProfilePlanError::new(
                ProfilePlanDetailCode::UnknownMember,
                Some(name.as_str()),
                Some(member),
                format!("profile `{name}` lists `{member}`, which is not a declared lint step"),
                "List only names declared by `[[tool.sc-lint.lint]]` rows.",
            ));
        }
        if !seen.insert(member.as_str()) {
            return Err(ProfilePlanError::new(
                ProfilePlanDetailCode::DuplicateMember,
                Some(name.as_str()),
                Some(member),
                format!("profile `{name}` lists `{member}` more than once"),
                "List each step at most once per profile.",
            ));
        }
    }
    Ok(())
}

/// Plans one consumer lint run: membership first, then platform scope.
///
/// `selector` of `None` or `all` selects every declared step; any other value
/// selects the single step with that name (profile names are expanded by the
/// configuration loader before planning). Declaration order is preserved and a
/// step is never selected twice. Under `scope`, unannotated steps and steps
/// tagged for `host` are always included, [`PlatformScope::All`] adds every
/// declaration, and [`PlatformScope::Named`] adds that platform's steps; other
/// steps are omitted without a report. Each selected step then follows the
/// execution matrix: same-host targets use plain cargo with `--target`;
/// Linux/macOS to `x86_64-pc-windows-msvc` uses `cargo xwin`; other combinations
/// become [`PlanDecision::Skip`].
///
/// # Errors
/// Returns a [`ProfilePlanError`] for duplicate step names, an unknown selector,
/// or a step whose platform/target declaration is invalid.
///
/// # Examples
/// ```ignore
/// let plan = plan_consumer_lint(&steps, None, PlatformScope::Host, Platform::Linux)?;
/// ```
pub fn plan_consumer_lint(
    steps: &[ConsumerProfileStep],
    selector: Option<&ProfileName>,
    scope: PlatformScope,
    host: Platform,
) -> Result<Vec<PlanDecision>, ProfilePlanError> {
    let members = select_members(steps, selector)?;
    let mut decisions = Vec::with_capacity(members.len());
    for step in members {
        validate_lint_step(step)?;
        if scope_selects(step.platform(), scope, host) {
            decisions.push(decide(step, host)?);
        }
    }
    Ok(decisions)
}

fn select_members<'a>(
    steps: &'a [ConsumerProfileStep],
    selector: Option<&ProfileName>,
) -> Result<Vec<&'a ConsumerProfileStep>, ProfilePlanError> {
    let mut seen = BTreeSet::new();
    for step in steps {
        if !seen.insert(step.name()) {
            return Err(ProfilePlanError::new(
                ProfilePlanDetailCode::DuplicateMember,
                Some(step.name()),
                Some("name"),
                format!("step `{}` is declared more than once", step.name()),
                "Give every lint step a unique name.",
            ));
        }
    }
    let Some(name) = selector.filter(|name| name.as_str() != CONSUMER_SELECTOR_ALL) else {
        return Ok(steps.iter().collect());
    };
    steps
        .iter()
        .find(|step| step.name() == name.as_str())
        .map(|step| vec![step])
        .ok_or_else(|| {
            ProfilePlanError::new(
                ProfilePlanDetailCode::UnknownMember,
                Some(name.as_str()),
                Some("selector"),
                format!("selector `{name}` names no declared lint step"),
                format!("Select `{CONSUMER_SELECTOR_ALL}` or one of the declared step names."),
            )
        })
}

fn scope_selects(platform: Option<Platform>, scope: PlatformScope, host: Platform) -> bool {
    match (platform, scope) {
        (None, _) | (_, PlatformScope::All) => true,
        (Some(tag), PlatformScope::Host) => tag == host,
        (Some(tag), PlatformScope::Named(named)) => tag == host || tag == named,
    }
}

fn decide(step: &ConsumerProfileStep, host: Platform) -> Result<PlanDecision, ProfilePlanError> {
    let Some(platform) = step.platform() else {
        return Ok(PlanDecision::Run(planned(step, None, false)?));
    };
    let skip = |reason| {
        PlanDecision::Skip(SkipRecord {
            name: step.name().to_string(),
            host,
            platform,
            target: step.target().cloned(),
            reason,
        })
    };
    match step.target() {
        None if platform == host => Ok(PlanDecision::Run(planned(step, None, false)?)),
        None => Ok(skip(SkipReason::UnsupportedCrossCommand)),
        Some(_) if platform == host => Ok(PlanDecision::Run(planned(step, step.target(), false)?)),
        Some(target) if uses_xwin(platform, target, host) => {
            Ok(PlanDecision::Run(planned(step, Some(target), true)?))
        }
        Some(_) => Ok(skip(SkipReason::UnsupportedCrossTarget)),
    }
}

/// Linux and macOS reach exactly the x86_64 Windows MSVC target through xwin.
fn uses_xwin(platform: Platform, target: &Target, host: Platform) -> bool {
    platform == Platform::Windows
        && matches!(host, Platform::Linux | Platform::MacOS)
        && target.as_str() == WINDOWS_XWIN_TARGET
}

fn planned(
    step: &ConsumerProfileStep,
    target: Option<&Target>,
    xwin: bool,
) -> Result<PlannedStep, ProfilePlanError> {
    let (command, args) = match target {
        Some(target) => targeted_argv(step, target, xwin)?,
        None => {
            let (program, rest) = step.command().split_first().ok_or_else(|| {
                invalid_shape(
                    step.name(),
                    format!("step `{}` has an empty command", step.name()),
                    "Give the step a non-empty argv array.",
                )
            })?;
            (
                OsString::from(program),
                rest.iter().map(OsString::from).collect(),
            )
        }
    };
    Ok(PlannedStep {
        name: step.name().to_string(),
        command,
        args,
        platform: step.platform(),
        target: step.target().cloned(),
        cargo_verb: CargoVerb::from_argv(step.command()),
        after_all: step.after_all(),
    })
}

/// Builds `cargo [xwin] <verb> <declared args> --target <triple> [-- <rest>]`.
///
/// Arguments after the verb are preserved literally; `--target` is inserted
/// before a `--` separator so it stays a cargo flag.
fn targeted_argv(
    step: &ConsumerProfileStep,
    target: &Target,
    xwin: bool,
) -> Result<(OsString, Vec<OsString>), ProfilePlanError> {
    let (Some(verb), Some(tail)) = (
        CargoVerb::from_argv(step.command()),
        step.command().get(2..),
    ) else {
        return Err(invalid_shape(
            step.name(),
            format!(
                "step `{}` has no literal cargo check/clippy verb",
                step.name()
            ),
            "Use a literal `cargo check` or `cargo clippy` command.",
        ));
    };
    let separator = tail
        .iter()
        .position(|argument| argument == "--")
        .unwrap_or(tail.len());
    let mut args: Vec<OsString> = Vec::with_capacity(tail.len() + 3);
    if xwin {
        args.push("xwin".into());
    }
    args.push(verb.as_str().into());
    args.extend(tail[..separator].iter().map(OsString::from));
    args.push("--target".into());
    args.push(target.as_str().into());
    args.extend(tail[separator..].iter().map(OsString::from));
    Ok(("cargo".into(), args))
}

/// A command is decoded and validated at the TOML boundary, so profile
/// orchestration never needs to split shell strings or infer consumer policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ConsumerProfileStep {
    name: String,
    command: Vec<String>,
    platform: Option<Platform>,
    target: Option<Target>,
    after_all: bool,
}

impl ConsumerProfileStep {
    /// Builds a step from already-decoded fields; semantic checks live in
    /// [`validate_lint_step`].
    pub(crate) fn new(
        name: impl Into<String>,
        command: Vec<String>,
        platform: Option<Platform>,
        target: Option<Target>,
    ) -> Self {
        Self {
            name: name.into(),
            command,
            platform,
            target,
            after_all: false,
        }
    }

    /// Sets the decoded `after_all` flag of the row.
    pub(crate) const fn with_after_all(mut self, after_all: bool) -> Self {
        self.after_all = after_all;
        self
    }

    pub(crate) const fn after_all(&self) -> bool {
        self.after_all
    }

    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn command(&self) -> &[String] {
        &self.command
    }

    pub(crate) const fn platform(&self) -> Option<Platform> {
        self.platform
    }

    pub(crate) const fn target(&self) -> Option<&Target> {
        self.target.as_ref()
    }
}

/// Everything the planner needs for one consumer lint selection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LintPlanInput<'a> {
    pub(crate) root: &'a Path,
    /// Declared steps, or the ordered members of a selected named profile.
    pub(crate) steps: Vec<ConsumerProfileStep>,
    /// A single-step selector for the planner; `None` selects every `steps` entry.
    pub(crate) selector: Option<ProfileName>,
}

pub(crate) fn profile_plan_config_error(error: ProfilePlanError, path: &Path) -> CliError {
    CliError::from(error).with_detail("config_path", json!(path.display().to_string()))
}
