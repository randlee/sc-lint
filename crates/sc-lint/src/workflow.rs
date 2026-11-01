use std::ffi::OsStr;
use std::ffi::OsString;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command as ProcessCommand;

use serde_json::Value;
use serde_json::json;

use crate::CheckTarget;
use crate::CliError;
use crate::ClippyTarget;
use crate::cli::LintProfile;
use crate::command::CommandSuccess;
use crate::config::CONSUMER_SELECTOR_ALL;
use crate::config::ConsumerProfile;
use crate::config::LoadedConfig;
use crate::consts;
use crate::profile_plan::PlanDecision;
use crate::profile_plan::PlannedStep;
use crate::profile_plan::Platform;
use crate::profile_plan::PlatformScope;
use crate::profile_plan::SkipRecord;
use crate::profile_plan::plan_consumer_lint;
use crate::scheduler::FailurePolicy;
use crate::scheduler::ScheduledStep;
use crate::scheduler::run_steps_scheduled;
use crate::test_selectors::TestRequest;

pub const WINDOWS_XWIN_TARGET: &str = "x86_64-pc-windows-msvc";

/// Execution lane of a planned step.
///
/// Built-in source lint steps declare `Cargo` (needs the shared cargo build
/// lane) or `Parallel` (independent of it) where they are constructed.
/// Consumer-configured steps are classified by [`Lane::for_program`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Lane {
    Cargo,
    Parallel,
}

impl Lane {
    /// `Cargo` iff the file stem of `program` is `cargo` (ASCII
    /// case-insensitive: `cargo`, `/usr/bin/cargo`, `cargo.exe`), else
    /// `Parallel`. A scheduling hint only: cargo serializes itself on its
    /// build-directory lock, so a wrapper that runs cargo is merely not
    /// pre-serialized.
    pub(crate) fn for_program(program: &OsStr) -> Self {
        let is_cargo = Path::new(program)
            .file_stem()
            .and_then(OsStr::to_str)
            .is_some_and(|stem| stem.eq_ignore_ascii_case("cargo"));
        if is_cargo {
            Self::Cargo
        } else {
            Self::Parallel
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StepPlan {
    name: String,
    kind: String,
    lane: Lane,
    command: OsString,
    args: Vec<OsString>,
    env: Vec<(OsString, OsString)>,
    cwd: Option<PathBuf>,
    after_all: bool,
    output_log: Option<(PathBuf, String)>,
}

impl StepPlan {
    pub(crate) fn new(
        name: impl Into<String>,
        kind: impl Into<String>,
        lane: Lane,
        command: impl Into<OsString>,
        args: impl IntoIterator<Item = impl Into<OsString>>,
    ) -> Self {
        Self {
            name: name.into(),
            kind: kind.into(),
            lane,
            command: command.into(),
            args: args.into_iter().map(Into::into).collect(),
            env: Vec::new(),
            cwd: None,
            after_all: false,
            output_log: None,
        }
    }

    pub(crate) fn with_env(mut self, key: impl Into<OsString>, value: impl Into<OsString>) -> Self {
        self.env.push((key.into(), value.into()));
        self
    }

    /// Runs the step from this repo-root-relative directory.
    pub(crate) fn with_cwd(mut self, cwd: PathBuf) -> Self {
        self.cwd = Some(cwd);
        self
    }

    pub(crate) fn with_optional_cwd(mut self, cwd: Option<PathBuf>) -> Self {
        self.cwd = cwd;
        self
    }

    pub(crate) fn with_output_log(
        mut self,
        root: Option<&Path>,
        file_name: impl Into<String>,
    ) -> Self {
        self.output_log = root.map(|root| (root.to_path_buf(), file_name.into()));
        self
    }

    /// Starts the step only after every other step has finished.
    pub(crate) fn with_after_all(mut self, after_all: bool) -> Self {
        self.after_all = after_all;
        self
    }

    /// Converts a planned consumer lint command into the executable step form.
    pub(crate) fn from_planned(step: &PlannedStep) -> Self {
        let mut plan = Self::new(
            step.name(),
            ConsumerProfile::Lint.as_str(),
            Lane::for_program(step.command()),
            step.command().clone(),
            step.args().iter().cloned(),
        )
        .with_optional_cwd(step.cwd().map(Path::to_path_buf))
        .with_after_all(step.after_all());
        for (key, value) in step.env() {
            plan = plan.with_env(key, value);
        }
        plan
    }

    pub(crate) const fn lane(&self) -> Lane {
        self.lane
    }

    pub(crate) const fn after_all(&self) -> bool {
        self.after_all
    }

    pub(crate) fn env(&self) -> &[(OsString, OsString)] {
        &self.env
    }

    pub(crate) fn cwd(&self) -> Option<&Path> {
        self.cwd.as_deref()
    }

    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn kind(&self) -> &str {
        &self.kind
    }

    /// The program followed by its arguments.
    pub(crate) fn argv(&self) -> impl Iterator<Item = &OsString> {
        std::iter::once(&self.command).chain(&self.args)
    }

    pub(crate) fn display_command(&self) -> String {
        self.argv()
            .map(|part| part.to_string_lossy())
            .collect::<Vec<_>>()
            .join(" ")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StepReport {
    name: String,
    kind: String,
    command: String,
    platform: Option<Platform>,
    target: Option<String>,
}

impl StepReport {
    pub(crate) fn success(step: &StepPlan) -> Self {
        Self {
            name: step.name().to_string(),
            kind: step.kind().to_string(),
            command: step.display_command(),
            platform: None,
            target: None,
        }
    }

    /// Records the declared platform and target of a planned step on its report.
    pub(crate) fn for_planned(mut self, step: &PlannedStep) -> Self {
        self.platform = step.platform();
        self.target = step.target().map(|target| target.as_str().to_string());
        self
    }

    pub(crate) fn to_json(&self) -> Value {
        let mut row = json!({
            "name": self.name,
            "kind": self.kind,
            "command": self.command,
            "status": "pass",
        });
        // Platform-tagged steps also report where they were declared to run.
        if let (Some(platform), Some(fields)) = (self.platform, row.as_object_mut()) {
            fields.insert("platform".to_string(), json!(platform.as_str()));
            fields.insert("target".to_string(), json!(self.target));
        }
        row
    }
}

/// JSON row for a selected step that was skipped on this host.
fn skipped_row(record: &SkipRecord) -> Value {
    json!({
        "name": record.name,
        "status": "skipped",
        "host": record.host.as_str(),
        "platform": record.platform.as_str(),
        "target": record.target.as_ref().map(|target| target.as_str()),
        "reason": record.reason.as_str(),
    })
}

/// Seam for injecting step execution in tests and alternate runtimes.
pub(crate) trait SystemAdapter: Send + Sync {
    fn cargo_xwin_available(&self, repo_root: &Path) -> bool;
    /// The platform planning treats as the local host.
    fn host_platform(&self) -> Platform;
    /// Runs one command chosen by the consumer lint planner.
    #[expect(
        clippy::result_large_err,
        reason = "The adapter seam preserves the shared top-level CliError contract across orchestration tests and production execution."
    )]
    fn run_planned_step(
        &self,
        repo_root: &Path,
        step: &PlannedStep,
    ) -> Result<StepReport, CliError>;
    #[expect(
        clippy::result_large_err,
        reason = "The adapter seam preserves the shared top-level CliError contract."
    )]
    fn run_planned_step_with_plan(
        &self,
        repo_root: &Path,
        step: &PlannedStep,
        plan: &StepPlan,
    ) -> Result<StepReport, CliError> {
        let _ = plan;
        self.run_planned_step(repo_root, step)
    }
    #[expect(
        clippy::result_large_err,
        reason = "The adapter seam preserves the shared top-level CliError contract across orchestration tests and production execution."
    )]
    fn run_step(&self, repo_root: &Path, step: &StepPlan) -> Result<StepReport, CliError>;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct HostSystemAdapter;

impl SystemAdapter for HostSystemAdapter {
    fn cargo_xwin_available(&self, repo_root: &Path) -> bool {
        ProcessCommand::new("cargo")
            .current_dir(repo_root)
            .arg("xwin")
            .arg("--version")
            .output()
            .is_ok_and(|output| output.status.success())
    }

    fn host_platform(&self) -> Platform {
        Platform::host()
    }

    fn run_planned_step(
        &self,
        repo_root: &Path,
        step: &PlannedStep,
    ) -> Result<StepReport, CliError> {
        self.run_step(repo_root, &StepPlan::from_planned(step))
            .map(|report| report.for_planned(step))
    }

    fn run_planned_step_with_plan(
        &self,
        repo_root: &Path,
        step: &PlannedStep,
        plan: &StepPlan,
    ) -> Result<StepReport, CliError> {
        self.run_step(repo_root, plan)
            .map(|report| report.for_planned(step))
    }

    fn run_step(&self, repo_root: &Path, step: &StepPlan) -> Result<StepReport, CliError> {
        let directory = step
            .cwd()
            .map_or_else(|| repo_root.to_path_buf(), |cwd| repo_root.join(cwd));
        let output = ProcessCommand::new(&step.command)
            .current_dir(directory)
            .args(&step.args)
            .envs(step.env().iter().map(|(key, value)| (key, value)))
            .output()
            .map_err(|error| {
                let missing = error.kind() == std::io::ErrorKind::NotFound;
                let mut diagnostic = CliError::backend_failure(format!("{} failed to start", step.name()))
                    .with_source(error)
                    .with_detail("step", json!(step.name()))
                    .with_detail("command", json!(step.display_command()))
                    .with_detail("root", json!(repo_root.display().to_string()));
                if missing {
                    diagnostic = diagnostic
                        .with_code("CLI.SC_LINT_BACKEND_NOT_FOUND")
                        .with_suggested_action(
                            "Install the backend named by the configured profile command, then rerun the profile.",
                        )
                        .with_documentation("sc-lint docs setup");
                }
                diagnostic
            })?;

        let log_write_error = step.output_log.as_ref().and_then(|(root, file_name)| {
            crate::step_logs::write_step_output(root, file_name, &output.stdout, &output.stderr)
                .err()
                .map(|error| (crate::step_logs::step_log_path(root, file_name), error))
        });

        if !output.status.success() || log_write_error.is_some() {
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            let cause = format!("stdout:\n{stdout}\nstderr:\n{stderr}");
            let mut error = CliError::backend_failure(format!("{} failed", step.name()))
                .with_detail("step", json!(step.name()))
                .with_detail("command", json!(step.display_command()))
                .with_detail("root", json!(repo_root.display().to_string()))
                .with_detail(consts::FIELD_EXIT_CODE, json!(output.status.code()))
                .with_detail("stdout", json!(stdout))
                .with_detail("stderr", json!(stderr));
            if let Some((destination, log_error)) = log_write_error {
                error = error
                    .with_detail(
                        "step_log_destination",
                        json!(destination.display().to_string()),
                    )
                    .with_detail("step_log_error", json!(log_error.to_string()));
            }
            error = error.with_cause(cause);
            return Err(error);
        }

        Ok(StepReport::success(step))
    }
}

#[expect(
    clippy::result_large_err,
    reason = "Profile orchestration keeps using the shared top-level CliError contract."
)]
pub fn run_lint_profile(
    loaded_config: &LoadedConfig,
    profile: LintProfile,
) -> Result<CommandSuccess, CliError> {
    run_lint_profile_with(loaded_config, profile, &HostSystemAdapter)
}

#[expect(
    clippy::result_large_err,
    reason = "Consumer lint profiles retain the shared top-level CliError contract."
)]
pub(crate) fn run_consumer_lint_profile(
    loaded_config: &LoadedConfig,
    selector: Option<&str>,
    scope: PlatformScope,
    policy: FailurePolicy,
) -> Result<CommandSuccess, CliError> {
    run_consumer_lint_with(loaded_config, selector, scope, policy, &HostSystemAdapter)
}

#[expect(
    clippy::result_large_err,
    reason = "Consumer test profiles retain the shared top-level CliError contract."
)]
pub(crate) fn run_consumer_test_profile(
    loaded_config: &LoadedConfig,
    request: &TestRequest,
    policy: FailurePolicy,
) -> Result<CommandSuccess, CliError> {
    run_consumer_test_with(loaded_config, request, policy, &HostSystemAdapter)
}

#[expect(
    clippy::result_large_err,
    reason = "Check-command orchestration keeps using the shared top-level CliError contract."
)]
pub fn run_check(
    loaded_config: &LoadedConfig,
    target: CheckTarget,
) -> Result<CommandSuccess, CliError> {
    run_check_with(loaded_config, target, &HostSystemAdapter)
}

#[expect(
    clippy::result_large_err,
    reason = "Clippy-command orchestration keeps using the shared top-level CliError contract."
)]
pub fn run_clippy(
    loaded_config: &LoadedConfig,
    target: ClippyTarget,
) -> Result<CommandSuccess, CliError> {
    run_clippy_with(loaded_config, target, &HostSystemAdapter)
}

#[expect(
    clippy::result_large_err,
    reason = "Top-level CI orchestration keeps using the shared top-level CliError contract."
)]
pub fn run_ci(loaded_config: &LoadedConfig) -> Result<CommandSuccess, CliError> {
    run_ci_with(loaded_config, &HostSystemAdapter)
}

#[expect(
    clippy::result_large_err,
    reason = "Tests need the same CliError contract as the production profile path."
)]
pub(crate) fn run_lint_profile_with(
    loaded_config: &LoadedConfig,
    profile: LintProfile,
    adapter: &dyn SystemAdapter,
) -> Result<CommandSuccess, CliError> {
    let repo_root = loaded_config.require_repo_root()?;
    let xwin_available = adapter.cargo_xwin_available(repo_root);
    let plans = lint_profile_plan(repo_root, profile, xwin_available);
    let steps = run_builtin_plan(repo_root, adapter, &plans)?;

    Ok(CommandSuccess::direct(json!({
        "status": "pass",
        "profile": profile.command_suffix(),
        "step_count": steps.len(),
        "steps": steps.into_iter().map(|step| step.to_json()).collect::<Vec<_>>(),
        "xwin": {
            "available": xwin_available,
            "included": matches!(profile, LintProfile::Full) && xwin_available,
            "target": WINDOWS_XWIN_TARGET,
        },
    })))
}

/// Plans the selected consumer lint steps for this host, preflights `cargo
/// xwin`, schedules the runnable steps and merges the platform-skipped rows
/// back in declaration order.
#[expect(
    clippy::result_large_err,
    reason = "Consumer lint execution retains the shared top-level CliError contract."
)]
pub(crate) fn run_consumer_lint_with(
    loaded_config: &LoadedConfig,
    selector: Option<&str>,
    scope: PlatformScope,
    policy: FailurePolicy,
    adapter: &dyn SystemAdapter,
) -> Result<CommandSuccess, CliError> {
    let input = loaded_config.consumer_lint_inputs(selector)?;
    let decisions = plan_consumer_lint(
        &input.steps,
        input.selector.as_ref(),
        scope,
        adapter.host_platform(),
    )?;
    preflight_xwin(input.root, adapter, &decisions)?;
    let log_names = crate::step_logs::step_log_file_names(decisions.iter().filter_map(
        |decision| match decision {
            PlanDecision::Run(step) => Some(step.name()),
            PlanDecision::Skip(_) => None,
        },
    ));
    let mut log_names = log_names.into_iter();
    let scheduled = decisions
        .iter()
        .enumerate()
        .filter_map(|(position, decision)| match decision {
            PlanDecision::Run(step) => Some(
                ScheduledStep::new(
                    position,
                    StepPlan::from_planned(step).with_output_log(
                        loaded_config.logging_root().map(PathBuf::as_path),
                        log_names.next().expect("each run step has a log name"),
                    ),
                )
                .with_planned(step),
            ),
            PlanDecision::Skip(_) => None,
        })
        .collect::<Vec<_>>();
    let mut reports = run_steps_scheduled(input.root, adapter, &scheduled, policy)?.into_iter();
    let rows = decisions
        .iter()
        .map(|decision| match decision {
            PlanDecision::Skip(record) => skipped_row(record),
            PlanDecision::Run(_) => reports
                .next()
                .expect("the scheduler reports every run decision")
                .to_json(),
        })
        .collect::<Vec<_>>();
    Ok(consumer_success(
        ConsumerProfile::Lint,
        selector.unwrap_or(CONSUMER_SELECTOR_ALL),
        rows,
    ))
}

/// A missing `cargo xwin` is a capability error before anything runs.
#[expect(
    clippy::result_large_err,
    reason = "Optional-capability failures stay in the shared top-level CliError contract."
)]
fn preflight_xwin(
    root: &Path,
    adapter: &dyn SystemAdapter,
    decisions: &[PlanDecision],
) -> Result<(), CliError> {
    let mut checked = None;
    for decision in decisions {
        if let PlanDecision::Run(step) = decision
            && step.uses_cargo_xwin()
        {
            let available = *checked.get_or_insert_with(|| adapter.cargo_xwin_available(root));
            ensure_xwin_available(step.name(), available)?;
        }
    }
    Ok(())
}

/// Runs the selected consumer test selectors (or lists them for `--list`).
#[expect(
    clippy::result_large_err,
    reason = "Consumer test execution retains the shared top-level CliError contract."
)]
pub(crate) fn run_consumer_test_with(
    loaded_config: &LoadedConfig,
    request: &TestRequest,
    policy: FailurePolicy,
    adapter: &dyn SystemAdapter,
) -> Result<CommandSuccess, CliError> {
    let (root, selectors) = loaded_config.test_selectors()?;
    if request.list() {
        return Ok(CommandSuccess::direct(selectors.list_json()));
    }
    let expanded = selectors.expand(request.selector())?;
    selectors.check_forwarded_args(request)?;
    let host = adapter.host_platform();
    let log_names = crate::step_logs::step_log_file_names(
        expanded
            .iter()
            .filter(|selector| selector.runs_on(host))
            .map(|selector| selector.name()),
    );
    let mut log_names = log_names.into_iter();
    let scheduled = expanded
        .iter()
        .enumerate()
        .filter(|(_, selector)| selector.runs_on(host))
        .map(|(position, selector)| {
            ScheduledStep::new(
                position,
                selector.step_plan(request.args()).with_output_log(
                    loaded_config.logging_root().map(PathBuf::as_path),
                    log_names.next().expect("each run selector has a log name"),
                ),
            )
        })
        .collect::<Vec<_>>();
    let mut reports = run_steps_scheduled(root, adapter, &scheduled, policy)?.into_iter();
    let rows = expanded
        .iter()
        .map(|selector| {
            if selector.runs_on(host) {
                reports
                    .next()
                    .expect("the scheduler reports every scheduled selector")
                    .to_json()
            } else {
                selector.skipped_row(host)
            }
        })
        .collect::<Vec<_>>();
    Ok(consumer_success(
        ConsumerProfile::Test,
        &selectors.selected_name(request.selector()),
        rows,
    ))
}

fn consumer_success(profile: ConsumerProfile, selector: &str, rows: Vec<Value>) -> CommandSuccess {
    CommandSuccess::direct(json!({
        "status": "pass",
        "profile": profile.as_str(),
        "selector": selector,
        "step_count": rows.len(),
        "steps": rows,
    }))
}

#[expect(
    clippy::result_large_err,
    reason = "Tests need the same CliError contract as the production check path."
)]
pub(crate) fn run_check_with(
    loaded_config: &LoadedConfig,
    target: CheckTarget,
    adapter: &dyn SystemAdapter,
) -> Result<CommandSuccess, CliError> {
    let repo_root = loaded_config.require_repo_root()?;
    let xwin_available = adapter.cargo_xwin_available(repo_root);
    let step = match target {
        CheckTarget::Native => cargo_step("check.native", "check", ["check", "--workspace"]),
        CheckTarget::Xwin => {
            ensure_xwin_available("check.xwin", xwin_available)?;
            cargo_step(
                "check.xwin",
                "check",
                [
                    "xwin",
                    "check",
                    "--workspace",
                    "--target",
                    WINDOWS_XWIN_TARGET,
                ],
            )
        }
    };
    let report = adapter.run_step(repo_root, &step)?;

    Ok(CommandSuccess::direct(json!({
        "status": "pass",
        "mode": target.command_suffix(),
        consts::FIELD_TOOL: "cargo",
        "step_count": 1,
        "steps": [report.to_json()],
        "xwin": {
            "available": xwin_available,
            "target": if matches!(target, CheckTarget::Xwin) {
                Value::String(WINDOWS_XWIN_TARGET.to_string())
            } else {
                Value::Null
            },
        },
    })))
}

#[expect(
    clippy::result_large_err,
    reason = "Tests need the same CliError contract as the production clippy path."
)]
pub(crate) fn run_clippy_with(
    loaded_config: &LoadedConfig,
    target: ClippyTarget,
    adapter: &dyn SystemAdapter,
) -> Result<CommandSuccess, CliError> {
    let repo_root = loaded_config.require_repo_root()?;
    let xwin_available = adapter.cargo_xwin_available(repo_root);
    let step = match target {
        ClippyTarget::Native => cargo_step(
            "clippy.native",
            "clippy",
            [
                "clippy",
                "--workspace",
                "--all-targets",
                "--",
                "-D",
                "warnings",
            ],
        ),
        ClippyTarget::Xwin => {
            ensure_xwin_available("clippy.xwin", xwin_available)?;
            cargo_step(
                "clippy.xwin",
                "clippy",
                [
                    "xwin",
                    "clippy",
                    "--workspace",
                    "--all-targets",
                    "--target",
                    WINDOWS_XWIN_TARGET,
                    "--",
                    "-D",
                    "warnings",
                ],
            )
        }
    };
    let report = adapter.run_step(repo_root, &step)?;

    Ok(CommandSuccess::direct(json!({
        "status": "pass",
        "mode": target.command_suffix(),
        consts::FIELD_TOOL: "cargo",
        "step_count": 1,
        "steps": [report.to_json()],
        "xwin": {
            "available": xwin_available,
            "target": if matches!(target, ClippyTarget::Xwin) {
                Value::String(WINDOWS_XWIN_TARGET.to_string())
            } else {
                Value::Null
            },
        },
    })))
}

#[expect(
    clippy::result_large_err,
    reason = "Tests need the same CliError contract as the production CI path."
)]
pub(crate) fn run_ci_with(
    loaded_config: &LoadedConfig,
    adapter: &dyn SystemAdapter,
) -> Result<CommandSuccess, CliError> {
    let repo_root = loaded_config.require_repo_root()?;
    let xwin_available = adapter.cargo_xwin_available(repo_root);
    let mut steps = run_builtin_plan(
        repo_root,
        adapter,
        &lint_profile_plan(repo_root, LintProfile::Ci, xwin_available),
    )?;
    steps.push(adapter.run_step(
        repo_root,
        &cargo_step("test", "test", ["test", "--workspace"]),
    )?);

    Ok(CommandSuccess::direct(json!({
        "status": "pass",
        "lint_profile": "ci",
        "tests_included": true,
        "step_count": steps.len(),
        "steps": steps.into_iter().map(|step| step.to_json()).collect::<Vec<_>>(),
        "xwin": {
            "available": xwin_available,
            "included": false,
            "target": WINDOWS_XWIN_TARGET,
        },
    })))
}

fn lint_profile_plan(
    repo_root: &Path,
    profile: LintProfile,
    xwin_available: bool,
) -> Vec<StepPlan> {
    let mut plan = match profile {
        LintProfile::Fast => fast_lint_steps(repo_root),
        LintProfile::Full => {
            let mut steps = gate_lint_steps(repo_root);
            steps.extend(repo_policy_lint_steps(repo_root));
            steps
        }
        // REQ-CLI-015: the CI profile never includes xwin-only steps.
        LintProfile::Ci => gate_lint_steps(repo_root),
    };

    plan.extend(xwin_lint_steps(profile, xwin_available));
    plan
}

fn fast_lint_steps(repo_root: &Path) -> Vec<StepPlan> {
    let mut steps = vec![cargo_step("fmt", "lint", ["fmt", "--all", "--check"])];
    steps.extend(source_check_steps(repo_root));
    steps
}

/// Python source checks that never touch the cargo build directory.
fn source_check_steps(repo_root: &Path) -> Vec<StepPlan> {
    [
        ("version", "sc_lint.check_version_sync"),
        ("manifests", "sc_lint.lint_manifests"),
        ("spell", "sc_lint.lint_codespell"),
        ("pytests", "sc_lint.run_pytests"),
    ]
    .into_iter()
    .map(|(name, module)| python_step(repo_root, name, "lint", Lane::Parallel, module))
    .collect()
}

/// The steps shared by the Full and CI profiles, in plan order.
fn gate_lint_steps(repo_root: &Path) -> Vec<StepPlan> {
    let mut steps = vec![
        cargo_step("fmt", "lint", ["fmt", "--all", "--check"]),
        cargo_step(
            "clippy",
            "lint",
            [
                "clippy",
                "--workspace",
                "--all-targets",
                "--",
                "-D",
                "warnings",
            ],
        ),
        python_step(
            repo_root,
            "deny",
            "lint",
            Lane::Cargo,
            "sc_lint.lint_cargo_deny",
        ),
        python_step(
            repo_root,
            "shear",
            "lint",
            Lane::Cargo,
            "sc_lint.lint_cargo_shear",
        ),
    ];
    steps.extend(source_check_steps(repo_root));
    steps.extend(
        ["sc-boundary", "sc-portability", "sc-runtime"]
            .map(|name| product_step(name, "lint", Lane::Cargo, ["lint", name])),
    );
    steps
}

/// Repository-policy checks that only the Full profile adds.
fn repo_policy_lint_steps(repo_root: &Path) -> Vec<StepPlan> {
    [
        ("function-length", "sc_lint.lint_function_length"),
        ("line-counts", "sc_lint.lint_line_counts"),
        ("identity-literals", "sc_lint.lint_identity_literals"),
    ]
    .into_iter()
    .map(|(name, module)| python_step(repo_root, name, "lint", Lane::Parallel, module))
    .collect()
}

fn xwin_lint_steps(profile: LintProfile, xwin_available: bool) -> Vec<StepPlan> {
    if matches!(profile, LintProfile::Full) && xwin_available {
        vec![
            cargo_step(
                "check.xwin",
                "check",
                [
                    "xwin",
                    "check",
                    "--workspace",
                    "--target",
                    WINDOWS_XWIN_TARGET,
                ],
            ),
            cargo_step(
                "clippy.xwin",
                "clippy",
                [
                    "xwin",
                    "clippy",
                    "--workspace",
                    "--all-targets",
                    "--target",
                    WINDOWS_XWIN_TARGET,
                    "--",
                    "-D",
                    "warnings",
                ],
            ),
        ]
    } else {
        Vec::new()
    }
}

/// Runs the built-in source lint plan through the shared scheduler with
/// keep-going semantics (REQ-CLI-023): every step runs, reports keep plan
/// order, and the error names every failed step by plan index.
#[expect(
    clippy::result_large_err,
    reason = "Step execution stays within the top-level CliError contract."
)]
fn run_builtin_plan(
    repo_root: &Path,
    adapter: &dyn SystemAdapter,
    plans: &[StepPlan],
) -> Result<Vec<StepReport>, CliError> {
    let steps = plans
        .iter()
        .enumerate()
        .map(|(index, plan)| ScheduledStep::new(index, plan.clone()))
        .collect::<Vec<_>>();
    run_steps_scheduled(repo_root, adapter, &steps, FailurePolicy::KeepGoing)
}

fn cargo_step(
    name: &'static str,
    kind: &'static str,
    args: impl IntoIterator<Item = impl Into<OsString>>,
) -> StepPlan {
    StepPlan::new(name, kind, Lane::Cargo, "cargo", args)
}

/// A step that re-enters the running `sc-lint` executable so composite
/// profiles reuse the native single-target dispatch path (issue #84). The
/// released archive therefore needs no source-tree wrapper for these steps.
fn product_step(
    name: &'static str,
    kind: &'static str,
    lane: Lane,
    args: impl IntoIterator<Item = impl Into<OsString>>,
) -> StepPlan {
    StepPlan::new(name, kind, lane, crate::dispatch::product_binary(), args)
}

fn python_step(
    repo_root: &Path,
    name: &'static str,
    kind: &'static str,
    lane: Lane,
    module: &str,
) -> StepPlan {
    let interpreter = crate::python_adapter::PythonInterpreter::resolve(repo_root);
    let step = StepPlan::new(name, kind, lane, interpreter.program, ["-m", module])
        .with_env("SC_LINT_BIN", crate::dispatch::product_binary());
    match interpreter.python_path {
        Some(python_path) => step.with_env("PYTHONPATH", python_path),
        None => step,
    }
}

#[expect(
    clippy::result_large_err,
    reason = "Optional-capability failures must stay in the shared top-level CliError contract."
)]
fn ensure_xwin_available(command_id: &str, xwin_available: bool) -> Result<(), CliError> {
    if xwin_available {
        return Ok(());
    }

    Err(CliError::capability(format!(
        "{command_id} requires `cargo xwin`, but that capability is not available",
    ))
    .with_detail("command", json!(command_id))
    .with_detail(consts::FIELD_TOOL, json!("cargo xwin"))
    .with_detail("target", json!(WINDOWS_XWIN_TARGET))
    .with_suggested_action(
        "Install `cargo-xwin` to enable Windows preflight or use the native check/clippy path instead.",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn python_steps_delegate_to_the_running_product_binary() {
        let step = python_step(
            Path::new("."),
            "pytests",
            "lint",
            Lane::Parallel,
            "sc_lint.run_pytests",
        );
        let product_binary = crate::dispatch::product_binary().into_os_string();

        assert!(step.env.iter().any(|(key, value)| {
            key.to_string_lossy() == "SC_LINT_BIN" && value == &product_binary
        }));
    }
}
