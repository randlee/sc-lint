pub fn normalize_sc_lint(input: &str) -> String {
    input.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::normalize_sc_lint;

    #[test]
    fn normalizes_extra_whitespace() {
        assert_eq!(
            normalize_sc_lint("boundary.allow(  \"rule\"   )"),
            "boundary.allow( \"rule\" )"
        );
    }
}
