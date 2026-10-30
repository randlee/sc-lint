```json
{
  "schema_version": "1.0.0",
  "success": false,
  "canceled": false,
  "aborted_by": null,
  "data": null,
  "error": {
    "code": "VALIDATION.INPUT",
    "message": "No single object can satisfy both result schemas. agent-runner-result.schema.json (checked by anchor_runner_contract.py validate-fence) requires exactly the top-level fields schema_version, success, canceled, aborted_by, data, error and metadata, with schema_version \"1.0.0\". anchor-tuning-result.schema.json sets additionalProperties:false at the top level and does not declare schema_version. Verified with jsonschema: the tuning result with schema_version \"1.0.0\" added passes agent-runner-result but fails anchor-tuning-result ('schema_version' was unexpected). Without schema_version it fails validate-fence with ENVELOPE.RESULT. The tuning outcome is unaffected and was not changed: accepted=true, sealed_question_set digest b665c1f54306d3bf2d6be7f53f468a3f27286197fc76e1457b876aa043a8bbf3, tuning_history_after_sha256 0166a2e84c1f51a53000d0df4097b5d749c69c796888999bfa81c2c799614f9c, oracle 9be91fdc273197f88521799ad0076f60405490c8fd5b4fdb9b748ae21fd4ffa1, and result.json (canonical-JSON sha256 e285c75630753aef3c93d462d069e1794b5d0762ee7c6239d01230f3c6294cda as reported by anchor_tuning_contract.py; file-byte sha256 73c6b07f9d31103e2064249309ca7ed3ff081d89d5f78b9fbe01166ca131b4d7) still passes anchor_tuning_contract.py result and the tuning schema.",
    "recoverable": true,
    "suggested_action": "Resolve the conflict in the contract, not the tuning data. Either (a) add schema_version (const \"1.0.0\") as an allowed top-level property in anchor-tuning-result.schema.json and the anchor-tuning-result.md.j2 template, or (b) have the runner validate the runner envelope and then validate the six-field tuning result (envelope minus schema_version) with anchor-tuning-result.schema.json and anchor_tuning_contract.py result. Under (b), the runner envelope is result.json with schema_version \"1.0.0\" prepended. Neither option needs new Jev calls."
  },
  "metadata": {
    "correlation_id": "v2-ANC-004-planning",
    "duration_ms": 0,
    "tool_calls": 3,
    "retry_count": 0,
    "jev_run_ids": [],
    "planner_observation_trust": null,
    "authoritative_sources_unchanged": true,
    "tuning_result_file": "docs/plans/phase-m/v2/anchor-runs/tuning/ANC-004/planning/result.json",
    "tuning_result_canonical_sha256": "e285c75630753aef3c93d462d069e1794b5d0762ee7c6239d01230f3c6294cda"
  }
}
```
