//! Runtime-owned test-scope policies. Rule 003 deliberately ignores name hints;
//! newer practices also use the shared classifier's `tests` convention.
pub(crate) use sc_lint_analyzer_support::{ScopeKind, classify_scope, is_test_only};

pub(crate) fn production(attrs: &[syn::Attribute], tests: Option<bool>) -> bool {
    classify_scope(attrs, ScopeKind::NonTest, tests) == ScopeKind::NonTest && !is_test_only(attrs)
}
