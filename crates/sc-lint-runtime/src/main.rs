use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use clap::Subcommand;
use sc_lint_runtime::AnalyzeOptions;
use sc_lint_runtime::RuntimeError;
use sc_lint_runtime::analyze_workspace_configured;
use sc_lint_runtime::render_findings_report;
use sc_lint_schema::OutputFormat;

#[derive(Debug, Parser)]
#[command(name = "sc-lint-runtime")]
#[command(about = "AST-sensitive Rust runtime analyzer")]
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

fn main() -> Result<()> {
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
                Err(RuntimeError::AnalyzerConfig(error)) => {
                    eprintln!("error: {error}");
                    std::process::exit(error.exit_code());
                }
                Err(error) => return Err(error.into()),
            };
            match OutputFormat::from(format) {
                OutputFormat::Text => println!("{}", render_findings_report(&report)),
                OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&report)?),
            }
        }
    }

    Ok(())
}
