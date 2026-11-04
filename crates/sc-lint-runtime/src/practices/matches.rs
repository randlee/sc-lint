//! Resource ownership within lock-result match arms and their returned values.
use super::{Binding, Scanner, bindings::ValueKind, expression_name, pattern_name};
use crate::api::{Api, lookup};

#[derive(Clone, Copy, PartialEq, Eq)]
enum MatchResource {
    LockResult,
    TryLockResult,
    Guard,
}

impl Scanner {
    fn match_resource(&self, expr: &syn::Expr) -> Option<MatchResource> {
        match expr {
            syn::Expr::Paren(expr) => self.match_resource(&expr.expr),
            syn::Expr::MethodCall(call) if self.kind(&call.receiver) == ValueKind::Mutex => {
                if call.method == "lock" {
                    Some(MatchResource::LockResult)
                } else if call.method == "try_lock" {
                    Some(MatchResource::TryLockResult)
                } else {
                    None
                }
            }
            syn::Expr::Call(call) => {
                let syn::Expr::Path(path) = &*call.func else {
                    return None;
                };
                match lookup(&self.imports.resolve(&path.path)) {
                    Some(Api::MutexLock) => Some(MatchResource::LockResult),
                    Some(Api::MutexTryLock) => Some(MatchResource::TryLockResult),
                    _ => None,
                }
            }
            _ if self.guard(expr) || self.kind(expr) == ValueKind::Guard => {
                Some(MatchResource::Guard)
            }
            _ => None,
        }
    }

    fn owned_binding(&self, name: String, kind: ValueKind) -> Binding {
        Binding {
            name,
            kind,
            guard: self.asynchronous,
        }
    }

    fn pattern_binding(&self, pat: &syn::Pat, kind: ValueKind) -> Option<Binding> {
        let name = pattern_name(pat)?;
        let kind = if borrowed_pattern(pat) {
            ValueKind::BorrowedGuard
        } else {
            kind
        };
        Some(self.owned_binding(name, kind))
    }

    pub(super) fn pattern_guard(&self, pat: &syn::Pat, expr: &syn::Expr) -> Option<Binding> {
        match pat {
            syn::Pat::Paren(pat) => return self.pattern_guard(&pat.pat, expr),
            syn::Pat::Type(pat) => return self.pattern_guard(&pat.pat, expr),
            syn::Pat::Or(pat) => {
                return pat
                    .cases
                    .iter()
                    .find_map(|case| self.pattern_guard(case, expr));
            }
            _ => {}
        }
        let resource = self.match_resource(expr)?;
        let syn::Pat::TupleStruct(pattern) = pat else {
            return None;
        };
        let variant = pattern.path.segments.last()?.ident.to_string();
        if variant == "Err" && resource == MatchResource::TryLockResult {
            let syn::Pat::TupleStruct(poisoned) = pattern.elems.first()? else {
                return None;
            };
            if poisoned.path.segments.last()?.ident != "Poisoned" {
                return None;
            }
            return self.pattern_binding(poisoned.elems.first()?, ValueKind::PoisonedGuard);
        }
        let kind = match variant.as_str() {
            "Ok" if resource != MatchResource::Guard => ValueKind::Guard,
            "Err" if resource == MatchResource::LockResult => ValueKind::PoisonedGuard,
            _ => return None,
        };
        self.pattern_binding(pattern.elems.first()?, kind)
    }

    pub(super) fn match_arm_bindings(&self, pat: &syn::Pat, expr: &syn::Expr) -> Vec<Binding> {
        match pat {
            syn::Pat::Paren(pat) => return self.match_arm_bindings(&pat.pat, expr),
            syn::Pat::Type(pat) => return self.match_arm_bindings(&pat.pat, expr),
            syn::Pat::Or(pat) => {
                return pat
                    .cases
                    .iter()
                    .map(|case| self.match_arm_bindings(case, expr))
                    .find(|bindings| !bindings.is_empty())
                    .unwrap_or_default();
            }
            _ => {}
        }
        let Some(resource) = self.match_resource(expr) else {
            return Vec::new();
        };
        if let Some(binding) = self.pattern_binding(pat, ValueKind::Guard) {
            // Move bindings own the scrutinee; ref bindings keep it borrowed.
            return vec![binding];
        }
        if let Some(binding) = self.pattern_guard(pat, expr) {
            return vec![binding];
        }
        if !unnamed_resource(pat, resource) {
            return Vec::new();
        }
        // Unnamed successful payloads retain the scrutinee resource. Blocking
        // lock results also retain errors and guarded-pattern fallback values.
        vec![self.owned_binding("<match scrutinee>".into(), ValueKind::Guard)]
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

fn unnamed_resource(pat: &syn::Pat, resource: MatchResource) -> bool {
    match pat {
        syn::Pat::Wild(_) | syn::Pat::Rest(_) => resource != MatchResource::TryLockResult,
        syn::Pat::Paren(pat) => unnamed_resource(&pat.pat, resource),
        syn::Pat::Type(pat) => unnamed_resource(&pat.pat, resource),
        syn::Pat::Or(pat) => pat
            .cases
            .iter()
            .any(|case| unnamed_resource(case, resource)),
        syn::Pat::TupleStruct(pattern) => {
            let Some(variant) = pattern.path.segments.last() else {
                return false;
            };
            let retains = variant.ident == "Ok"
                || (variant.ident == "Err" && resource != MatchResource::TryLockResult);
            retains
                && pattern
                    .elems
                    .iter()
                    .any(|pat| unnamed_resource(pat, MatchResource::Guard))
        }
        _ => false,
    }
}

fn borrowed_pattern(pat: &syn::Pat) -> bool {
    match pat {
        syn::Pat::Ident(pat) => pat.by_ref.is_some(),
        syn::Pat::Paren(pat) => borrowed_pattern(&pat.pat),
        syn::Pat::Type(pat) => borrowed_pattern(&pat.pat),
        _ => false,
    }
}
