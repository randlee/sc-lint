//! The one step scheduler shared by the built-in lint plan and the consumer
//! lint and test profiles (REQ-CLI-023 as amended, ADR-020 decision 10).
//!
//! Every `Lane::Parallel` step gets its own worker (no cap), every
//! `Lane::Cargo` step runs on one serialized lane worker in declaration order,
//! and `after_all` steps run last, one at a time, in declaration order. A
//! failure never stops other steps under [`FailurePolicy::KeepGoing`]; under
//! [`FailurePolicy::FailFast`] only not-yet-started cargo-lane steps and
//! `after_all` steps are skipped. Reports keep declaration order.

use std::any::Any;
use std::panic::AssertUnwindSafe;
use std::panic::catch_unwind;
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::thread;
use std::thread::ScopedJoinHandle;

use serde::Serialize;
use serde_json::Value;
use serde_json::json;

use crate::CliError;
use crate::consts;
use crate::error::ErrorCode;
use crate::profile_plan::PlannedStep;
use crate::workflow::Lane;
use crate::workflow::StepPlan;
use crate::workflow::StepReport;
use crate::workflow::SystemAdapter;

/// Stable codes for consumer profile execution (ADR-020 decision 10).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProfileCode {
    InvalidAfterAll,
    UnknownSelector,
    GroupCycle,
    StepsFailed,
}

impl ErrorCode for ProfileCode {
    fn as_str(&self) -> &'static str {
        match self {
            Self::InvalidAfterAll => "profile.invalid_after_all",
            Self::UnknownSelector => "test.unknown_selector",
            Self::GroupCycle => "test.group_cycle",
            Self::StepsFailed => "profile.steps_failed",
        }
    }
}

impl ProfileCode {
    /// The ADR-020 decision 10 recovery text for this code.
    pub(crate) const fn recovery(self) -> &'static str {
        match self {
            Self::InvalidAfterAll => "remove after_all from that selector",
            Self::UnknownSelector => "use a name from sc-lint test --list",
            Self::GroupCycle => "break the cycle in [tool.sc-lint.test]",
            Self::StepsFailed => "fix the failing steps and rerun",
        }
    }

    /// A `config` error carrying this code; `cause` names the selector or step
    /// and the file. Never used for [`ProfileCode::StepsFailed`].
    pub(crate) fn config_error(self, cause: String, config_path: &Path) -> CliError {
        CliError::config(cause.clone())
            .with_code(self.as_str())
            .with_cause(cause)
            .with_suggested_action(self.recovery())
            .with_detail("config_path", json!(config_path.display().to_string()))
    }
}

/// What the scheduler does after the first failed step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FailurePolicy {
    /// Every step runs; the run fails after all of them finish.
    KeepGoing,
    /// Stop starting cargo-lane and `after_all` steps; running steps finish.
    FailFast,
}

/// One unit of work. `index` is the zero-based position in the caller's
/// selected list (used only in reports); `planned` is set for consumer lint
/// steps so the adapter's `run_planned_step` seam (which records
/// platform/target on the report) stays in use.
pub(crate) struct ScheduledStep<'a> {
    index: usize,
    plan: StepPlan,
    planned: Option<&'a PlannedStep>,
}

impl<'a> ScheduledStep<'a> {
    pub(crate) fn new(index: usize, plan: StepPlan) -> Self {
        Self {
            index,
            plan,
            planned: None,
        }
    }

    pub(crate) fn with_planned(mut self, planned: &'a PlannedStep) -> Self {
        self.planned = Some(planned);
        self
    }

    fn name(&self) -> &str {
        self.plan.name()
    }
}

/// The result of one scheduled step.
pub(crate) enum StepOutcome {
    Done(StepReport),
    Failed(CliError),
    Skipped,
}

/// One `error.details.failed_steps[]` entry.
#[derive(Debug, Serialize)]
struct FailedStep {
    index: usize,
    name: String,
    exit_code: Option<i32>,
    message: String,
}

/// One `error.details.skipped_steps[]` entry.
#[derive(Debug, Serialize)]
struct SkippedStep {
    index: usize,
    name: String,
    reason: &'static str,
}

/// Runs `steps` and returns their reports in declaration order, or one
/// `profile.steps_failed` error naming every failed (and fail-fast skipped)
/// step.
#[expect(
    clippy::result_large_err,
    reason = "Step execution stays within the top-level CliError contract."
)]
pub(crate) fn run_steps_scheduled(
    repo_root: &Path,
    adapter: &dyn SystemAdapter,
    steps: &[ScheduledStep<'_>],
    policy: FailurePolicy,
) -> Result<Vec<StepReport>, CliError> {
    let failed = AtomicBool::new(false);
    let mut outcomes = run_main_steps(repo_root, adapter, steps, policy, &failed);
    for (index, step) in steps.iter().enumerate() {
        if step.plan.after_all() {
            outcomes[index] = Some(run_unless_skipped(
                repo_root, adapter, step, policy, &failed,
            ));
        }
    }
    let outcomes = outcomes
        .into_iter()
        .map(|outcome| outcome.expect("the scheduler records an outcome for every step"))
        .collect();
    aggregate_outcomes(steps, outcomes)
}

/// The steps one worker runs and the outcomes it returns.
type Worker<'scope> = (
    Vec<usize>,
    ScopedJoinHandle<'scope, Vec<(usize, StepOutcome)>>,
);

/// Runs every non-`after_all` step: Parallel steps on their own workers, Cargo
/// steps on one lane worker. Returns once every worker has joined (the
/// barrier before the `after_all` tail); `after_all` slots stay `None`.
fn run_main_steps(
    repo_root: &Path,
    adapter: &dyn SystemAdapter,
    steps: &[ScheduledStep<'_>],
    policy: FailurePolicy,
    failed: &AtomicBool,
) -> Vec<Option<StepOutcome>> {
    let mut outcomes: Vec<Option<StepOutcome>> = steps.iter().map(|_| None).collect();
    thread::scope(|scope| {
        let mut workers = Vec::new();
        for index in main_indices(steps, Lane::Parallel) {
            let step = &steps[index];
            let spawned = thread::Builder::new()
                .name(format!("sc-lint-{}", step.name()))
                .spawn_scoped(scope, move || {
                    vec![(index, run_one(repo_root, adapter, step, failed))]
                });
            let started = started_worker(vec![index], spawned, steps, &mut outcomes, failed);
            workers.extend(started);
        }
        let lane = main_indices(steps, Lane::Cargo);
        let lane_steps = lane.clone();
        let spawned = thread::Builder::new()
            .name("sc-lint-cargo-lane".to_string())
            .spawn_scoped(scope, move || {
                lane_steps
                    .into_iter()
                    .map(|index| {
                        let outcome =
                            run_unless_skipped(repo_root, adapter, &steps[index], policy, failed);
                        (index, outcome)
                    })
                    .collect::<Vec<_>>()
            });
        workers.extend(started_worker(lane, spawned, steps, &mut outcomes, failed));
        join_workers(workers, steps, &mut outcomes, failed);
    });
    outcomes
}

/// The declaration-order indices of the non-`after_all` steps on `lane`.
fn main_indices(steps: &[ScheduledStep<'_>], lane: Lane) -> Vec<usize> {
    steps
        .iter()
        .enumerate()
        .filter(|(_, step)| !step.plan.after_all() && step.plan.lane() == lane)
        .map(|(index, _)| index)
        .collect()
}

/// The worker when it started; otherwise every step it would have run is
/// recorded as failed and `None` is returned.
fn started_worker<'scope>(
    indices: Vec<usize>,
    spawned: std::io::Result<ScopedJoinHandle<'scope, Vec<(usize, StepOutcome)>>>,
    steps: &[ScheduledStep<'_>],
    outcomes: &mut [Option<StepOutcome>],
    failed: &AtomicBool,
) -> Option<Worker<'scope>> {
    match spawned {
        Ok(worker) => Some((indices, worker)),
        Err(error) => {
            let cause = error.to_string();
            fail_all(&indices, steps, outcomes, failed, |name| {
                CliError::internal(format!("{name} worker failed to start")).with_source(&cause)
            });
            None
        }
    }
}

/// Joins every worker. `run_one` catches step panics, so a join error means
/// the worker itself failed; every step it owned is a failure.
fn join_workers(
    workers: Vec<Worker<'_>>,
    steps: &[ScheduledStep<'_>],
    outcomes: &mut [Option<StepOutcome>],
    failed: &AtomicBool,
) {
    for (indices, worker) in workers {
        match worker.join() {
            Ok(results) => {
                for (index, outcome) in results {
                    outcomes[index] = Some(outcome);
                }
            }
            Err(_) => fail_all(&indices, steps, outcomes, failed, |name| {
                CliError::internal(format!("{name} worker panicked"))
            }),
        }
    }
}

fn fail_all(
    indices: &[usize],
    steps: &[ScheduledStep<'_>],
    outcomes: &mut [Option<StepOutcome>],
    failed: &AtomicBool,
    error: impl Fn(&str) -> CliError,
) {
    for &index in indices {
        failed.store(true, Ordering::SeqCst);
        outcomes[index] = Some(StepOutcome::Failed(error(steps[index].name())));
    }
}

/// Under fail-fast a step that has not started after a failure is skipped.
fn run_unless_skipped(
    repo_root: &Path,
    adapter: &dyn SystemAdapter,
    step: &ScheduledStep<'_>,
    policy: FailurePolicy,
    failed: &AtomicBool,
) -> StepOutcome {
    if policy == FailurePolicy::FailFast && failed.load(Ordering::SeqCst) {
        return StepOutcome::Skipped;
    }
    run_one(repo_root, adapter, step, failed)
}

/// The only place a step executes. A panic becomes a failed outcome naming
/// the step; any failure sets `failed`.
#[expect(
    clippy::result_large_err,
    reason = "The adapter seam returns the shared top-level CliError contract by value."
)]
fn run_one(
    repo_root: &Path,
    adapter: &dyn SystemAdapter,
    step: &ScheduledStep<'_>,
    failed: &AtomicBool,
) -> StepOutcome {
    let result = catch_unwind(AssertUnwindSafe(|| match step.planned {
        Some(planned) => adapter.run_planned_step_with_plan(repo_root, planned, &step.plan),
        None => adapter.run_step(repo_root, &step.plan),
    }));
    let outcome = match result {
        Ok(Ok(report)) => StepOutcome::Done(report),
        Ok(Err(error)) => StepOutcome::Failed(error),
        Err(payload) => StepOutcome::Failed(CliError::internal(format!(
            "{} panicked during execution: {}",
            step.name(),
            panic_message(payload.as_ref())
        ))),
    };
    if matches!(outcome, StepOutcome::Failed(_)) {
        failed.store(true, Ordering::SeqCst);
    }
    outcome
}

fn panic_message(payload: &(dyn Any + Send)) -> &str {
    payload
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
        .unwrap_or("non-string panic payload")
}

/// Returns every report in declaration order, or the `profile.steps_failed`
/// error listing every failed step (and every fail-fast skipped step).
#[expect(
    clippy::result_large_err,
    reason = "Step execution stays within the top-level CliError contract."
)]
pub(crate) fn aggregate_outcomes(
    steps: &[ScheduledStep<'_>],
    outcomes: Vec<StepOutcome>,
) -> Result<Vec<StepReport>, CliError> {
    let mut reports = Vec::with_capacity(outcomes.len());
    let mut failed = Vec::new();
    let mut skipped = Vec::new();
    for (step, outcome) in steps.iter().zip(outcomes) {
        match outcome {
            StepOutcome::Done(report) => reports.push(report),
            StepOutcome::Failed(error) => failed.push(FailedStep {
                index: step.index,
                name: step.name().to_string(),
                exit_code: step_exit_code(&error),
                message: failure_message(&error),
            }),
            StepOutcome::Skipped => skipped.push(SkippedStep {
                index: step.index,
                name: step.name().to_string(),
                reason: "fail-fast",
            }),
        }
    }
    if failed.is_empty() {
        debug_assert!(skipped.is_empty(), "a step is skipped only after a failure");
        return Ok(reports);
    }
    let listing = failed
        .iter()
        .map(|step| format!("[{}] {}", step.index, step.name))
        .collect::<Vec<_>>()
        .join(", ");
    let code = ProfileCode::StepsFailed;
    let mut error = CliError::backend_failure(format!(
        "{} of {} steps failed: {listing}",
        failed.len(),
        steps.len()
    ))
    .with_code(code.as_str())
    .with_suggested_action(code.recovery())
    .with_detail("failed_steps", json!(failed));
    if !skipped.is_empty() {
        error = error.with_detail("skipped_steps", json!(skipped));
    }
    Err(error)
}

/// The process exit code a step error recorded, `None` when the process did
/// not start or the step panicked.
fn step_exit_code(error: &CliError) -> Option<i32> {
    error
        .details
        .get(consts::FIELD_EXIT_CODE)
        .and_then(Value::as_i64)
        .and_then(|code| i32::try_from(code).ok())
}

/// The step error message followed by its cause, so step output is kept.
fn failure_message(error: &CliError) -> String {
    match error.cause.as_deref() {
        Some(cause) => format!("{}: {cause}", error.message),
        None => error.message.clone(),
    }
}
