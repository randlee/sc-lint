use crate::api::{Api, known_glob_path, lookup};
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
        if !path.is_empty() {
            for glob in &self.globs {
                let mut candidate = glob.clone();
                candidate.extend(path.iter().cloned());
                let candidate = self.expand_aliases(candidate);
                if known_glob_path(&candidate)
                    || (candidate.len() > 1 && known_glob_path(&candidate[..candidate.len() - 1]))
                {
                    return candidate;
                }
            }
        }
        path
    }

    fn tree(&mut self, tree: &syn::UseTree) {
        crate::imports::visit_use_tree(tree, &mut |import| match import {
            crate::imports::Import::Name { mut path, rename } => {
                if path.last().is_some_and(|name| name == "self") {
                    path.pop();
                }
                if let Some(name) = rename.or_else(|| path.last().cloned()) {
                    self.aliases.insert(name, path);
                }
            }
            crate::imports::Import::Glob(path) => {
                if !path.is_empty() {
                    self.globs.push(path);
                }
            }
        });
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
            self.tree(&item.tree);
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
    PoisonedGuard,
    NeverCallable,
    Other,
}

pub(super) fn type_kind(ty: &syn::Type, imports: &Imports) -> ValueKind {
    match ty {
        syn::Type::FnPtr(function) if super::flow::never_output(&function.output) => {
            ValueKind::NeverCallable
        }
        syn::Type::Reference(reference) => type_kind(&reference.elem, imports),
        syn::Type::Paren(paren) => type_kind(&paren.elem, imports),
        syn::Type::Path(path) => {
            let resolved = imports.resolve(&path.path).join("::");
            match lookup(&resolved) {
                Some(Api::Mutex) => ValueKind::Mutex,
                Some(Api::MutexGuard) => ValueKind::Guard,
                Some(Api::Runtime) => ValueKind::Runtime,
                Some(Api::Handle) => ValueKind::Handle,
                Some(Api::Builder) => ValueKind::Builder,
                _ => wrapped_kind(path, imports),
            }
        }
        _ => ValueKind::Other,
    }
}

fn wrapped_kind(path: &syn::TypePath, imports: &Imports) -> ValueKind {
    // Common ownership wrappers preserve the identity of their contained value.
    let name = imports.resolve(&path.path).join("::");
    if lookup(&name) != Some(Api::Owner) {
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
