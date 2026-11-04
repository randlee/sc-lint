use crate::api::{Api, lookup, lookup_path};
use sc_lint_analyzer_support::is_test_only;
use sc_lint_analyzer_support::span_start_line;
use std::collections::BTreeSet;
use syn::spanned::Spanned;
use syn::visit::Visit;

pub(crate) struct SleepSite {
    pub(crate) line: usize,
    pub(crate) column: usize,
    pub(crate) enclosing_fn: Option<String>,
}

pub(crate) fn find_blocking_sleeps(file: &syn::File) -> Vec<SleepSite> {
    let mut imports = ImportVisitor::default();
    imports.visit_file(file);
    let mut calls = CallVisitor::new(imports.bindings);
    calls.visit_file(file);
    calls.sites
}

#[derive(Clone, Default)]
struct SleepBindings {
    thread_modules: BTreeSet<String>,
    sleeps: BTreeSet<String>,
    glob_thread: bool,
}

#[derive(Default)]
struct ImportVisitor {
    bindings: SleepBindings,
}

struct CallVisitor {
    bindings: SleepBindings,
    sites: Vec<SleepSite>,
    functions: Vec<String>,
}

impl CallVisitor {
    fn new(bindings: SleepBindings) -> Self {
        Self {
            bindings,
            sites: Vec::new(),
            functions: Vec::new(),
        }
    }

    fn visit_function(&mut self, name: String, block: &syn::Block) {
        self.functions.push(name);
        self.visit_block(block);
        self.functions.pop();
    }

    fn is_sleep(&self, expression: &syn::Expr) -> bool {
        let syn::Expr::Path(path) = expression else {
            return false;
        };
        if path.qself.is_some() || path.path.segments.iter().any(has_arguments) {
            return false;
        }
        let parts = &path.path.segments;
        lookup_path(&path.path) == Some(Api::ThreadSleep)
            || (parts.len() == 2
                && parts[1].ident == "sleep"
                && self
                    .bindings
                    .thread_modules
                    .iter()
                    .any(|name| parts[0].ident == name.as_str()))
            || (parts.len() == 1
                && (self
                    .bindings
                    .sleeps
                    .iter()
                    .any(|name| parts[0].ident == name.as_str())
                    || (parts[0].ident == "sleep" && self.bindings.glob_thread)))
    }
}

impl<'ast> Visit<'ast> for ImportVisitor {
    fn visit_item_use(&mut self, item: &'ast syn::ItemUse) {
        if !is_test_only(&item.attrs) {
            crate::imports::visit_use_tree(&item.tree, &mut |import| {
                self.bindings.import(import);
            });
        }
    }

    fn visit_item_mod(&mut self, item: &'ast syn::ItemMod) {
        if !is_test_only(&item.attrs) {
            syn::visit::visit_item_mod(self, item);
        }
    }

    fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
        if !is_test_only(&item.attrs) {
            syn::visit::visit_item_fn(self, item);
        }
    }

    fn visit_item_impl(&mut self, item: &'ast syn::ItemImpl) {
        if !is_test_only(&item.attrs) {
            syn::visit::visit_item_impl(self, item);
        }
    }

    fn visit_item_trait(&mut self, item: &'ast syn::ItemTrait) {
        if !is_test_only(&item.attrs) {
            syn::visit::visit_item_trait(self, item);
        }
    }

    fn visit_impl_item_fn(&mut self, item: &'ast syn::ImplItemFn) {
        if !is_test_only(&item.attrs) {
            syn::visit::visit_impl_item_fn(self, item);
        }
    }

    fn visit_trait_item_fn(&mut self, item: &'ast syn::TraitItemFn) {
        if !is_test_only(&item.attrs) {
            syn::visit::visit_trait_item_fn(self, item);
        }
    }

    fn visit_item_const(&mut self, item: &'ast syn::ItemConst) {
        if !is_test_only(&item.attrs) {
            syn::visit::visit_item_const(self, item);
        }
    }

    fn visit_item_static(&mut self, item: &'ast syn::ItemStatic) {
        if !is_test_only(&item.attrs) {
            syn::visit::visit_item_static(self, item);
        }
    }
}

impl<'ast> Visit<'ast> for CallVisitor {
    fn visit_item_use(&mut self, item: &'ast syn::ItemUse) {
        if !is_test_only(&item.attrs) {
            syn::visit::visit_item_use(self, item);
        }
    }

    fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
        if !is_test_only(&item.attrs) {
            self.visit_function(item.sig.ident.to_string(), &item.block);
        }
    }

    fn visit_impl_item_fn(&mut self, item: &'ast syn::ImplItemFn) {
        if !is_test_only(&item.attrs) {
            self.visit_function(item.sig.ident.to_string(), &item.block);
        }
    }

    fn visit_trait_item_fn(&mut self, item: &'ast syn::TraitItemFn) {
        if !is_test_only(&item.attrs)
            && let Some(block) = &item.default
        {
            self.visit_function(item.sig.ident.to_string(), block);
        }
    }

    fn visit_item_mod(&mut self, item: &'ast syn::ItemMod) {
        if !is_test_only(&item.attrs) {
            syn::visit::visit_item_mod(self, item);
        }
    }

    fn visit_item_impl(&mut self, item: &'ast syn::ItemImpl) {
        if !is_test_only(&item.attrs) {
            syn::visit::visit_item_impl(self, item);
        }
    }

    fn visit_item_trait(&mut self, item: &'ast syn::ItemTrait) {
        if !is_test_only(&item.attrs) {
            syn::visit::visit_item_trait(self, item);
        }
    }

    fn visit_item_const(&mut self, item: &'ast syn::ItemConst) {
        if !is_test_only(&item.attrs) {
            syn::visit::visit_item_const(self, item);
        }
    }

    fn visit_item_static(&mut self, item: &'ast syn::ItemStatic) {
        if !is_test_only(&item.attrs) {
            syn::visit::visit_item_static(self, item);
        }
    }

    fn visit_expr_call(&mut self, call: &'ast syn::ExprCall) {
        if self.is_sleep(&call.func) {
            self.sites.push(SleepSite {
                line: span_start_line(call.span()),
                column: call.span().start().column,
                enclosing_fn: self.functions.last().cloned(),
            });
        }
        syn::visit::visit_expr_call(self, call);
    }
}

impl SleepBindings {
    fn import(&mut self, import: crate::imports::Import) {
        match import {
            crate::imports::Import::Name { path, rename } => {
                if rename.is_none() && path == ["std", "thread", "self"] {
                    self.thread_modules.insert("thread".into());
                } else {
                    bind_import(self, &path, rename);
                }
            }
            crate::imports::Import::Glob(path) => {
                if lookup(&path.join("::")) == Some(Api::Thread) {
                    self.glob_thread = true;
                }
                if lookup(&path.join("::")) == Some(Api::Std) {
                    self.thread_modules.insert("thread".into());
                }
            }
        }
    }
}

fn bind_import(bindings: &mut SleepBindings, path: &[String], rename: Option<String>) {
    if lookup(&path.join("::")) == Some(Api::Thread) {
        bindings
            .thread_modules
            .insert(rename.clone().unwrap_or_else(|| "thread".into()));
    }
    if lookup(&path.join("::")) == Some(Api::ThreadSleep) {
        bindings
            .sleeps
            .insert(rename.unwrap_or_else(|| "sleep".into()));
    }
}

fn has_arguments(segment: &syn::PathSegment) -> bool {
    !matches!(segment.arguments, syn::PathArguments::None)
}
