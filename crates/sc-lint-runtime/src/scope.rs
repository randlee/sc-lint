//! Runtime-owned test-scope policies. Rule 003 deliberately ignores name hints;
//! newer practices also use the shared classifier's `tests` convention.
use sc_lint_analyzer_support::{ScopeKind, classify_scope as shared_scope, is_test_only};

pub(crate) fn production(attrs: &[syn::Attribute], tests: Option<bool>) -> bool {
    classify(attrs, ScopeKind::NonTest, tests) == ScopeKind::NonTest
}

pub(crate) fn classify(
    attrs: &[syn::Attribute],
    inherited: ScopeKind,
    tests: Option<bool>,
) -> ScopeKind {
    if is_test_only(attrs) {
        ScopeKind::Test
    } else {
        shared_scope(attrs, inherited, tests)
    }
}
