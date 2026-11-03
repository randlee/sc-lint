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
                match call.method.to_string().as_str() {
                    "lock" => Some(MatchResource::LockResult),
                    "try_lock" => Some(MatchResource::TryLockResult),
                    _ => None,
                }
            }
            syn::Expr::Call(call) => {
                let syn::Expr::Path(path) = &*call.func else {
                    return None;
                };
                match lookup(&self.imports.resolve(&path.path).join("::")) {
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

    pub(super) fn pattern_guard(&self, pat: &syn::Pat, expr: &syn::Expr) -> Option<Binding> {
        if let syn::Pat::Paren(pat) = pat {
            return self.pattern_guard(&pat.pat, expr);
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
            let name = poisoned.elems.first().and_then(pattern_name)?;
            return Some(self.owned_binding(name, ValueKind::PoisonedGuard));
        }
        let kind = match variant.as_str() {
            "Ok" if resource != MatchResource::Guard => ValueKind::Guard,
            "Err" if resource == MatchResource::LockResult => ValueKind::PoisonedGuard,
            _ => return None,
        };
        let name = pattern.elems.first().and_then(pattern_name)?;
        Some(self.owned_binding(name, kind))
    }

    pub(super) fn match_arm_bindings(&self, pat: &syn::Pat, expr: &syn::Expr) -> Vec<Binding> {
        let Some(resource) = self.match_resource(expr) else {
            return Vec::new();
        };
        if let Some(name) = pattern_name(pat) {
            // A move binding (including @ and typed patterns) owns the whole
            // scrutinee. Use its name so an explicit drop releases it.
            let kind = if borrowed_pattern(pat) {
                ValueKind::BorrowedGuard
            } else {
                ValueKind::Guard
            };
            return vec![self.owned_binding(name, kind)];
        }
        if let Some(binding) = self.pattern_guard(pat, expr) {
            return vec![binding];
        }
        if resource == MatchResource::TryLockResult || !unnamed_resource(pat) {
            return Vec::new();
        }
        // Blocking lock-result wildcards retain a scrutinee resource, including
        // fallback arms reached after a guarded Ok pattern fails.
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

fn unnamed_resource(pat: &syn::Pat) -> bool {
    match pat {
        syn::Pat::Wild(_) => true,
        syn::Pat::Paren(pat) => unnamed_resource(&pat.pat),
        syn::Pat::Type(pat) => unnamed_resource(&pat.pat),
        syn::Pat::TupleStruct(pattern) => {
            pattern
                .path
                .segments
                .last()
                .is_some_and(|part| part.ident == "Ok" || part.ident == "Err")
                && pattern.elems.len() == 1
                && matches!(pattern.elems.first(), Some(syn::Pat::Wild(_)))
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
