use std::path::Path;
use std::path::PathBuf;

use anyhow as anyhow_crate;
use sc_lint_analyzer_support::AnalyzerConfigError;
use sc_lint_analyzer_support::AnalyzerName;
use sc_lint_analyzer_support::CatalogRuleId;
use sc_lint_analyzer_support::RuleCatalog;
use sc_lint_analyzer_support::WorkspaceMetadata;
use sc_lint_analyzer_support::analyzer_settings_from;
use sc_lint_analyzer_support::load_workspace_metadata;
use sc_lint_analyzer_support::read_config_document;
use sc_lint_directives::{DirectiveError, SiteRuleId};
use sc_lint_schema::CrateId;
use sc_lint_schema::NodeId;
use sc_lint_schema::OutputFormat;
use sc_lint_schema::OwnerId;
use serde::Serialize;
use serde::Serializer;
use thiserror::Error;

mod module_cfg;
mod platform_adapters;
mod portability;
mod predicates;
mod site_allow;
#[cfg(test)]
mod tests;

const DEFAULT_RULES_TOML: &str = include_str!("../config/defaults.toml");
const ALL_PORT_IDS: &[CatalogRuleId] = &[
    CatalogRuleId::new("PORT-001"),
    CatalogRuleId::new("PORT-002"),
    CatalogRuleId::new("PORT-003"),
    CatalogRuleId::new("PORT-004"),
    CatalogRuleId::new("PORT-005"),
    CatalogRuleId::new("PORT-006"),
    CatalogRuleId::new("PORT-007"),
    CatalogRuleId::new("PORT-008"),
    CatalogRuleId::new("PORT-009"),
    CatalogRuleId::new("PORT-010"),
];

pub const PORTABILITY_CATALOG: RuleCatalog = RuleCatalog {
    analyzer: AnalyzerName::new("portability"),
    rules: ALL_PORT_IDS,
    path_addressable_rules: ALL_PORT_IDS,
    families: &[],
    default_fail: ALL_PORT_IDS,
    own_keys: &[
        "config_home_env",
        "unix_path_prefixes",
        "targets",
        "platform_adapters",
    ],
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnalyzeOptions {
    pub root: PathBuf,
    pub format: OutputFormat,
}

pub type FindingsReport = sc_lint_schema::FindingsReport<RuleId>;
pub type Finding = sc_lint_schema::Finding<RuleId>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[non_exhaustive]
pub enum RuleId {
    Port001,
    Port002,
    Port003,
    Port004,
    Port005,
    Port006,
    Port007,
    Port008,
    Port009,
    Port010,
}

impl RuleId {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Port001 => "PORT-001",
            Self::Port002 => "PORT-002",
            Self::Port003 => "PORT-003",
            Self::Port004 => "PORT-004",
            Self::Port005 => "PORT-005",
            Self::Port006 => "PORT-006",
            Self::Port007 => "PORT-007",
            Self::Port008 => "PORT-008",
            Self::Port009 => "PORT-009",
            Self::Port010 => "PORT-010",
        }
    }
}

impl Serialize for RuleId {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

#[derive(Debug, Error)]
#[error(transparent)]
pub struct PortabilityErrorSource(Box<dyn std::error::Error + Send + Sync>);

impl From<anyhow_crate::Error> for PortabilityErrorSource {
    fn from(value: anyhow_crate::Error) -> Self {
        Self(value.into())
    }
}

#[derive(Debug, Error)]
pub enum PortabilityError {
    #[error("PORTABILITY.INVALID_SITE_ALLOW at {}:{line}: {error}", path.display())]
    InvalidSiteAllow {
        path: PathBuf,
        line: usize,
        error: DirectiveError,
    },
    #[error("PORTABILITY.UNKNOWN_RULE at {}:{line}: unsupported portability rule `{rule}`; recovery: use PORT-010 or remove the allowance", path.display())]
    UnknownRule {
        path: PathBuf,
        line: usize,
        rule: SiteRuleId,
    },
    #[error("failed to analyze portability findings for root `{}`: {source:#}", root.display())]
    AnalyzeFindings {
        root: PathBuf,
        #[source]
        source: PortabilityErrorSource,
    },
    #[error(transparent)]
    AnalyzerConfig(#[from] AnalyzerConfigError),
    #[error(transparent)]
    AdapterConfig(#[from] platform_adapters::AdapterConfigError),
    #[error("failed to scan platform adapter declarations under root `{}`: {source:#}", root.display())]
    AdapterScan {
        root: PathBuf,
        #[source]
        source: PortabilityErrorSource,
    },
}

pub fn analyze_workspace(
    options: &AnalyzeOptions,
) -> std::result::Result<FindingsReport, PortabilityError> {
    analyze_workspace_configured(options, None, None)
}

pub fn analyze_workspace_configured(
    options: &AnalyzeOptions,
    config: Option<&Path>,
    rule_filter: Option<&str>,
) -> std::result::Result<FindingsReport, PortabilityError> {
    analyze_workspace_configured_with_metadata_loader(
        options,
        config,
        rule_filter,
        load_workspace_metadata,
    )
}

fn analyze_workspace_configured_with_metadata_loader(
    options: &AnalyzeOptions,
    config: Option<&Path>,
    rule_filter: Option<&str>,
    mut load_metadata: impl FnMut(&Path) -> anyhow_crate::Result<WorkspaceMetadata>,
) -> std::result::Result<FindingsReport, PortabilityError> {
    let document =
        read_config_document(PORTABILITY_CATALOG.analyzer.as_str(), &options.root, config)?;
    let settings = analyzer_settings_from(&PORTABILITY_CATALOG, &document, rule_filter)?;
    let analyzer_keys = portability::AnalyzerPortabilityKeys::from_settings(&settings)?;
    portability::validate_legacy_portability_config(&document)?;
    let records = analyzer_keys.platform_adapter_records();
    let scan_error = |source: anyhow_crate::Error| {
        if records.is_empty() {
            PortabilityError::AnalyzeFindings {
                root: options.root.clone(),
                source: source.into(),
            }
        } else {
            PortabilityError::AdapterScan {
                root: options.root.clone(),
                source: source.into(),
            }
        }
    };
    let workspace = if settings.enabled || !records.is_empty() {
        Some(load_metadata(&options.root).map_err(scan_error)?)
    } else {
        None
    };
    let module_cfg = workspace
        .as_ref()
        .map(module_cfg::ModuleCfg::collect)
        .transpose()
        .map_err(scan_error)?
        .unwrap_or_default();
    let platform_adapters = platform_adapters::validate(&options.root, records, &module_cfg)?;
    let (findings, mut allowed, scanned_crates) =
        if let Some(workspace) = workspace.as_ref().filter(|_| settings.enabled) {
            let (findings, allowed) = portability::analyze_portability(
                &options.root,
                workspace,
                &document,
                &settings,
                &analyzer_keys,
                &platform_adapters,
                &module_cfg,
            )
            .map_err(|source| map_portability_analysis_error(source, &options.root))?;
            let scanned_crates = portability::count_scanned_crates(workspace);
            (findings, allowed, scanned_crates)
        } else {
            (Vec::new(), Vec::new(), 0)
        };
    let (findings, status) =
        settings.apply(findings, |rule_id| CatalogRuleId::new(rule_id.as_str()));
    allowed.retain(|entry| {
        let path = sc_lint_analyzer_support::RepoRelativePath::parse(entry.source_path())
            .expect("analyzer allowances already have validated repository-relative paths");
        settings.includes(CatalogRuleId::new(entry.rule_id().as_str()), Some(&path))
    });
    Ok(FindingsReport {
        tool: "sc-lint-portability",
        version: env!("CARGO_PKG_VERSION"),
        schema_version: if allowed.is_empty() {
            sc_lint_schema::ANALYZER_REPORT_SCHEMA_VERSION_V1
        } else {
            sc_lint_schema::ANALYZER_REPORT_SCHEMA_VERSION_V2
        },
        status,
        scanned_crates,
        findings,
        allowed,
    })
}

fn map_portability_analysis_error(source: anyhow_crate::Error, root: &Path) -> PortabilityError {
    match source.downcast::<site_allow::SiteAllowError>() {
        Ok(site_allow::SiteAllowError::InvalidDirective { path, line, error }) => {
            PortabilityError::InvalidSiteAllow { path, line, error }
        }
        Ok(site_allow::SiteAllowError::UnknownRule { path, line, rule }) => {
            PortabilityError::UnknownRule { path, line, rule }
        }
        Err(source) => PortabilityError::AnalyzeFindings {
            root: root.to_path_buf(),
            source: source.into(),
        },
    }
}

pub fn render_findings_report(report: &FindingsReport) -> String {
    sc_lint_analyzer_support::render_findings_report(report)
}
