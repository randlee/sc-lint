use std::path::PathBuf;

use clap::Parser;
use clap::Subcommand;
use clap::ValueEnum;
use serde::Serialize;

#[derive(Debug, Clone, Parser)]
#[command(name = "sc-lint")]
#[command(about = "Stable top-level CLI for the sc-lint tool family")]
#[command(
    after_long_help = "Documentation guides: `sc-lint docs` (overview), `installation`, `using-sc-lint`, `configuration`, `just-setup`, `ci`, `upgrade`, `troubleshooting`, `best-practices`, and one guide for each `sc-lint-*` package: `sc-lint`, `sc-lint-attributes`, `sc-lint-boundary`, `sc-lint-directives`, `sc-lint-portability`, `sc-lint-runtime`, and `sc-lint-schema`. Use `sc-lint docs --path` for the installed bundle path."
)]
#[command(disable_version_flag = true)]
pub struct Cli {
    #[arg(long, global = true)]
    pub json: bool,
    #[arg(long, global = true)]
    pub version: bool,
    #[arg(long, global = true, value_name = "path")]
    pub root: Option<PathBuf>,
    #[arg(long, global = true, value_name = "path")]
    pub config: Option<PathBuf>,
    #[arg(long, global = true, value_name = "path")]
    pub log_root: Option<PathBuf>,
    #[arg(long, global = true)]
    pub log_console: bool,
    #[command(subcommand)]
    pub(crate) command: Option<Command>,
}

#[derive(Debug, Clone, Subcommand)]
pub enum Command {
    Lint {
        #[arg(value_enum)]
        target: LintTarget,
        /// Run the explicitly configured consumer profile instead of the source-maintainer profile.
        #[arg(long)]
        consumer: bool,
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
    Compatibility {
        #[command(subcommand)]
        command: CompatibilityCommand,
    },
    /// Install or repair the managed sc-lint release required by sc-lint.toml.
    Setup {
        /// Report the selected release and install location without changing either.
        #[arg(long)]
        dry_run: bool,
    },
    /// Inspect or update the managed sc-lint release required by sc-lint.toml.
    Upgrade {
        /// Report whether the configured minimum version requires an update.
        #[arg(long)]
        check: bool,
        /// Report the selected release and install location without changing either.
        #[arg(long)]
        dry_run: bool,
    },
    /// Materialize the product-owned consumer integration files.
    Init {
        /// Generate the canonical thin Just integration.
        #[arg(long)]
        just: bool,
        /// Verify that the generated integration is current without changing files.
        #[arg(long)]
        check: bool,
        /// Report the changes that would be made without changing files.
        #[arg(long)]
        dry_run: bool,
    },
    /// Discover and print the installed offline documentation bundle.
    Docs {
        /// Guide to print or resolve. Without a guide, print the overview.
        #[arg(value_enum)]
        guide: Option<DocsGuide>,
        /// Print the installed filesystem path instead of guide contents.
        #[arg(long)]
        path: bool,
    },
    /// Run the complete explicitly configured consumer test profile.
    Test,
    Version,
    Ci,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum DocsGuide {
    #[value(name = "README.md")]
    Overview,
    #[value(name = "installation", alias = "setup")]
    Installation,
    #[value(name = "using-sc-lint")]
    UsingScLint,
    Configuration,
    #[value(name = "just-setup")]
    JustSetup,
    Ci,
    Upgrade,
    Troubleshooting,
    #[value(name = "best-practices")]
    BestPractices,
    #[value(name = "sc-lint")]
    ScLint,
    #[value(name = "sc-lint-attributes")]
    ScLintAttributes,
    #[value(name = "sc-lint-boundary")]
    ScLintBoundary,
    #[value(name = "sc-lint-directives")]
    ScLintDirectives,
    #[value(name = "sc-lint-portability")]
    ScLintPortability,
    #[value(name = "sc-lint-runtime")]
    ScLintRuntime,
    #[value(name = "sc-lint-schema")]
    ScLintSchema,
}

#[derive(Debug, Clone, Subcommand)]
pub enum CompatibilityCommand {
    /// Verify the installed sc-lint binary satisfies sc-lint.toml.
    Check {
        /// Binary to probe. Defaults to `sc-lint` resolved from PATH.
        #[arg(long, value_name = "path")]
        binary: Option<PathBuf>,
    },
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
