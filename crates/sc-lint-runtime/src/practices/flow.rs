//! Reachability of branch tails, independent of the guard-state transfer walk.
use super::{Scanner, bindings::ValueKind};
use std::collections::BTreeSet;
use syn::visit::Visit;

#[derive(Default)]
pub(super) struct NeverFunctions {
    pub(super) names: BTreeSet<String>,
    ordinary: BTreeSet<String>,
    owner: String,
}
impl NeverFunctions {
    fn record(&mut self, name: String, output: &syn::ReturnType) {
        if never_output(output) {
            if !self.ordinary.contains(&name) {
                self.names.insert(name);
            }
        } else {
            self.names.remove(&name);
            self.ordinary.insert(name);
        }
    }
}
impl<'ast> Visit<'ast> for NeverFunctions {
    fn visit_item_impl(&mut self, item: &'ast syn::ItemImpl) {
        if !super::production(&item.attrs, None) {
            return;
        }
        let owner = self.owner.clone();
        if let syn::Type::Path(path) = &*item.self_ty {
            self.owner = path
                .path
                .segments
                .last()
                .map_or_else(String::new, |part| part.ident.to_string());
        }
        syn::visit::visit_item_impl(self, item);
        self.owner = owner;
    }
    fn visit_impl_item_fn(&mut self, item: &'ast syn::ImplItemFn) {
        if !super::production(&item.attrs, None) {
            return;
        }
        self.record(
            format!("{}::{}", self.owner, item.sig.ident),
            &item.sig.output,
        );
        syn::visit::visit_impl_item_fn(self, item);
    }

    fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
        if !super::production(&item.attrs, None) {
            return;
        }
        self.record(item.sig.ident.to_string(), &item.sig.output);
        syn::visit::visit_item_fn(self, item);
    }
    fn visit_item_mod(&mut self, item: &'ast syn::ItemMod) {
        if super::production(&item.attrs, Some(item.ident == "tests")) {
            syn::visit::visit_item_mod(self, item);
        }
    }
}

pub(super) fn never_output(output: &syn::ReturnType) -> bool {
    matches!(output, syn::ReturnType::Type(_, ty) if matches!(&**ty, syn::Type::Never(_)))
}

impl Scanner {
    pub(super) fn block_continues(&self, block: &syn::Block) -> bool {
        block.stmts.iter().all(|stmt| match stmt {
            syn::Stmt::Expr(expr, _) => self.expr_continues(expr),
            syn::Stmt::Local(local) => local
                .init
                .as_ref()
                .is_none_or(|init| self.expr_continues(&init.expr)),
            syn::Stmt::Macro(statement) => !self.never_macro(&statement.mac),
            syn::Stmt::Item(_) => true,
        })
    }
    pub(super) fn expr_continues(&self, expr: &syn::Expr) -> bool {
        match expr {
            syn::Expr::Return(_) | syn::Expr::Break(_) | syn::Expr::Continue(_) => false,
            syn::Expr::Macro(expr) => !self.never_macro(&expr.mac),
            syn::Expr::Block(expr) => {
                self.block_continues(&expr.block)
                    || expr.label.as_ref().is_some_and(|label| {
                        let mut breaks = Breaks {
                            seen: false,
                            label: Some(label.name.ident.to_string()),
                        };
                        breaks.visit_block(&expr.block);
                        breaks.seen
                    })
            }
            syn::Expr::Unsafe(expr) => self.block_continues(&expr.block),
            syn::Expr::Paren(expr) => self.expr_continues(&expr.expr),
            syn::Expr::Group(expr) => self.expr_continues(&expr.expr),
            syn::Expr::Try(expr) => self.expr_continues(&expr.expr),
            syn::Expr::Await(expr) => self.expr_continues(&expr.base),
            syn::Expr::Let(expr) => self.expr_continues(&expr.expr),
            syn::Expr::If(branch) => {
                self.expr_continues(&branch.cond)
                    && (self.block_continues(&branch.then_branch)
                        || branch
                            .else_branch
                            .as_ref()
                            .is_none_or(|(_, other)| self.expr_continues(other)))
            }
            syn::Expr::Match(branch) => {
                self.expr_continues(&branch.expr)
                    && branch.arms.iter().any(|arm| self.expr_continues(&arm.body))
            }
            syn::Expr::Call(call) => {
                self.expr_continues(&call.func)
                    && call.args.iter().all(|arg| self.expr_continues(arg))
                    && !self.never_call(&call.func)
            }
            syn::Expr::MethodCall(call) => {
                self.expr_continues(&call.receiver)
                    && call.args.iter().all(|arg| self.expr_continues(arg))
                    && !self.never_method(call)
            }
            syn::Expr::Loop(looped) => {
                let mut breaks = Breaks {
                    seen: false,
                    label: None,
                };
                breaks.visit_block(&looped.body);
                breaks.seen
            }
            _ => true,
        }
    }
    fn never_macro(&self, mac: &syn::Macro) -> bool {
        matches!(
            self.imports.resolve(&mac.path).join("::").as_str(),
            "panic"
                | "unreachable"
                | "todo"
                | "unimplemented"
                | "std::panic"
                | "core::panic"
                | "std::unreachable"
                | "core::unreachable"
                | "std::todo"
                | "core::todo"
                | "std::unimplemented"
                | "core::unimplemented"
        )
    }
    fn never_method(&self, call: &syn::ExprMethodCall) -> bool {
        matches!(&*call.receiver, syn::Expr::Path(path) if path.path.is_ident("self"))
            && self
                .never_functions
                .contains(&format!("{}::{}", self.owner, call.method))
    }
    fn never_call(&self, expr: &syn::Expr) -> bool {
        if let syn::Expr::Paren(expr) = expr {
            return self.never_call(&expr.expr);
        }
        if self.kind(expr) == ValueKind::NeverCallable {
            return true;
        }
        let syn::Expr::Path(path) = expr else {
            return false;
        };
        let resolved = self.imports.resolve(&path.path);
        if matches!(
            resolved.join("::").as_str(),
            "std::process::exit" | "std::process::abort"
        ) {
            return true;
        }
        if path
            .path
            .get_ident()
            .is_some_and(|name| self.bindings.iter().any(|binding| *name == binding.name))
        {
            return false;
        }
        if resolved.len() == 2 {
            let owner = if resolved[0] == "Self" {
                &self.owner
            } else {
                &resolved[0]
            };
            return self
                .never_functions
                .contains(&format!("{}::{}", owner, resolved[1]));
        }
        resolved.len() == 1
            && resolved
                .last()
                .is_some_and(|name| self.never_functions.contains(name))
    }
}

struct Breaks {
    seen: bool,
    label: Option<String>,
}
impl<'ast> Visit<'ast> for Breaks {
    fn visit_expr_break(&mut self, expr: &'ast syn::ExprBreak) {
        if self.label.as_ref().is_none_or(|target| {
            expr.label
                .as_ref()
                .is_some_and(|label| label.ident == target)
        }) {
            self.seen = true;
        }
    }
    fn visit_expr_closure(&mut self, _: &'ast syn::ExprClosure) {}
    fn visit_expr_async(&mut self, _: &'ast syn::ExprAsync) {}
    fn visit_item_fn(&mut self, _: &'ast syn::ItemFn) {}
}
