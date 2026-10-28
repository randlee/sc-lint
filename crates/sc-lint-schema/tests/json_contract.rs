use sc_lint_schema::Finding;
use sc_lint_schema::FindingsReport;
use sc_lint_schema::NodeId;
use sc_lint_schema::OwnerId;
use sc_lint_schema::ReportStatus;
use sc_lint_schema::analyzer_report_schema;
use sc_lint_schema::validate_analyzer_report;

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

#[test]
fn published_analyzer_report_schema_accepts_contract_fixture_and_rejects_drift() {
    let schema: serde_json::Value =
        serde_json::from_str(analyzer_report_schema()).expect("published schema is JSON");
    jsonschema::validator_for(&schema).expect("published schema is valid JSON Schema");

    let report: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/findings-report.json"))
            .expect("report fixture is JSON");
    assert_eq!(validate_analyzer_report(&report), Ok(()));

    let drifted = serde_json::json!({
        "tool": "sc-lint-schema-test",
        "version": "0.1.0",
        "schema_version": "0.1.0",
        "status": "pass",
        "scanned_crates": 0,
        "findings": [{ "rule_id": "RULE-001" }]
    });
    let error = validate_analyzer_report(&drifted).expect_err("missing finding fields drift");
    assert!(error.instance_path.starts_with("/findings/0"));
}
