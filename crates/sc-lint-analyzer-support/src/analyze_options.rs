use std::path::PathBuf;

use sc_lint_schema::OutputFormat;

/// Rule-neutral inputs shared by the AST analyzers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnalyzeOptions {
    pub root: PathBuf,
    pub format: OutputFormat,
}
