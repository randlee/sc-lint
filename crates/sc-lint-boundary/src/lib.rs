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
use sc_lint_analyzer_support::AnalyzerName;
use sc_lint_analyzer_support::CatalogRuleId;
use sc_lint_analyzer_support::RuleCatalog;
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
use serde::Serializer;
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
mod manifest_policy;
mod package_policy;
mod render;
#[cfg(test)]
mod tests;

const SC_LINT_SCHEMA_VERSION: &str = "0.2.0";
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[non_exhaustive]
pub enum RuleId {
    ScbCycle001,
    ScbCycle002,
    ScbCycle003,
    ScbBoundary001,
    ScbBoundary002,
    ScbBoundary003,
    ScbCaller001,
    ScbDependency001,
    ScbDependency002,
    ScbDependency003,
    ScbManifest001,
    ScbManifest002,
}

impl RuleId {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ScbCycle001 => "SCB-CYCLE-001",
            Self::ScbCycle002 => "SCB-CYCLE-002",
            Self::ScbCycle003 => "SCB-CYCLE-003",
            Self::ScbBoundary001 => "SCB-BOUNDARY-001",
            Self::ScbBoundary002 => "SCB-BOUNDARY-002",
            Self::ScbBoundary003 => "SCB-BOUNDARY-003",
            Self::ScbCaller001 => "SCB-CALLER-001",
            Self::ScbDependency001 => "SCB-DEPENDENCY-001",
            Self::ScbDependency002 => "SCB-DEPENDENCY-002",
            Self::ScbDependency003 => "SCB-DEPENDENCY-003",
            Self::ScbManifest001 => "SCB-MANIFEST-001",
            Self::ScbManifest002 => "SCB-MANIFEST-002",
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleFilter {
    Cycles,
    // SCB-CALLER-001 is enforced as part of boundary policy, so it routes
    // through the existing boundaries filter instead of a separate callers
    // variant.
    Boundaries,
    InternalOnly,
    ForbidExternalImpls,
    Dependencies,
    Manifests,
}

impl RuleFilter {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Cycles => "cycles",
            Self::Boundaries => "boundaries",
            Self::InternalOnly => "internal_only",
            Self::ForbidExternalImpls => "forbid_external_impls",
            Self::Dependencies => "dependencies",
            Self::Manifests => "manifests",
        }
    }
}

impl fmt::Display for RuleFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleFilterParseError {
    invalid_value: String,
}

impl RuleFilterParseError {
    fn new(invalid_value: impl Into<String>) -> Self {
        Self {
            invalid_value: invalid_value.into(),
        }
    }
}

impl fmt::Display for RuleFilterParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "unsupported rule filter `{}`; supported: cycles, boundaries, internal_only, forbid_external_impls, dependencies, manifests",
            self.invalid_value
        )
    }
}

impl std::error::Error for RuleFilterParseError {}

impl TryFrom<&str> for RuleFilter {
    type Error = RuleFilterParseError;

    fn try_from(value: &str) -> std::result::Result<Self, Self::Error> {
        match value {
            "cycles" => Ok(Self::Cycles),
            "boundaries" => Ok(Self::Boundaries),
            "internal_only" => Ok(Self::InternalOnly),
            "forbid_external_impls" => Ok(Self::ForbidExternalImpls),
            "dependencies" => Ok(Self::Dependencies),
            "manifests" => Ok(Self::Manifests),
            other => Err(RuleFilterParseError::new(other)),
        }
    }
}

const fn rule(id: RuleId) -> CatalogRuleId {
    CatalogRuleId::new(id.as_str())
}

pub const ALL_BOUNDARY_IDS: &[CatalogRuleId] = &[
    rule(RuleId::ScbCycle001),
    rule(RuleId::ScbCycle002),
    rule(RuleId::ScbCycle003),
    rule(RuleId::ScbBoundary001),
    rule(RuleId::ScbBoundary002),
    rule(RuleId::ScbBoundary003),
    rule(RuleId::ScbCaller001),
    rule(RuleId::ScbDependency001),
    rule(RuleId::ScbDependency002),
    rule(RuleId::ScbDependency003),
    rule(RuleId::ScbManifest001),
    rule(RuleId::ScbManifest002),
];

/// Every id except SCB-CYCLE-002/003: the set `analysis::finding_is_failure` fails on.
pub const DEFAULT_FAIL_IDS: &[CatalogRuleId] = &[
    rule(RuleId::ScbCycle001),
    rule(RuleId::ScbBoundary001),
    rule(RuleId::ScbBoundary002),
    rule(RuleId::ScbBoundary003),
    rule(RuleId::ScbCaller001),
    rule(RuleId::ScbDependency001),
    rule(RuleId::ScbDependency002),
    rule(RuleId::ScbDependency003),
    rule(RuleId::ScbManifest001),
    rule(RuleId::ScbManifest002),
];

/// Families keep today's CLI spellings (kebab-case, as clap rendered the old
/// `RuleFilterArg`); members are what each `RuleFilter` arm runs.
pub const BOUNDARY_CATALOG: RuleCatalog = RuleCatalog {
    analyzer: AnalyzerName::new("boundary"),
    rules: ALL_BOUNDARY_IDS,
    families: &[
        (
            "cycles",
            &[
                rule(RuleId::ScbCycle001),
                rule(RuleId::ScbCycle002),
                rule(RuleId::ScbCycle003),
            ],
        ),
        (
            "boundaries",
            &[
                rule(RuleId::ScbBoundary001),
                rule(RuleId::ScbBoundary002),
                rule(RuleId::ScbBoundary003),
                rule(RuleId::ScbCaller001),
            ],
        ),
        (
            "internal-only",
            &[rule(RuleId::ScbBoundary001), rule(RuleId::ScbBoundary002)],
        ),
        ("forbid-external-impls", &[rule(RuleId::ScbBoundary003)]),
        (
            "dependencies",
            &[
                rule(RuleId::ScbDependency001),
                rule(RuleId::ScbDependency002),
                rule(RuleId::ScbDependency003),
            ],
        ),
        (
            "manifests",
            &[rule(RuleId::ScbManifest001), rule(RuleId::ScbManifest002)],
        ),
    ],
    default_fail: DEFAULT_FAIL_IDS,
    own_keys: &[],
};

fn disabled_report() -> FindingsReport {
    FindingsReport {
        tool: SC_LINT_BOUNDARY_TOOL,
        version: SC_LINT_BOUNDARY_VERSION,
        schema_version: SC_LINT_SCHEMA_VERSION,
        status: ReportStatus::Pass,
        scanned_crates: 0,
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
            schema_version: SC_LINT_SCHEMA_VERSION,
            nodes: self.nodes,
            edges: self.edges,
        }
    }
}

pub fn analyze_workspace(
    options: &AnalyzeOptions,
) -> std::result::Result<FindingsReport, BoundaryError> {
    if options.rule == Some(RuleFilter::Manifests) {
        return analyze_manifests_only(options);
    }

    let root = options.root.as_path();
    let inventory = inventory::load_boundary_inventory(root)
        .map_err(|e| BoundaryError::inventory_load(root, e))?;
    let metadata =
        graph::load_metadata(root).map_err(|e| BoundaryError::inventory_load(root, e))?;
    inventory::validate_owner_crate_paths(&inventory, &metadata)
        .map_err(|e| BoundaryError::inventory_load(root, e))?;
    if options.rule == Some(RuleFilter::Dependencies) {
        let dependency_report = package_policy::analyze_package_policy(&metadata, &inventory)
            .map_err(|e| BoundaryError::package_policy(root, e))?;
        return Ok(findings_report(
            dependency_report.scanned_crates,
            dependency_report.findings,
        ));
    }
    let graph = graph::build_workspace_graph_from(&metadata)
        .map_err(|e| BoundaryError::graph_build(root, e))?;
    let mut findings = Vec::with_capacity(inventory.summary().recommended_finding_capacity());
    let filter = options.rule;
    use RuleFilter::{Boundaries, Cycles, ForbidExternalImpls, InternalOnly};
    if rule_selected(filter, &[Cycles]) {
        findings.extend(analysis::analyze_cycles(&graph));
    }
    if rule_selected(filter, &[Boundaries, InternalOnly]) {
        findings.extend(analysis::analyze_internal_only(&graph));
    }
    if rule_selected(filter, &[Boundaries, ForbidExternalImpls]) {
        findings.extend(analysis::analyze_forbid_external_impls(&graph));
    }
    if rule_selected(filter, &[Boundaries]) {
        findings.extend(analysis::analyze_named_callers(&graph, &inventory));
    }
    if filter.is_none() {
        let package_report = package_policy::analyze_package_policy(&metadata, &inventory)
            .map_err(|e| BoundaryError::package_policy(root, e))?;
        findings.extend(package_report.findings);
        let manifest_report = manifest_policy::analyze_manifest_policy(root, &metadata)
            .map_err(|e| BoundaryError::manifest_policy(root, e))?;
        findings.extend(manifest_report.findings);
    }
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
    Ok(findings_report(scanned_crates, findings))
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
        schema_version: SC_LINT_SCHEMA_VERSION,
        status,
        scanned_crates,
        findings,
    }
}

/// Reads `[tool.sc-lint.analyzers.boundary]` and `--rule-filter` (rule ids or
/// family names), preserving legacy fast paths for exact legacy families.
pub fn analyze_workspace_configured(
    options: &AnalyzeOptions,
    config: Option<&Path>,
    rule_filter: Option<&str>,
) -> std::result::Result<FindingsReport, BoundaryError> {
    let settings = load_analyzer_settings(&BOUNDARY_CATALOG, &options.root, config, rule_filter)?;
    if !settings.enabled {
        return Ok(disabled_report());
    }
    let mut options = options.clone();
    options.rule = legacy_family_rule_filter(settings.rule_filter.as_ref());
    let mut report = analyze_workspace(&options)?;
    let findings: Vec<Finding> = std::mem::take(&mut report.findings)
        .into_iter()
        .filter(|finding| {
            // Only manifest findings carry a path: their first owner id is the
            // repo-relative manifest path.
            !(matches!(
                finding.rule_id,
                RuleId::ScbManifest001 | RuleId::ScbManifest002
            ) && finding
                .owner_ids
                .first()
                .is_some_and(|owner| settings.path_excluded(owner.as_str())))
        })
        .collect();
    let (findings, status) = settings.apply(findings, |rule| rule.as_str());
    report.findings = findings;
    report.status = status;
    Ok(report)
}

fn legacy_family_rule_filter(selected: Option<&BTreeSet<CatalogRuleId>>) -> Option<RuleFilter> {
    let selected = selected?;
    [
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
