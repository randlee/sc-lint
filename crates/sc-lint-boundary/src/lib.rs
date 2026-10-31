use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fmt;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::sync::OnceLock;

use anyhow as anyhow_crate;
use anyhow::Context;
use anyhow::Result;
use quote::ToTokens;
use sc_lint_analyzer_support::CatalogRuleId;
use sc_lint_analyzer_support::RepoRelativePath;
use sc_lint_analyzer_support::load_analyzer_settings;
use sc_lint_directives::AttributeInput;
use sc_lint_directives::Directive;
pub use sc_lint_schema::CrateId;
use sc_lint_schema::NodeId;
use sc_lint_schema::OutputFormat;
use sc_lint_schema::OwnerId;
use sc_lint_schema::ReportStatus;
use serde::Deserialize;
use serde::Serialize;
use syn::Attribute;
use syn::File;
use syn::Ident;
use syn::ImplItem;
use syn::Item;
use syn::Receiver;
use syn::Type;
use syn::visit::Visit;
use thiserror::Error;

mod analysis;
mod graph;
mod inventory;
mod manifest_dependency_policy;
mod manifest_policy;
mod package_policy;
mod render;
mod rule_catalog;
#[cfg(test)]
mod tests;

use rule_catalog::rule;

const DEFAULT_RULES_TOML: &str = include_str!("../config/defaults.toml");
const SC_LINT_BOUNDARY_TOOL: &str = "sc-lint-boundary";
const SC_LINT_BOUNDARY_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("{0}")]
pub struct BoundaryErrorSource(Box<str>);

impl From<anyhow_crate::Error> for BoundaryErrorSource {
    fn from(value: anyhow_crate::Error) -> Self {
        Self(format!("{value:#}").into_boxed_str())
    }
}

#[derive(Debug, Error)]
pub enum BoundaryError {
    #[error("failed to load boundary inventory for root `{}`: {source:#}", root.display())]
    InventoryLoad {
        root: PathBuf,
        #[source]
        source: BoundaryErrorSource,
    },
    #[error("failed to analyze manifest policy for root `{}`: {source:#}", root.display())]
    ManifestPolicyAnalysis {
        root: PathBuf,
        #[source]
        source: BoundaryErrorSource,
    },
    #[error("failed to analyze package dependency policy for root `{}`: {source:#}", root.display())]
    PackagePolicyAnalysis {
        root: PathBuf,
        #[source]
        source: BoundaryErrorSource,
    },
    #[error("failed to build workspace graph for root `{}`: {source:#}", root.display())]
    WorkspaceGraphBuild {
        root: PathBuf,
        #[source]
        source: BoundaryErrorSource,
    },
    #[error(transparent)]
    AnalyzerConfig(#[from] sc_lint_analyzer_support::AnalyzerConfigError),
}

impl BoundaryError {
    fn inventory_load(root: &Path, source: impl Into<BoundaryErrorSource>) -> Self {
        Self::InventoryLoad {
            root: root.to_path_buf(),
            source: source.into(),
        }
    }

    fn manifest_policy(root: &Path, source: impl Into<BoundaryErrorSource>) -> Self {
        Self::ManifestPolicyAnalysis {
            root: root.to_path_buf(),
            source: source.into(),
        }
    }

    fn package_policy(root: &Path, source: impl Into<BoundaryErrorSource>) -> Self {
        Self::PackagePolicyAnalysis {
            root: root.to_path_buf(),
            source: source.into(),
        }
    }

    fn graph_build(root: &Path, source: impl Into<BoundaryErrorSource>) -> Self {
        Self::WorkspaceGraphBuild {
            root: root.to_path_buf(),
            source: source.into(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
struct RuleDefaults {
    trait_self_loop: TraitSelfLoopDefaults,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
struct TraitSelfLoopDefaults {
    ignored_trait_paths: Vec<String>,
    ignored_trait_names: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnalyzeOptions {
    pub root: PathBuf,
    pub format: OutputFormat,
    pub rule: Option<RuleFilter>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportGraphOptions {
    pub root: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphOutputFormat {
    Json,
    Turtle,
}

pub type FindingsReport = sc_lint_schema::FindingsReport<RuleId>;
pub type Finding = sc_lint_schema::Finding<RuleId>;

pub use rule_catalog::{
    ALL_BOUNDARY_IDS, BOUNDARY_CATALOG, DEFAULT_FAIL_IDS, RuleFilter, RuleFilterParseError, RuleId,
};

fn disabled_report() -> FindingsReport {
    FindingsReport {
        tool: SC_LINT_BOUNDARY_TOOL,
        version: SC_LINT_BOUNDARY_VERSION,
        schema_version: sc_lint_schema::ANALYZER_REPORT_SCHEMA_VERSION_V2,
        status: ReportStatus::Pass,
        scanned_crates: 0,
        allowed: Vec::new(),
        findings: Vec::new(),
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct GraphExport {
    pub tool: &'static str,
    pub version: &'static str,
    pub schema_version: &'static str,
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct GraphNode {
    pub id: NodeId,
    pub kind: &'static str,
    pub label: String,
    pub visibility: Option<&'static str>,
    pub package: String,
    pub target: Option<String>,
    pub manifest_path: String,
    pub source_path: Option<String>,
    pub module_path: Option<String>,
    pub impl_kind: Option<ImplKind>,
    pub impl_trait: Option<String>,
    pub attributes: Vec<LintAttribute>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct GraphEdge {
    pub kind: &'static str,
    pub from: NodeId,
    pub to: NodeId,
}

pub(crate) fn workspace_packages(
    metadata: &cargo_metadata::Metadata,
) -> Vec<&cargo_metadata::Package> {
    let workspace_members = metadata.workspace_members.iter().collect::<BTreeSet<_>>();
    metadata
        .packages
        .iter()
        .filter(|package| workspace_members.contains(&package.id))
        .collect()
}

pub(crate) fn is_library_target(target: &cargo_metadata::Target) -> bool {
    target.is_lib()
        || target.is_rlib()
        || target.is_dylib()
        || target.is_cdylib()
        || target.is_staticlib()
        || target.is_proc_macro()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EdgeKind {
    Contains,
    Targets,
    Implements,
    Declares,
    References,
    ReferencesType,
    ReferencesExpr,
}

impl EdgeKind {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Contains => "contains",
            Self::Targets => "targets",
            Self::Implements => "implements",
            Self::Declares => "declares",
            Self::References => "references",
            Self::ReferencesType => "references_type",
            Self::ReferencesExpr => "references_expr",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum ReferenceKind {
    Type,
    Expr,
}

impl ReferenceKind {
    fn edge_kind(self) -> EdgeKind {
        match self {
            Self::Type => EdgeKind::ReferencesType,
            Self::Expr => EdgeKind::ReferencesExpr,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ImplKind {
    Trait,
    Inherent,
}

impl ImplKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Trait => "trait",
            Self::Inherent => "inherent",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct CollectedReference {
    path: String,
    kind: ReferenceKind,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct LintAttribute {
    pub scope: &'static str,
    pub name: &'static str,
    pub values: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ModulePath(String);

impl ModulePath {
    fn crate_root() -> Self {
        Self("crate".to_string())
    }

    fn child(&self, name: &str) -> Self {
        Self(format!("{}::{name}", self.0))
    }

    fn as_str(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for ModulePath {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for ModulePath {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TargetContext {
    package_name: String,
    target_name: String,
    manifest_path: String,
    crate_id: CrateId,
    root_module_path: ModulePath,
    workspace_dependency_roots: BTreeMap<String, CrateId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ItemVisibility {
    Private,
    Public,
    Crate,
    Restricted,
}

impl ItemVisibility {
    fn as_str(self) -> &'static str {
        match self {
            Self::Private => "private",
            Self::Public => "public",
            Self::Crate => "crate",
            Self::Restricted => "restricted",
        }
    }
}

#[derive(Debug, Default)]
struct GraphBuilder {
    nodes: Vec<GraphNode>,
    edges: Vec<GraphEdge>,
    reference_impl_ids: BTreeSet<NodeId>,
}

impl GraphBuilder {
    fn add_node(&mut self, node: GraphNode) {
        debug_assert!(
            !node.package.is_empty(),
            "graph nodes must carry a non-empty package name"
        );
        if !self.nodes.iter().any(|existing| existing.id == node.id) {
            self.nodes.push(node);
        }
    }

    fn add_edge(&mut self, kind: EdgeKind, from: impl Into<NodeId>, to: impl Into<NodeId>) {
        let edge = GraphEdge {
            kind: kind.as_str(),
            from: from.into(),
            to: to.into(),
        };
        if !self.edges.contains(&edge) {
            self.edges.push(edge);
        }
    }

    fn add_workspace_target(
        &mut self,
        package_name: &str,
        manifest_path: &str,
        target_name: &str,
        source_path: &Path,
    ) {
        let crate_id = graph::crate_id(package_name, target_name);
        self.add_node(GraphNode {
            id: NodeId::new(String::from(crate_id)),
            kind: "crate",
            label: target_name.to_string(),
            visibility: None,
            package: package_name.to_string(),
            target: Some(target_name.to_string()),
            manifest_path: manifest_path.to_string(),
            source_path: Some(source_path.display().to_string()),
            module_path: Some("crate".to_string()),
            impl_kind: None,
            impl_trait: None,
            attributes: Vec::new(),
        });
    }

    fn finish(mut self) -> GraphExport {
        self.nodes.sort_by(|left, right| left.id.cmp(&right.id));
        self.edges.sort_by(|left, right| {
            left.kind
                .cmp(right.kind)
                .then_with(|| left.from.cmp(&right.from))
                .then_with(|| left.to.cmp(&right.to))
        });
        // Deferred method resolution can make previously distinct edges equal.
        self.edges.dedup();

        GraphExport {
            tool: SC_LINT_BOUNDARY_TOOL,
            version: SC_LINT_BOUNDARY_VERSION,
            schema_version: sc_lint_schema::ANALYZER_REPORT_SCHEMA_VERSION_V2,
            nodes: self.nodes,
            edges: self.edges,
        }
    }
}

pub fn analyze_workspace(
    options: &AnalyzeOptions,
) -> std::result::Result<FindingsReport, BoundaryError> {
    analyze_workspace_with_paths(options).map(|(report, _)| report)
}

/// Source file of every graph node, keyed by node id. Path-less findings
/// (SCB-CYCLE-001/002/003) resolve their files from it for path exclusions.
type NodeSourcePaths = BTreeMap<NodeId, PathBuf>;

/// `analyze_workspace` plus the source file of every graph node. The
/// manifest-only and dependency-only passes build no graph and return an
/// empty map.
pub(crate) fn analyze_workspace_with_paths(
    options: &AnalyzeOptions,
) -> std::result::Result<(FindingsReport, NodeSourcePaths), BoundaryError> {
    if options.rule == Some(RuleFilter::Manifests) {
        return Ok((analyze_manifests_only(options)?, NodeSourcePaths::new()));
    }

    let root = options.root.as_path();
    let inventory = inventory::load_boundary_inventory(root)
        .map_err(|e| BoundaryError::inventory_load(root, e))?;
    let metadata =
        graph::load_metadata(root).map_err(|e| BoundaryError::inventory_load(root, e))?;
    inventory::validate_owner_crate_paths(&inventory, &metadata)
        .map_err(|e| BoundaryError::inventory_load(root, e))?;
    let workspace_packages = workspace_packages(&metadata);
    let resolver =
        inventory::WorkspacePackageResolver::from_workspace_packages(&workspace_packages);
    let known_non_workspace_packages =
        known_non_workspace_packages(&metadata, &inventory, &workspace_packages);
    let validate_package_policy =
        options.rule.is_none() || options.rule == Some(RuleFilter::Dependencies);
    let (inventory, semantic_diagnostics) = inventory::validate_inventory(
        inventory,
        &resolver,
        &known_non_workspace_packages,
        validate_package_policy,
    )
    .map_err(|error| BoundaryError::package_policy(root, error))?;
    let semantic_findings = semantic_diagnostics
        .iter()
        .map(inventory::InventoryDiagnostic::finding)
        .collect::<Vec<_>>();
    if options.rule == Some(RuleFilter::Dependencies) {
        let dependency_report = package_policy::analyze_package_policy(&metadata, &inventory)
            .map_err(|e| BoundaryError::package_policy(root, e))?;
        let mut findings = semantic_findings;
        findings.extend(dependency_report.findings);
        return Ok((
            findings_report(dependency_report.scanned_crates, findings),
            NodeSourcePaths::new(),
        ));
    }
    let graph = graph::build_workspace_graph_from(&metadata)
        .map_err(|e| BoundaryError::graph_build(root, e))?;
    let mut findings = semantic_findings;
    findings.reserve(inventory.summary().recommended_finding_capacity());
    findings.extend(selected_graph_rule_findings(
        options.rule,
        root,
        &graph,
        &metadata,
        &inventory,
    )?);
    findings.sort_by(|left, right| {
        analysis::finding_sort_key(left)
            .cmp(&analysis::finding_sort_key(right))
            .then_with(|| left.message.cmp(&right.message))
    });
    let scanned_crates = graph
        .nodes
        .iter()
        .filter(|node| node.kind == "crate")
        .count();
    Ok((
        findings_report(scanned_crates, findings),
        node_source_paths(&graph),
    ))
}

/// Findings of every rule pass `filter` selects over the built graph; an
/// unfiltered run also adds the package and manifest policy passes.
fn selected_graph_rule_findings(
    filter: Option<RuleFilter>,
    root: &Path,
    graph: &GraphExport,
    metadata: &cargo_metadata::Metadata,
    inventory: &inventory::ValidatedInventory,
) -> std::result::Result<Vec<Finding>, BoundaryError> {
    use RuleFilter::{Boundaries, Cycles, ForbidExternalImpls, InternalOnly};
    let mut findings = Vec::new();
    if rule_selected(filter, &[Cycles]) {
        findings.extend(analysis::analyze_cycles(graph));
    }
    if rule_selected(filter, &[Boundaries, InternalOnly]) {
        findings.extend(analysis::analyze_internal_only(graph));
    }
    if rule_selected(filter, &[Boundaries, ForbidExternalImpls]) {
        findings.extend(analysis::analyze_forbid_external_impls(graph));
    }
    if rule_selected(filter, &[Boundaries]) {
        findings.extend(analysis::analyze_named_callers(
            graph,
            &inventory.as_raw_inventory(),
        ));
    }
    if filter.is_none() {
        let package_report = package_policy::analyze_package_policy(metadata, inventory)
            .map_err(|e| BoundaryError::package_policy(root, e))?;
        findings.extend(package_report.findings);
        let manifest_report = manifest_policy::analyze_manifest_policy(root, metadata)
            .map_err(|e| BoundaryError::manifest_policy(root, e))?;
        findings.extend(manifest_report.findings);
    }
    Ok(findings)
}

fn node_source_paths(graph: &GraphExport) -> NodeSourcePaths {
    graph
        .nodes
        .iter()
        .filter_map(|node| {
            let path = node.source_path.as_ref()?;
            Some((node.id.clone(), PathBuf::from(path)))
        })
        .collect()
}

/// An unfiltered run selects every rule; `--rule X` selects the passes that accept X.
fn rule_selected(filter: Option<RuleFilter>, accepted: &[RuleFilter]) -> bool {
    filter.is_none_or(|rule| accepted.contains(&rule))
}

/// `--rule manifests` needs only package metadata, not the inventory or graph.
fn analyze_manifests_only(
    options: &AnalyzeOptions,
) -> std::result::Result<FindingsReport, BoundaryError> {
    let root = options.root.as_path();
    let metadata = graph::load_metadata_without_deps(root)
        .map_err(|e| BoundaryError::manifest_policy(root, e))?;
    let manifest_report = manifest_policy::analyze_manifest_policy(root, &metadata)
        .map_err(|e| BoundaryError::manifest_policy(root, e))?;
    Ok(findings_report(
        manifest_report.scanned_crates,
        manifest_report.findings,
    ))
}

fn findings_report(scanned_crates: usize, findings: Vec<Finding>) -> FindingsReport {
    let status = if findings.iter().any(analysis::finding_is_failure) {
        ReportStatus::Fail
    } else {
        ReportStatus::Pass
    };
    FindingsReport {
        tool: SC_LINT_BOUNDARY_TOOL,
        version: SC_LINT_BOUNDARY_VERSION,
        schema_version: sc_lint_schema::ANALYZER_REPORT_SCHEMA_VERSION_V2,
        status,
        scanned_crates,
        findings,
        allowed: Vec::new(),
    }
}

/// Reads `[tool.sc-lint.analyzers.boundary]` and `--rule-filter` (rule ids or
/// family names). Exact full-family selections preserve legacy fast paths;
/// mixed and subset selections use full analysis before findings are filtered.
pub fn analyze_workspace_configured(
    options: &AnalyzeOptions,
    config: Option<&Path>,
    rule_filter: Option<&str>,
) -> std::result::Result<FindingsReport, BoundaryError> {
    let settings = load_analyzer_settings(&BOUNDARY_CATALOG, &options.root, config, rule_filter)?;
    if !settings.enabled {
        return Ok(disabled_report());
    }
    if settings.rule_filter.as_ref().is_some_and(|selected| {
        selected.len() == 1 && selected.contains(&rule(RuleId::ScbBoundary004))
    }) {
        return analyze_inventory_semantics(&options.root);
    }
    let policy_config = if manifest_policy_selected(&settings) {
        manifest_dependency_policy::parse_policy(&settings, settings.rule_filter.as_ref())?
    } else {
        manifest_dependency_policy::ManifestPolicyConfig::default()
    };
    let mut options = options.clone();
    options.rule = legacy_family_rule_filter(settings.rule_filter.as_ref());
    if inventory_free_manifest_policy_selected(&settings) {
        return analyze_inventory_free_manifest_policy(&options, &settings, &policy_config);
    }
    let (mut report, node_paths) = analyze_workspace_with_paths(&options)?;
    if manifest_policy_selected(&settings) && !policy_config.is_empty() {
        let metadata = graph::load_metadata(&options.root)
            .map_err(|error| BoundaryError::manifest_policy(&options.root, error))?;
        let inventory = if policy_config.requires_inventory() {
            Some(load_validated_inventory(&options.root, &metadata)?)
        } else {
            None
        };
        report.findings.extend(
            manifest_dependency_policy::analyze_policy(
                &options.root,
                &metadata,
                inventory.as_ref(),
                &policy_config,
            )
            .map_err(|error| BoundaryError::manifest_policy(&options.root, error))?,
        );
        report.findings.sort_by(|left, right| {
            analysis::finding_sort_key(left)
                .cmp(&analysis::finding_sort_key(right))
                .then_with(|| left.message.cmp(&right.message))
        });
        report.status = if report.findings.iter().any(analysis::finding_is_failure) {
            ReportStatus::Fail
        } else {
            ReportStatus::Pass
        };
    }
    Ok(apply_configured_rule_filter(
        report,
        &settings,
        &options.root,
        &node_paths,
    ))
}

fn analyze_inventory_free_manifest_policy(
    options: &AnalyzeOptions,
    settings: &sc_lint_analyzer_support::AnalyzerSettings,
    policy_config: &manifest_dependency_policy::ManifestPolicyConfig,
) -> std::result::Result<FindingsReport, BoundaryError> {
    let mut report = analyze_manifests_only(options)?;
    if !policy_config.is_empty() {
        let metadata = graph::load_metadata(&options.root)
            .map_err(|error| BoundaryError::manifest_policy(&options.root, error))?;
        report.findings.extend(
            manifest_dependency_policy::analyze_policy(
                &options.root,
                &metadata,
                None,
                policy_config,
            )
            .map_err(|error| BoundaryError::manifest_policy(&options.root, error))?,
        );
    }
    Ok(apply_configured_rule_filter(
        report,
        settings,
        options.root.as_path(),
        &NodeSourcePaths::new(),
    ))
}

fn apply_configured_rule_filter(
    report: FindingsReport,
    settings: &sc_lint_analyzer_support::AnalyzerSettings,
    root: &Path,
    node_paths: &NodeSourcePaths,
) -> FindingsReport {
    apply_configured_rule_filter_with_canonicalizer(report, settings, root, node_paths, |path| {
        dunce::canonicalize(path)
    })
}

fn apply_configured_rule_filter_with_canonicalizer(
    mut report: FindingsReport,
    settings: &sc_lint_analyzer_support::AnalyzerSettings,
    root: &Path,
    node_paths: &NodeSourcePaths,
    canonicalize: impl FnOnce(&Path) -> std::io::Result<PathBuf>,
) -> FindingsReport {
    let canonical_root = canonicalize(root).ok();
    let findings: Vec<_> = std::mem::take(&mut report.findings)
        .into_iter()
        .filter(|finding| {
            !path_less_finding_excluded(
                finding,
                settings,
                root,
                canonical_root.as_deref(),
                node_paths,
            )
        })
        .map(|finding| {
            let path = manifest_finding_path(&finding);
            (finding, path)
        })
        .collect();
    let (findings, status) =
        settings.apply(findings, |rule: &RuleId| CatalogRuleId::new(rule.as_str()));
    report.findings = findings;
    report.status = status;
    report
}

/// SCB-CYCLE-001/002/003 findings carry no path; they are excluded when any
/// source file of their nodes matches a global or per-rule path exclusion.
fn path_less_finding_excluded(
    finding: &Finding,
    settings: &sc_lint_analyzer_support::AnalyzerSettings,
    root: &Path,
    canonical_root: Option<&Path>,
    node_paths: &NodeSourcePaths,
) -> bool {
    if !matches!(
        finding.rule_id,
        RuleId::ScbCycle001 | RuleId::ScbCycle002 | RuleId::ScbCycle003
    ) {
        return false;
    }
    let paths = finding_source_paths(finding, root, canonical_root, node_paths);
    let paths: Vec<&Path> = paths.iter().map(PathBuf::as_path).collect();
    settings.excluded_by_any_path(CatalogRuleId::new(finding.rule_id.as_str()), &paths)
}

/// Repo-relative source files of a finding's nodes. Nodes absent from the
/// map or outside the root (also after canonicalising it, which covers
/// symlinked temp roots such as macOS `/var`) contribute nothing.
fn finding_source_paths(
    finding: &Finding,
    root: &Path,
    canonical_root: Option<&Path>,
    node_paths: &NodeSourcePaths,
) -> BTreeSet<PathBuf> {
    finding
        .node_ids
        .iter()
        .filter_map(|id| node_paths.get(id))
        .filter_map(|path| {
            path.strip_prefix(root)
                .ok()
                .or_else(|| canonical_root.and_then(|root| path.strip_prefix(root).ok()))
                .map(Path::to_path_buf)
        })
        .collect()
}

fn manifest_policy_selected(settings: &sc_lint_analyzer_support::AnalyzerSettings) -> bool {
    let manifest_policy_ids = [
        rule(RuleId::ScbManifest003),
        rule(RuleId::ScbManifest004),
        rule(RuleId::ScbManifest005),
        rule(RuleId::ScbManifest006),
    ];
    settings.rule_filter.as_ref().is_none_or(|selected| {
        manifest_policy_ids
            .iter()
            .any(|rule| selected.contains(rule))
    })
}

fn inventory_free_manifest_policy_selected(
    settings: &sc_lint_analyzer_support::AnalyzerSettings,
) -> bool {
    let inventory_free_rules = [
        rule(RuleId::ScbManifest001),
        rule(RuleId::ScbManifest002),
        rule(RuleId::ScbManifest004),
        rule(RuleId::ScbManifest005),
    ];
    settings.rule_filter.as_ref().is_some_and(|selected| {
        !selected.is_empty()
            && selected
                .iter()
                .all(|selected_rule| inventory_free_rules.contains(selected_rule))
    })
}

fn load_validated_inventory(
    root: &Path,
    metadata: &cargo_metadata::Metadata,
) -> std::result::Result<inventory::ValidatedInventory, BoundaryError> {
    let raw = inventory::load_boundary_inventory(root)
        .map_err(|error| BoundaryError::inventory_load(root, error))?;
    inventory::validate_owner_crate_paths(&raw, metadata)
        .map_err(|error| BoundaryError::inventory_load(root, error))?;
    let packages = workspace_packages(metadata);
    let resolver = inventory::WorkspacePackageResolver::from_workspace_packages(&packages);
    let known_non_workspace_packages = known_non_workspace_packages(metadata, &raw, &packages);
    let (inventory, _) =
        inventory::validate_inventory(raw, &resolver, &known_non_workspace_packages, true)
            .map_err(|error| BoundaryError::package_policy(root, error))?;
    Ok(inventory)
}

fn manifest_finding_path(finding: &Finding) -> Option<RepoRelativePath> {
    matches!(
        finding.rule_id,
        RuleId::ScbManifest001
            | RuleId::ScbManifest002
            | RuleId::ScbManifest003
            | RuleId::ScbManifest004
            | RuleId::ScbManifest005
            | RuleId::ScbManifest006
    )
    .then(|| finding.owner_ids.first())
    .flatten()
    .and_then(|owner| RepoRelativePath::parse(owner.as_str()).ok())
}

fn analyze_inventory_semantics(root: &Path) -> std::result::Result<FindingsReport, BoundaryError> {
    let inventory = inventory::load_boundary_inventory(root)
        .map_err(|error| BoundaryError::inventory_load(root, error))?;
    let metadata =
        graph::load_metadata(root).map_err(|error| BoundaryError::inventory_load(root, error))?;
    inventory::validate_owner_crate_paths(&inventory, &metadata)
        .map_err(|error| BoundaryError::inventory_load(root, error))?;
    let workspace_packages = workspace_packages(&metadata);
    let resolver =
        inventory::WorkspacePackageResolver::from_workspace_packages(&workspace_packages);
    let known_non_workspace_packages =
        known_non_workspace_packages(&metadata, &inventory, &workspace_packages);
    let (_, diagnostics) =
        inventory::validate_inventory(inventory, &resolver, &known_non_workspace_packages, false)
            .map_err(|error| BoundaryError::package_policy(root, error))?;
    Ok(findings_report(
        workspace_packages.len(),
        diagnostics
            .iter()
            .map(inventory::InventoryDiagnostic::finding)
            .collect(),
    ))
}

fn known_non_workspace_packages(
    metadata: &cargo_metadata::Metadata,
    inventory: &inventory::BoundaryInventory,
    workspace_packages: &[&cargo_metadata::Package],
) -> BTreeSet<String> {
    let workspace_package_names = workspace_packages
        .iter()
        .map(|package| package.name.to_string())
        .collect::<BTreeSet<_>>();
    metadata
        .packages
        .iter()
        .map(|package| package.name.to_string())
        .filter(|package| !workspace_package_names.contains(package))
        .chain(
            inventory
                .records
                .iter()
                .filter(|record| !record.status.state.requires_workspace_package())
                .map(|record| record.owner_package.to_string()),
        )
        .collect()
}

fn legacy_family_rule_filter(selected: Option<&BTreeSet<CatalogRuleId>>) -> Option<RuleFilter> {
    let selected = selected?;
    [
        (RuleFilter::Cycles, "cycles"),
        (RuleFilter::Boundaries, "boundaries"),
        (RuleFilter::InternalOnly, "internal-only"),
        (RuleFilter::ForbidExternalImpls, "forbid-external-impls"),
        (RuleFilter::Dependencies, "dependencies"),
        (RuleFilter::Manifests, "manifests"),
    ]
    .into_iter()
    .find_map(|(filter, family)| {
        BOUNDARY_CATALOG
            .families
            .iter()
            .find(|(name, _)| *name == family)
            .filter(|(_, rules)| {
                selected.len() == rules.len() && rules.iter().all(|rule| selected.contains(rule))
            })
            .map(|_| filter)
    })
}

pub fn export_workspace_graph(
    options: &ExportGraphOptions,
) -> std::result::Result<GraphExport, BoundaryError> {
    graph::build_workspace_graph(&options.root)
        .map_err(|e| BoundaryError::graph_build(&options.root, e))
}

pub fn render_findings_report(report: &FindingsReport) -> String {
    render::render_findings_report(report)
}

pub fn render_graph_export(graph: &GraphExport, format: GraphOutputFormat) -> String {
    render::render_graph_export(graph, format)
}

pub fn render_graph_export_json(graph: &GraphExport) -> String {
    render::render_graph_export_json(graph)
}

pub fn render_graph_export_turtle(graph: &GraphExport) -> String {
    render::render_graph_export_turtle(graph)
}
