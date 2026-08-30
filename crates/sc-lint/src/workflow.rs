use std::ffi::OsString;
use std::path::Path;
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

pub const WINDOWS_XWIN_TARGET: &str = "x86_64-pc-windows-msvc";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StepPlan {
    name: String,
    kind: String,
    command: OsString,
    args: Vec<OsString>,
    env: Vec<(OsString, OsString)>,
}

impl StepPlan {
    fn new(
        name: impl Into<String>,
        kind: impl Into<String>,
        command: impl Into<OsString>,
        args: impl IntoIterator<Item = impl Into<OsString>>,
    ) -> Self {
        Self {
            name: name.into(),
            kind: kind.into(),
            command: command.into(),
            args: args.into_iter().map(Into::into).collect(),
            env: Vec::new(),
        }
    }

    fn with_env(mut self, key: impl Into<OsString>, value: impl Into<OsString>) -> Self {
        self.env.push((key.into(), value.into()));
        self
    }

    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn kind(&self) -> &str {
        &self.kind
    }

    pub(crate) fn display_command(&self) -> String {
        let mut parts = Vec::with_capacity(self.args.len() + 1);
        parts.push(self.command.to_string_lossy().to_string());
        parts.extend(
            self.args
                .iter()
                .map(|arg| arg.to_string_lossy().to_string()),
        );
        parts.join(" ")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StepReport {
    name: String,
    kind: String,
    command: String,
}

impl StepReport {
    pub(crate) fn success(step: &StepPlan) -> Self {
        Self {
            name: step.name().to_string(),
            kind: step.kind().to_string(),
            command: step.display_command(),
        }
    }

    fn to_json(&self) -> Value {
        json!({
            "name": self.name,
            "kind": self.kind,
            "command": self.command,
            "status": "pass",
        })
    }
}

/// Seam for injecting step execution in tests and alternate runtimes.
pub(crate) trait SystemAdapter {
    fn cargo_xwin_available(&self, repo_root: &Path) -> bool;
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

    fn run_step(&self, repo_root: &Path, step: &StepPlan) -> Result<StepReport, CliError> {
        let output = ProcessCommand::new(&step.command)
            .current_dir(repo_root)
            .args(&step.args)
            .envs(step.env.iter().map(|(key, value)| (key, value)))
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

        if !output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            let cause = if stderr.is_empty() {
                stdout.clone()
            } else {
                stderr.clone()
            };
            let mut error = CliError::backend_failure(format!("{} failed", step.name()))
                .with_detail("step", json!(step.name()))
                .with_detail("command", json!(step.display_command()))
                .with_detail("root", json!(repo_root.display().to_string()))
                .with_detail(consts::FIELD_EXIT_CODE, json!(output.status.code()))
                .with_detail("stdout", json!(stdout))
                .with_detail("stderr", json!(stderr));
            if !cause.is_empty() {
                error = error.with_cause(cause);
            }
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
pub fn run_consumer_lint_profile(
    loaded_config: &LoadedConfig,
    selector: Option<&str>,
) -> Result<CommandSuccess, CliError> {
    run_consumer_profile_with(
        loaded_config,
        ConsumerProfile::Lint,
        selector,
        &HostSystemAdapter,
    )
}

#[expect(
    clippy::result_large_err,
    reason = "Consumer test profiles retain the shared top-level CliError contract."
)]
pub fn run_consumer_test_profile(
    loaded_config: &LoadedConfig,
    selector: Option<&str>,
) -> Result<CommandSuccess, CliError> {
    run_consumer_profile_with(
        loaded_config,
        ConsumerProfile::Test,
        selector,
        &HostSystemAdapter,
    )
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
    let steps = run_steps(repo_root, adapter, &plans)?;

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

#[expect(
    clippy::result_large_err,
    reason = "Tests and production share the same explicit consumer profile execution path."
)]
pub(crate) fn run_consumer_profile_with(
    loaded_config: &LoadedConfig,
    profile: ConsumerProfile,
    selector: Option<&str>,
    adapter: &dyn SystemAdapter,
) -> Result<CommandSuccess, CliError> {
    let (root, configured_steps) = loaded_config.consumer_profile(profile, selector)?;
    let plans = configured_steps
        .iter()
        .map(|step| {
            let (command, args) = step
                .command()
                .split_first()
                .expect("validated command argv");
            StepPlan::new(step.name(), profile.as_str(), command, args)
        })
        .collect::<Vec<_>>();
    let steps = run_steps(root, adapter, &plans)?;
    Ok(CommandSuccess::direct(json!({
        "status": "pass",
        "profile": profile.as_str(),
        "selector": selector.unwrap_or(CONSUMER_SELECTOR_ALL),
        "step_count": steps.len(),
        "steps": steps.into_iter().map(|step| step.to_json()).collect::<Vec<_>>(),
    })))
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
    let mut steps = run_steps(
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
        LintProfile::Fast => vec![
            cargo_step("fmt", "lint", ["fmt", "--all", "--check"]),
            python_step(repo_root, "version", "lint", "sc_lint.check_version_sync"),
            python_step(repo_root, "manifests", "lint", "sc_lint.lint_manifests"),
            python_step(repo_root, "spell", "lint", "sc_lint.lint_codespell"),
            python_step(repo_root, "pytests", "lint", "sc_lint.run_pytests"),
        ],
        LintProfile::Full => vec![
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
            python_step(repo_root, "deny", "lint", "sc_lint.lint_cargo_deny"),
            python_step(repo_root, "shear", "lint", "sc_lint.lint_cargo_shear"),
            python_step(repo_root, "version", "lint", "sc_lint.check_version_sync"),
            python_step(repo_root, "manifests", "lint", "sc_lint.lint_manifests"),
            python_step(repo_root, "spell", "lint", "sc_lint.lint_codespell"),
            python_step(repo_root, "pytests", "lint", "sc_lint.run_pytests"),
            product_step("sc-boundary", "lint", ["lint", "sc-boundary"]),
            product_step("sc-portability", "lint", ["lint", "sc-portability"]),
            python_step(repo_root, "line-counts", "lint", "sc_lint.lint_line_counts"),
            python_step(
                repo_root,
                "identity-literals",
                "lint",
                "sc_lint.lint_identity_literals",
            ),
        ],
        LintProfile::Ci => vec![
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
            python_step(repo_root, "deny", "lint", "sc_lint.lint_cargo_deny"),
            python_step(repo_root, "shear", "lint", "sc_lint.lint_cargo_shear"),
            python_step(repo_root, "version", "lint", "sc_lint.check_version_sync"),
            python_step(repo_root, "manifests", "lint", "sc_lint.lint_manifests"),
            python_step(repo_root, "spell", "lint", "sc_lint.lint_codespell"),
            python_step(repo_root, "pytests", "lint", "sc_lint.run_pytests"),
            product_step("sc-boundary", "lint", ["lint", "sc-boundary"]),
            product_step("sc-portability", "lint", ["lint", "sc-portability"]),
            // REQ-CLI-015: the CI profile never includes xwin-only steps.
        ],
    };

    if matches!(profile, LintProfile::Full) && xwin_available {
        plan.push(cargo_step(
            "check.xwin",
            "check",
            [
                "xwin",
                "check",
                "--workspace",
                "--target",
                WINDOWS_XWIN_TARGET,
            ],
        ));
        plan.push(cargo_step(
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
        ));
    }

    plan
}

#[expect(
    clippy::result_large_err,
    reason = "Step execution stays within the top-level CliError contract."
)]
fn run_steps(
    repo_root: &Path,
    adapter: &dyn SystemAdapter,
    steps: &[StepPlan],
) -> Result<Vec<StepReport>, CliError> {
    steps
        .iter()
        .map(|step| adapter.run_step(repo_root, step))
        .collect()
}

fn cargo_step(
    name: &'static str,
    kind: &'static str,
    args: impl IntoIterator<Item = impl Into<OsString>>,
) -> StepPlan {
    StepPlan::new(name, kind, "cargo", args)
}

/// A step that re-enters the running `sc-lint` executable so composite
/// profiles reuse the native single-target dispatch path (issue #84). The
/// released archive therefore needs no source-tree wrapper for these steps.
fn product_step(
    name: &'static str,
    kind: &'static str,
    args: impl IntoIterator<Item = impl Into<OsString>>,
) -> StepPlan {
    StepPlan::new(name, kind, crate::dispatch::product_binary(), args)
}

fn python_step(repo_root: &Path, name: &'static str, kind: &'static str, module: &str) -> StepPlan {
    let interpreter = crate::python_adapter::PythonInterpreter::resolve(repo_root);
    let step = StepPlan::new(name, kind, interpreter.program, ["-m", module]);
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
