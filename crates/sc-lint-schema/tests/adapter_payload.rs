use sc_lint_schema::{PayloadDrift, validate_adapter_payload};
use serde_json::{Value, json};

fn expect_drift(value: Value, key_path: &str, problem: &str) -> PayloadDrift {
    validate_adapter_payload(&value)
        .expect_err("payload must be rejected")
        .tap_assertions(key_path, problem)
}

trait DriftAssertions {
    fn tap_assertions(self, key_path: &str, problem: &str) -> Self;
}

impl DriftAssertions for PayloadDrift {
    fn tap_assertions(self, key_path: &str, problem: &str) -> Self {
        assert_eq!(self.key_path, key_path);
        assert_eq!(self.problem, problem);
        self
    }
}

#[test]
fn validate_adapter_payload_accepts_conforming_payloads() {
    let success = json!({
        "adapter_schema": "sc-lint-python-v1",
        "ok": true,
        "summary": "done",
        "data": {},
        "diagnostics": [],
        "future": true
    });
    let failure = json!({
        "adapter_schema": "sc-lint-python-v1",
        "ok": false,
        "error": {
            "kind": "config",
            "message": "invalid config",
            "details": {},
            "suggested_action": "fix config",
            "future_error_field": true
        },
        "diagnostics": []
    });
    let additive = json!({
        "adapter_schema": "sc-lint-python-v1",
        "ok": true,
        "summary": "done",
        "data": {},
        "diagnostics": [],
        "future": {"version": 2}
    });

    assert_eq!(validate_adapter_payload(&success), Ok(()));
    assert_eq!(validate_adapter_payload(&failure), Ok(()));
    assert_eq!(validate_adapter_payload(&additive), Ok(()));
}

#[test]
fn validate_adapter_payload_rejects_missing_key() {
    let mut missing_ok = json!({"adapter_schema":"v", "diagnostics": []});
    missing_ok.as_object_mut().unwrap().remove("ok");
    let mut missing_data =
        json!({"adapter_schema":"v", "ok":true, "summary":"ok", "data":{}, "diagnostics":[]});
    missing_data.as_object_mut().unwrap().remove("data");
    let mut missing_summary =
        json!({"adapter_schema":"v", "ok":true, "summary":"ok", "data":{}, "diagnostics":[]});
    missing_summary.as_object_mut().unwrap().remove("summary");
    let mut missing_error = json!({"adapter_schema":"v", "ok":false, "error":{"kind":"config","message":"bad"}, "diagnostics":[]});
    missing_error.as_object_mut().unwrap().remove("error");

    expect_drift(missing_ok, "ok", "missing required key");
    expect_drift(missing_data, "data", "missing required key");
    expect_drift(missing_summary, "summary", "missing required key");
    expect_drift(missing_error, "error", "missing required key");
}

#[test]
fn validate_adapter_payload_rejects_wrong_typed_key() {
    let base =
        json!({"adapter_schema":"v", "ok":true, "summary":"ok", "data":{}, "diagnostics":[]});
    let mut wrong_diagnostics = base.clone();
    wrong_diagnostics["diagnostics"] = json!("x");
    let mut wrong_diagnostic_item = base.clone();
    wrong_diagnostic_item["diagnostics"] = json!([1]);
    let wrong_error_kind = json!({"adapter_schema":"v", "ok":false, "error":{"kind":3,"message":"bad"}, "diagnostics":[]});
    let root_array = json!([]);
    let both_success_and_error = json!({"adapter_schema":"v", "ok":true, "summary":"ok", "data":{}, "error":{}, "diagnostics":[]});
    let both_failure_and_data = json!({"adapter_schema":"v", "ok":false, "error":{"kind":"config","message":"bad"}, "data":{}, "diagnostics":[]});

    expect_drift(wrong_diagnostics, "diagnostics", "expected array");
    expect_drift(wrong_diagnostic_item, "diagnostics[0]", "expected string");
    let error_kind = expect_drift(wrong_error_kind, "error.kind", "expected string");
    assert_eq!(error_kind.to_string(), "`error.kind`: expected string");
    expect_drift(root_array, "$", "expected object");
    expect_drift(
        both_success_and_error,
        "error",
        "must be absent when ok is true",
    );
    expect_drift(
        both_failure_and_data,
        "data",
        "must be absent when ok is false",
    );
}

#[test]
fn validate_adapter_payload_reports_the_first_drift_in_check_order() {
    let payload = json!({"adapter_schema":"v", "diagnostics":[]});
    expect_drift(payload, "ok", "missing required key");
}
