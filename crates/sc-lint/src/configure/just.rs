use std::path::Path;

use crate::CliError;

pub(crate) const MARKER_BEGIN: &str = "# >>> sc-lint managed integration >>>";
pub(crate) const MARKER_END: &str = "# <<< sc-lint managed integration <<<";
pub(crate) const MANAGED_IMPORT: &str = "import '.sc-lint/justfile'";

pub(crate) fn managed_block(newline: &str) -> String {
    format!("{MARKER_BEGIN}{newline}{MANAGED_IMPORT}{newline}{MARKER_END}{newline}")
}

#[expect(
    clippy::result_large_err,
    reason = "Just marker failures must retain the shared structured configure recovery envelope."
)]
pub(crate) fn insert_or_replace(source: &str) -> Result<String, CliError> {
    let newline = if source.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let begin = source.match_indices(MARKER_BEGIN).collect::<Vec<_>>();
    let end = source.match_indices(MARKER_END).collect::<Vec<_>>();
    if begin.len() > 1 || end.len() > 1 || begin.len() != end.len() {
        return Err(conflict(
            "the managed marker is missing, duplicated, or malformed",
        ));
    }
    if let Some((start, _)) = begin.first() {
        let end_offset = end[0].0 + MARKER_END.len();
        let suffix = &source[end_offset..];
        let suffix = suffix
            .strip_prefix("\r\n")
            .or_else(|| suffix.strip_prefix('\n'))
            .unwrap_or(suffix);
        return Ok(format!(
            "{}{}{}",
            &source[..*start],
            managed_block(newline),
            suffix
        ));
    }
    let separator = if source.is_empty() || source.ends_with('\n') {
        ""
    } else {
        newline
    };
    Ok(format!("{source}{separator}{}", managed_block(newline)))
}

#[expect(
    clippy::result_large_err,
    reason = "Just validation failures must retain the shared structured configure recovery envelope."
)]
pub(crate) fn validate(target: &Path, bytes: &[u8]) -> Result<(), CliError> {
    let source = std::str::from_utf8(bytes).map_err(|error| invalid(target, error))?;
    if source.contains('\0') || source.trim().is_empty() {
        return Err(invalid(target, "a Justfile must be non-empty UTF-8 text"));
    }
    let begin = source.matches(MARKER_BEGIN).count();
    let end = source.matches(MARKER_END).count();
    if begin != end || begin > 1 {
        return Err(conflict(
            "the managed marker is missing, duplicated, or malformed",
        ));
    }
    if begin == 1 && !source.contains(MANAGED_IMPORT) {
        return Err(conflict(
            "the managed marker does not contain the canonical import",
        ));
    }
    Ok(())
}

fn invalid(target: &Path, cause: impl std::fmt::Display) -> CliError {
    CliError::config(format!(
        "generated Justfile `{}` is invalid",
        target.display()
    ))
    .with_code("CLI.CONFIGURE_UNMANAGED_COLLISION")
    .with_cause(cause.to_string())
    .with_suggested_action("Review the exportable patch; no repository files were changed.")
    .with_documentation("sc-lint docs troubleshooting")
}

fn conflict(cause: &str) -> CliError {
    CliError::config("the existing Justfile cannot accept sc-lint's managed integration")
        .with_code("CLI.CONFIGURE_UNMANAGED_COLLISION")
        .with_cause(cause)
        .with_suggested_action(
            "Review the exportable patch; no user-owned Justfile content was modified.",
        )
        .with_documentation("sc-lint docs troubleshooting")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marker_insert_preserves_crlf_and_replaces_only_managed_range() {
        let source = "# user comment\r\nuser:\r\n    echo user\r\n";
        let inserted = insert_or_replace(source).expect("insert marker");
        assert!(inserted.contains("# user comment\r\nuser:\r\n    echo user\r\n"));
        assert!(inserted.contains("\r\nimport '.sc-lint/justfile'\r\n"));
        let replaced = insert_or_replace(&inserted).expect("replace marker");
        assert_eq!(inserted, replaced);
    }

    #[test]
    fn duplicate_marker_is_a_no_write_conflict() {
        let source = format!("{}{}", managed_block("\n"), managed_block("\n"));
        assert_eq!(
            insert_or_replace(&source).expect_err("conflict").code(),
            "CLI.CONFIGURE_UNMANAGED_COLLISION"
        );
    }
}
