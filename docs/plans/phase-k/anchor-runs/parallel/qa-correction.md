QA correction: k-1 excludes internal schema.d/ and analyzer-catalog/ files from consumer copies and adds test_install_omits_internal_schema_and_catalog.

Independent triage identified ambiguous conditional source reuse. D5 now explicitly requires applicable atm-core implementation logic as the starting point, while distinguishing regression inputs from an exclusion implementation. This clarifies existing ANC-001@3; questions and threshold remain unchanged. The exact replacement and its authority are in k1-source-reuse-change.json.

Captured review plan-20261013T162229Z-lint-k-1 PASS: eight anchors satisfied, one not applicable. Earlier inconclusive runs are retained. Request/response pairs and exact inputs for both captured reviews are retained locally, outside this PR. review-evidence.tar.gz contains the updated complete bead snapshot.
