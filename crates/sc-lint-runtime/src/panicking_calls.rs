use crate::position::SourcePosition;
use crate::practices::{PracticeRule, Site};
use crate::scope::production;
use syn::visit::Visit;

pub(crate) fn detect_panicking_calls(file: &syn::File) -> Vec<Site> {
    let mut visitor = Scanner {
        function: "<module>".into(),
        sites: Vec::new(),
    };
    visitor.visit_file(file);
    visitor.sites
}

struct Scanner {
    function: String,
    sites: Vec<Site>,
}

impl Scanner {
    fn function(&mut self, name: &syn::Ident, visit: impl FnOnce(&mut Self)) {
        let previous = std::mem::replace(&mut self.function, name.to_string());
        visit(self);
        self.function = previous;
    }

    fn record(&mut self, position: SourcePosition, detail: String) {
        self.sites.push(Site {
            rule: PracticeRule::PanickingCall,
            position,
            function: self.function.clone(),
            detail,
        });
    }
}

impl<'ast> Visit<'ast> for Scanner {
    fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
        if production(&item.attrs, None) {
            self.function(&item.sig.ident, |visitor| {
                syn::visit::visit_item_fn(visitor, item)
            });
        }
    }
    fn visit_impl_item_fn(&mut self, item: &'ast syn::ImplItemFn) {
        if production(&item.attrs, None) {
            self.function(&item.sig.ident, |visitor| {
                syn::visit::visit_impl_item_fn(visitor, item)
            });
        }
    }
    fn visit_trait_item_fn(&mut self, item: &'ast syn::TraitItemFn) {
        if production(&item.attrs, None) {
            self.function(&item.sig.ident, |visitor| {
                syn::visit::visit_trait_item_fn(visitor, item)
            });
        }
    }
    fn visit_item_mod(&mut self, item: &'ast syn::ItemMod) {
        if production(&item.attrs, Some(item.ident == "tests")) {
            syn::visit::visit_item_mod(self, item);
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
    fn visit_expr_method_call(&mut self, expr: &'ast syn::ExprMethodCall) {
        if !production(&expr.attrs, None) {
            return;
        }
        if expr.method == "unwrap" || expr.method == "expect" {
            self.record(
                SourcePosition::start(expr),
                format!("panicking method call `.{}`", expr.method),
            );
        }
        syn::visit::visit_expr_method_call(self, expr);
    }
    fn visit_expr_macro(&mut self, expr: &'ast syn::ExprMacro) {
        if production(&expr.attrs, None) {
            syn::visit::visit_expr_macro(self, expr);
        }
    }
    fn visit_item_macro(&mut self, item: &'ast syn::ItemMacro) {
        if production(&item.attrs, None) {
            syn::visit::visit_item_macro(self, item);
        }
    }
    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        if let Some(name) = mac.path.segments.last().map(|part| part.ident.to_string())
            && matches!(
                name.as_str(),
                "panic" | "unreachable" | "todo" | "unimplemented"
            )
        {
            self.record(
                SourcePosition::start(mac),
                format!("panicking macro `{name}!`"),
            );
        }
    }
}
