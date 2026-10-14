use super::*;
use crate::inventory::BoundaryInventory;
use crate::inventory::BoundaryRecord;
use crate::inventory::CallersSection;
use crate::inventory::ReferenceScope;

pub(crate) fn analyze_cycles(graph: &GraphExport) -> Vec<Finding> {
    let node_map: BTreeMap<_, _> = graph
        .nodes
        .iter()
        .map(|node| (node.id.clone(), node))
        .collect();
    let owner_graph = build_owner_graph(graph, &node_map);
    let sccs = strongly_connected_components(&owner_graph);

    let mut findings = Vec::new();
    for component in sccs {
        if component.len() > 1 {
            if !component_allows_recursive_value_container(&component, &node_map) {
                findings.push(multi_owner_cycle_finding(&component, &owner_graph));
            }
            continue;
        }

        let owner_id = &component[0];
        if let Some(owner_edges) = owner_graph.self_refs.get(owner_id) {
            findings.extend(self_loop_findings(owner_id, owner_edges, &node_map));
        }
    }

    findings
}

fn multi_owner_cycle_finding(component: &[OwnerId], owner_graph: &OwnerGraph) -> Finding {
    let mut owners = component.to_vec();
    owners.sort();
    Finding {
        rule_id: RuleId::ScbCycle001,
        kind: "multi_owner_architectural_cycle".into(),
        message: format!(
            "architectural cycle across owners: {}",
            owners
                .iter()
                .map(OwnerId::as_str)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        owner_ids: owners.clone(),
        node_ids: owner_contributors(&owners, owner_graph),
    }
}

/// Node ids a single owner's self-referencing sources contribute, split into
/// inherent type/method self-loops and trait-impl self-loops keyed by trait.
#[derive(Default)]
struct SelfLoopNodes {
    inherent: BTreeSet<NodeId>,
    by_trait: BTreeMap<String, BTreeSet<NodeId>>,
}

fn self_loop_findings(
    owner_id: &OwnerId,
    owner_edges: &[OwnerRefEdge],
    node_map: &BTreeMap<NodeId, &GraphNode>,
) -> Vec<Finding> {
    let mut per_source: BTreeMap<NodeId, Vec<&OwnerRefEdge>> = BTreeMap::new();
    for edge in owner_edges
        .iter()
        .filter(|edge| !is_delegation_edge(edge, owner_edges, node_map))
    {
        per_source
            .entry(edge.source_node_id.clone())
            .or_default()
            .push(edge);
    }

    let mut nodes = SelfLoopNodes::default();
    for (source_node_id, source_edges) in per_source {
        record_self_loop_source(&source_node_id, &source_edges, node_map, &mut nodes);
    }

    let mut findings = Vec::new();
    if !nodes.inherent.is_empty() {
        findings.push(Finding {
            rule_id: RuleId::ScbCycle002,
            kind: "type_method_self_loop".into(),
            message: format!("type/method self-loop on owner {owner_id}"),
            owner_ids: vec![owner_id.clone()],
            node_ids: nodes.inherent.into_iter().collect(),
        });
    }
    for (trait_name, node_ids) in nodes.by_trait {
        findings.push(Finding {
            rule_id: RuleId::ScbCycle003,
            kind: "trait_impl_self_loop".into(),
            message: format!("trait-impl self-loop on owner {owner_id} via {trait_name}"),
            owner_ids: vec![owner_id.clone()],
            node_ids: node_ids.into_iter().collect(),
        });
    }
    findings
}

fn record_self_loop_source(
    source_node_id: &NodeId,
    source_edges: &[&OwnerRefEdge],
    node_map: &BTreeMap<NodeId, &GraphNode>,
    nodes: &mut SelfLoopNodes,
) {
    let allow_rule = source_edges.iter().any(|edge| {
        edge.node_ids.iter().any(|node_id| {
            node_map
                .get(node_id)
                .map(|node| graph::node_has_allow_rule(node, "cycle.type_method_self_loop"))
                .unwrap_or(false)
        })
    });
    if allow_rule {
        return;
    }
    let edge_node_ids = || {
        source_edges
            .iter()
            .flat_map(|edge| edge.node_ids.iter().cloned())
    };

    let is_type_method_self_loop = source_edges.iter().all(|edge| {
        edge.source_kind == "method"
            && edge.owner_kind == "type"
            && edge.target_owner_id == edge.source_owner_id
            && edge.source_impl_kind != Some(ImplKind::Trait)
    });
    if is_type_method_self_loop {
        let has_expr_ref = source_edges
            .iter()
            .any(|edge| edge.reference_kind == ReferenceKind::Expr);
        let has_type_ref = source_edges
            .iter()
            .any(|edge| edge.reference_kind == ReferenceKind::Type);
        if has_expr_ref && !has_type_ref {
            nodes.inherent.extend(edge_node_ids());
        }
        return;
    }

    let is_trait_impl_self_loop = source_edges.iter().all(|edge| {
        edge.source_kind == "method"
            && edge.owner_kind == "type"
            && edge.target_owner_id == edge.source_owner_id
            && edge.source_impl_kind == Some(ImplKind::Trait)
    });
    if is_trait_impl_self_loop {
        let trait_name = node_map
            .get(source_node_id)
            .and_then(|node| node.impl_trait.as_deref())
            .unwrap_or("unknown_trait");
        if !is_non_architectural_trait_impl_self_loop(trait_name) {
            nodes
                .by_trait
                .entry(trait_name.to_string())
                .or_default()
                .extend(edge_node_ids());
        }
    }
}

fn component_allows_recursive_value_container(
    owners: &[OwnerId],
    node_map: &BTreeMap<NodeId, &GraphNode>,
) -> bool {
    owners.iter().all(|owner_id| {
        node_map
            .get(&NodeId::new(owner_id.as_str()))
            .map(|node| {
                node.kind == "type"
                    && graph::node_has_allow_rule(node, "cycle.recursive_value_container")
            })
            .unwrap_or(false)
    })
}

pub(crate) fn is_non_architectural_trait_impl_self_loop(trait_path: &str) -> bool {
    let defaults = &graph::default_rule_defaults().trait_self_loop;
    defaults
        .ignored_trait_paths
        .iter()
        .any(|ignored| ignored == trait_path)
        || defaults
            .ignored_trait_names
            .iter()
            .any(|ignored| ignored == graph::trait_terminal_name(trait_path))
}

pub(crate) fn analyze_internal_only(graph: &GraphExport) -> Vec<Finding> {
    let node_map: BTreeMap<_, _> = graph
        .nodes
        .iter()
        .map(|node| (node.id.clone(), node))
        .collect();
    let mut findings = Vec::new();

    for node in &graph.nodes {
        let has_internal_only = node
            .attributes
            .iter()
            .any(|attr| attr.scope == "boundary" && attr.name == "internal_only");
        if !has_internal_only {
            continue;
        }

        if node.visibility != Some("private") {
            findings.push(Finding {
                rule_id: RuleId::ScbBoundary001,
                kind: "internal_only_visibility_violation".into(),
                message: format!(
                    "internal_only item {} must not be externally visible (visibility={})",
                    node.id.as_str(),
                    node.visibility.unwrap_or("unknown")
                ),
                owner_ids: vec![OwnerId::new(node.id.as_str())],
                node_ids: vec![node.id.clone()],
            });
        }

        let target_module_path = node.module_path.clone();
        let mut seen_external_sources = BTreeSet::new();
        for edge in graph.edges.iter().filter(|edge| {
            matches!(edge.kind, "references_type" | "references_expr") && edge.to == node.id
        }) {
            let Some(source_node) = node_map.get(&edge.from) else {
                continue;
            };
            if source_node.id == node.id {
                continue;
            }
            if source_node.module_path == target_module_path {
                continue;
            }
            let source_owner_id = owner_id_for_node_id(&source_node.id, source_node.kind)
                .unwrap_or_else(|| OwnerId::new(source_node.id.as_str()));
            if !seen_external_sources.insert(source_owner_id.clone()) {
                continue;
            }
            findings.push(Finding {
                rule_id: RuleId::ScbBoundary002,
                kind: "internal_only_external_reference".into(),
                message: format!(
                    "internal_only item {} referenced from {}",
                    node.id.as_str(),
                    source_owner_id.as_str()
                ),
                owner_ids: vec![OwnerId::new(node.id.as_str())],
                node_ids: vec![NodeId::new(source_owner_id.as_str()), node.id.clone()],
            });
        }
    }

    findings.sort_by(|left, right| {
        left.rule_id
            .cmp(&right.rule_id)
            .then_with(|| left.message.cmp(&right.message))
    });
    findings
}

pub(crate) fn analyze_forbid_external_impls(graph: &GraphExport) -> Vec<Finding> {
    let node_map: BTreeMap<_, _> = graph
        .nodes
        .iter()
        .map(|node| (node.id.clone(), node))
        .collect();
    let mut findings = Vec::new();

    for trait_node in &graph.nodes {
        let has_forbid_external_impls = trait_node
            .attributes
            .iter()
            .any(|attr| attr.scope == "boundary" && attr.name == "forbid_external_impls");
        if !has_forbid_external_impls {
            continue;
        }

        for edge in graph
            .edges
            .iter()
            .filter(|edge| edge.kind == "implements" && edge.to == trait_node.id)
        {
            let Some(impl_node) = node_map.get(&edge.from) else {
                continue;
            };
            if impl_node.module_path == trait_node.module_path {
                continue;
            }
            findings.push(Finding {
                rule_id: RuleId::ScbBoundary003,
                kind: "forbid_external_impls_violation".into(),
                message: format!(
                    "trait {} forbids external impls but is implemented from module {}",
                    trait_node.id.as_str(),
                    impl_node.module_path.as_deref().unwrap_or("unknown_module")
                ),
                owner_ids: vec![OwnerId::new(trait_node.id.as_str())],
                node_ids: vec![impl_node.id.clone(), trait_node.id.clone()],
            });
        }
    }

    findings.sort_by(|left, right| left.message.cmp(&right.message));
    findings
}

pub(crate) fn analyze_named_callers(
    graph: &GraphExport,
    inventory: &BoundaryInventory,
) -> Vec<Finding> {
    let node_map: BTreeMap<_, _> = graph
        .nodes
        .iter()
        .map(|node| (node.id.clone(), node))
        .collect();
    let mut findings = Vec::new();
    let mut seen = BTreeSet::new();

    for edge in graph
        .edges
        .iter()
        .filter(|edge| matches!(edge.kind, "references_type" | "references_expr"))
    {
        let Some(source_node) = node_map.get(&edge.from) else {
            continue;
        };
        let Some(target_node) = node_map.get(&edge.to) else {
            continue;
        };
        let Some(source_owner_id) = owner_id_for_node_id(&source_node.id, source_node.kind) else {
            continue;
        };
        let Some(target_owner_id) = owner_id_for_node_id(&target_node.id, target_node.kind) else {
            continue;
        };
        let Some(target_crate_path) = graph_crate_path(&target_node.id) else {
            continue;
        };
        let Some(target_relative_path) = graph_relative_path(&target_node.id) else {
            continue;
        };
        let Some(source_crate_path) = graph_crate_path(&source_owner_id) else {
            continue;
        };
        let Some(source_relative_path) = graph_relative_path(&source_owner_id) else {
            continue;
        };

        for record in &inventory.records {
            let Some(callers) = &record.callers else {
                continue;
            };
            if record.owner_crate_path.as_ref() != target_crate_path {
                continue;
            }
            for approved_entry in &callers.approved {
                if approved_entry.symbol.normalized() != target_relative_path {
                    continue;
                }
                if caller_is_exempt(record, callers, source_crate_path, source_relative_path) {
                    continue;
                }

                let caller_identity = format!("{source_crate_path}::{source_relative_path}");
                if approved_entry
                    .callers
                    .iter()
                    .any(|approved| approved.as_str() == caller_identity)
                {
                    continue;
                }

                let finding_key = (
                    approved_entry.symbol.normalized().to_string(),
                    caller_identity.clone(),
                );
                if !seen.insert(finding_key) {
                    continue;
                }

                findings.push(Finding {
                    rule_id: RuleId::ScbCaller001,
                    kind: "named_caller_allowlist_violation".into(),
                    message: format!(
                        "restricted symbol {} called by unapproved external caller {}",
                        approved_entry.symbol.as_str(),
                        caller_identity
                    ),
                    owner_ids: vec![target_owner_id.clone(), source_owner_id.clone()],
                    node_ids: vec![source_node.id.clone(), target_node.id.clone()],
                });
            }
        }
    }

    findings.sort_by(|left, right| left.message.cmp(&right.message));
    findings
}

fn caller_is_exempt(
    record: &BoundaryRecord,
    _callers: &CallersSection,
    caller_package: &str,
    _caller_path: &str,
) -> bool {
    matches!(record.references.scope, ReferenceScope::OutsideOwnerCrate)
        && caller_package == record.owner_crate_path.as_ref()
}

pub(crate) fn finding_is_failure(finding: &Finding) -> bool {
    matches!(
        finding.rule_id,
        RuleId::ScbCycle001
            | RuleId::ScbBoundary001
            | RuleId::ScbBoundary002
            | RuleId::ScbBoundary003
            | RuleId::ScbBoundary004
            | RuleId::ScbCaller001
            | RuleId::ScbDependency001
            | RuleId::ScbDependency002
            | RuleId::ScbDependency003
            | RuleId::ScbDependency004
            | RuleId::ScbManifest001
            | RuleId::ScbManifest002
            | RuleId::ScbManifest003
            | RuleId::ScbManifest004
            | RuleId::ScbManifest005
            | RuleId::ScbManifest006
    )
}

pub(crate) fn finding_sort_key(finding: &Finding) -> (u8, RuleId) {
    let severity = if finding_is_failure(finding) { 0 } else { 1 };
    (severity, finding.rule_id)
}

#[derive(Debug, Clone)]
struct OwnerRefEdge {
    source_owner_id: OwnerId,
    target_owner_id: OwnerId,
    owner_kind: &'static str,
    source_kind: &'static str,
    source_node_id: NodeId,
    target_node_id: NodeId,
    source_impl_kind: Option<ImplKind>,
    reference_kind: ReferenceKind,
    node_ids: Vec<NodeId>,
}

#[derive(Default)]
struct OwnerGraph {
    adjacency: BTreeMap<OwnerId, BTreeSet<OwnerId>>,
    self_refs: BTreeMap<OwnerId, Vec<OwnerRefEdge>>,
    ref_edges: Vec<OwnerRefEdge>,
}

fn build_owner_graph<'a>(
    graph: &'a GraphExport,
    node_map: &BTreeMap<NodeId, &'a GraphNode>,
) -> OwnerGraph {
    let mut owner_graph = OwnerGraph::default();

    for edge in graph
        .edges
        .iter()
        .filter(|edge| matches!(edge.kind, "references_type" | "references_expr"))
    {
        let Some(source_node) = node_map.get(&edge.from) else {
            continue;
        };
        let Some(target_node) = node_map.get(&edge.to) else {
            continue;
        };
        if target_node.kind == "variant" {
            continue;
        }

        let Some(source_owner_id) = owner_id_for_node_id(&source_node.id, source_node.kind) else {
            continue;
        };
        let Some(target_owner_id) = owner_id_for_node_id(&target_node.id, target_node.kind) else {
            continue;
        };

        let owner_edge = OwnerRefEdge {
            source_owner_id: source_owner_id.clone(),
            target_owner_id: target_owner_id.clone(),
            owner_kind: owner_kind_for_node_id(&source_owner_id, node_map).unwrap_or("module"),
            source_kind: source_node.kind,
            source_node_id: source_node.id.clone(),
            target_node_id: target_node.id.clone(),
            source_impl_kind: source_node.impl_kind,
            reference_kind: match edge.kind {
                "references_type" => ReferenceKind::Type,
                "references_expr" => ReferenceKind::Expr,
                _ => continue,
            },
            node_ids: vec![source_node.id.clone(), target_node.id.clone()],
        };

        owner_graph
            .adjacency
            .entry(source_owner_id.clone())
            .or_default()
            .insert(target_owner_id.clone());
        owner_graph
            .adjacency
            .entry(target_owner_id.clone())
            .or_default();

        if source_owner_id == target_owner_id {
            owner_graph
                .self_refs
                .entry(source_owner_id.clone())
                .or_default()
                .push(owner_edge.clone());
        }

        owner_graph.ref_edges.push(owner_edge);
    }

    owner_graph
}

fn is_delegation_edge(
    edge: &OwnerRefEdge,
    owner_edges: &[OwnerRefEdge],
    node_map: &BTreeMap<NodeId, &GraphNode>,
) -> bool {
    if edge.reference_kind != ReferenceKind::Expr || edge.source_node_id == edge.target_node_id {
        return false;
    }
    let (Some(source), Some(callee)) = (
        node_map.get(&edge.source_node_id),
        node_map.get(&edge.target_node_id),
    ) else {
        return false;
    };
    if source.kind != "method" || callee.kind != "method" || callee.label == source.label {
        return false;
    }
    let private_helper = callee.visibility != Some("public");
    let receiver_call = source.impl_kind == Some(ImplKind::Trait);
    (private_helper || receiver_call)
        && !reaches(owner_edges, &edge.target_node_id, &edge.source_node_id)
}

fn reaches(owner_edges: &[OwnerRefEdge], from: &NodeId, to: &NodeId) -> bool {
    let mut seen = BTreeSet::new();
    let mut stack = vec![from.clone()];
    while let Some(node) = stack.pop() {
        if node == *to {
            return true;
        }
        if !seen.insert(node.clone()) {
            continue;
        }
        stack.extend(
            owner_edges
                .iter()
                .filter(|edge| {
                    edge.reference_kind == ReferenceKind::Expr && edge.source_node_id == node
                })
                .map(|edge| edge.target_node_id.clone()),
        );
    }
    false
}

fn owner_contributors(owners: &[OwnerId], owner_graph: &OwnerGraph) -> Vec<NodeId> {
    let owner_set: BTreeSet<_> = owners.iter().cloned().collect();
    let mut nodes = BTreeSet::new();
    for edge in &owner_graph.ref_edges {
        if owner_set.contains(&edge.source_owner_id) && owner_set.contains(&edge.target_owner_id) {
            for node_id in &edge.node_ids {
                nodes.insert(node_id.clone());
            }
        }
    }
    nodes.into_iter().collect()
}

fn owner_kind_for_node_id(
    owner_id: &OwnerId,
    node_map: &BTreeMap<NodeId, &GraphNode>,
) -> Option<&'static str> {
    node_map
        .get(&NodeId::new(owner_id.as_str()))
        .map(|node| node.kind)
}

fn owner_id_for_node_id(node_id: &NodeId, node_kind: &str) -> Option<OwnerId> {
    match node_kind {
        "module" | "type" | "trait" => Some(OwnerId::new(node_id.as_str())),
        "function" => node_id
            .rsplit_once("::")
            .map(|(parent, _)| OwnerId::new(parent)),
        "method" => node_id
            .split_once("::impl::")
            .or_else(|| node_id.rsplit_once("::"))
            .map(|(parent, _)| OwnerId::new(parent)),
        "variant" => node_id
            .rsplit_once("::variant::")
            .map(|(parent, _)| OwnerId::new(parent)),
        "field" => {
            let (parent, _) = node_id.rsplit_once("::field::")?;
            if let Some((enum_parent, _variant)) = parent.rsplit_once("::variant::") {
                Some(OwnerId::new(enum_parent))
            } else {
                Some(OwnerId::new(parent))
            }
        }
        _ => None,
    }
}

fn strongly_connected_components(owner_graph: &OwnerGraph) -> Vec<Vec<OwnerId>> {
    fn visit(
        node: &OwnerId,
        owner_graph: &OwnerGraph,
        visited: &mut BTreeSet<OwnerId>,
        order: &mut Vec<OwnerId>,
    ) {
        if !visited.insert(node.clone()) {
            return;
        }
        if let Some(neighbors) = owner_graph.adjacency.get(node) {
            for neighbor in neighbors {
                visit(neighbor, owner_graph, visited, order);
            }
        }
        order.push(node.clone());
    }

    fn reverse_graph(owner_graph: &OwnerGraph) -> BTreeMap<OwnerId, BTreeSet<OwnerId>> {
        let mut reversed: BTreeMap<OwnerId, BTreeSet<OwnerId>> = BTreeMap::new();
        for (source, targets) in &owner_graph.adjacency {
            reversed.entry(source.clone()).or_default();
            for target in targets {
                reversed
                    .entry(target.clone())
                    .or_default()
                    .insert(source.clone());
            }
        }
        reversed
    }

    fn collect_component(
        node: &OwnerId,
        reversed: &BTreeMap<OwnerId, BTreeSet<OwnerId>>,
        visited: &mut BTreeSet<OwnerId>,
        component: &mut Vec<OwnerId>,
    ) {
        if !visited.insert(node.clone()) {
            return;
        }
        component.push(node.clone());
        if let Some(neighbors) = reversed.get(node) {
            for neighbor in neighbors {
                collect_component(neighbor, reversed, visited, component);
            }
        }
    }

    let mut visited = BTreeSet::new();
    let mut order = Vec::new();
    for node in owner_graph.adjacency.keys() {
        visit(node, owner_graph, &mut visited, &mut order);
    }

    let reversed = reverse_graph(owner_graph);
    let mut assigned = BTreeSet::new();
    let mut components = Vec::new();
    while let Some(node) = order.pop() {
        if assigned.contains(&node) {
            continue;
        }
        let mut component = Vec::new();
        collect_component(&node, &reversed, &mut assigned, &mut component);
        components.push(component);
    }
    components
}

fn graph_crate_path(graph_id: &str) -> Option<&str> {
    let prefix = graph_id.split("::module::").next().unwrap_or(graph_id);
    let mut segments = prefix.split("::");
    match (segments.next(), segments.next(), segments.next()) {
        (Some("crate"), Some(_package), Some(crate_path)) => Some(crate_path),
        _ => None,
    }
}

fn graph_relative_path(graph_id: &str) -> Option<&str> {
    if let Some((_, relative)) = graph_id.split_once("::module::") {
        return Some(relative.strip_prefix("crate::").unwrap_or(relative));
    }
    Some("crate")
}
