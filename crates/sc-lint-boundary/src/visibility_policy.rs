//! SCB-BOUNDARY-005: compare each boundary record's declared `[implementation]`
//! type and constructor visibility with the source (REQ-SCB-027, #861).

use crate::Finding;
use crate::GraphExport;
use crate::GraphNode;
use crate::OwnerId;
use crate::RuleId;
use crate::inventory::BoundaryInventory;
use crate::inventory::BoundaryRecord;
use crate::inventory::Constructor;
use crate::inventory::Visibility;

/// Source visibility, ordered from narrowest to widest. `pub(super)`,
/// `pub(self)` and `pub(in path)` are `Restricted`, which REQ-SCB-027 counts
/// as no wider than `pub(crate)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum SourceVisibility {
    Private,
    Restricted,
    Crate,
    Public,
}

impl SourceVisibility {
    fn from_graph(label: Option<&str>) -> Option<Self> {
        match label? {
            "private" => Some(Self::Private),
            "restricted" => Some(Self::Restricted),
            "crate" => Some(Self::Crate),
            "public" => Some(Self::Public),
            _ => None,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Private => "private",
            Self::Restricted => "pub(super)/pub(self)/pub(in path)",
            Self::Crate => "pub(crate)",
            Self::Public => "pub",
        }
    }
}

/// The widest source visibility a declaration allows, or `None` when the
/// declaration is not checked.
#[derive(Debug, Clone, Copy)]
struct Limit {
    widest: SourceVisibility,
    declared: &'static str,
}

fn type_limit(visibility: &Visibility) -> Option<Limit> {
    match visibility {
        Visibility::Public => None,
        Visibility::PubCrate => Some(Limit {
            widest: SourceVisibility::Crate,
            declared: "pub(crate)",
        }),
        // Restricted (`pub(super)`, `pub(in path)`) also exceeds `private`.
        Visibility::Private => Some(Limit {
            widest: SourceVisibility::Private,
            declared: "private",
        }),
        // trait_only fails only when the type is `pub`.
        Visibility::TraitOnly => Some(Limit {
            widest: SourceVisibility::Crate,
            declared: "trait_only",
        }),
    }
}

fn constructor_limit(constructor: Option<&Constructor>) -> Option<Limit> {
    match constructor? {
        Constructor::None | Constructor::Public => None,
        Constructor::PubCrate => Some(Limit {
            widest: SourceVisibility::Crate,
            declared: "pub(crate)",
        }),
        Constructor::Private => Some(Limit {
            widest: SourceVisibility::Private,
            declared: "private",
        }),
    }
}

/// Graph module path (`crate::a::b`) for a record module (`owner_crate::a::b`).
fn graph_module_path(record: &BoundaryRecord, module: &str) -> Option<String> {
    let crate_path = record.owner_crate_path.as_ref();
    if module == crate_path {
        return Some("crate".to_string());
    }
    module
        .strip_prefix(crate_path)
        .and_then(|rest| rest.strip_prefix("::"))
        .map(|rest| format!("crate::{rest}"))
}

fn in_owner_library<'a>(
    graph: &'a GraphExport,
    record: &'a BoundaryRecord,
) -> impl Iterator<Item = &'a GraphNode> + 'a {
    let package = record.owner_package.as_ref();
    let target = record.owner_crate_path.as_ref();
    graph
        .nodes
        .iter()
        .filter(move |node| node.package == package && node.target.as_deref() == Some(target))
}

/// The declared type node: a struct, enum, union or type alias named `name`
/// declared (visibility recorded) directly in `module_path`.
fn find_type<'a>(
    graph: &'a GraphExport,
    record: &'a BoundaryRecord,
    module_path: &str,
    name: &str,
) -> Option<&'a GraphNode> {
    in_owner_library(graph, record).find(|node| {
        node.kind == "type"
            && node.label == name
            && node.visibility.is_some()
            && node.module_path.as_deref() == Some(module_path)
    })
}

/// The inherent associated `fn new` of `type_name` in the owner library. An
/// inherent impl may sit in a different module from the type, so every
/// inherent `new` declared on a type with that name is considered and the
/// widest one is returned.
fn find_inherent_new<'a>(
    graph: &'a GraphExport,
    record: &'a BoundaryRecord,
    type_name: &str,
) -> Option<&'a GraphNode> {
    let suffix = format!("::{type_name}::new");
    in_owner_library(graph, record)
        .filter(|node| {
            node.kind == "method"
                && node.impl_kind == Some(crate::ImplKind::Inherent)
                && node.id.as_str().ends_with(&suffix)
        })
        .max_by_key(|node| SourceVisibility::from_graph(node.visibility))
}

fn finding(record: &BoundaryRecord, item: &str, declared: &str, actual: &str) -> Finding {
    Finding {
        rule_id: RuleId::ScbBoundary005,
        kind: "declared_visibility_mismatch".into(),
        message: format!(
            "boundary `{}` record `{}`: {item} declared `{declared}` but source is `{actual}`",
            record.boundary_id,
            record.record_path.display(),
        ),
        owner_ids: vec![OwnerId::new(record.boundary_id.to_string())],
        node_ids: Vec::new(),
    }
}

fn check_item(
    findings: &mut Vec<Finding>,
    record: &BoundaryRecord,
    item: &str,
    limit: Limit,
    node: Option<&GraphNode>,
) {
    match node.and_then(|node| SourceVisibility::from_graph(node.visibility)) {
        None => findings.push(finding(record, item, limit.declared, "not found")),
        Some(actual) if actual > limit.widest => {
            findings.push(finding(record, item, limit.declared, actual.as_str()));
        }
        Some(_) => {}
    }
}

pub(crate) fn analyze_declared_visibility(
    graph: &GraphExport,
    inventory: &BoundaryInventory,
) -> Vec<Finding> {
    let mut findings = Vec::new();
    for record in &inventory.records {
        let implementation = &record.implementation;
        let type_check = type_limit(&implementation.visibility);
        let constructor_check = constructor_limit(implementation.constructor.as_ref());
        if type_check.is_none() && constructor_check.is_none() {
            continue;
        }
        let (Some(type_name), Some(module)) = (
            implementation.implementation_type.as_deref(),
            implementation.module.as_deref(),
        ) else {
            continue;
        };
        let type_node = graph_module_path(record, module)
            .and_then(|module_path| find_type(graph, record, &module_path, type_name));
        let type_item = format!("type `{module}::{type_name}`");
        if let Some(limit) = type_check {
            check_item(&mut findings, record, &type_item, limit, type_node);
        }
        if let Some(limit) = constructor_check {
            let constructor = type_node.and_then(|_| find_inherent_new(graph, record, type_name));
            check_item(
                &mut findings,
                record,
                &format!("constructor `{module}::{type_name}::new`"),
                limit,
                constructor,
            );
        }
    }
    findings.sort_by(|left, right| left.message.cmp(&right.message));
    findings
}
