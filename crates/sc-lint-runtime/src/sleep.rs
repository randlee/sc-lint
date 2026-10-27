use sc_lint_analyzer_support::span_start_line;
use std::collections::BTreeSet;
use syn::spanned::Spanned;
use syn::visit::Visit;

pub(crate) struct SleepSite {
    pub(crate) line: usize,
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
        let names = path
            .path
            .segments
            .iter()
            .map(|segment| segment.ident.to_string())
            .collect::<Vec<_>>();
        names == ["std", "thread", "sleep"]
            || (names.len() == 2
                && names[1] == "sleep"
                && self.bindings.thread_modules.contains(&names[0]))
            || (names.len() == 1
                && (self.bindings.sleeps.contains(&names[0])
                    || (names[0] == "sleep" && self.bindings.glob_thread)))
    }
}

impl<'ast> Visit<'ast> for ImportVisitor {
    fn visit_item_use(&mut self, item: &'ast syn::ItemUse) {
        if !is_test_only(&item.attrs) {
            import_tree(&mut self.bindings, &item.tree, Vec::new());
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
}

impl<'ast> Visit<'ast> for CallVisitor {
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
                enclosing_fn: self.functions.last().cloned(),
            });
        }
        syn::visit::visit_expr_call(self, call);
    }
}

fn import_tree(bindings: &mut SleepBindings, tree: &syn::UseTree, mut prefix: Vec<String>) {
    match tree {
        syn::UseTree::Path(path) => {
            prefix.push(path.ident.to_string());
            import_tree(bindings, &path.tree, prefix);
        }
        syn::UseTree::Name(name) => {
            let name = name.ident.to_string();
            if name == "self" && prefix == ["std", "thread"] {
                bindings.thread_modules.insert("thread".into());
            } else {
                prefix.push(name);
                bind_import(bindings, &prefix, None);
            }
        }
        syn::UseTree::Rename(rename) => {
            prefix.push(rename.ident.to_string());
            bind_import(bindings, &prefix, Some(rename.rename.to_string()));
        }
        syn::UseTree::Glob(_) => {
            if prefix == ["std", "thread"] {
                bindings.glob_thread = true;
            }
            if prefix == ["std"] {
                bindings.thread_modules.insert("thread".into());
            }
        }
        syn::UseTree::Group(group) => {
            for item in &group.items {
                import_tree(bindings, item, prefix.clone());
            }
        }
    }
}

fn bind_import(bindings: &mut SleepBindings, path: &[String], rename: Option<String>) {
    if path == ["std", "thread"] {
        bindings
            .thread_modules
            .insert(rename.clone().unwrap_or_else(|| "thread".into()));
    }
    if path == ["std", "thread", "sleep"] {
        bindings
            .sleeps
            .insert(rename.unwrap_or_else(|| "sleep".into()));
    }
}

fn has_arguments(segment: &syn::PathSegment) -> bool {
    !matches!(segment.arguments, syn::PathArguments::None)
}

fn is_test_only(attributes: &[syn::Attribute]) -> bool {
    attributes.iter().any(|attribute| {
        attribute.path().is_ident("test")
            || attribute
                .path()
                .segments
                .last()
                .is_some_and(|segment| segment.ident == "test")
            || cfg_is_test(attribute)
    })
}

fn cfg_is_test(attribute: &syn::Attribute) -> bool {
    if !attribute.path().is_ident("cfg") {
        return false;
    }
    attribute
        .parse_args::<syn::Meta>()
        .is_ok_and(|meta| meta_is_test_only(&meta))
}

fn meta_is_test_only(meta: &syn::Meta) -> bool {
    if meta.path().is_ident("test") {
        return true;
    }
    let syn::Meta::List(list) = meta else {
        return false;
    };
    if !list.path.is_ident("all") {
        return false;
    }
    let nested = list.parse_args_with(
        syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated,
    );
    nested.is_ok_and(|items| items.iter().any(meta_is_test_only))
}
