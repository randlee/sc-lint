mod cli;
mod command;
mod config;
pub mod consts;
mod contract;
mod dispatch;
mod error;
pub(crate) mod python_adapter;
mod render;
mod workflow;

#[cfg(test)]
mod tests;

#[cfg(test)]
use clap::CommandFactory;
use clap::Parser;
use serde_json::Value;
use std::ffi::OsString;
use std::process::ExitCode;

use crate::cli::OutputMode;

pub use cli::CheckTarget;
pub use cli::Cli;
pub use cli::ClippyTarget;
pub use cli::Command;
pub use cli::LintTarget;
pub use cli::ViewTarget;
pub use command::CommandContext;
pub use command::DispatchTelemetry;
pub use config::LoadedConfig;
pub use contract::CommandEnvelope;
pub use error::CliError;
pub use error::CliErrorKind;
pub use render::RenderedOutput;
pub use workflow::WINDOWS_XWIN_TARGET;

pub struct ImmediateOutcome {
    rendered: render::RenderedOutput,
    exit_code: u8,
}

impl ImmediateOutcome {
    pub fn write(self) -> ExitCode {
        write_rendered_output(self.rendered, self.exit_code)
    }
}

pub enum ParsedInvocation {
    Ready(Cli),
    Immediate(ImmediateOutcome),
}

impl ParsedInvocation {
    pub fn parse<I, T>(args: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: Into<OsString> + Clone,
    {
        parse_args(args)
    }
}

impl RenderedOutput {
    pub fn render_failure(command_id: &str, json_mode: bool, error: &CliError) -> Self {
        render_failure(command_id, json_mode, error)
    }

    pub fn write(self, exit_code: u8) -> ExitCode {
        write_rendered_output(self, exit_code)
    }
}

pub struct ExecutionOutcome {
    pub rendered: RenderedOutput,
    pub exit_code: u8,
    pub ok: bool,
    pub summary: String,
    pub error: Option<CliError>,
    pub dispatch: Option<DispatchTelemetry>,
}

impl ExecutionOutcome {
    pub fn run(context: CommandContext, loaded_config: &LoadedConfig, json_mode: bool) -> Self {
        execute(context, loaded_config, json_mode)
    }
}

pub fn run<I, T>(args: I) -> ExitCode
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    match parse_args(args) {
        ParsedInvocation::Ready(cli) => {
            let context = match command::CommandContext::from_cli(&cli) {
                Ok(context) => context,
                Err(error) => {
                    let rendered = render_error(
                        "cli.parse_error",
                        OutputMode::from_json_flag(cli.json),
                        &error,
                    );
                    return write_rendered_output(rendered, error.exit_code());
                }
            };
            let loaded_config = match config::LoadedConfig::load(&cli, &context) {
                Ok(loaded_config) => loaded_config,
                Err(error) => {
                    let rendered = render_error(
                        context.command_id(),
                        OutputMode::from_json_flag(cli.json),
                        &error,
                    );
                    return write_rendered_output(rendered, error.exit_code());
                }
            };
            let outcome = execute(context, &loaded_config, cli.json);
            write_rendered_output(outcome.rendered, outcome.exit_code)
        }
        ParsedInvocation::Immediate(outcome) => outcome.write(),
    }
}

pub(crate) fn parse_args<I, T>(args: I) -> ParsedInvocation
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let argv: Vec<OsString> = args.into_iter().map(Into::into).collect();
    match Cli::try_parse_from(argv.clone()) {
        Ok(cli) => ParsedInvocation::Ready(cli),
        Err(error) => ParsedInvocation::Immediate(handle_parse_error(&argv, error)),
    }
}

pub(crate) fn execute(
    context: CommandContext,
    loaded_config: &LoadedConfig,
    json_mode: bool,
) -> ExecutionOutcome {
    let result = command::execute(&context, loaded_config);
    let output_mode = OutputMode::from_json_flag(json_mode);
    match result {
        Ok(success) => {
            let envelope = CommandEnvelope::success(context.command_id(), success.data);
            let rendered = render_success(&context, output_mode, &envelope);
            let summary = envelope
                .data
                .as_ref()
                .and_then(|value| value.get("summary"))
                .and_then(Value::as_str)
                .unwrap_or("command completed")
                .to_string();
            ExecutionOutcome {
                rendered,
                exit_code: 0,
                ok: true,
                summary,
                error: None,
                dispatch: success.dispatch,
            }
        }
        Err(error) => {
            let exit_code = error.exit_code();
            let summary = error.message.clone();
            let rendered = render_error(context.command_id(), output_mode, &error);
            ExecutionOutcome {
                rendered,
                exit_code,
                ok: false,
                summary,
                error: Some(error),
                dispatch: None,
            }
        }
    }
}

fn handle_parse_error(argv: &[OsString], error: clap::Error) -> ImmediateOutcome {
    use clap::error::ErrorKind;

    let json_mode = argv.iter().any(|value| value == "--json");
    match error.kind() {
        ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => ImmediateOutcome {
            rendered: render::RenderedOutput::stdout(error.to_string()),
            exit_code: 0,
        },
        _ => {
            let cli_error = CliError::usage(error.render().to_string()).with_suggested_action(
                "Run `sc-lint --help` to inspect the supported command surface.",
            );
            ImmediateOutcome {
                rendered: render_error(
                    "cli.parse_error",
                    OutputMode::from_json_flag(json_mode),
                    &cli_error,
                ),
                exit_code: cli_error.exit_code(),
            }
        }
    }
}

fn render_success(
    context: &command::CommandContext,
    output_mode: OutputMode,
    envelope: &CommandEnvelope<Value>,
) -> render::RenderedOutput {
    if output_mode.is_json() {
        render::RenderedOutput::stdout(render::render_success_json(envelope))
    } else {
        render::RenderedOutput::stdout(render::render_success_human(context, envelope))
    }
}

fn render_error(
    command_id: &str,
    output_mode: OutputMode,
    error: &CliError,
) -> render::RenderedOutput {
    if output_mode.is_json() {
        render::RenderedOutput::stderr(render::render_error_json(command_id, error))
    } else {
        render::RenderedOutput::stderr(render::render_error_human(command_id, error))
    }
}

pub(crate) fn render_failure(
    command_id: &str,
    json_mode: bool,
    error: &CliError,
) -> RenderedOutput {
    render_error(command_id, OutputMode::from_json_flag(json_mode), error)
}

pub(crate) fn write_rendered_output(rendered: RenderedOutput, exit_code: u8) -> ExitCode {
    if let Some(stdout) = rendered.stdout {
        println!("{stdout}");
    }
    if let Some(stderr) = rendered.stderr {
        eprintln!("{stderr}");
    }
    ExitCode::from(exit_code)
}

#[cfg(test)]
pub(crate) fn help_text() -> String {
    let mut command = Cli::command();
    let mut bytes = Vec::new();
    command
        .write_long_help(&mut bytes)
        .expect("help text writes to a buffer");
    String::from_utf8(bytes).expect("clap help output is valid utf-8")
}
