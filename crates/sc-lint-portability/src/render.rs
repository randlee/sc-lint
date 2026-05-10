use crate::FindingsReport;

pub fn render_findings_report(report: &FindingsReport) -> String {
    format!(
        "{} {} status={} scanned_crates={} findings={}",
        report.tool,
        report.version,
        report.status.as_str(),
        report.scanned_crates,
        report.findings.len()
    )
}
