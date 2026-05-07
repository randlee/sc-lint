use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn sc_lint(attr: TokenStream, item: TokenStream) -> TokenStream {
    let normalized = sc_lint_directives::normalize_sc_lint(&attr.to_string());
    if normalized.is_empty() {
        return item;
    }
    item
}

#[cfg(test)]
mod tests {
    use sc_lint_directives::normalize_sc_lint;

    #[test]
    fn shares_directive_normalization() {
        assert_eq!(
            normalize_sc_lint("boundary.internal_only"),
            "boundary.internal_only"
        );
    }
}
