use crate::api::{Api, lookup};
use crate::imports::{Imports, ModuleScoped};
use crate::position::SourcePosition;
use crate::scope::production;
use syn::visit::Visit;

pub(crate) struct SleepSite {
    pub(crate) position: SourcePosition,
    pub(crate) enclosing_fn: Option<String>,
}

pub(crate) fn find_blocking_sleeps(file: &syn::File) -> Vec<SleepSite> {
    let mut calls = CallVisitor {
        imports: Imports::collect(file),
        module: String::new(),
        sites: Vec::new(),
        functions: Vec::new(),
    };
    calls.visit_file(file);
    calls.sites
}

struct CallVisitor {
    imports: Imports,
    module: String,
    sites: Vec<SleepSite>,
    functions: Vec<String>,
}

impl ModuleScoped for CallVisitor {
    fn module_scope(&mut self) -> (&mut Imports, &mut String) {
        (&mut self.imports, &mut self.module)
    }
}

impl CallVisitor {
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
        lookup(&self.imports.resolve(&path.path)) == Some(Api::ThreadSleep)
    }
}

impl<'ast> Visit<'ast> for CallVisitor {
    fn visit_item_use(&mut self, item: &'ast syn::ItemUse) {
        if production(&item.attrs, None) {
            syn::visit::visit_item_use(self, item);
        }
    }

    fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
        if production(&item.attrs, None) {
            self.visit_function(item.sig.ident.to_string(), &item.block);
        }
    }

    fn visit_impl_item_fn(&mut self, item: &'ast syn::ImplItemFn) {
        if production(&item.attrs, None) {
            self.visit_function(item.sig.ident.to_string(), &item.block);
        }
    }

    fn visit_trait_item_fn(&mut self, item: &'ast syn::TraitItemFn) {
        if production(&item.attrs, None)
            && let Some(block) = &item.default
        {
            self.visit_function(item.sig.ident.to_string(), block);
        }
    }

    fn visit_item_mod(&mut self, item: &'ast syn::ItemMod) {
        if production(&item.attrs, None) {
            self.with_module(item, |this| syn::visit::visit_item_mod(this, item));
        }
    }
    fn visit_block(&mut self, block: &'ast syn::Block) {
        self.imports.push_block(block);
        syn::visit::visit_block(self, block);
        self.imports.pop();
    }

    fn visit_item_impl(&mut self, item: &'ast syn::ItemImpl) {
        if production(&item.attrs, None) {
            syn::visit::visit_item_impl(self, item);
        }
    }

    fn visit_item_trait(&mut self, item: &'ast syn::ItemTrait) {
        if production(&item.attrs, None) {
            syn::visit::visit_item_trait(self, item);
        }
    }

    fn visit_item_const(&mut self, item: &'ast syn::ItemConst) {
        if production(&item.attrs, None) {
            syn::visit::visit_item_const(self, item);
        }
    }

    fn visit_item_static(&mut self, item: &'ast syn::ItemStatic) {
        if production(&item.attrs, None) {
            syn::visit::visit_item_static(self, item);
        }
    }

    fn visit_expr_call(&mut self, call: &'ast syn::ExprCall) {
        if self.is_sleep(&call.func) {
            self.sites.push(SleepSite {
                position: SourcePosition::start(call),
                enclosing_fn: self.functions.last().cloned(),
            });
        }
        syn::visit::visit_expr_call(self, call);
    }
}

fn has_arguments(segment: &syn::PathSegment) -> bool {
    !matches!(segment.arguments, syn::PathArguments::None)
}
