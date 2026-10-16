use sc_lint_schema::Finding;
use sc_lint_schema::FindingsReport;
use sc_lint_schema::NodeId;
use sc_lint_schema::OwnerId;
use sc_lint_schema::ReportStatus;

#[test]
fn finding_matches_frozen_json_contract() {
    let finding = Finding {
        rule_id: "RULE-001",
        kind: "contract_fixture".into(),
        message: "machine contract fixture".to_string(),
        owner_ids: vec![OwnerId::new("crate::example::example")],
        node_ids: vec![NodeId::new("crate::example::example::item")],
    };
    let expected: serde_json::Value = serde_json::from_str(include_str!("fixtures/finding.json"))
        .expect("finding baseline is valid JSON");

    assert_eq!(serde_json::to_value(finding).unwrap(), expected);
}

#[test]
fn findings_report_matches_frozen_json_contract() {
    let report = FindingsReport {
        tool: "sc-lint-schema-test",
        version: "0.1.0",
        schema_version: "0.1.0",
        status: ReportStatus::Fail,
        scanned_crates: 1,
        findings: vec![Finding {
            rule_id: "RULE-001",
            kind: "contract_fixture".into(),
            message: "machine contract fixture".to_string(),
            owner_ids: vec![OwnerId::new("crate::example::example")],
            node_ids: vec![NodeId::new("crate::example::example::item")],
        }],
    };
    let expected: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/findings-report.json"))
            .expect("report baseline is valid JSON");

    assert_eq!(serde_json::to_value(report).unwrap(), expected);
}
