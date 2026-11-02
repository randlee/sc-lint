use std::collections::BTreeMap;
use syn::visit::Visit;

#[derive(Default)]
pub(super) struct Imports {
    aliases: BTreeMap<String, Vec<String>>,
    globs: Vec<Vec<String>>,
}

impl Imports {
    pub(super) fn collect(file: &syn::File) -> Self {
        let mut imports = Self::default();
        imports.visit_file(file);
        imports
    }

    pub(super) fn resolve(&self, path: &syn::Path) -> Vec<String> {
        self.expand(
            path.segments
                .iter()
                .map(|part| part.ident.to_string())
                .collect(),
        )
    }

    fn expand_aliases(&self, mut path: Vec<String>) -> Vec<String> {
        for _ in 0..=self.aliases.len() {
            let Some(prefix) = path.first().and_then(|name| self.aliases.get(name)) else {
                break;
            };
            if prefix.first() == path.first() {
                break;
            }
            path = prefix.iter().chain(path.iter().skip(1)).cloned().collect();
        }
        path
    }

    fn expand(&self, path: Vec<String>) -> Vec<String> {
        let path = self.expand_aliases(path);
        if path.len() == 1 {
            for glob in &self.globs {
                let candidate = self.expand_prefix(glob, &path[0]);
                if matches!(
                    candidate.join("::").as_str(),
                    "std::thread::sleep_until"
                        | "std::sync::Mutex"
                        | "std::mem::drop"
                        | "tokio::time::sleep"
                        | "tokio::time::sleep_until"
                        | "tokio::runtime::Runtime"
                        | "tokio::runtime::Handle"
                        | "tokio::runtime::Builder"
                        | "futures::executor::block_on"
                ) {
                    return candidate;
                }
            }
        }
        path
    }

    fn expand_prefix(&self, prefix: &[String], name: &str) -> Vec<String> {
        let mut path = prefix.to_vec();
        path.push(name.to_owned());
        self.expand_aliases(path)
    }

    fn tree(&mut self, tree: &syn::UseTree, mut prefix: Vec<String>) {
        match tree {
            syn::UseTree::Path(path) => {
                prefix.push(path.ident.to_string());
                self.tree(&path.tree, prefix);
            }
            syn::UseTree::Name(name) => {
                if name.ident != "self" {
                    prefix.push(name.ident.to_string());
                }
                if let Some(name) = prefix.last() {
                    self.aliases.insert(name.clone(), prefix.clone());
                }
            }
            syn::UseTree::Rename(rename) => {
                if rename.ident != "self" {
                    prefix.push(rename.ident.to_string());
                }
                self.aliases.insert(rename.rename.to_string(), prefix);
            }
            syn::UseTree::Glob(_) => {
                if !prefix.is_empty() {
                    self.globs.push(prefix);
                }
            }
            syn::UseTree::Group(group) => {
                for tree in &group.items {
                    self.tree(tree, prefix.clone());
                }
            }
        }
    }
}

impl<'ast> Visit<'ast> for Imports {
    fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
        if super::production(&item.attrs, None) {
            syn::visit::visit_item_fn(self, item);
        }
    }
    fn visit_impl_item_fn(&mut self, item: &'ast syn::ImplItemFn) {
        if super::production(&item.attrs, None) {
            syn::visit::visit_impl_item_fn(self, item);
        }
    }
    fn visit_trait_item_fn(&mut self, item: &'ast syn::TraitItemFn) {
        if super::production(&item.attrs, None) {
            syn::visit::visit_trait_item_fn(self, item);
        }
    }
    fn visit_item_impl(&mut self, item: &'ast syn::ItemImpl) {
        if super::production(&item.attrs, None) {
            syn::visit::visit_item_impl(self, item);
        }
    }
    fn visit_item_trait(&mut self, item: &'ast syn::ItemTrait) {
        if super::production(&item.attrs, None) {
            syn::visit::visit_item_trait(self, item);
        }
    }
    fn visit_item_const(&mut self, item: &'ast syn::ItemConst) {
        if super::production(&item.attrs, None) {
            syn::visit::visit_item_const(self, item);
        }
    }
    fn visit_item_static(&mut self, item: &'ast syn::ItemStatic) {
        if super::production(&item.attrs, None) {
            syn::visit::visit_item_static(self, item);
        }
    }

    fn visit_item_use(&mut self, item: &'ast syn::ItemUse) {
        if super::production(&item.attrs, None) {
            self.tree(&item.tree, Vec::new());
        }
    }
    fn visit_item_mod(&mut self, item: &'ast syn::ItemMod) {
        if super::production(&item.attrs, Some(item.ident == "tests")) {
            syn::visit::visit_item_mod(self, item);
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ValueKind {
    Mutex,
    Runtime,
    Handle,
    Builder,
    Guard,
    Other,
}

pub(super) fn type_kind(ty: &syn::Type, imports: &Imports) -> ValueKind {
    match ty {
        syn::Type::Reference(reference) => type_kind(&reference.elem, imports),
        syn::Type::Paren(paren) => type_kind(&paren.elem, imports),
        syn::Type::Path(path) => {
            let resolved = imports.resolve(&path.path).join("::");
            match resolved.as_str() {
                "std::sync::Mutex" => ValueKind::Mutex,
                "std::sync::MutexGuard" => ValueKind::Guard,
                "tokio::runtime::Runtime" => ValueKind::Runtime,
                "tokio::runtime::Handle" => ValueKind::Handle,
                "tokio::runtime::Builder" => ValueKind::Builder,
                _ => wrapped_kind(path, imports),
            }
        }
        _ => ValueKind::Other,
    }
}

fn wrapped_kind(path: &syn::TypePath, imports: &Imports) -> ValueKind {
    // Common ownership wrappers preserve the identity of their contained value.
    let name = imports.resolve(&path.path).join("::");
    if !matches!(
        name.as_str(),
        "Arc"
            | "Rc"
            | "Box"
            | "std::sync::Arc"
            | "std::rc::Rc"
            | "std::boxed::Box"
            | "alloc::sync::Arc"
            | "alloc::rc::Rc"
            | "alloc::boxed::Box"
    ) {
        return ValueKind::Other;
    }
    if let Some(segment) = path.path.segments.last()
        && let syn::PathArguments::AngleBracketed(args) = &segment.arguments
    {
        for arg in &args.args {
            if let syn::GenericArgument::Type(ty) = arg {
                return type_kind(ty, imports);
            }
        }
    }
    ValueKind::Other
}
