use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::Result;
use clap::Parser;
use clap::Subcommand;
use sc_lint_analyzer_support::AnalyzerConfigError;
use sc_lint_boundary::AnalyzeOptions;
use sc_lint_boundary::BoundaryError;
use sc_lint_boundary::ExportGraphOptions;
use sc_lint_boundary::GraphOutputFormat;
use sc_lint_boundary::analyze_workspace_configured;
use sc_lint_boundary::export_workspace_graph;
use sc_lint_boundary::render_findings_report;
use sc_lint_boundary::render_graph_export_json;
use sc_lint_boundary::render_graph_export_turtle;
use sc_lint_schema::OutputFormat;

#[derive(Debug, Parser)]
#[command(name = "sc-lint-boundary")]
#[command(about = "AST-sensitive Rust boundary analyzer")]
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
        #[arg(long = "rule-filter", alias = "rule")]
        rule_filter: Option<String>,
        #[arg(long)]
        config: Option<PathBuf>,
    },
    ExportGraph {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long, default_value = "json")]
        format: GraphFormatArg,
    },
}

#[derive(Debug, Clone, clap::ValueEnum)]
enum FormatArg {
    Text,
    Json,
}

#[derive(Debug, Clone, clap::ValueEnum)]
enum GraphFormatArg {
    Json,
    Turtle,
}

impl From<FormatArg> for OutputFormat {
    fn from(value: FormatArg) -> Self {
        match value {
            FormatArg::Text => OutputFormat::Text,
            FormatArg::Json => OutputFormat::Json,
        }
    }
}

impl From<GraphFormatArg> for GraphOutputFormat {
    fn from(value: GraphFormatArg) -> Self {
        match value {
            GraphFormatArg::Json => GraphOutputFormat::Json,
            GraphFormatArg::Turtle => GraphOutputFormat::Turtle,
        }
    }
}

fn analyzer_config_exit_code(error: &AnalyzerConfigError) -> u8 {
    match error {
        AnalyzerConfigError::Selector { .. } => 2,
        _ => 3,
    }
}

fn analyzer_config_message(error: &AnalyzerConfigError) -> String {
    match error {
        AnalyzerConfigError::Selector { analyzer, selector } => {
            format!("{analyzer}: unknown --rule-filter selector `{selector}`")
        }
        _ => error.to_string(),
    }
}

fn main() -> Result<ExitCode> {
    let cli = Cli::parse();

    match cli.command {
        Command::Analyze {
            root,
            format,
            rule_filter,
            config,
        } => {
            let options = AnalyzeOptions {
                root,
                format: format.clone().into(),
                rule: None,
            };
            let report = match analyze_workspace_configured(
                &options,
                config.as_deref(),
                rule_filter.as_deref(),
            ) {
                Ok(report) => report,
                Err(BoundaryError::AnalyzerConfig(error)) => {
                    eprintln!("error: {}", analyzer_config_message(&error));
                    return Ok(ExitCode::from(analyzer_config_exit_code(&error)));
                }
                Err(error) => return Err(error.into()),
            };
            match OutputFormat::from(format) {
                OutputFormat::Text => {
                    println!("{}", render_findings_report(&report));
                }
                OutputFormat::Json => {
                    println!("{}", serde_json::to_string_pretty(&report)?);
                }
            }
        }
        Command::ExportGraph { root, format } => {
            let graph = export_workspace_graph(&ExportGraphOptions { root })?;
            match GraphOutputFormat::from(format) {
                GraphOutputFormat::Json => {
                    println!("{}", render_graph_export_json(&graph));
                }
                GraphOutputFormat::Turtle => {
                    println!("{}", render_graph_export_turtle(&graph));
                }
            }
        }
    }

    Ok(ExitCode::SUCCESS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analyzer_config_exit_codes_are_cli_owned() {
        assert_eq!(
            analyzer_config_exit_code(&AnalyzerConfigError::Selector {
                analyzer: "boundary",
                selector: "SCB-NOPE-001".to_owned(),
            }),
            2
        );
        assert_eq!(
            analyzer_config_exit_code(&AnalyzerConfigError::UnknownKey {
                analyzer: "boundary",
                key: "targets".to_owned(),
            }),
            3
        );
        assert_eq!(
            analyzer_config_message(&AnalyzerConfigError::Selector {
                analyzer: "boundary",
                selector: "SCB-NOPE-001".to_owned(),
            }),
            "boundary: unknown --rule-filter selector `SCB-NOPE-001`"
        );
    }
}
