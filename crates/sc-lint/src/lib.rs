mod cli;
mod command;
mod contract;
mod error;
mod logging;
mod render;

#[cfg(test)]
mod tests;

use std::ffi::OsString;
use std::process::ExitCode;

use clap::CommandFactory;
use clap::Parser;
use command::CommandContext;
use render::RenderedOutput;
use serde_json::Value;

pub use cli::CheckTarget;
pub use cli::Cli;
pub use cli::ClippyTarget;
pub use cli::Command;
pub use cli::LintTarget;
pub use cli::ViewTarget;
pub use contract::CommandEnvelope;
pub use error::CliError;
pub use error::CliErrorKind;

pub fn main() -> ExitCode {
    run(std::env::args_os())
}

pub fn run<I, T>(args: I) -> ExitCode
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let argv: Vec<OsString> = args.into_iter().map(Into::into).collect();
    match Cli::try_parse_from(argv.clone()) {
        Ok(cli) => execute(cli),
        Err(error) => handle_parse_error(&argv, error),
    }
}

fn execute(cli: Cli) -> ExitCode {
    let context = CommandContext::from_cli(&cli);
    let logger = logging::initialize_logger(&context, &cli);

    let logger = match logger {
        Ok(logger) => Some(logger),
        Err(error) => {
            let rendered = render_error(context.command_id(), cli.json, &error);
            return write_rendered_output(rendered, error.exit_code());
        }
    };

    if let Some(logger) = logger.as_ref() {
        logging::emit_entry(logger, &context, &cli);
    }

    let result = command::execute(&context);
    let exit_code = match result {
        Ok(data) => {
            if let Some(logger) = logger.as_ref() {
                logging::emit_completion(logger, &context, true, "command completed");
                logging::flush(logger);
            }

            let envelope = CommandEnvelope::success(context.command_id(), data);
            let rendered = render_success(&context, cli.json, &envelope);
            write_rendered_output(rendered, 0)
        }
        Err(error) => {
            if let Some(logger) = logger.as_ref() {
                logging::emit_error(logger, &context, &error);
                logging::emit_completion(logger, &context, false, &error.message);
                logging::flush(logger);
            }

            let exit_code = error.exit_code();
            let rendered = render_error(context.command_id(), cli.json, &error);
            write_rendered_output(rendered, exit_code)
        }
    };

    if let Some(logger) = logger.as_ref() {
        logging::shutdown(logger);
    }

    exit_code
}

fn handle_parse_error(argv: &[OsString], error: clap::Error) -> ExitCode {
    use clap::error::ErrorKind;

    let json_mode = argv.iter().any(|value| value == "--json");
    match error.kind() {
        ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => {
            print!("{error}");
            ExitCode::SUCCESS
        }
        _ => {
            let cli_error = CliError::usage(error.render().to_string()).with_suggested_action(
                "Run `sc-lint --help` to inspect the supported command surface.",
            );
            let rendered = render_error("cli", json_mode, &cli_error);
            write_rendered_output(rendered, cli_error.exit_code())
        }
    }
}

fn render_success(
    context: &CommandContext,
    json_mode: bool,
    envelope: &CommandEnvelope<Value>,
) -> RenderedOutput {
    if json_mode {
        RenderedOutput::stdout(render::render_success_json(envelope))
    } else {
        RenderedOutput::stdout(render::render_success_human(context, envelope))
    }
}

fn render_error(command_id: &str, json_mode: bool, error: &CliError) -> RenderedOutput {
    if json_mode {
        RenderedOutput::stderr(render::render_error_json(command_id, error))
    } else {
        RenderedOutput::stderr(render::render_error_human(command_id, error))
    }
}

fn write_rendered_output(rendered: RenderedOutput, exit_code: u8) -> ExitCode {
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
