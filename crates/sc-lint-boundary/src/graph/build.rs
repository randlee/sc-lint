use super::merge::{SourceFileTask, merge_source_file_partials};
use super::*;
use std::collections::BTreeMap;
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum NodeKind {
    Module,
    Type,
    Trait,
    Function,
    Method,
    Impl,
    Variant,
    Field,
    TraitRef,
}

impl NodeKind {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::Module => "module",
            Self::Type => "type",
            Self::Trait => "trait",
            Self::Function => "function",
            Self::Method => "method",
            Self::Impl => "impl",
            Self::Variant => "variant",
            Self::Field => "field",
            Self::TraitRef => "trait_ref",
        }
    }
}

pub(crate) fn build_workspace_graph(root: &Path) -> Result<GraphExport> {
    build_workspace_graph_from(&load_metadata(root)?)
}

pub(crate) fn build_workspace_graph_from(
    metadata: &cargo_metadata::Metadata,
) -> Result<GraphExport> {
    let mut builder = GraphBuilder::default();
    let mut pending_files = queue_workspace_sources(metadata, &mut builder)?;
    merge_source_file_partials(&mut builder, &mut pending_files)?;
    resolve_trait_method_edges(&mut builder);
    Ok(builder.finish())
}

fn queue_workspace_sources(
    metadata: &cargo_metadata::Metadata,
    builder: &mut GraphBuilder,
) -> Result<Vec<SourceFileTask>> {
    let workspace_packages = crate::workspace_packages(metadata);
    let mut pending_files = Vec::new();
    for package in &workspace_packages {
        let manifest_path = package.manifest_path.as_std_path().display().to_string();
        for target in package
            .targets
            .iter()
            .filter(|target| is_supported_target(target))
        {
            pending_files.push(queue_target_source(
                builder,
                &workspace_packages,
                package,
                target,
                &manifest_path,
            )?);
        }
    }
    Ok(pending_files)
}

fn queue_target_source(
    builder: &mut GraphBuilder,
    workspace_packages: &[&cargo_metadata::Package],
    package: &cargo_metadata::Package,
    target: &cargo_metadata::Target,
    manifest_path: &str,
) -> Result<SourceFileTask> {
    let source_path = target.src_path.as_std_path().to_path_buf();
    let context = TargetContext {
        package_name: package.name.to_string(),
        target_name: target.name.clone(),
        manifest_path: manifest_path.to_string(),
        crate_id: crate_id(&package.name, &target.name),
        root_module_path: ModulePath::crate_root(),
        workspace_dependency_roots: workspace_dependency_roots(workspace_packages, package),
    };
    builder.add_workspace_target(
        &context.package_name,
        &context.manifest_path,
        &context.target_name,
        &source_path,
    );
    let root_module_id = format!(
        "{}::module::{}",
        context.crate_id.as_str(),
        context.root_module_path
    );
    builder.add_node(GraphNode {
        id: NodeId::new(root_module_id.clone()),
        kind: NodeKind::Module.as_str(),
        label: "crate".to_string(),
        visibility: None,
        package: context.package_name.clone(),
        target: Some(context.target_name.clone()),
        manifest_path: context.manifest_path.clone(),
        source_path: Some(source_path.display().to_string()),
        module_path: Some(context.root_module_path.to_string()),
        impl_kind: None,
        impl_trait: None,
        attributes: Vec::new(),
    });
    builder.add_edge(
        EdgeKind::Contains,
        context.crate_id.clone(),
        root_module_id.clone(),
    );
    let root_dir = source_path
        .parent()
        .with_context(|| format!("missing parent dir for {}", source_path.display()))?
        .to_path_buf();
    Ok(SourceFileTask {
        context,
        parent_module_id: NodeId::new(root_module_id),
        module_path: ModulePath::crate_root(),
        module_dir: root_dir,
        source_path,
        resolution_context: None,
    })
}

fn resolve_trait_method_edges(builder: &mut GraphBuilder) {
    let methods: BTreeMap<_, _> = builder
        .nodes
        .iter()
        .filter(|node| node.kind == "method")
        .map(|node| (node.id.clone(), node))
        .collect();
    let known: BTreeSet<_> = builder.nodes.iter().map(|node| node.id.clone()).collect();
    let reference_impl_ids = &builder.reference_impl_ids;
    for edge in &mut builder.edges {
        if !matches!(edge.kind, "references" | "references_expr") {
            continue;
        }
        let Some((owner, method)) = edge.to.rsplit_once("::") else {
            continue;
        };
        let variant = NodeId::new(format!("{owner}::variant::{method}"));
        if known.contains(&variant) {
            edge.to = variant;
            continue;
        }
        if known.contains(&edge.to) {
            continue;
        }
        // Self::method inside a trait impl resolves to that impl when no
        // inherent method exists. Otherwise accept only an unambiguous impl.
        if let Some((source_owner, _)) = edge.from.split_once("::impl::")
            && source_owner == owner
            && let Some((source_impl, _)) = edge.from.rsplit_once("::")
        {
            let candidate = NodeId::new(format!("{source_impl}::{method}"));
            if methods.contains_key(&candidate) {
                edge.to = candidate;
                continue;
            }
        }
        let prefix = format!("{owner}::impl::");
        let mut candidates = methods.values().filter(|node| {
            node.id.starts_with(&prefix)
                && node.label == method
                && node
                    .id
                    .rsplit_once("::")
                    .is_none_or(|(impl_id, _)| !reference_impl_ids.contains(&NodeId::new(impl_id)))
        });
        if let Some(candidate) = candidates.next()
            && candidates.next().is_none()
        {
            edge.to = candidate.id.clone();
        }
    }
}

fn workspace_dependency_roots(
    workspace_packages: &[&cargo_metadata::Package],
    package: &cargo_metadata::Package,
) -> BTreeMap<String, CrateId> {
    let workspace_packages_by_name = workspace_packages
        .iter()
        .map(|workspace_package| (workspace_package.name.as_str(), *workspace_package))
        .collect::<BTreeMap<_, _>>();
    let mut dependency_roots = BTreeMap::new();

    for dependency in &package.dependencies {
        let Some(workspace_package) = workspace_packages_by_name.get(dependency.name.as_str())
        else {
            continue;
        };
        let Some(library_target) = workspace_package
            .targets
            .iter()
            .find(|target| target.is_lib())
        else {
            continue;
        };
        let dependency_root = dependency
            .rename
            .clone()
            .unwrap_or_else(|| library_target.name.clone());
        dependency_roots.insert(
            dependency_root,
            crate_id(&workspace_package.name, &library_target.name),
        );
    }

    dependency_roots
}
