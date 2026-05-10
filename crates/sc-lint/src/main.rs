mod logging;

use std::process::ExitCode;
use std::time::Instant;

use sc_lint::CommandContext;
use sc_lint::LoadedConfig;
use sc_lint::ParsedInvocation;

fn main() -> ExitCode {
    match sc_lint::parse_args(std::env::args_os()) {
        ParsedInvocation::Ready(cli) => run_with_logging(cli),
        ParsedInvocation::Immediate(outcome) => {
            sc_lint::write_rendered_output(outcome.rendered, outcome.exit_code)
        }
    }
}

fn run_with_logging(cli: sc_lint::Cli) -> ExitCode {
    let context = match CommandContext::from_cli(&cli) {
        Ok(context) => context,
        Err(error) => {
            let rendered = sc_lint::render_failure("cli.parse_error", cli.json, &error);
            return sc_lint::write_rendered_output(rendered, error.exit_code());
        }
    };
    let loaded_config = match LoadedConfig::load(&cli, &context) {
        Ok(loaded_config) => loaded_config,
        Err(error) => {
            let rendered = sc_lint::render_failure(context.command_id(), cli.json, &error);
            return sc_lint::write_rendered_output(rendered, error.exit_code());
        }
    };

    let observed = logging::ObservedCommand::from_context(&context, &loaded_config);

    let logger = match logging::initialize_logger(&observed, &cli) {
        Ok(logger) => Some(logger),
        Err(error) => {
            let rendered = sc_lint::render_failure(context.command_id(), cli.json, &error);
            return sc_lint::write_rendered_output(rendered, error.exit_code());
        }
    };

    let started_at = Instant::now();
    if let Some(logger) = logger.as_ref() {
        logging::log_entry(logger, &observed, &cli);
        if let Some(tool) = context.dispatch_tool() {
            logging::log_dispatch_start(logger, &observed, tool);
        }
    }

    let outcome = sc_lint::execute(context.clone(), &loaded_config, cli.json);

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

    sc_lint::write_rendered_output(outcome.rendered, outcome.exit_code)
}
