use crate::CliError;

#[allow(dead_code, reason = "F.4a transaction wiring consumes these marker helpers next.")]
pub(crate) const MARKER_BEGIN: &str = "# >>> sc-lint managed integration >>>";
#[allow(dead_code, reason = "F.4a transaction wiring consumes these marker helpers next.")]
pub(crate) const MARKER_END: &str = "# <<< sc-lint managed integration <<<";
#[allow(dead_code, reason = "F.4a transaction wiring consumes these marker helpers next.")]
pub(crate) const MANAGED_IMPORT: &str = "import '.sc-lint/justfile'";

#[allow(dead_code, reason = "F.4a transaction wiring consumes this marker helper next.")]
pub(crate) fn managed_block(newline: &str) -> String {
    format!("{MARKER_BEGIN}{newline}{MANAGED_IMPORT}{newline}{MARKER_END}{newline}")
}

#[allow(dead_code, reason = "F.4a transaction wiring consumes this marker helper next.")]
pub(crate) fn insert_or_replace(source: &str) -> Result<String, CliError> {
    let newline = if source.contains("\r\n") { "\r\n" } else { "\n" };
    let begin = source.match_indices(MARKER_BEGIN).collect::<Vec<_>>();
    let end = source.match_indices(MARKER_END).collect::<Vec<_>>();
    if begin.len() > 1 || end.len() > 1 || begin.len() != end.len() {
        return Err(conflict("the managed marker is missing, duplicated, or malformed"));
    }
    if let Some((start, _)) = begin.first() {
        let end_offset = end[0].0 + MARKER_END.len();
        let suffix = &source[end_offset..];
        let suffix = suffix.strip_prefix("\r\n").or_else(|| suffix.strip_prefix('\n')).unwrap_or(suffix);
        return Ok(format!("{}{}{}", &source[..*start], managed_block(newline), suffix));
    }
    let separator = if source.is_empty() || source.ends_with('\n') { "" } else { newline };
    Ok(format!("{source}{separator}{}", managed_block(newline)))
}

fn conflict(cause: &str) -> CliError {
    CliError::config("the existing Justfile cannot accept sc-lint's managed integration")
        .with_code("CLI.CONFIGURE_UNMANAGED_COLLISION")
        .with_cause(cause)
        .with_suggested_action("Review the exportable patch; no user-owned Justfile content was modified.")
        .with_documentation("sc-lint docs troubleshooting")
}
