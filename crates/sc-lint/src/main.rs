mod logging;

use std::process::ExitCode;
use std::time::Instant;

pub use sc_lint::Cli;
pub use sc_lint::CliError;
pub use sc_lint::WINDOWS_XWIN_TARGET;

mod command {
    pub use sc_lint::CommandContext;
    pub use sc_lint::DispatchTelemetry;
}

mod config {
    pub use sc_lint::LoadedConfig;
}

fn main() -> ExitCode {
    match sc_lint::ParsedInvocation::parse(std::env::args_os()) {
        sc_lint::ParsedInvocation::Ready(cli) => run_with_logging(cli),
        sc_lint::ParsedInvocation::Immediate(outcome) => outcome.write(),
    }
}

fn run_with_logging(cli: sc_lint::Cli) -> ExitCode {
    let context = match sc_lint::CommandContext::from_cli(&cli) {
        Ok(context) => context,
        Err(error) => {
            return sc_lint::RenderedOutput::render_failure("cli.parse_error", cli.json, &error)
                .write(error.exit_code());
        }
    };
    let loaded_config = match sc_lint::LoadedConfig::load(&cli, &context) {
        Ok(loaded_config) => loaded_config,
        Err(error) => {
            return sc_lint::RenderedOutput::render_failure(context.command_id(), cli.json, &error)
                .write(error.exit_code());
        }
    };
    let observed = match logging::ObservedCommand::from_context(&context, &loaded_config) {
        Ok(observed) => observed,
        Err(error) => {
            return sc_lint::RenderedOutput::render_failure(context.command_id(), cli.json, &error)
                .write(error.exit_code());
        }
    };
    let logger = match logging::initialize_logger(&observed, &cli) {
        Ok(logger) => Some(logger),
        Err(error) => {
            return sc_lint::RenderedOutput::render_failure(context.command_id(), cli.json, &error)
                .write(error.exit_code());
        }
    };
    let started_at = Instant::now();
    if let Some(logger) = logger.as_ref() {
        logging::log_entry(logger, &observed, &cli);
        if let Some(tool) = context.dispatch_tool() {
            logging::log_dispatch_start(logger, &observed, tool);
        }
    }
    let outcome = sc_lint::ExecutionOutcome::run(context.clone(), &loaded_config, cli.json);
    if let Some(logger) = logger.as_ref() {
        if let Some(dispatch) = outcome.dispatch.as_ref() {
            logging::log_dispatch_result(logger, &observed, dispatch);
        }
        if let Some(error) = outcome.error.as_ref() {
            logging::log_error(logger, &observed, error);
        }
        logging::log_completion(
            logger,
            &observed,
            outcome.ok,
            &outcome.summary,
            started_at.elapsed(),
        );
        logging::flush(logger);
        logging::shutdown(logger);
    }
    outcome.rendered.write(outcome.exit_code)
}
