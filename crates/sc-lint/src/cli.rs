use std::path::PathBuf;

use clap::Parser;
use clap::Subcommand;
use clap::ValueEnum;
use serde::Serialize;

use crate::profile_plan::Platform;
use crate::profile_plan::PlatformScope;

#[derive(Debug, Clone, Parser)]
#[command(name = "sc-lint")]
#[command(about = "Stable top-level CLI for the sc-lint tool family")]
#[command(
    after_long_help = "Documentation guides: `sc-lint docs` (overview), `installation` (alias: `setup`), `using-sc-lint`, `configuration`, `just-setup`, `ci`, `upgrade`, `troubleshooting`, `best-practices`, `cli-reference`, and one guide for each `sc-lint-*` package: `sc-lint`, `sc-lint-analyzer-support`, `sc-lint-attributes`, `sc-lint-boundary`, `sc-lint-directives`, `sc-lint-portability`, `sc-lint-runtime`, and `sc-lint-schema`. The overview accepts `README`, `README.md`, or `overview`. Use `sc-lint docs --path` for the installed bundle path."
)]
#[command(disable_version_flag = true)]
pub struct Cli {
    /// Write the stable JSON envelope to stdout (errors are written to stderr).
    #[arg(long, global = true)]
    pub json: bool,
    /// Print the sc-lint and analyzer versions.
    #[arg(long, short = 'V', global = true)]
    pub version: bool,
    /// Repository root used by lint, view, check, clippy, compatibility check, test, and CI.
    #[arg(long, global = true, value_name = "PATH")]
    pub root: Option<PathBuf>,
    /// Read the consumer configuration from this path, relative to the repository root when relative.
    #[arg(long, global = true, value_name = "PATH")]
    pub config: Option<PathBuf>,
    /// Override the configured command log directory.
    #[arg(long, global = true, value_name = "PATH")]
    pub log_root: Option<PathBuf>,
    /// Enable console logging for this command.
    #[arg(long, global = true)]
    pub log_console: bool,
    #[command(subcommand)]
    pub(crate) command: Option<Command>,
}

#[derive(Debug, Clone, Subcommand)]
pub enum Command {
    /// Run one analyzer, maintainer lint profile, or configured consumer lint profile.
    Lint {
        #[arg(value_enum, value_name = "TARGET")]
        target: LintTarget,
        /// Run a configured consumer profile. Supported only with `ci`.
        #[arg(long)]
        consumer: bool,
        /// Select a consumer lint profile or step (`all` runs every step in declaration order); requires `--consumer`.
        #[arg(long, requires = "consumer")]
        profile: Option<String>,
        /// Consumer CI platform scope: `host` runs unannotated and host-tagged steps; `all` also reports unsupported cross-platform steps; a named platform adds its steps. Requires `--consumer`.
        #[arg(long = "platform-scope", value_enum, requires = "consumer")]
        platform_scope: Option<PlatformScopeArg>,
        /// Run these analyzer rules (`<RULE-ID|family>[,...]`); forwarded unchanged to the selected analyzer.
        #[arg(long = "rule-filter", conflicts_with = "consumer")]
        rule_filter: Option<String>,
        /// Stop starting new cargo-lane and `after_all` steps after the first failure; running steps finish.
        #[arg(long = "fail-fast", requires = "consumer")]
        fail_fast: bool,
    },
    /// Show analyzer findings or the reserved graph view.
    View {
        #[arg(value_enum, value_name = "TARGET")]
        target: ViewTarget,
    },
    /// Run the native or Windows cross-target cargo check.
    Check {
        #[arg(value_enum, value_name = "TARGET")]
        target: CheckTarget,
    },
    /// Run the native or Windows cross-target cargo clippy analysis.
    Clippy {
        #[arg(value_enum, value_name = "TARGET")]
        target: ClippyTarget,
    },
    /// Check the configured consumer installation and version requirement.
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
    /// Create or verify the product-owned consumer integration files.
    Init {
        /// Generate the canonical thin Just integration.
        #[arg(long, required = true)]
        just: bool,
        /// Verify that the generated integration is current without changing files.
        #[arg(long)]
        check: bool,
        /// Report the changes that would be made without changing files.
        #[arg(long)]
        dry_run: bool,
    },
    /// Read an installed guide from the offline documentation bundle.
    #[command(
        after_help = "Guide aliases: `README.md` and `overview` select `README`; `setup` selects `installation`."
    )]
    Docs {
        /// Guide to print or resolve. Defaults to the overview (`README`).
        #[arg(value_enum)]
        guide: Option<DocsGuide>,
        /// Print the guide or bundle path without a label.
        #[arg(long)]
        path: bool,
    },
    /// Run the explicitly configured consumer test profile.
    Test {
        /// Selector to run, then the arguments appended to its command. Omitted
        /// runs `default` (every row of a `[[tool.sc-lint.test]]` list). Flags
        /// (`--fail-fast`, `--list`, `--config`, `--json`) must precede the
        /// selector; everything after it is forwarded verbatim. One positional,
        /// so clap stops parsing flags at the selector.
        #[arg(
            value_name = "SELECTOR [ARGS]",
            trailing_var_arg = true,
            allow_hyphen_values = true
        )]
        selector_and_args: Vec<String>,
        /// Print the declared selectors instead of running them.
        #[arg(long, conflicts_with_all = ["selector_and_args", "fail_fast"])]
        list: bool,
        /// Same as `lint --consumer --fail-fast`.
        #[arg(long = "fail-fast")]
        fail_fast: bool,
    },
    /// Print the product, backend, and contract versions.
    Version,
    /// Run the source-maintainer CI profile. Use `sc-lint lint ci --consumer` for a configured consumer profile.
    Ci,
}

/// Command-line spelling of a consumer `--platform-scope`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum PlatformScopeArg {
    #[value(name = "host")]
    Host,
    #[value(name = "all")]
    All,
    #[value(name = "linux")]
    Linux,
    #[value(name = "macos")]
    Macos,
    #[value(name = "windows")]
    Windows,
}

impl From<PlatformScopeArg> for PlatformScope {
    fn from(value: PlatformScopeArg) -> Self {
        match value {
            PlatformScopeArg::Host => Self::Host,
            PlatformScopeArg::All => Self::All,
            PlatformScopeArg::Linux => Self::Named(Platform::Linux),
            PlatformScopeArg::Macos => Self::Named(Platform::MacOS),
            PlatformScopeArg::Windows => Self::Named(Platform::Windows),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum DocsGuide {
    #[value(name = "README", aliases = ["README.md", "overview"])]
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
    #[value(name = "cli-reference")]
    CliReference,
    #[value(name = "sc-lint")]
    ScLint,
    #[value(name = "sc-lint-attributes")]
    ScLintAttributes,
    #[value(name = "sc-lint-analyzer-support")]
    ScLintAnalyzerSupport,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DocsGuideMetadata {
    pub(crate) name: &'static str,
    pub(crate) path: &'static str,
}

impl DocsGuide {
    pub(crate) const fn metadata(self) -> DocsGuideMetadata {
        match self {
            Self::Overview => DocsGuideMetadata {
                name: "README.md",
                path: "README.md",
            },
            Self::Installation => DocsGuideMetadata {
                name: "installation",
                path: "installation.md",
            },
            Self::UsingScLint => DocsGuideMetadata {
                name: "using-sc-lint",
                path: "using-sc-lint.md",
            },
            Self::Configuration => DocsGuideMetadata {
                name: "configuration",
                path: "configuration.md",
            },
            Self::JustSetup => DocsGuideMetadata {
                name: "just-setup",
                path: "just-setup.md",
            },
            Self::Ci => DocsGuideMetadata {
                name: "ci",
                path: "ci.md",
            },
            Self::Upgrade => DocsGuideMetadata {
                name: "upgrade",
                path: "upgrade.md",
            },
            Self::Troubleshooting => DocsGuideMetadata {
                name: "troubleshooting",
                path: "troubleshooting.md",
            },
            Self::BestPractices => DocsGuideMetadata {
                name: "best-practices",
                path: "best-practices.md",
            },
            Self::CliReference => DocsGuideMetadata {
                name: "cli-reference",
                path: "cli-reference/README.md",
            },
            Self::ScLint => DocsGuideMetadata {
                name: "sc-lint",
                path: "packages/sc-lint.md",
            },
            Self::ScLintAttributes => DocsGuideMetadata {
                name: "sc-lint-attributes",
                path: "packages/sc-lint-attributes.md",
            },
            Self::ScLintAnalyzerSupport => DocsGuideMetadata {
                name: "sc-lint-analyzer-support",
                path: "packages/sc-lint-analyzer-support.md",
            },
            Self::ScLintBoundary => DocsGuideMetadata {
                name: "sc-lint-boundary",
                path: "packages/sc-lint-boundary.md",
            },
            Self::ScLintDirectives => DocsGuideMetadata {
                name: "sc-lint-directives",
                path: "packages/sc-lint-directives.md",
            },
            Self::ScLintPortability => DocsGuideMetadata {
                name: "sc-lint-portability",
                path: "packages/sc-lint-portability.md",
            },
            Self::ScLintRuntime => DocsGuideMetadata {
                name: "sc-lint-runtime",
                path: "packages/sc-lint-runtime.md",
            },
            Self::ScLintSchema => DocsGuideMetadata {
                name: "sc-lint-schema",
                path: "packages/sc-lint-schema.md",
            },
        }
    }
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
    #[value(
        name = "sc-boundary",
        help = "Run the Rust dependency and manifest boundary analyzer"
    )]
    ScBoundary,
    #[value(
        name = "sc-portability",
        help = "Run the Rust cross-platform portability analyzer"
    )]
    ScPortability,
    #[value(
        name = "sc-runtime",
        help = "Run the Rust async and runtime-pattern analyzer"
    )]
    ScRuntime,
    #[value(
        name = "function-length",
        help = "Check configured function-length limits"
    )]
    FunctionLength,
    #[value(
        name = "line-counts",
        help = "Check configured source line-count limits"
    )]
    LineCounts,
    #[value(
        name = "identity-literals",
        help = "Check configured identity and sentinel literals"
    )]
    IdentityLiterals,
    #[value(name = "fast", help = "Run the source-maintainer fast lint profile")]
    Fast,
    #[value(
        name = "full",
        help = "Run the complete source-maintainer lint profile"
    )]
    Full,
    #[value(
        name = "ci",
        help = "Run the source-maintainer profile, or the configured consumer profile with `--consumer`"
    )]
    Ci,
}

impl LintTarget {
    pub const fn command_suffix(self) -> &'static str {
        match self {
            Self::ScBoundary => "sc-boundary",
            Self::ScPortability => "sc-portability",
            Self::ScRuntime => "sc-runtime",
            Self::FunctionLength => "function-length",
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
            | Self::FunctionLength
            | Self::LineCounts
            | Self::IdentityLiterals => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ViewTarget {
    #[value(name = "graph", help = "Reserved graph view; not implemented yet")]
    Graph,
    #[value(
        name = "findings",
        help = "Show findings through the installed Python adapter"
    )]
    Findings,
}

impl ViewTarget {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum CheckTarget {
    #[value(name = "native", help = "Run cargo check for the current host")]
    Native,
    #[value(
        name = "xwin",
        help = "Run cargo check for x86_64-pc-windows-msvc with cargo-xwin"
    )]
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
    #[value(name = "native", help = "Run cargo clippy for the current host")]
    Native,
    #[value(
        name = "xwin",
        help = "Run cargo clippy for x86_64-pc-windows-msvc with cargo-xwin"
    )]
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
