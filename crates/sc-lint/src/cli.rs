use std::path::PathBuf;

use clap::Parser;
use clap::Subcommand;
use clap::ValueEnum;

#[derive(Debug, Clone, Parser)]
#[command(name = "sc-lint")]
#[command(about = "Stable top-level CLI for the sc-lint tool family")]
#[command(disable_version_flag = true)]
pub struct Cli {
    #[arg(long, global = true)]
    pub json: bool,
    #[arg(long, global = true, value_name = "path")]
    pub root: Option<PathBuf>,
    #[arg(long, global = true, value_name = "path")]
    pub log_root: Option<PathBuf>,
    #[arg(long, global = true)]
    pub log_console: bool,
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Clone, Subcommand)]
pub enum Command {
    Lint {
        #[arg(value_enum)]
        target: LintTarget,
    },
    View {
        #[arg(value_enum)]
        target: ViewTarget,
    },
    Check {
        #[arg(value_enum)]
        target: CheckTarget,
    },
    Clippy {
        #[arg(value_enum)]
        target: ClippyTarget,
    },
    Version,
    Ci,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum LintTarget {
    #[value(name = "sc-boundary")]
    ScBoundary,
    #[value(name = "sc-portability")]
    ScPortability,
    #[value(name = "sc-runtime")]
    ScRuntime,
    #[value(name = "fast")]
    Fast,
    #[value(name = "full")]
    Full,
    #[value(name = "ci")]
    Ci,
}

impl LintTarget {
    pub const fn command_suffix(self) -> &'static str {
        match self {
            Self::ScBoundary => "sc-boundary",
            Self::ScPortability => "sc-portability",
            Self::ScRuntime => "sc-runtime",
            Self::Fast => "fast",
            Self::Full => "full",
            Self::Ci => "ci",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ViewTarget {
    #[value(name = "graph")]
    Graph,
    #[value(name = "findings")]
    Findings,
}

impl ViewTarget {
    pub const fn command_suffix(self) -> &'static str {
        match self {
            Self::Graph => "graph",
            Self::Findings => "findings",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum CheckTarget {
    #[value(name = "native")]
    Native,
    #[value(name = "xwin")]
    Xwin,
}

impl CheckTarget {
    pub const fn command_suffix(self) -> &'static str {
        match self {
            Self::Native => "native",
            Self::Xwin => "xwin",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ClippyTarget {
    #[value(name = "native")]
    Native,
    #[value(name = "xwin")]
    Xwin,
}

impl ClippyTarget {
    pub const fn command_suffix(self) -> &'static str {
        match self {
            Self::Native => "native",
            Self::Xwin => "xwin",
        }
    }
}
