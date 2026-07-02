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

const SC_LINT_SCHEMA_VERSION: &str = "0.1.0";
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
        let manifest_report =
            manifest_policy::analyze_manifest_policy(&options.root).map_err(|source| {
                BoundaryError::ManifestPolicyAnalysis {
                    root: options.root.clone(),
                    source: source.into(),
                }
            })?;
        let status = if manifest_report
            .findings
            .iter()
            .any(analysis::finding_is_failure)
        {
            ReportStatus::Fail
        } else {
            ReportStatus::Pass
        };
        return Ok(FindingsReport {
            tool: SC_LINT_BOUNDARY_TOOL,
            version: SC_LINT_BOUNDARY_VERSION,
            schema_version: SC_LINT_SCHEMA_VERSION,
            status,
            scanned_crates: manifest_report.scanned_crates,
            findings: manifest_report.findings,
        });
    }

    let inventory = inventory::load_boundary_inventory(&options.root).map_err(|source| {
        BoundaryError::InventoryLoad {
            root: options.root.clone(),
            source: source.into(),
        }
    })?;
    if options.rule == Some(RuleFilter::Dependencies) {
        let metadata = graph::load_metadata(&options.root).map_err(|source| {
            BoundaryError::PackagePolicyAnalysis {
                root: options.root.clone(),
                source: source.into(),
            }
        })?;
        let dependency_report = package_policy::analyze_package_policy(&metadata, &inventory)
            .map_err(|source| BoundaryError::PackagePolicyAnalysis {
                root: options.root.clone(),
                source: source.into(),
            })?;
        let status = if dependency_report
            .findings
            .iter()
            .any(analysis::finding_is_failure)
        {
            ReportStatus::Fail
        } else {
            ReportStatus::Pass
        };
        return Ok(FindingsReport {
            tool: SC_LINT_BOUNDARY_TOOL,
            version: SC_LINT_BOUNDARY_VERSION,
            schema_version: SC_LINT_SCHEMA_VERSION,
            status,
            scanned_crates: dependency_report.scanned_crates,
            findings: dependency_report.findings,
        });
    }
    let graph = graph::build_workspace_graph(&options.root).map_err(|source| {
        BoundaryError::WorkspaceGraphBuild {
            root: options.root.clone(),
            source: source.into(),
        }
    })?;
    let inventory_summary = inventory.summary();
    let mut findings = Vec::with_capacity(inventory_summary.recommended_finding_capacity());
    let filter = options.rule;
    if filter.is_none() || filter == Some(RuleFilter::Cycles) {
        findings.extend(analysis::analyze_cycles(&graph));
    }
    if filter.is_none()
        || filter == Some(RuleFilter::Boundaries)
        || filter == Some(RuleFilter::InternalOnly)
    {
        findings.extend(analysis::analyze_internal_only(&graph));
    }
    if filter.is_none()
        || filter == Some(RuleFilter::Boundaries)
        || filter == Some(RuleFilter::ForbidExternalImpls)
    {
        findings.extend(analysis::analyze_forbid_external_impls(&graph));
    }
    if filter.is_none() || filter == Some(RuleFilter::Boundaries) {
        findings.extend(analysis::analyze_named_callers(&graph, &inventory));
    }
    if filter.is_none() {
        let metadata = graph::load_metadata(&options.root).map_err(|source| {
            BoundaryError::PackagePolicyAnalysis {
                root: options.root.clone(),
                source: source.into(),
            }
        })?;
        findings.extend(
            package_policy::analyze_package_policy(&metadata, &inventory)
                .map_err(|source| BoundaryError::PackagePolicyAnalysis {
                    root: options.root.clone(),
                    source: source.into(),
                })?
                .findings,
        );
    }
    if filter.is_none() {
        findings.extend(
            manifest_policy::analyze_manifest_policy(&options.root)
                .map_err(|source| BoundaryError::ManifestPolicyAnalysis {
                    root: options.root.clone(),
                    source: source.into(),
                })?
                .findings,
        );
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
    let status = if findings.iter().any(analysis::finding_is_failure) {
        ReportStatus::Fail
    } else {
        ReportStatus::Pass
    };

    Ok(FindingsReport {
        tool: SC_LINT_BOUNDARY_TOOL,
        version: SC_LINT_BOUNDARY_VERSION,
        schema_version: SC_LINT_SCHEMA_VERSION,
        status,
        scanned_crates,
        findings,
    })
}

pub fn export_workspace_graph(
    options: &ExportGraphOptions,
) -> std::result::Result<GraphExport, BoundaryError> {
    graph::build_workspace_graph(&options.root).map_err(|source| {
        BoundaryError::WorkspaceGraphBuild {
            root: options.root.clone(),
            source: source.into(),
        }
    })
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
