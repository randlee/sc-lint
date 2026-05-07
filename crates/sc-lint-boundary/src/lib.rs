pub fn banner() -> &'static str {
    "sc-lint-boundary bootstrap"
}

pub fn normalize_rule(input: &str) -> String {
    sc_lint_directives::normalize_sc_lint(input)
}

#[cfg(test)]
mod tests {
    use super::{banner, normalize_rule};

    #[test]
    fn exposes_bootstrap_banner() {
        assert_eq!(banner(), "sc-lint-boundary bootstrap");
    }

    #[test]
    fn normalizes_rule_text() {
        assert_eq!(
            normalize_rule("cycle.type_method_self_loop"),
            "cycle.type_method_self_loop"
        );
    }
}
