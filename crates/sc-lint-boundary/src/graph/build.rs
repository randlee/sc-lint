use super::*;
use crate::render::hex_encode;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use syn::ItemEnum;
use syn::ItemFn;
use syn::ItemImpl;
use syn::ItemMod;
use syn::ItemStruct;
use syn::ItemTrait;
use syn::ItemType;
use syn::ItemUnion;

use super::reference_collector::collect_references_with;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NodeKind {
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
    fn as_str(self) -> &'static str {
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
    let metadata = load_metadata(root)?;
    let workspace_members = metadata.workspace_members.clone();
    let workspace_packages = metadata
        .packages
        .iter()
        .filter(|package| workspace_members.iter().any(|id| id == &package.id))
        .collect::<Vec<_>>();
    let mut builder = GraphBuilder::default();

    for package in &workspace_packages {
        let manifest_path = package.manifest_path.as_std_path().display().to_string();
        for target in &package.targets {
            if !is_supported_target(target) {
                continue;
            }

            let source_path = target.src_path.as_std_path().to_path_buf();
            let context = TargetContext {
                package_name: package.name.to_string(),
                target_name: target.name.clone(),
                manifest_path: manifest_path.clone(),
                crate_id: crate_id(&package.name, &target.name),
                root_module_path: ModulePath::crate_root(),
                workspace_dependency_roots: workspace_dependency_roots(
                    &workspace_packages,
                    package,
                ),
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
            ingest_module_items(
                &mut builder,
                &context,
                &NodeId::new(root_module_id),
                &context.root_module_path,
                &root_dir,
                &source_path,
                parse_rust_file(&source_path)?,
            )?;
        }
    }

    Ok(builder.finish())
}

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

struct ModuleItemContext<'a> {
    context: &'a TargetContext,
    parent_module_id: &'a NodeId,
    module_path: &'a ModulePath,
    module_dir: &'a Path,
    source_path: &'a Path,
    local_owner_names: &'a BTreeSet<String>,
}

fn ingest_module_items(
    builder: &mut GraphBuilder,
    context: &TargetContext,
    parent_module_id: &NodeId,
    module_path: &ModulePath,
    module_dir: &Path,
    source_path: &Path,
    file: File,
) -> Result<()> {
    let local_owner_names = collect_owner_names(&file.items);
    let item_context = ModuleItemContext {
        context,
        parent_module_id,
        module_path,
        module_dir,
        source_path,
        local_owner_names: &local_owner_names,
    };
    for item in file.items {
        ingest_module_item(builder, &item_context, item)?;
    }
    Ok(())
}

fn ingest_module_item(
    builder: &mut GraphBuilder,
    context: &ModuleItemContext<'_>,
    item: Item,
) -> Result<()> {
    match item {
        Item::Mod(item) => ingest_module(builder, context, item),
        Item::Struct(item) => ingest_struct(builder, context, item),
        Item::Enum(item) => ingest_enum(builder, context, item),
        Item::Union(item) => ingest_union(builder, context, item),
        Item::Type(item) => ingest_type(builder, context, item),
        Item::Trait(item) => ingest_trait(builder, context, item),
        Item::Fn(item) => ingest_function(builder, context, item),
        Item::Impl(item) => ingest_impl(builder, context, item),
        _ => Ok(()),
    }
}

fn ingest_module(
    builder: &mut GraphBuilder,
    context: &ModuleItemContext<'_>,
    item: ItemMod,
) -> Result<()> {
    let name = item.ident.to_string();
    let child_path = context.module_path.child(&name);
    let child_id = format!(
        "{}::module::{child_path}",
        context.context.crate_id.as_str()
    );
    builder.add_node(GraphNode {
        id: NodeId::new(child_id.clone()),
        kind: NodeKind::Module.as_str(),
        label: name.clone(),
        visibility: Some(visibility_label(&item.vis).as_str()),
        package: context.context.package_name.clone(),
        target: Some(context.context.target_name.clone()),
        manifest_path: context.context.manifest_path.clone(),
        source_path: Some(context.source_path.display().to_string()),
        module_path: Some(child_path.to_string()),
        impl_kind: None,
        impl_trait: None,
        attributes: parse_lint_attributes(&item.attrs)?,
    });
    builder.add_edge(
        EdgeKind::Contains,
        context.parent_module_id.clone(),
        child_id.clone(),
    );
    if let Some((_, items)) = item.content {
        return ingest_module_items(
            builder,
            context.context,
            &NodeId::new(child_id),
            &child_path,
            &context.module_dir.join(&name),
            context.source_path,
            File {
                shebang: None,
                attrs: Vec::new(),
                items,
            },
        );
    }
    let child_source =
        resolve_module_source(context.source_path, context.module_dir, &name, &item.attrs)
            .with_context(|| format!("while resolving module `{child_path}`"))?;
    let child_dir = child_module_dir(context.module_dir, &name, &child_source, &item.attrs);
    ingest_module_items(
        builder,
        context.context,
        &NodeId::new(child_id),
        &child_path,
        &child_dir,
        &child_source,
        parse_rust_file(&child_source)?,
    )
}

fn child_module_dir(
    module_dir: &Path,
    name: &str,
    source: &Path,
    attributes: &[Attribute],
) -> PathBuf {
    if has_explicit_module_path(attributes)
        || source.file_name().and_then(|name| name.to_str()) == Some("mod.rs")
    {
        return source
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| module_dir.join(name));
    }
    module_dir.join(name)
}

fn ingest_struct(
    builder: &mut GraphBuilder,
    context: &ModuleItemContext<'_>,
    item: ItemStruct,
) -> Result<()> {
    let owner = item.ident.to_string();
    let id = add_item_node(
        builder,
        context.context,
        ItemNodeArgs {
            parent_module_id: context.parent_module_id,
            module_path: context.module_path,
            source_path: context.source_path,
            ident: &item.ident,
            kind: NodeKind::Type,
            visibility: visibility_label(&item.vis),
            attributes: parse_lint_attributes(&item.attrs)?,
        },
    );
    add_reference_edges(
        builder,
        context.context,
        &id,
        context.module_path,
        collect_references_with(
            context.local_owner_names,
            Some(&owner),
            &context.context.workspace_dependency_roots,
            |collector| collector.visit_fields(&item.fields),
        ),
    );
    add_field_nodes(
        builder,
        context.context,
        FieldNodeArgs {
            parent_id: &id,
            module_path: context.module_path,
            source_path: context.source_path,
            local_owner_names: context.local_owner_names,
            owner_name: Some(&owner),
            fields: &item.fields,
        },
    );
    Ok(())
}

fn ingest_enum(
    builder: &mut GraphBuilder,
    context: &ModuleItemContext<'_>,
    item: ItemEnum,
) -> Result<()> {
    let owner = item.ident.to_string();
    let id = add_item_node(
        builder,
        context.context,
        ItemNodeArgs {
            parent_module_id: context.parent_module_id,
            module_path: context.module_path,
            source_path: context.source_path,
            ident: &item.ident,
            kind: NodeKind::Type,
            visibility: visibility_label(&item.vis),
            attributes: parse_lint_attributes(&item.attrs)?,
        },
    );
    add_reference_edges(
        builder,
        context.context,
        &id,
        context.module_path,
        collect_references_with(
            context.local_owner_names,
            Some(&owner),
            &context.context.workspace_dependency_roots,
            |collector| {
                for variant in &item.variants {
                    collector.visit_fields(&variant.fields);
                }
            },
        ),
    );
    add_enum_variants(builder, context, &id, &owner, &item);
    Ok(())
}

fn add_enum_variants(
    builder: &mut GraphBuilder,
    context: &ModuleItemContext<'_>,
    enum_id: &NodeId,
    owner: &str,
    item: &ItemEnum,
) {
    for variant in &item.variants {
        let id = NodeId::new(format!("{enum_id}::variant::{}", variant.ident));
        builder.add_node(GraphNode {
            id: id.clone(),
            kind: NodeKind::Variant.as_str(),
            label: variant.ident.to_string(),
            visibility: None,
            package: context.context.package_name.clone(),
            target: Some(context.context.target_name.clone()),
            manifest_path: context.context.manifest_path.clone(),
            source_path: Some(context.source_path.display().to_string()),
            module_path: Some(context.module_path.to_string()),
            impl_kind: None,
            impl_trait: None,
            attributes: Vec::new(),
        });
        builder.add_edge(EdgeKind::Contains, enum_id.clone(), id.clone());
        add_field_nodes(
            builder,
            context.context,
            FieldNodeArgs {
                parent_id: &id,
                module_path: context.module_path,
                source_path: context.source_path,
                local_owner_names: context.local_owner_names,
                owner_name: Some(owner),
                fields: &variant.fields,
            },
        );
    }
}

fn ingest_union(
    builder: &mut GraphBuilder,
    context: &ModuleItemContext<'_>,
    item: ItemUnion,
) -> Result<()> {
    let owner = item.ident.to_string();
    let id = add_item_node(
        builder,
        context.context,
        ItemNodeArgs {
            parent_module_id: context.parent_module_id,
            module_path: context.module_path,
            source_path: context.source_path,
            ident: &item.ident,
            kind: NodeKind::Type,
            visibility: visibility_label(&item.vis),
            attributes: parse_lint_attributes(&item.attrs)?,
        },
    );
    add_reference_edges(
        builder,
        context.context,
        &id,
        context.module_path,
        collect_references_with(
            context.local_owner_names,
            Some(&owner),
            &context.context.workspace_dependency_roots,
            |collector| collector.visit_fields_named(&item.fields),
        ),
    );
    let fields = syn::Fields::Named(item.fields.clone());
    add_field_nodes(
        builder,
        context.context,
        FieldNodeArgs {
            parent_id: &id,
            module_path: context.module_path,
            source_path: context.source_path,
            local_owner_names: context.local_owner_names,
            owner_name: Some(&owner),
            fields: &fields,
        },
    );
    Ok(())
}

fn ingest_type(
    builder: &mut GraphBuilder,
    context: &ModuleItemContext<'_>,
    item: ItemType,
) -> Result<()> {
    let owner = item.ident.to_string();
    let id = add_item_node(
        builder,
        context.context,
        ItemNodeArgs {
            parent_module_id: context.parent_module_id,
            module_path: context.module_path,
            source_path: context.source_path,
            ident: &item.ident,
            kind: NodeKind::Type,
            visibility: visibility_label(&item.vis),
            attributes: parse_lint_attributes(&item.attrs)?,
        },
    );
    add_reference_edges(
        builder,
        context.context,
        &id,
        context.module_path,
        collect_references_with(
            context.local_owner_names,
            Some(&owner),
            &context.context.workspace_dependency_roots,
            |collector| collector.visit_type(&item.ty),
        ),
    );
    Ok(())
}

fn ingest_trait(
    builder: &mut GraphBuilder,
    context: &ModuleItemContext<'_>,
    item: ItemTrait,
) -> Result<()> {
    let owner = item.ident.to_string();
    let id = add_item_node(
        builder,
        context.context,
        ItemNodeArgs {
            parent_module_id: context.parent_module_id,
            module_path: context.module_path,
            source_path: context.source_path,
            ident: &item.ident,
            kind: NodeKind::Trait,
            visibility: visibility_label(&item.vis),
            attributes: parse_lint_attributes(&item.attrs)?,
        },
    );
    add_reference_edges(
        builder,
        context.context,
        &id,
        context.module_path,
        collect_references_with(
            context.local_owner_names,
            Some(&owner),
            &context.context.workspace_dependency_roots,
            |collector| {
                for trait_item in &item.items {
                    collector.visit_trait_item(trait_item);
                }
            },
        ),
    );
    Ok(())
}

fn ingest_function(
    builder: &mut GraphBuilder,
    context: &ModuleItemContext<'_>,
    item: ItemFn,
) -> Result<()> {
    let id = add_item_node(
        builder,
        context.context,
        ItemNodeArgs {
            parent_module_id: context.parent_module_id,
            module_path: context.module_path,
            source_path: context.source_path,
            ident: &item.sig.ident,
            kind: NodeKind::Function,
            visibility: visibility_label(&item.vis),
            attributes: parse_lint_attributes(&item.attrs)?,
        },
    );
    add_reference_edges(
        builder,
        context.context,
        &id,
        context.module_path,
        collect_references_with(
            context.local_owner_names,
            None,
            &context.context.workspace_dependency_roots,
            |collector| collector.visit_item_fn(&item),
        ),
    );
    Ok(())
}

fn ingest_impl(
    builder: &mut GraphBuilder,
    context: &ModuleItemContext<'_>,
    item: ItemImpl,
) -> Result<()> {
    let owner = impl_owner_name(&item.self_ty)?;
    let owner_id = ensure_impl_owner(builder, context, &owner);
    let trait_path = item
        .trait_
        .as_ref()
        .map(|(_, path, _)| trait_path_key(path));
    let impl_id = add_impl_node(builder, context, &owner, &owner_id, trait_path.clone());
    add_impl_trait_edge(builder, context, &item, &impl_id);
    add_impl_methods(
        builder, context, item, &owner, &owner_id, &impl_id, trait_path,
    )?;
    Ok(())
}

fn ensure_impl_owner(
    builder: &mut GraphBuilder,
    context: &ModuleItemContext<'_>,
    owner: &str,
) -> NodeId {
    let id = NodeId::new(format!("{}::{owner}", context.parent_module_id));
    if builder.nodes.iter().any(|node| node.id == id.as_str()) {
        return id;
    }
    builder.add_node(GraphNode {
        id: id.clone(),
        kind: NodeKind::Type.as_str(),
        label: owner.to_string(),
        visibility: None,
        package: context.context.package_name.clone(),
        target: Some(context.context.target_name.clone()),
        manifest_path: context.context.manifest_path.clone(),
        source_path: Some(context.source_path.display().to_string()),
        module_path: Some(context.module_path.to_string()),
        impl_kind: None,
        impl_trait: None,
        attributes: Vec::new(),
    });
    builder.add_edge(
        EdgeKind::Contains,
        context.parent_module_id.clone(),
        id.clone(),
    );
    id
}

fn add_impl_node(
    builder: &mut GraphBuilder,
    context: &ModuleItemContext<'_>,
    owner: &str,
    owner_id: &NodeId,
    trait_path: Option<String>,
) -> NodeId {
    let id = trait_path
        .as_ref()
        .map(|path| NodeId::new(format!("{owner_id}::impl::{}", hex_encode(path.as_bytes()))))
        .unwrap_or_else(|| NodeId::new(format!("{owner_id}::impl::inherent")));
    let kind = if trait_path.is_some() {
        ImplKind::Trait
    } else {
        ImplKind::Inherent
    };
    builder.add_node(GraphNode {
        id: id.clone(),
        kind: NodeKind::Impl.as_str(),
        label: trait_path
            .as_ref()
            .map(|path| format!("impl {path} for {owner}"))
            .unwrap_or_else(|| format!("impl {owner}")),
        visibility: None,
        package: context.context.package_name.clone(),
        target: Some(context.context.target_name.clone()),
        manifest_path: context.context.manifest_path.clone(),
        source_path: Some(context.source_path.display().to_string()),
        module_path: Some(context.module_path.to_string()),
        impl_kind: Some(kind),
        impl_trait: trait_path,
        attributes: Vec::new(),
    });
    builder.add_edge(
        EdgeKind::Contains,
        context.parent_module_id.clone(),
        id.clone(),
    );
    builder.add_edge(EdgeKind::Targets, id.clone(), owner_id.clone());
    id
}

fn add_impl_trait_edge(
    builder: &mut GraphBuilder,
    context: &ModuleItemContext<'_>,
    item: &ItemImpl,
    impl_id: &NodeId,
) {
    let Some((_, path, _)) = &item.trait_ else {
        return;
    };
    let reference = trait_path_key(path);
    let target =
        resolve_reference_target(context.context, impl_id, context.module_path, &reference);
    ensure_trait_reference_node(
        builder,
        context.context,
        context.source_path,
        context.module_path,
        &target,
        &reference,
    );
    builder.add_edge(EdgeKind::Implements, impl_id.clone(), target);
}

fn add_impl_methods(
    builder: &mut GraphBuilder,
    context: &ModuleItemContext<'_>,
    item: ItemImpl,
    owner: &str,
    owner_id: &NodeId,
    impl_id: &NodeId,
    trait_path: Option<String>,
) -> Result<()> {
    let kind = if item.trait_.is_some() {
        ImplKind::Trait
    } else {
        ImplKind::Inherent
    };
    for impl_item in item.items {
        let ImplItem::Fn(method) = impl_item else {
            continue;
        };
        let id = NodeId::new(format!("{owner_id}::{}", method.sig.ident));
        builder.add_node(GraphNode {
            id: id.clone(),
            kind: NodeKind::Method.as_str(),
            label: method.sig.ident.to_string(),
            visibility: Some(visibility_label(&method.vis).as_str()),
            package: context.context.package_name.clone(),
            target: Some(context.context.target_name.clone()),
            manifest_path: context.context.manifest_path.clone(),
            source_path: Some(context.source_path.display().to_string()),
            module_path: Some(context.module_path.to_string()),
            impl_kind: Some(kind),
            impl_trait: trait_path.clone(),
            attributes: parse_lint_attributes(&method.attrs)?,
        });
        builder.add_edge(EdgeKind::Declares, owner_id.clone(), id.clone());
        builder.add_edge(EdgeKind::Contains, impl_id.clone(), id.clone());
        add_reference_edges(
            builder,
            context.context,
            &id,
            context.module_path,
            collect_references_with(
                context.local_owner_names,
                Some(owner),
                &context.context.workspace_dependency_roots,
                |collector| collector.visit_impl_item_fn(&method),
            ),
        );
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
        let Some(library_target) = workspace_package.targets.iter().find(|target| {
            target
                .kind
                .iter()
                .any(|kind| matches!(kind, cargo_metadata::TargetKind::Lib))
        }) else {
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
