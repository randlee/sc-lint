use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::Result;
use clap::Parser;
use clap::Subcommand;
use sc_lint_analyzer_support::AnalyzerConfigError;
use sc_lint_portability::AnalyzeOptions;
use sc_lint_portability::analyze_workspace_configured;
use sc_lint_portability::render_findings_report;
use sc_lint_schema::OutputFormat;

#[derive(Debug, Parser)]
#[command(name = "sc-lint-portability")]
#[command(version)]
#[command(about = "AST-sensitive Rust portability analyzer")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Analyze Rust source for portability issues.
    Analyze {
        /// Workspace root to inspect.
        #[arg(long, default_value = ".")]
        root: PathBuf,
        /// Findings output format.
        #[arg(long, default_value = "text")]
        format: FormatArg,
        /// Analyzer configuration file; defaults to discovery from the workspace root.
        #[arg(long)]
        config: Option<PathBuf>,
        /// Comma-separated PORT rule IDs; overrides configured excludes.
        #[arg(long, value_name = "PORT-RULE-ID[,..]")]
        rule_filter: Option<String>,
    },
}

#[derive(Debug, Clone, clap::ValueEnum)]
enum FormatArg {
    /// Human-readable findings.
    Text,
    /// JSON findings report.
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

fn analyzer_config_exit_code(error: &AnalyzerConfigError) -> u8 {
    if error.is_selector() { 2 } else { 3 }
}

fn analyzer_config_message(error: &AnalyzerConfigError) -> String {
    match error {
        AnalyzerConfigError::Selector { analyzer, selector } => {
            format!(
                "{analyzer}: unknown --rule-filter selector `{selector}`; valid rule IDs are PORT-001 through PORT-010"
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
                    eprintln!("error: {}", analyzer_config_message(&error));
                    return Ok(ExitCode::from(analyzer_config_exit_code(&error)));
                }
                Err(sc_lint_portability::PortabilityError::AdapterConfig(error)) => {
                    eprintln!("error: {error}");
                    return Ok(ExitCode::from(3));
                }
                Err(error @ sc_lint_portability::PortabilityError::InvalidSiteAllow { .. })
                | Err(error @ sc_lint_portability::PortabilityError::UnknownRule { .. }) => {
                    eprintln!("error: {error}");
                    return Ok(ExitCode::from(3));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analyzer_config_exit_codes_are_cli_owned() {
        assert_eq!(
            analyzer_config_exit_code(&AnalyzerConfigError::Selector {
                analyzer: "portability",
                selector: "PORT-999".to_owned(),
            }),
            2
        );
        assert_eq!(
            analyzer_config_exit_code(&AnalyzerConfigError::UnknownKey {
                analyzer: "portability",
                key: "unknown".to_owned(),
            }),
            3
        );
        let message = analyzer_config_message(&AnalyzerConfigError::Selector {
            analyzer: "portability",
            selector: "PORT-999".to_owned(),
        });
        assert!(message.contains("PORT-999"));
        assert!(message.contains("PORT-001 through PORT-010"));
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
            docli::render::markdown::render(&model),
            include_str!("../../../docs-bundle/cli-reference/sc-lint-portability.md")
        );
        assert_eq!(
            docli::render::html::render(&model),
            include_str!("../../../docs-bundle/cli-reference/sc-lint-portability/index.html")
        );
    }
}
