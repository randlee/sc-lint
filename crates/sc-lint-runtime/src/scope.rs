//! Runtime-owned test-scope policies. Rule 003 deliberately ignores name hints;
//! newer practices also use the shared classifier's `tests` convention.
pub(crate) use sc_lint_analyzer_support::{ScopeKind, classify_scope};

pub(crate) fn production(attrs: &[syn::Attribute], tests: Option<bool>) -> bool {
    classify_scope(attrs, ScopeKind::NonTest, tests) == ScopeKind::NonTest && !is_test_only(attrs)
}

pub(crate) fn is_test_only(attributes: &[syn::Attribute]) -> bool {
    attributes
        .iter()
        .any(|attribute| attr_is_test(attribute) || cfg_is_test(attribute))
}

/// `#[test]`, or a path of two or more segments ending in `test`
/// (`#[tokio::test]`, `#[async_std::test]`): both expand to a `#[test]` fn.
fn attr_is_test(attribute: &syn::Attribute) -> bool {
    let path = attribute.path();
    path.is_ident("test")
        || (path.segments.len() >= 2
            && path
                .segments
                .last()
                .is_some_and(|segment| segment.ident == "test"))
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
