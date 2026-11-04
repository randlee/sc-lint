//! Reusable branch snapshots contain state only, never cloned binding names.
use super::{Binding, BindingState, Scanner};
use syn::visit::Visit;

impl Scanner {
    fn snapshot(&mut self) -> Vec<BindingState> {
        let mut states = self.state_buffers.pop().unwrap_or_default();
        states.extend(self.bindings.iter().map(|binding| BindingState {
            kind: binding.kind,
            guard: binding.guard,
        }));
        states
    }
    fn recycle(&mut self, mut states: Vec<BindingState>) {
        states.clear();
        self.state_buffers.push(states);
    }
    fn restore(&mut self, states: Vec<BindingState>) {
        for (binding, state) in self.bindings.iter_mut().zip(&states) {
            binding.kind = state.kind;
            binding.guard = state.guard;
        }
        self.recycle(states);
    }
    pub(super) fn deferred(&mut self, asynchronous: bool, visit: impl FnOnce(&mut Self)) {
        let states = self.snapshot();
        for binding in &mut self.bindings {
            binding.guard = false;
        }
        let old = std::mem::replace(&mut self.asynchronous, asynchronous);
        visit(self);
        self.asynchronous = old;
        self.restore(states);
    }
    fn branch(
        &mut self,
        continues: bool,
        visit: impl FnOnce(&mut Self),
    ) -> Option<Vec<BindingState>> {
        let mut states = self.snapshot();
        visit(self);
        for (binding, state) in self.bindings.iter_mut().zip(&mut states) {
            std::mem::swap(&mut binding.kind, &mut state.kind);
            std::mem::swap(&mut binding.guard, &mut state.guard);
        }
        if continues {
            Some(states)
        } else {
            self.recycle(states);
            None
        }
    }
    fn merge_guards(&mut self, branches: impl IntoIterator<Item = Option<Vec<BindingState>>>) {
        for binding in &mut self.bindings {
            binding.guard = false;
        }
        for branch in branches.into_iter().flatten() {
            for (binding, state) in self.bindings.iter_mut().zip(&branch) {
                binding.guard |= state.guard;
            }
            self.recycle(branch);
        }
    }
    pub(super) fn condition_guard(&self, cond: &syn::Expr) -> Option<Binding> {
        let syn::Expr::Let(binding) = cond else {
            return None;
        };
        self.pattern_guard(&binding.pat, &binding.expr)
    }
    pub(super) fn visit_conditional(&mut self, branch: &syn::ExprIf) {
        self.visit_expr(&branch.cond);
        let binding = self.condition_guard(&branch.cond);
        let then = self.branch(self.block_continues(&branch.then_branch), |this| {
            let count = this.bindings.len();
            this.bindings.extend(binding);
            this.visit_block(&branch.then_branch);
            this.bindings.truncate(count);
        });
        let otherwise = if let Some((_, other)) = &branch.else_branch {
            self.branch(self.expr_continues(other), |this| this.visit_expr(other))
        } else {
            Some(self.snapshot())
        };
        self.merge_guards([then, otherwise]);
    }
    pub(super) fn visit_match_branches(&mut self, branch: &syn::ExprMatch) {
        self.visit_expr(&branch.expr);
        if branch.arms.is_empty() {
            return;
        }
        let mut merged = self.snapshot();
        for state in &mut merged {
            state.guard = false;
        }
        for arm in &branch.arms {
            let bindings = self.match_arm_bindings(&arm.pat, &branch.expr);
            let state = self.branch(self.expr_continues(&arm.body), |this| {
                let count = this.bindings.len();
                this.bindings.extend(bindings);
                this.visit_arm(arm);
                this.bindings.truncate(count);
            });
            if let Some(state) = state {
                for (merged, state) in merged.iter_mut().zip(&state) {
                    merged.guard |= state.guard;
                }
                self.recycle(state);
            }
        }
        for (binding, state) in self.bindings.iter_mut().zip(&merged) {
            binding.guard = state.guard;
        }
        self.recycle(merged);
    }
    pub(super) fn conditional(&mut self, visit: impl FnOnce(&mut Self)) {
        let states = self.snapshot();
        visit(self);
        self.restore(states);
    }
}
