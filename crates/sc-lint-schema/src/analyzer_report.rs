use std::error::Error;
use std::fmt;
use std::sync::OnceLock;

use crate::{ANALYZER_REPORT_SCHEMA_VERSION_V1, ANALYZER_REPORT_SCHEMA_VERSION_V2};

use jsonschema::Validator;
use serde_json::Value;

const ANALYZER_REPORT_V1_SCHEMA: &str = include_str!("../schema/analyzer-report-v1.schema.json");
const ANALYZER_REPORT_V2_SCHEMA: &str = include_str!("../schema/analyzer-report-v2.schema.json");

static ANALYZER_REPORT_V1_VALIDATOR: OnceLock<Result<Validator, String>> = OnceLock::new();
static ANALYZER_REPORT_V2_VALIDATOR: OnceLock<Result<Validator, String>> = OnceLock::new();

/// A mismatch between analyzer JSON and the published analyzer-report schema.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnalyzerReportDrift {
    pub instance_path: String,
    pub problem: String,
}

impl fmt::Display for AnalyzerReportDrift {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "`{}`: {}", self.instance_path, self.problem)
    }
}

impl Error for AnalyzerReportDrift {}

/// Validates an analyzer backend report against the published JSON Schema.
///
/// The compiled validator is reused for every backend response. A schema
/// compilation error is returned as a drift at `$schema`; the checked-in
/// artifact is also parsed and validated in the schema crate's tests.
pub fn validate_analyzer_report(value: &Value) -> Result<(), AnalyzerReportDrift> {
    let version = value
        .get("schema_version")
        .and_then(Value::as_str)
        .unwrap_or("");
    let (schema_text, cache) = match version {
        ANALYZER_REPORT_SCHEMA_VERSION_V1 => {
            (ANALYZER_REPORT_V1_SCHEMA, &ANALYZER_REPORT_V1_VALIDATOR)
        }
        ANALYZER_REPORT_SCHEMA_VERSION_V2 => {
            (ANALYZER_REPORT_V2_SCHEMA, &ANALYZER_REPORT_V2_VALIDATOR)
        }
        other => {
            return Err(AnalyzerReportDrift {
                instance_path: "/schema_version".to_string(),
                problem: format!("unsupported analyzer report schema version `{other}`"),
            });
        }
    };
    let validator = cache.get_or_init(|| {
        let schema: Value = serde_json::from_str(schema_text)
            .map_err(|error| format!("published schema is invalid JSON: {error}"))?;
        jsonschema::validator_for(&schema)
            .map_err(|error| format!("published schema is invalid: {error}"))
    });

    let validator = validator.as_ref().map_err(|problem| AnalyzerReportDrift {
        instance_path: "$schema".to_string(),
        problem: problem.clone(),
    })?;

    match validator.validate(value) {
        Ok(()) => Ok(()),
        Err(error) => {
            let instance_path = error.instance_path().to_string();
            Err(AnalyzerReportDrift {
                instance_path: if instance_path.is_empty() {
                    "$".to_string()
                } else {
                    instance_path
                },
                problem: error.to_string(),
            })
        }
    }
}

/// Returns the published schema document packaged with this crate.
pub const fn analyzer_report_schema() -> &'static str {
    ANALYZER_REPORT_V2_SCHEMA
}

pub const fn analyzer_report_v1_schema() -> &'static str {
    ANALYZER_REPORT_V1_SCHEMA
}
