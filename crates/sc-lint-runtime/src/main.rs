use std::path::PathBuf;

use anyhow::Result;
use clap::CommandFactory;
use clap::Parser;
use clap::Subcommand;
use sc_lint_analyzer_support::AnalyzerConfigError;
use sc_lint_runtime::AnalyzeOptions;
use sc_lint_runtime::RuntimeError;
use sc_lint_runtime::analyze_workspace_configured;
use sc_lint_runtime::render_findings_report;
use sc_lint_schema::OutputFormat;

#[derive(Debug, Parser)]
#[command(name = "sc-lint-runtime")]
#[command(version)]
#[command(about = "AST-sensitive Rust runtime analyzer")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Analyze Rust source for runtime and async safety issues.
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
        /// Comma-separated rule IDs: SCB-RUNTIME-001 through SCB-RUNTIME-008. Rule families are not supported.
        #[arg(long, value_name = "SCB-RUNTIME-001..008[,..]")]
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

fn analyzer_config_exit_code(error: &AnalyzerConfigError) -> i32 {
    if error.is_selector() { 2 } else { 3 }
}

fn analyzer_config_message(error: &AnalyzerConfigError) -> String {
    match error {
        AnalyzerConfigError::Selector { analyzer, selector } => {
            format!(
                "{analyzer}: unknown --rule-filter selector `{selector}`; valid selectors are SCB-RUNTIME-001 through SCB-RUNTIME-008 (no rule families)"
            )
        }
        _ => error.to_string(),
    }
}

fn main() -> Result<()> {
    if std::env::args_os().len() == 1 {
        Cli::command().print_help()?;
        return Ok(());
    }
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
                    eprintln!("error: {}", analyzer_config_message(&error));
                    std::process::exit(analyzer_config_exit_code(&error));
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

#[cfg(test)]
mod tests {
    use super::*;

    fn normalize_line_endings(text: &str) -> String {
        text.replace("\r\n", "\n")
    }

    #[test]
    fn analyzer_config_exit_codes_are_cli_owned() {
        assert_eq!(
            analyzer_config_exit_code(&AnalyzerConfigError::Selector {
                analyzer: "runtime",
                selector: "SCB-RUNTIME-999".to_owned(),
            }),
            2
        );
        assert_eq!(
            analyzer_config_exit_code(&AnalyzerConfigError::UnknownKey {
                analyzer: "runtime",
                key: "unknown".to_owned(),
            }),
            3
        );
        let message = analyzer_config_message(&AnalyzerConfigError::Selector {
            analyzer: "runtime",
            selector: "SCB-RUNTIME-999".to_owned(),
        });
        assert!(message.contains("SCB-RUNTIME-999"));
        assert!(message.contains("no rule families"));
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
                "../../../docs-bundle/cli-reference/sc-lint-runtime.md"
            ))
        );
        assert_eq!(
            normalize_line_endings(&docli::render::html::render(&model)),
            normalize_line_endings(include_str!(
                "../../../docs-bundle/cli-reference/sc-lint-runtime/index.html"
            ))
        );
    }
}
