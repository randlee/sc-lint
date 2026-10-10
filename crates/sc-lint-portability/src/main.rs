use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::Context;
use anyhow::Result;
use clap::Parser;
use clap::Subcommand;
use sc_lint_portability::AnalyzeOptions;
use sc_lint_portability::analyze_workspace_configured;
use sc_lint_portability::render_findings_report;
use sc_lint_schema::OutputFormat;

#[derive(Debug, Parser)]
#[command(name = "sc-lint-portability")]
#[command(about = "AST-sensitive Rust portability analyzer")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Analyze {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long, default_value = "text")]
        format: FormatArg,
        #[arg(long)]
        config: Option<PathBuf>,
        #[arg(long)]
        rule_filter: Option<String>,
    },
}

#[derive(Debug, Clone, clap::ValueEnum)]
enum FormatArg {
    Text,
    Json,
}

impl From<FormatArg> for OutputFormat {
    fn from(value: FormatArg) -> Self {
        match value {
            FormatArg::Text => OutputFormat::Text,
            FormatArg::Json => OutputFormat::Json,
        }
    }
}

fn main() -> Result<ExitCode> {
    let cli = Cli::parse();

    match cli.command {
        Command::Analyze {
            root,
            format,
            config,
            rule_filter,
        } => {
            let options = AnalyzeOptions {
                root,
                format: format.clone().into(),
            };
            let report = match analyze_workspace_configured(
                &options,
                config.as_deref(),
                rule_filter.as_deref(),
            ) {
                Ok(report) => report,
                Err(sc_lint_portability::PortabilityError::AnalyzerConfig(error)) => {
                    eprintln!("error: {error}");
                    let exit_code = u8::try_from(error.exit_code())
                        .context("analyzer config exit code does not fit in u8")?;
                    return Ok(ExitCode::from(exit_code));
                }
                Err(error) => return Err(error.into()),
            };
            match OutputFormat::from(format) {
                OutputFormat::Text => println!("{}", render_findings_report(&report)),
                OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&report)?),
            }
        }
    }

    Ok(ExitCode::SUCCESS)
}
