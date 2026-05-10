mod cli;
mod command;
mod config;
mod contract;
mod dispatch;
mod error;
mod render;
mod workflow;

#[cfg(test)]
mod tests;

use std::ffi::OsString;
use std::process::ExitCode;

use clap::CommandFactory;
use clap::Parser;
use serde_json::Value;

pub use cli::CheckTarget;
pub use cli::Cli;
pub use cli::ClippyTarget;
pub use cli::Command;
pub use cli::LintProfile;
pub use cli::LintTarget;
pub use cli::OutputMode;
pub use cli::ViewTarget;
pub use command::CommandContext;
pub use command::DispatchTelemetry;
pub use config::LoadedConfig;
pub use config::RepoRoot;
pub use contract::CommandEnvelope;
pub use error::CliError;
pub use error::CliErrorKind;
pub use render::RenderedOutput;
pub use workflow::WINDOWS_XWIN_TARGET;

pub struct ImmediateOutcome {
    pub rendered: RenderedOutput,
    pub exit_code: u8,
}

pub enum ParsedInvocation {
    Ready(Cli),
    Immediate(ImmediateOutcome),
}

pub struct ExecutionOutcome {
    pub context: CommandContext,
    pub rendered: RenderedOutput,
    pub exit_code: u8,
    pub ok: bool,
    pub summary: String,
    pub error: Option<CliError>,
    pub dispatch: Option<DispatchTelemetry>,
}

pub fn run<I, T>(args: I) -> ExitCode
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    match parse_args(args) {
        ParsedInvocation::Ready(cli) => {
            let context = CommandContext::from_cli(&cli);
            let loaded_config = match LoadedConfig::load(&cli, &context) {
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
        ParsedInvocation::Immediate(outcome) => {
            write_rendered_output(outcome.rendered, outcome.exit_code)
        }
    }
}

pub fn parse_args<I, T>(args: I) -> ParsedInvocation
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

pub fn execute(
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
            ExecutionOutcome {
                context,
                rendered,
                exit_code: 0,
                ok: true,
                summary: "command completed".to_string(),
                error: None,
                dispatch: success.dispatch,
            }
        }
        Err(error) => {
            let exit_code = error.exit_code();
            let summary = error.message.clone();
            let rendered = render_error(context.command_id(), output_mode, &error);
            ExecutionOutcome {
                context,
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
            rendered: RenderedOutput::stdout(error.to_string()),
            exit_code: 0,
        },
        _ => {
            let cli_error = CliError::usage(error.render().to_string()).with_suggested_action(
                "Run `sc-lint --help` to inspect the supported command surface.",
            );
            ImmediateOutcome {
                rendered: render_error("cli", OutputMode::from_json_flag(json_mode), &cli_error),
                exit_code: cli_error.exit_code(),
            }
        }
    }
}

fn render_success(
    context: &CommandContext,
    output_mode: OutputMode,
    envelope: &CommandEnvelope<Value>,
) -> RenderedOutput {
    if output_mode.is_json() {
        RenderedOutput::stdout(render::render_success_json(envelope))
    } else {
        RenderedOutput::stdout(render::render_success_human(context, envelope))
    }
}

fn render_error(command_id: &str, output_mode: OutputMode, error: &CliError) -> RenderedOutput {
    if output_mode.is_json() {
        RenderedOutput::stderr(render::render_error_json(command_id, error))
    } else {
        RenderedOutput::stderr(render::render_error_human(command_id, error))
    }
}

pub fn render_failure(command_id: &str, json_mode: bool, error: &CliError) -> RenderedOutput {
    render_error(command_id, OutputMode::from_json_flag(json_mode), error)
}

pub fn write_rendered_output(rendered: RenderedOutput, exit_code: u8) -> ExitCode {
    if let Some(stdout) = rendered.stdout {
        println!("{stdout}");
    }
    if let Some(stderr) = rendered.stderr {
        eprintln!("{stderr}");
    }
    ExitCode::from(exit_code)
}

pub fn help_text() -> String {
    let mut command = Cli::command();
    let mut bytes = Vec::new();
    command
        .write_long_help(&mut bytes)
        .expect("help text writes to a buffer");
    String::from_utf8(bytes).expect("clap help output is valid utf-8")
}
