//! Per-item ingestion helpers that turn one parsed source file into graph nodes and edges.

use super::build::NodeKind;
use super::merge::SourceFileTask;
use super::reference_collector::collect_references_with;
use super::*;
use std::collections::BTreeSet;

fn collect_owner_names(items: &[Item]) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for item in items {
        match item {
            Item::Struct(item_struct) => {
                names.insert(item_struct.ident.to_string());
            }
            Item::Enum(item_enum) => {
                names.insert(item_enum.ident.to_string());
            }
            Item::Union(item_union) => {
                names.insert(item_union.ident.to_string());
            }
            Item::Type(item_type) => {
                names.insert(item_type.ident.to_string());
            }
            Item::Trait(item_trait) => {
                names.insert(item_trait.ident.to_string());
            }
            _ => {}
        }
    }
    names
}

fn visibility_label(visibility: &syn::Visibility) -> ItemVisibility {
    match visibility {
        syn::Visibility::Inherited => ItemVisibility::Private,
        syn::Visibility::Public(_) => ItemVisibility::Public,
        syn::Visibility::Restricted(restricted) => {
            if restricted.path.is_ident("crate") {
                ItemVisibility::Crate
            } else {
                ItemVisibility::Restricted
            }
        }
    }
}

pub(super) fn ingest_module_items(
    builder: &mut GraphBuilder,
    task: &SourceFileTask,
    file: File,
    pending_files: &mut Vec<SourceFileTask>,
) -> Result<()> {
    let context = &task.context;
    let parent_module_id = &task.parent_module_id;
    let module_path = &task.module_path;
    let module_dir = &task.module_dir;
    let source_path = task.source_path.as_path();
    let local_owner_names = collect_owner_names(&file.items);

    for item in file.items {
        match item {
            Item::Mod(item_mod) => {
                let name = item_mod.ident.to_string();
                let child_module_path = module_path.child(&name);
                let child_module_id =
                    format!("{}::module::{child_module_path}", context.crate_id.as_str());
                let attributes = parse_lint_attributes(&item_mod.attrs)?;

                builder.add_node(GraphNode {
                    id: NodeId::new(child_module_id.clone()),
                    kind: NodeKind::Module.as_str(),
                    label: name.clone(),
                    visibility: Some(visibility_label(&item_mod.vis).as_str()),
                    package: context.package_name.clone(),
                    target: Some(context.target_name.clone()),
                    manifest_path: context.manifest_path.clone(),
                    source_path: Some(source_path.display().to_string()),
                    module_path: Some(child_module_path.to_string()),
                    impl_kind: None,
                    impl_trait: None,
                    attributes,
                });
                builder.add_edge(
                    EdgeKind::Contains,
                    parent_module_id.clone(),
                    child_module_id.clone(),
                );

                if let Some((_, items)) = item_mod.content {
                    let child_module_dir = module_dir.join(&name);
                    let inline_file = File {
                        shebang: None,
                        attrs: Vec::new(),
                        items,
                    };
                    let inline_task = SourceFileTask {
                        context: context.clone(),
                        parent_module_id: NodeId::new(child_module_id.clone()),
                        module_path: child_module_path,
                        module_dir: child_module_dir,
                        source_path: task.source_path.clone(),
                        resolution_context: None,
                    };
                    ingest_module_items(builder, &inline_task, inline_file, pending_files)?;
                } else {
                    let child_source_path =
                        resolve_module_source(source_path, module_dir, &name, &item_mod.attrs)
                            .with_context(|| {
                                format!("while resolving module `{child_module_path}`")
                            })?;
                    let child_module_dir = if has_explicit_module_path(&item_mod.attrs) {
                        // #[path = "..."] points at the real child source file, so the child
                        // module directory must be derived from that resolved location.
                        child_source_path
                            .parent()
                            .map(Path::to_path_buf)
                            .unwrap_or_else(|| module_dir.join(&name))
                    } else if child_source_path.file_name().and_then(|name| name.to_str())
                        == Some("mod.rs")
                    {
                        // mod.rs is itself the child module root, so sibling lookups should
                        // start from the file's parent directory.
                        child_source_path
                            .parent()
                            .map(Path::to_path_buf)
                            .unwrap_or_else(|| module_dir.join(&name))
                    } else {
                        module_dir.join(&name)
                    };
                    pending_files.push(SourceFileTask {
                        context: context.clone(),
                        parent_module_id: NodeId::new(child_module_id),
                        module_path: child_module_path.clone(),
                        module_dir: child_module_dir,
                        source_path: child_source_path,
                        resolution_context: Some(child_module_path),
                    });
                }
            }
            Item::Struct(item_struct) => {
                let owner_name = item_struct.ident.to_string();
                let node_id = add_item_node(
                    builder,
                    context,
                    ItemNodeArgs {
                        parent_module_id,
                        module_path,
                        source_path,
                        ident: &item_struct.ident,
                        kind: NodeKind::Type,
                        visibility: visibility_label(&item_struct.vis),
                        attributes: parse_lint_attributes(&item_struct.attrs)?,
                    },
                );
                add_reference_edges(
                    builder,
                    context,
                    &node_id,
                    module_path,
                    collect_references_with(
                        &local_owner_names,
                        Some(&owner_name),
                        &context.workspace_dependency_roots,
                        |collector| {
                            collector.visit_fields(&item_struct.fields);
                        },
                    ),
                );
                add_field_nodes(
                    builder,
                    context,
                    FieldNodeArgs {
                        parent_id: &node_id,
                        module_path,
                        source_path,
                        local_owner_names: &local_owner_names,
                        owner_name: Some(&owner_name),
                        fields: &item_struct.fields,
                    },
                );
            }
            Item::Enum(item_enum) => {
                let owner_name = item_enum.ident.to_string();
                let node_id = add_item_node(
                    builder,
                    context,
                    ItemNodeArgs {
                        parent_module_id,
                        module_path,
                        source_path,
                        ident: &item_enum.ident,
                        kind: NodeKind::Type,
                        visibility: visibility_label(&item_enum.vis),
                        attributes: parse_lint_attributes(&item_enum.attrs)?,
                    },
                );
                add_reference_edges(
                    builder,
                    context,
                    &node_id,
                    module_path,
                    collect_references_with(
                        &local_owner_names,
                        Some(&owner_name),
                        &context.workspace_dependency_roots,
                        |collector| {
                            for variant in &item_enum.variants {
                                collector.visit_fields(&variant.fields);
                            }
                        },
                    ),
                );
                for variant in &item_enum.variants {
                    let variant_id = NodeId::new(format!("{node_id}::variant::{}", variant.ident));
                    builder.add_node(GraphNode {
                        id: variant_id.clone(),
                        kind: NodeKind::Variant.as_str(),
                        label: variant.ident.to_string(),
                        visibility: None,
                        package: context.package_name.clone(),
                        target: Some(context.target_name.clone()),
                        manifest_path: context.manifest_path.clone(),
                        source_path: Some(source_path.display().to_string()),
                        module_path: Some(module_path.to_string()),
                        impl_kind: None,
                        impl_trait: None,
                        attributes: Vec::new(),
                    });
                    builder.add_edge(EdgeKind::Contains, node_id.clone(), variant_id.clone());
                    add_field_nodes(
                        builder,
                        context,
                        FieldNodeArgs {
                            parent_id: &variant_id,
                            module_path,
                            source_path,
                            local_owner_names: &local_owner_names,
                            owner_name: Some(&owner_name),
                            fields: &variant.fields,
                        },
                    );
                }
            }
            Item::Union(item_union) => {
                let owner_name = item_union.ident.to_string();
                let node_id = add_item_node(
                    builder,
                    context,
                    ItemNodeArgs {
                        parent_module_id,
                        module_path,
                        source_path,
                        ident: &item_union.ident,
                        kind: NodeKind::Type,
                        visibility: visibility_label(&item_union.vis),
                        attributes: parse_lint_attributes(&item_union.attrs)?,
                    },
                );
                add_reference_edges(
                    builder,
                    context,
                    &node_id,
                    module_path,
                    collect_references_with(
                        &local_owner_names,
                        Some(&owner_name),
                        &context.workspace_dependency_roots,
                        |collector| {
                            collector.visit_fields_named(&item_union.fields);
                        },
                    ),
                );
                let union_fields = syn::Fields::Named(item_union.fields.clone());
                add_field_nodes(
                    builder,
                    context,
                    FieldNodeArgs {
                        parent_id: &node_id,
                        module_path,
                        source_path,
                        local_owner_names: &local_owner_names,
                        owner_name: Some(&owner_name),
                        fields: &union_fields,
                    },
                );
            }
            Item::Type(item_type) => {
                let owner_name = item_type.ident.to_string();
                let node_id = add_item_node(
                    builder,
                    context,
                    ItemNodeArgs {
                        parent_module_id,
                        module_path,
                        source_path,
                        ident: &item_type.ident,
                        kind: NodeKind::Type,
                        visibility: visibility_label(&item_type.vis),
                        attributes: parse_lint_attributes(&item_type.attrs)?,
                    },
                );
                add_reference_edges(
                    builder,
                    context,
                    &node_id,
                    module_path,
                    collect_references_with(
                        &local_owner_names,
                        Some(&owner_name),
                        &context.workspace_dependency_roots,
                        |collector| {
                            collector.visit_type(&item_type.ty);
                        },
                    ),
                );
            }
            Item::Trait(item_trait) => {
                let owner_name = item_trait.ident.to_string();
                let node_id = add_item_node(
                    builder,
                    context,
                    ItemNodeArgs {
                        parent_module_id,
                        module_path,
                        source_path,
                        ident: &item_trait.ident,
                        kind: NodeKind::Trait,
                        visibility: visibility_label(&item_trait.vis),
                        attributes: parse_lint_attributes(&item_trait.attrs)?,
                    },
                );
                add_reference_edges(
                    builder,
                    context,
                    &node_id,
                    module_path,
                    collect_references_with(
                        &local_owner_names,
                        Some(&owner_name),
                        &context.workspace_dependency_roots,
                        |collector| {
                            for trait_item in &item_trait.items {
                                collector.visit_trait_item(trait_item);
                            }
                        },
                    ),
                );
            }
            Item::Fn(item_fn) => {
                let function_ident = item_fn.sig.ident.clone();
                let node_id = add_item_node(
                    builder,
                    context,
                    ItemNodeArgs {
                        parent_module_id,
                        module_path,
                        source_path,
                        ident: &function_ident,
                        kind: NodeKind::Function,
                        visibility: visibility_label(&item_fn.vis),
                        attributes: parse_lint_attributes(&item_fn.attrs)?,
                    },
                );
                add_reference_edges(
                    builder,
                    context,
                    &node_id,
                    module_path,
                    collect_references_with(
                        &local_owner_names,
                        None,
                        &context.workspace_dependency_roots,
                        |collector| {
                            collector.visit_item_fn(&item_fn);
                        },
                    ),
                );
            }
            Item::Impl(item_impl) => {
                let owner = impl_owner(&item_impl.self_ty)?;
                let owner_name = &owner.name;
                let owner_node_id = NodeId::new(format!("{parent_module_id}::{owner_name}"));
                let trait_path = item_impl
                    .trait_
                    .as_ref()
                    .map(|(_, path, _)| trait_path_key(path));
                let impl_node_id = if let Some((_, path, _)) = &item_impl.trait_ {
                    NodeId::new(format!("{owner_node_id}::{}", trait_impl_key(&owner, path)))
                } else {
                    NodeId::new(format!("{owner_node_id}::impl::inherent"))
                };
                let owner_label = if owner.is_reference {
                    &owner.self_type
                } else {
                    owner_name
                };

                if owner.is_reference {
                    builder.reference_impl_ids.insert(impl_node_id.clone());
                }

                if !builder
                    .nodes
                    .iter()
                    .any(|node| node.id == owner_node_id.as_str())
                {
                    builder.add_node(GraphNode {
                        id: owner_node_id.clone(),
                        kind: NodeKind::Type.as_str(),
                        label: owner_name.to_string(),
                        visibility: None,
                        package: context.package_name.clone(),
                        target: Some(context.target_name.clone()),
                        manifest_path: context.manifest_path.clone(),
                        source_path: Some(source_path.display().to_string()),
                        module_path: Some(module_path.to_string()),
                        impl_kind: None,
                        impl_trait: None,
                        attributes: Vec::new(),
                    });
                    builder.add_edge(
                        EdgeKind::Contains,
                        parent_module_id.clone(),
                        owner_node_id.clone(),
                    );
                }

                builder.add_node(GraphNode {
                    id: impl_node_id.clone(),
                    kind: NodeKind::Impl.as_str(),
                    label: trait_path
                        .as_ref()
                        .map(|path| format!("impl {path} for {owner_label}"))
                        .unwrap_or_else(|| format!("impl {owner_label}")),
                    visibility: None,
                    package: context.package_name.clone(),
                    target: Some(context.target_name.clone()),
                    manifest_path: context.manifest_path.clone(),
                    source_path: Some(source_path.display().to_string()),
                    module_path: Some(module_path.to_string()),
                    impl_kind: Some(if trait_path.is_some() {
                        ImplKind::Trait
                    } else {
                        ImplKind::Inherent
                    }),
                    impl_trait: trait_path.clone(),
                    attributes: Vec::new(),
                });
                builder.add_edge(
                    EdgeKind::Contains,
                    parent_module_id.clone(),
                    impl_node_id.clone(),
                );
                builder.add_edge(
                    EdgeKind::Targets,
                    impl_node_id.clone(),
                    owner_node_id.clone(),
                );

                if let Some((_, path, _)) = &item_impl.trait_ {
                    let trait_reference_path = trait_path_key(path);
                    let trait_target_node_id = resolve_reference_target(
                        context,
                        &impl_node_id,
                        module_path,
                        &trait_reference_path,
                    );
                    ensure_trait_reference_node(
                        builder,
                        context,
                        source_path,
                        module_path,
                        &trait_target_node_id,
                        &trait_reference_path,
                    );
                    builder.add_edge(
                        EdgeKind::Implements,
                        impl_node_id.clone(),
                        trait_target_node_id,
                    );
                }

                for impl_item in item_impl.items {
                    if let ImplItem::Fn(method) = impl_item {
                        let method_owner = if item_impl.trait_.is_some() {
                            &impl_node_id
                        } else {
                            &owner_node_id
                        };
                        let method_id =
                            NodeId::new(format!("{method_owner}::{}", method.sig.ident));
                        builder.add_node(GraphNode {
                            id: method_id.clone(),
                            kind: NodeKind::Method.as_str(),
                            label: method.sig.ident.to_string(),
                            visibility: Some(visibility_label(&method.vis).as_str()),
                            package: context.package_name.clone(),
                            target: Some(context.target_name.clone()),
                            manifest_path: context.manifest_path.clone(),
                            source_path: Some(source_path.display().to_string()),
                            module_path: Some(module_path.to_string()),
                            impl_kind: Some(if item_impl.trait_.is_some() {
                                ImplKind::Trait
                            } else {
                                ImplKind::Inherent
                            }),
                            impl_trait: trait_path.clone(),
                            attributes: parse_lint_attributes(&method.attrs)?,
                        });
                        builder.add_edge(
                            EdgeKind::Declares,
                            owner_node_id.clone(),
                            method_id.clone(),
                        );
                        builder.add_edge(
                            EdgeKind::Contains,
                            impl_node_id.clone(),
                            method_id.clone(),
                        );
                        add_reference_edges(
                            builder,
                            context,
                            &method_id,
                            module_path,
                            collect_references_with(
                                &local_owner_names,
                                Some(owner_name),
                                &context.workspace_dependency_roots,
                                |collector| {
                                    collector.set_impl_self_type(&item_impl.self_ty);
                                    collector.visit_impl_item_fn(&method);
                                },
                            ),
                        );
                    }
                }
            }
            _ => {}
        }
    }

    Ok(())
}

fn add_item_node(
    builder: &mut GraphBuilder,
    context: &TargetContext,
    args: ItemNodeArgs<'_>,
) -> NodeId {
    let id = format!("{}::{}", args.parent_module_id, args.ident);
    builder.add_node(GraphNode {
        id: NodeId::new(id.clone()),
        kind: args.kind.as_str(),
        label: args.ident.to_string(),
        visibility: Some(args.visibility.as_str()),
        package: context.package_name.clone(),
        target: Some(context.target_name.clone()),
        manifest_path: context.manifest_path.clone(),
        source_path: Some(args.source_path.display().to_string()),
        module_path: Some(args.module_path.to_string()),
        impl_kind: None,
        impl_trait: None,
        attributes: args.attributes,
    });
    builder.add_edge(
        EdgeKind::Contains,
        args.parent_module_id.clone(),
        NodeId::new(id.clone()),
    );
    NodeId::new(id)
}

fn add_field_nodes(builder: &mut GraphBuilder, context: &TargetContext, args: FieldNodeArgs<'_>) {
    match args.fields {
        syn::Fields::Named(named) => {
            for field in &named.named {
                let label = field
                    .ident
                    .as_ref()
                    .map(ToString::to_string)
                    .unwrap_or_else(|| "field".to_string());
                let field_id = NodeId::new(format!("{}::field::{label}", args.parent_id));
                builder.add_node(GraphNode {
                    id: field_id.clone(),
                    kind: NodeKind::Field.as_str(),
                    label: label.clone(),
                    visibility: Some(visibility_label(&field.vis).as_str()),
                    package: context.package_name.clone(),
                    target: Some(context.target_name.clone()),
                    manifest_path: context.manifest_path.clone(),
                    source_path: Some(args.source_path.display().to_string()),
                    module_path: Some(args.module_path.to_string()),
                    impl_kind: None,
                    impl_trait: None,
                    attributes: Vec::new(),
                });
                builder.add_edge(EdgeKind::Contains, args.parent_id.clone(), field_id.clone());
                add_reference_edges(
                    builder,
                    context,
                    &field_id,
                    args.module_path,
                    collect_references_with(
                        args.local_owner_names,
                        args.owner_name,
                        &context.workspace_dependency_roots,
                        |collector| {
                            collector.visit_type(&field.ty);
                        },
                    ),
                );
            }
        }
        syn::Fields::Unnamed(unnamed) => {
            for (index, field) in unnamed.unnamed.iter().enumerate() {
                let label = index.to_string();
                let field_id = NodeId::new(format!("{}::field::{label}", args.parent_id));
                builder.add_node(GraphNode {
                    id: field_id.clone(),
                    kind: NodeKind::Field.as_str(),
                    label: label.clone(),
                    visibility: Some(visibility_label(&field.vis).as_str()),
                    package: context.package_name.clone(),
                    target: Some(context.target_name.clone()),
                    manifest_path: context.manifest_path.clone(),
                    source_path: Some(args.source_path.display().to_string()),
                    module_path: Some(args.module_path.to_string()),
                    impl_kind: None,
                    impl_trait: None,
                    attributes: Vec::new(),
                });
                builder.add_edge(EdgeKind::Contains, args.parent_id.clone(), field_id.clone());
                add_reference_edges(
                    builder,
                    context,
                    &field_id,
                    args.module_path,
                    collect_references_with(
                        args.local_owner_names,
                        args.owner_name,
                        &context.workspace_dependency_roots,
                        |collector| {
                            collector.visit_type(&field.ty);
                        },
                    ),
                );
            }
        }
        syn::Fields::Unit => {}
    }
}

fn ensure_trait_reference_node(
    builder: &mut GraphBuilder,
    context: &TargetContext,
    source_path: &Path,
    module_path: &ModulePath,
    trait_node_id: &NodeId,
    trait_label: &str,
) {
    if builder
        .nodes
        .iter()
        .any(|node| node.id == trait_node_id.as_str())
    {
        return;
    }

    builder.add_node(GraphNode {
        id: trait_node_id.clone(),
        kind: NodeKind::TraitRef.as_str(),
        label: trait_label.to_string(),
        visibility: None,
        package: context.package_name.clone(),
        target: Some(context.target_name.clone()),
        manifest_path: context.manifest_path.clone(),
        source_path: Some(source_path.display().to_string()),
        module_path: Some(module_path.to_string()),
        impl_kind: None,
        impl_trait: None,
        attributes: Vec::new(),
    });
}

fn add_reference_edges(
    builder: &mut GraphBuilder,
    context: &TargetContext,
    source_node_id: &NodeId,
    module_path: &ModulePath,
    referenced_paths: BTreeSet<CollectedReference>,
) {
    for referenced in referenced_paths {
        let target_node_id =
            resolve_reference_target(context, source_node_id, module_path, &referenced.path);
        builder.add_edge(
            referenced.kind.edge_kind(),
            source_node_id.clone(),
            target_node_id.clone(),
        );
        builder.add_edge(EdgeKind::References, source_node_id.clone(), target_node_id);
    }
}

fn resolve_reference_target(
    context: &TargetContext,
    source_node_id: &NodeId,
    module_path: &ModulePath,
    referenced_path: &str,
) -> NodeId {
    let crate_prefix = source_node_id
        .split("::module::")
        .next()
        .unwrap_or(source_node_id);

    if let Some((dependency_root, rest)) = referenced_path.split_once("::")
        && let Some(dependency_crate_id) = context.workspace_dependency_roots.get(dependency_root)
    {
        return NodeId::new(format!("{dependency_crate_id}::module::crate::{rest}"));
    }

    if let Some(rest) = referenced_path.strip_prefix("crate::") {
        return NodeId::new(format!("{crate_prefix}::module::crate::{rest}"));
    }

    if let Some(rest) = referenced_path.strip_prefix("self::") {
        return NodeId::new(format!("{crate_prefix}::module::{module_path}::{rest}"));
    }

    if referenced_path.starts_with("super::") {
        let mut module_segments: Vec<String> = module_path
            .as_str()
            .split("::")
            .map(ToOwned::to_owned)
            .collect();
        let mut rest = referenced_path;
        while let Some(stripped) = rest.strip_prefix("super::") {
            if module_segments.len() > 1 {
                module_segments.pop();
            }
            rest = stripped;
        }
        return NodeId::new(format!(
            "{crate_prefix}::module::{}::{rest}",
            module_segments.join("::")
        ));
    }

    NodeId::new(format!(
        "{crate_prefix}::module::{module_path}::{referenced_path}"
    ))
}

struct ItemNodeArgs<'a> {
    parent_module_id: &'a NodeId,
    module_path: &'a ModulePath,
    source_path: &'a Path,
    ident: &'a Ident,
    kind: NodeKind,
    visibility: ItemVisibility,
    attributes: Vec<LintAttribute>,
}

struct FieldNodeArgs<'a> {
    parent_id: &'a NodeId,
    module_path: &'a ModulePath,
    source_path: &'a Path,
    local_owner_names: &'a BTreeSet<String>,
    owner_name: Option<&'a str>,
    fields: &'a syn::Fields,
}
