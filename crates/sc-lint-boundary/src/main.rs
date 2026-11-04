use std::io::{self, Write};
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
#[command(version)]
#[command(about = "AST-sensitive Rust boundary analyzer")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Analyze Rust dependencies and manifest policy in a workspace.
    Analyze {
        /// Workspace root to inspect.
        #[arg(long, default_value = ".")]
        root: PathBuf,
        /// Findings output format.
        #[arg(long, default_value = "text")]
        format: FormatArg,
        /// Comma-separated rule IDs or supported rule families; overrides configured excludes.
        #[arg(
            long = "rule-filter",
            alias = "rule",
            value_name = "RULE-ID|FAMILY[,..]"
        )]
        rule_filter: Option<String>,
        /// Analyzer configuration file; defaults to discovery from the workspace root.
        #[arg(long)]
        config: Option<PathBuf>,
    },
    /// Export the workspace dependency graph.
    ExportGraph {
        /// Workspace root to inspect.
        #[arg(long, default_value = ".")]
        root: PathBuf,
        /// Graph serialization format.
        #[arg(long, default_value = "json")]
        format: GraphFormatArg,
    },
}

#[derive(Debug, Clone, clap::ValueEnum)]
enum FormatArg {
    /// Human-readable findings.
    Text,
    /// JSON findings report.
    Json,
}

#[derive(Debug, Clone, clap::ValueEnum)]
enum GraphFormatArg {
    /// Export the graph as JSON.
    Json,
    /// Export the graph as Turtle.
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
    if error.is_selector() { 2 } else { 3 }
}

fn analyzer_config_message(error: &AnalyzerConfigError) -> String {
    match error {
        AnalyzerConfigError::Selector { analyzer, selector } => {
            format!(
                "{analyzer}: unknown --rule-filter selector `{selector}`; valid selectors include rule IDs and families: cycles, boundaries, internal-only, forbid-external-impls, dependencies, manifests"
            )
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
                    write_stdout(&render_findings_report(&report))?;
                }
                OutputFormat::Json => {
                    write_stdout(&serde_json::to_string_pretty(&report)?)?;
                }
            }
        }
        Command::ExportGraph { root, format } => {
            let graph = export_workspace_graph(&ExportGraphOptions { root })?;
            match GraphOutputFormat::from(format) {
                GraphOutputFormat::Json => {
                    write_stdout(&render_graph_export_json(&graph))?;
                }
                GraphOutputFormat::Turtle => {
                    write_stdout(&render_graph_export_turtle(&graph))?;
                }
            }
        }
    }

    Ok(ExitCode::SUCCESS)
}

fn write_stdout(value: &str) -> io::Result<()> {
    let mut stdout = io::stdout().lock();
    if let Err(error) = writeln!(stdout, "{value}")
        && error.kind() != io::ErrorKind::BrokenPipe
    {
        return Err(error);
    }
    if let Err(error) = stdout.flush()
        && error.kind() != io::ErrorKind::BrokenPipe
    {
        return Err(error);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn normalize_line_endings(text: &str) -> String {
        text.replace("\r\n", "\n")
    }

    #[test]
    fn analyze_cli_parses_rule_alias_rule_filter_and_config() {
        let alias = Cli::try_parse_from([
            "sc-lint-boundary",
            "analyze",
            "--rule",
            "cycles",
            "--config",
            "sc-lint.toml",
        ])
        .expect("--rule and --config should parse");
        let Command::Analyze {
            rule_filter,
            config,
            ..
        } = alias.command
        else {
            panic!("analyze command should parse");
        };
        assert_eq!(rule_filter.as_deref(), Some("cycles"));
        assert_eq!(config, Some(PathBuf::from("sc-lint.toml")));

        let canonical = Cli::try_parse_from([
            "sc-lint-boundary",
            "analyze",
            "--rule-filter",
            "SCB-CYCLE-001",
            "--config",
            "boundary.toml",
        ])
        .expect("--rule-filter and --config should parse");
        let Command::Analyze {
            rule_filter,
            config,
            ..
        } = canonical.command
        else {
            panic!("analyze command should parse");
        };
        assert_eq!(rule_filter.as_deref(), Some("SCB-CYCLE-001"));
        assert_eq!(config, Some(PathBuf::from("boundary.toml")));
    }

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
        let message = analyzer_config_message(&AnalyzerConfigError::Selector {
            analyzer: "boundary",
            selector: "SCB-NOPE-001".to_owned(),
        });
        assert!(message.contains("SCB-NOPE-001"));
        assert!(message.contains("valid selectors"));
    }

    #[test]
    fn cli_reference_matches_real_clap_definition() {
        use clap::CommandFactory;
        let model = docli::from_clap(&Cli::command());
        if let Ok(path) = std::env::var("SC_DOCLI_MODEL_OUT") {
            std::fs::write(
                path,
                serde_json::to_vec_pretty(&model).expect("serialize cli model"),
            )
            .expect("write cli model");
            return;
        }
        assert_eq!(
            normalize_line_endings(&docli::render::markdown::render(&model)),
            normalize_line_endings(include_str!(
                "../../../docs-bundle/cli-reference/sc-lint-boundary.md"
            ))
        );
        assert_eq!(
            normalize_line_endings(&docli::render::html::render(&model)),
            normalize_line_endings(include_str!(
                "../../../docs-bundle/cli-reference/sc-lint-boundary/index.html"
            ))
        );
    }
}
