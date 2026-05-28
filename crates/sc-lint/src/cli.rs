use std::path::PathBuf;

use clap::Parser;
use clap::Subcommand;
use clap::ValueEnum;
use serde::Serialize;

#[derive(Debug, Clone, Parser)]
#[command(name = "sc-lint")]
#[command(about = "Stable top-level CLI for the sc-lint tool family")]
pub struct Cli {
    #[arg(long, global = true)]
    pub json: bool,
    #[arg(long, global = true, value_name = "path")]
    pub root: Option<PathBuf>,
    #[arg(long, global = true, value_name = "path")]
    pub config: Option<PathBuf>,
    #[arg(long, global = true, value_name = "path")]
    pub log_root: Option<PathBuf>,
    #[arg(long, global = true)]
    pub log_console: bool,
    #[command(subcommand)]
    pub(crate) command: Command,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum LintProfile {
    Fast,
    Full,
    Ci,
}

impl LintProfile {
    pub const fn command_suffix(self) -> &'static str {
        match self {
            Self::Fast => "fast",
            Self::Full => "full",
            Self::Ci => "ci",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputMode {
    Human,
    Json,
}

impl OutputMode {
    pub const fn from_json_flag(json: bool) -> Self {
        if json { Self::Json } else { Self::Human }
    }

    pub const fn is_json(self) -> bool {
        matches!(self, Self::Json)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum LintTarget {
    #[value(name = "sc-boundary")]
    ScBoundary,
    #[value(name = "sc-portability")]
    ScPortability,
    #[value(name = "sc-runtime")]
    ScRuntime,
    #[value(name = "line-counts")]
    LineCounts,
    #[value(name = "identity-literals")]
    IdentityLiterals,
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
            Self::LineCounts => "line-counts",
            Self::IdentityLiterals => "identity-literals",
            Self::Fast => "fast",
            Self::Full => "full",
            Self::Ci => "ci",
        }
    }

    pub const fn profile(self) -> Option<LintProfile> {
        match self {
            Self::Fast => Some(LintProfile::Fast),
            Self::Full => Some(LintProfile::Full),
            Self::Ci => Some(LintProfile::Ci),
            Self::ScBoundary
            | Self::ScPortability
            | Self::ScRuntime
            | Self::LineCounts
            | Self::IdentityLiterals => None,
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

impl ViewTarget {}

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
