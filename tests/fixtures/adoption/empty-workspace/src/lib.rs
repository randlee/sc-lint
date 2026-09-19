//! Minimal consumer workspace used by the release-archive smoke test.

/// Returns the fixture name.
#[must_use]
pub fn name() -> &'static str {
    "empty-workspace"
}

#[cfg(test)]
mod tests {
    #[test]
    fn reports_name() {
        assert_eq!(super::name(), "empty-workspace");
    }
}
