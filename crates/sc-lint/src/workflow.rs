use std::ffi::OsString;
use std::path::Path;
use std::process::Command as ProcessCommand;

use serde_json::Value;
use serde_json::json;

use crate::CheckTarget;
use crate::CliError;
use crate::ClippyTarget;
use crate::LintProfile;
use crate::command::CommandSuccess;
use crate::config::LoadedConfig;

pub const WINDOWS_XWIN_TARGET: &str = "x86_64-pc-windows-msvc";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StepPlan {
    name: &'static str,
    kind: &'static str,
    command: OsString,
    args: Vec<OsString>,
}

impl StepPlan {
    fn new(
        name: &'static str,
        kind: &'static str,
        command: impl Into<OsString>,
        args: impl IntoIterator<Item = impl Into<OsString>>,
    ) -> Self {
        Self {
            name,
            kind,
            command: command.into(),
            args: args.into_iter().map(Into::into).collect(),
        }
    }

    pub(crate) const fn name(&self) -> &'static str {
        self.name
    }

    pub(crate) const fn kind(&self) -> &'static str {
        self.kind
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
    name: &'static str,
    kind: &'static str,
    command: String,
}

impl StepReport {
    pub(crate) fn success(step: &StepPlan) -> Self {
        Self {
            name: step.name(),
            kind: step.kind(),
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
            .output()
            .map_err(|error| {
                CliError::backend_failure(format!("{} failed to start", step.name))
                    .with_source(error)
                    .with_detail("step", json!(step.name))
                    .with_detail("command", json!(step.display_command()))
                    .with_detail("root", json!(repo_root.display().to_string()))
            })?;

        if !output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            let cause = if stderr.is_empty() {
                stdout.clone()
            } else {
                stderr.clone()
            };
            let mut error = CliError::backend_failure(format!("{} failed", step.name))
                .with_detail("step", json!(step.name))
                .with_detail("command", json!(step.display_command()))
                .with_detail("root", json!(repo_root.display().to_string()))
                .with_detail("exit_code", json!(output.status.code()))
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
        "tool": "cargo",
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
        "tool": "cargo",
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
            python_step(repo_root, "version", "lint", ".just/check_version_sync.py"),
            python_step(repo_root, "manifests", "lint", ".just/lint_manifests.py"),
            python_step(repo_root, "spell", "lint", ".just/lint_codespell.py"),
            python_step(repo_root, "pytests", "lint", ".just/run_pytests.py"),
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
            python_step(repo_root, "deny", "lint", ".just/lint_cargo_deny.py"),
            python_step(repo_root, "shear", "lint", ".just/lint_cargo_shear.py"),
            python_step(repo_root, "version", "lint", ".just/check_version_sync.py"),
            python_step(repo_root, "manifests", "lint", ".just/lint_manifests.py"),
            python_step(repo_root, "spell", "lint", ".just/lint_codespell.py"),
            python_step(repo_root, "pytests", "lint", ".just/run_pytests.py"),
            python_step(
                repo_root,
                "sc-boundary",
                "lint",
                ".just/lint_sc_boundary.py",
            ),
            python_step(
                repo_root,
                "sc-portability",
                "lint",
                ".just/lint_sc_portability.py",
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
            python_step(repo_root, "deny", "lint", ".just/lint_cargo_deny.py"),
            python_step(repo_root, "shear", "lint", ".just/lint_cargo_shear.py"),
            python_step(repo_root, "version", "lint", ".just/check_version_sync.py"),
            python_step(repo_root, "manifests", "lint", ".just/lint_manifests.py"),
            python_step(repo_root, "spell", "lint", ".just/lint_codespell.py"),
            python_step(repo_root, "pytests", "lint", ".just/run_pytests.py"),
            python_step(
                repo_root,
                "sc-boundary",
                "lint",
                ".just/lint_sc_boundary.py",
            ),
            python_step(
                repo_root,
                "sc-portability",
                "lint",
                ".just/lint_sc_portability.py",
            ),
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

fn python_step(
    repo_root: &Path,
    name: &'static str,
    kind: &'static str,
    relative_script: &str,
) -> StepPlan {
    StepPlan::new(
        name,
        kind,
        python_command(),
        [repo_root.join(relative_script).into_os_string()],
    )
}

fn python_command() -> &'static str {
    if cfg!(windows) { "python" } else { "python3" }
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
    .with_detail("tool", json!("cargo xwin"))
    .with_detail("target", json!(WINDOWS_XWIN_TARGET))
    .with_suggested_action(
        "Install `cargo-xwin` to enable Windows preflight or use the native check/clippy path instead.",
    ))
}
