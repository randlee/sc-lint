use serde::Serialize;

use sc_lint_schema::FindingsReport;

pub fn render_findings_report<R>(report: &FindingsReport<R>) -> String
where
    R: Serialize,
{
    format!(
        "{} {} status={} scanned_crates={} findings={}",
        report.tool,
        report.version,
        report.status.as_str(),
        report.scanned_crates,
        report.findings.len()
    )
}
