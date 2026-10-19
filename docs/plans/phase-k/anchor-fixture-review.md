# Phase K anchor fixture review

All fixtures below are drafts for approval. They are illustrative cases, not new requirements or accepted oracle labels.

| Anchor | Pass example | Fail example |
| --- | --- | --- |
| ANC-001 | An atm-core parser already detects the reported failure. The sprint ports its parser logic into a reusable sc-lint interface and puts atm-core's allowlist in consumer configuration. | An atm-core parser already detects the same failure and can use sc-lint's interface, but the sprint rewrites the behavior from scratch without reusing its source. |
| ANC-002 | An atm-core test fails on the reported malformed policy. A sc-lint regression test ports the same input and expected rejection through its own API. | The same atm-core test exists, but sc-lint adds only a happy-path test that never exercises the reported malformed policy. |
| ANC-003 | A tool plan places the substantive logic in Rust and explains how the crate boundary fits; its Python binding only translates inputs and results. | A tool plan expressly says Rust is appropriate for its tool and sc-lint boundary, yet chooses Python for the substantive rule solely for convenience. |
| ANC-004 | The Python binding forwards arguments to the Rust matcher and converts the Rust result to the public Python return shape. | The Rust matcher exists, but the Python binding independently reimplements the matching rule and can return a different result. |
| ANC-005 | Two repositories run the same rule with different configured allowlists; no rule checks the repository name. | The rule detects the repository name atm-core and silently enables an atm-core-only allowlist with no consumer configuration. |
| ANC-006 | Using the chosen supported form (a configuration table or a file), an optional allowlist is supplied and enforced; without that allowlist, the same input retains its legacy result for that policy. | No optional allowlist is supplied, yet the same input changes result solely because the new optional-policy mechanism exists. |
| ANC-007 | A smaller consumer runs without an atm-core manifest and receives the corrected result of an intended boundary bug fix. | The same consumer cannot run the boundary check because sc-lint now requires an atm-core policy file. |
| ANC-008 | A gap-fix sprint cites the GitHub issue reporting a malformed-policy failure and includes a regression test that reproduces that failure. | A gap-fix sprint cites the issue but tests only a generic success case, leaving the reported failure untested. |
| ANC-009 | A sprint fixes GitHub issue #197 with a lint-rule change and regression test and makes no unrelated tool or workflow changes. | A sprint fixes an in-scope issue but also redesigns an unrelated agent skill and adds a new team process. |
