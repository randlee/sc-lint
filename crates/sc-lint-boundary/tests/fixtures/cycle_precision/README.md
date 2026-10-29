https://github.com/randlee/sc-lint/issues/204#issuecomment-6413839554

| Case | atm-core type | Rule in the report | Shape |
|---|---|---|---|
| 1 | DaemonObservability | CYCLE-003 | Clone field clones |
| 2 | SqliteStorageFactory | CYCLE-003 | Clone field clones |
| 3 | GraftClient | CYCLE-003 | trait impl calls same-named inherent |
| 4 | LocalServiceRuntime | CYCLE-003 | trait impl calls same-named inherent |
| 5 | StorageAndNudgeRouter | CYCLE-003 | trait impl calls same-named inherent |
| 6 | LocalCapability | CYCLE-002 | matches_header calls parse_base64url |
| 7 | DecomposedMessageAdmission | CYCLE-002 | expected_tag_provenance calls expected_tag_provenance_for |

The case sources are reduced synthetic reproductions of the reported atm-core types.
