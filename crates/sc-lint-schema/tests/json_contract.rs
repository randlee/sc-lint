use sc_lint_schema::Finding;
use sc_lint_schema::FindingsReport;
use sc_lint_schema::NodeId;
use sc_lint_schema::OwnerId;
use sc_lint_schema::ReportStatus;
use sc_lint_schema::analyzer_report_schema;
use sc_lint_schema::analyzer_report_v1_schema;
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
        allowed: Vec::new(),
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

/// Removes every `additionalProperties: false` from a schema copy.
fn without_additional_properties(schema: &mut serde_json::Value) {
    match schema {
        serde_json::Value::Object(map) => {
            map.remove("additionalProperties");
            map.values_mut().for_each(without_additional_properties);
        }
        serde_json::Value::Array(items) => items.iter_mut().for_each(without_additional_properties),
        _ => {}
    }
}

#[test]
fn published_analyzer_report_schema_rejects_unknown_keys() {
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/findings-report.json"))
            .expect("report fixture is JSON");
    let mut top_level = fixture.clone();
    top_level["unexpected"] = serde_json::json!(true);
    let mut in_finding = fixture.clone();
    in_finding["findings"][0]["unexpected"] = serde_json::json!(true);

    let top_error = validate_analyzer_report(&top_level).expect_err("unknown top-level key");
    assert_eq!(top_error.instance_path, "$", "{top_error}");
    assert!(top_error.problem.contains("unexpected"), "{top_error}");
    let finding_error = validate_analyzer_report(&in_finding).expect_err("unknown finding key");
    assert_eq!(
        finding_error.instance_path, "/findings/0",
        "{finding_error}"
    );
    assert!(
        finding_error.problem.contains("unexpected"),
        "{finding_error}"
    );

    // Mutation check: the same documents pass once `additionalProperties: false`
    // is removed, so the rejections above come from those two keywords.
    let mut relaxed: serde_json::Value =
        serde_json::from_str(analyzer_report_v1_schema()).expect("v1 schema is JSON");
    without_additional_properties(&mut relaxed);
    let relaxed = jsonschema::validator_for(&relaxed).expect("relaxed schema compiles");
    assert!(relaxed.is_valid(&top_level));
    assert!(relaxed.is_valid(&in_finding));

    let v2_fixture: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/findings-report-v2.json"))
            .expect("v2 report fixture is JSON");
    let mut v2_top_level = v2_fixture.clone();
    v2_top_level["unexpected"] = serde_json::json!(true);
    let mut v2_in_finding = v2_fixture.clone();
    v2_in_finding["findings"] = serde_json::json!([{
        "rule_id": "RULE-001",
        "kind": "contract_fixture",
        "message": "machine contract fixture",
        "owner_ids": ["crate::example::example"],
        "node_ids": ["crate::example::example::item"],
        "unexpected": true
    }]);
    let mut v2_in_allowed = v2_fixture.clone();
    v2_in_allowed["allowed"][0]["unexpected"] = serde_json::json!(true);

    for (value, expected_path) in [
        (&v2_top_level, "$"),
        (&v2_in_finding, "/findings/0"),
        (&v2_in_allowed, "/allowed/0"),
    ] {
        let error = validate_analyzer_report(value).expect_err("v2 rejects unknown keys");
        assert_eq!(error.instance_path, expected_path, "{error}");
    }

    let mut relaxed_v2: serde_json::Value =
        serde_json::from_str(analyzer_report_schema()).expect("v2 schema is JSON");
    without_additional_properties(&mut relaxed_v2);
    let relaxed_v2 = jsonschema::validator_for(&relaxed_v2).expect("relaxed v2 schema compiles");
    assert!(relaxed_v2.is_valid(&v2_top_level));
    assert!(relaxed_v2.is_valid(&v2_in_finding));
    assert!(relaxed_v2.is_valid(&v2_in_allowed));
}

#[test]
fn analyzer_report_v1_and_v2_dispatch_preserve_closed_contracts() {
    let v1: serde_json::Value = serde_json::from_str(include_str!("fixtures/findings-report.json"))
        .expect("v1 fixture is JSON");
    let v2: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/findings-report-v2.json"))
            .expect("v2 fixture is JSON");
    assert_eq!(validate_analyzer_report(&v1), Ok(()));
    assert_eq!(validate_analyzer_report(&v2), Ok(()));

    let v1_schema: serde_json::Value = serde_json::from_str(analyzer_report_v1_schema()).unwrap();
    let old_validator = jsonschema::validator_for(&v1_schema).unwrap();
    assert!(
        !old_validator.is_valid(&v2),
        "the strict v1 validator rejects v2's allowed property"
    );

    let mut unknown = v2.clone();
    unknown["allowed"][0]["unexpected"] = serde_json::json!(true);
    assert!(validate_analyzer_report(&unknown).is_err());
    let mut malformed = v2;
    malformed["allowed"][0]
        .as_object_mut()
        .unwrap()
        .remove("reason");
    let error = validate_analyzer_report(&malformed).expect_err("allowed entries require reasons");
    assert!(error.instance_path.starts_with("/allowed/0"), "{error}");
}

#[test]
fn v1_rejects_allowed_and_unknown_versions_are_rejected() {
    let mut v1: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/findings-report.json"))
            .expect("v1 fixture is JSON");
    v1["allowed"] = serde_json::json!([]);
    assert!(validate_analyzer_report(&v1).is_err());
    v1["schema_version"] = serde_json::json!("0.3.0");
    let error = validate_analyzer_report(&v1).expect_err("unknown schema versions fail");
    assert_eq!(error.instance_path, "/schema_version");
}
