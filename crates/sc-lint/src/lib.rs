//! `sc-lint` is a Rust lint tool for repository policy enforcement.
//!
//! It runs boundary, portability and runtime analyzers plus consumer-declared
//! lint and test steps from `sc-lint.toml`, and reports results through a
//! stable JSON-first machine contract. Most users install the `sc-lint`
//! binary; `sc-lint docs` prints the offline user manual.
//!
//! The library target backs the `sc-lint` binary. The supported machine
//! interface is the CLI and its JSON contract (ADR-006).
//!
//! Documentation and source: <https://github.com/randlee/sc-lint>.

mod cli;
mod command;
mod compatibility;
mod config;
pub mod consts;
mod consumer_integration;
mod contract;
mod dispatch;
mod docs;
mod entry;
mod error;
mod installer;
mod profile_host;
mod profile_plan;
pub(crate) mod python_adapter;
mod render;
mod scheduler;
mod step_exec;
mod step_logs;
mod step_paths;
mod test_selectors;
mod workflow;

#[cfg(test)]
mod tests;

#[cfg(test)]
pub(crate) mod test_support;

pub use cli::CheckTarget;
pub use cli::Cli;
pub use cli::ClippyTarget;
pub use cli::Command;
pub use cli::CompatibilityCommand;
pub use cli::DocsGuide;
pub use cli::LintTarget;
pub use cli::PlatformScopeArg;
pub use cli::ViewTarget;
pub use command::CommandContext;
pub use command::DispatchTelemetry;
pub use config::LoadedConfig;
pub use config::MinimumVersion;
pub use contract::CommandEnvelope;
pub use entry::ExecutionOutcome;
pub use entry::ImmediateOutcome;
pub use entry::ParsedInvocation;
pub use entry::run;
pub use entry::run_code;
pub use entry::version_json;
pub use error::CliError;
pub use error::CliErrorKind;
pub use profile_plan::PROFILE_PLAN_ERROR_CODE;
pub use render::RenderedOutput;
pub use workflow::WINDOWS_XWIN_TARGET;

#[cfg(test)]
pub(crate) use entry::help_text;
#[cfg(test)]
pub(crate) use entry::parse_args;
