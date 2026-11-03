//! Resource ownership within lock-result match arms and their returned values.
use super::{Binding, Scanner, bindings::ValueKind, expression_name};

impl Scanner {
    pub(super) fn match_arm_bindings(&self, pat: &syn::Pat, expr: &syn::Expr) -> Vec<Binding> {
        if !self.guard(expr) {
            return Vec::new();
        }
        if let Some(binding) = self.pattern_guard(pat, expr) {
            return vec![binding];
        }
        // Wildcards do not move the resource out of the lock-result scrutinee.
        // This also covers fallback arms reached after a match guard fails.
        vec![Binding {
            name: "<match scrutinee>".into(),
            kind: ValueKind::Guard,
            guard: self.asynchronous,
        }]
    }

    pub(super) fn release_poisoned_guard(&mut self, call: &syn::ExprMethodCall) {
        if call.method == "into_inner"
            && self.kind(&call.receiver) == ValueKind::PoisonedGuard
            && let Some(name) = expression_name(&call.receiver)
            && let Some(binding) = self
                .bindings
                .iter_mut()
                .rev()
                .find(|binding| binding.name == name)
        {
            // into_inner consumes PoisonError and transfers its guard to the
            // expression result; it is no longer owned by the error binding.
            binding.guard = false;
        }
    }

    pub(super) fn match_result_guard(&self, branch: &syn::ExprMatch) -> bool {
        self.guard(&branch.expr)
            && branch.arms.iter().any(|arm| {
                self.expr_continues(&arm.body)
                    && self
                        .pattern_guard(&arm.pat, &branch.expr)
                        .is_some_and(|binding| returns_guard(&arm.body, &binding))
            })
    }
}

fn returns_guard(expr: &syn::Expr, binding: &Binding) -> bool {
    match expr {
        syn::Expr::Path(_) => {
            binding.kind == ValueKind::Guard
                && expression_name(expr).as_deref() == Some(&binding.name)
        }
        syn::Expr::MethodCall(call) => {
            binding.kind == ValueKind::PoisonedGuard
                && call.method == "into_inner"
                && expression_name(&call.receiver).as_deref() == Some(&binding.name)
        }
        syn::Expr::Block(block) => block.block.stmts.last().is_some_and(
            |stmt| matches!(stmt, syn::Stmt::Expr(expr, None) if returns_guard(expr, binding)),
        ),
        syn::Expr::Paren(expr) => returns_guard(&expr.expr, binding),
        syn::Expr::Group(expr) => returns_guard(&expr.expr, binding),
        _ => false,
    }
}
