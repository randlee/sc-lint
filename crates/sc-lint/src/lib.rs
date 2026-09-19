mod cli;
mod command;
mod config;
pub mod consts;
mod consumer_integration;
mod contract;
mod dispatch;
mod docs;
mod entry;
mod error;
mod installer;
pub(crate) mod python_adapter;
mod render;
mod workflow;

#[cfg(test)]
mod tests;

pub use cli::CheckTarget;
pub use cli::Cli;
pub use cli::ClippyTarget;
pub use cli::Command;
pub use cli::CompatibilityCommand;
pub use cli::DocsGuide;
pub use cli::LintTarget;
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
pub use render::RenderedOutput;
pub use workflow::WINDOWS_XWIN_TARGET;

#[cfg(test)]
pub(crate) use entry::help_text;
#[cfg(test)]
pub(crate) use entry::parse_args;
