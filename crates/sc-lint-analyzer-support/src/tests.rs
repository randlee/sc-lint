use serde::Serialize;

use sc_lint_schema::Finding;
use sc_lint_schema::FindingKind;
use sc_lint_schema::FindingsReport;
use sc_lint_schema::NodeId;
use sc_lint_schema::OwnerId;
use sc_lint_schema::ReportStatus;

use super::render_findings_report;

#[derive(Serialize)]
enum TestRule {
    Shared,
}

#[test]
fn text_report_rendering_is_shared_and_stable() {
    let report = FindingsReport {
        tool: "shared-analyzer",
        version: "0.5.0",
        schema_version: "0.1.0",
        status: ReportStatus::Pass,
        scanned_crates: 2,
        findings: vec![Finding {
            rule_id: TestRule::Shared,
            kind: FindingKind::new("shared"),
            message: "shared support fixture".to_owned(),
            owner_ids: vec![OwnerId::new("crate::fixture")],
            node_ids: vec![NodeId::new("crate::fixture::node")],
        }],
    };

    assert_eq!(
        render_findings_report(&report),
        "shared-analyzer 0.5.0 status=pass scanned_crates=2 findings=1"
    );
}
