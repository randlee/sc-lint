# Anchors - phase-k

Constraints captured during planning. Each anchor is a testable statement written by the orchestrator and approved by the user; the user's verbatim words are kept beneath it as provenance. Every anchor is judged at the planning gate and again at the development gate. Edit `anchors.json`, not this file; this file is rendered.

Intent: `intent.md`

| ID | Status | Summary | Stated |
| --- | --- | --- | --- |
| ANC-001 | locked | Source reuse | 2026-10-12 |
| ANC-002 | locked | Test reuse | 2026-10-12 |
| ANC-003 | locked | Rust tool implementation | 2026-10-12 |
| ANC-004 | locked | Thin Python bindings | 2026-10-12 |
| ANC-005 | locked | General-purpose product | 2026-10-12 |
| ANC-006 | locked | Optional policy manifests | 2026-10-12 |
| ANC-007 | locked | Consumer compatibility | 2026-10-12 |
| ANC-008 | locked | Concrete gap closure | 2026-10-12 |
| ANC-009 | locked | Minimal scope | 2026-10-12 |

## ANC-001

Anchor: When atm-core source implements a reported failure and can be adapted to a consumer-neutral sc-lint interface, Phase K must reuse its source logic.

> This plan should use code and TESTS directly from atm-core repo where it makes sense.

Said by Rand, Phase K planning, session 01a12885-fd21-7043-a747-0d6f9e4d9869 Rand approved the exact pass/fail examples in anchor-fixture-review.md after opening that file in VS Code (approval: anchor-fixture-approval.json). The examples clarify the previously approved intent, not new goals., 2026-10-12. Status: locked. Version 3.

Summary: Source reuse

Scope: Implements a gap with an existing applicable atm-core implementation.

Beads: anchor `lint-k-anc-001`, plan questions `lint-k-anc-001-plan`, dev questions `lint-k-anc-001-dev`

Fixtures:
- pass: An atm-core parser already detects the reported failure. The sprint ports its parser logic into a reusable sc-lint interface and puts atm-core's allowlist in consumer configuration.
- fail: An atm-core parser already detects the same failure and can use sc-lint's interface, but the sprint rewrites the behavior from scratch without reusing its source.

## ANC-002

Anchor: When atm-core already tests a reported failure being ported, Phase K regression coverage must reuse that test case through the sc-lint interface.

> This plan should use code and TESTS directly from atm-core repo where it makes sense.

Said by Rand, Phase K planning, session 01a12885-fd21-7043-a747-0d6f9e4d9869 Rand approved the exact pass/fail examples in anchor-fixture-review.md after opening that file in VS Code (approval: anchor-fixture-approval.json). The examples clarify the previously approved intent, not new goals., 2026-10-12. Status: locked. Version 3.

Summary: Test reuse

Scope: Adds regression coverage for behavior already tested in atm-core.

Beads: anchor `lint-k-anc-002`, plan questions `lint-k-anc-002-plan`, dev questions `lint-k-anc-002-dev`

Fixtures:
- pass: An atm-core test fails on the reported malformed policy. A sc-lint regression test ports the same input and expected rejection through its own API.
- fail: The same atm-core test exists, but sc-lint adds only a happy-path test that never exercises the reported malformed policy.

## ANC-003

Anchor: Phase K tool logic must use Rust when Rust fits the tool's sc-lint crate boundary.

> We should implement tools in rust as appropriate w/ minimal python bindings.

Said by Rand, Phase K planning, session 01a12885-fd21-7043-a747-0d6f9e4d9869 Rand approved the exact pass/fail examples in anchor-fixture-review.md after opening that file in VS Code (approval: anchor-fixture-approval.json). The examples clarify the previously approved intent, not new goals., 2026-10-12. Status: locked. Version 3.

Summary: Rust tool implementation

Scope: Adds or relocates lint tool implementation logic.

Beads: anchor `lint-k-anc-003`, plan questions `lint-k-anc-003-plan`, dev questions `lint-k-anc-003-dev`

Fixtures:
- pass: A tool plan places the substantive logic in Rust and explains how the crate boundary fits; its Python binding only translates inputs and results.
- fail: A tool plan expressly says Rust is appropriate for its tool and sc-lint boundary, yet chooses Python for the substantive rule solely for convenience.

## ANC-004

Anchor: When a Phase K rule is implemented in Rust, its Python binding must not independently implement the same rule.

> We should implement tools in rust as appropriate w/ minimal python bindings.

Said by Rand, Phase K planning, session 01a12885-fd21-7043-a747-0d6f9e4d9869 Rand approved the exact pass/fail examples in anchor-fixture-review.md after opening that file in VS Code (approval: anchor-fixture-approval.json). The examples clarify the previously approved intent, not new goals., 2026-10-12. Status: locked. Version 3.

Summary: Thin Python bindings

Scope: Adds or modifies Python bindings for Rust tools.

Beads: anchor `lint-k-anc-004`, plan questions `lint-k-anc-004-plan`, dev questions `lint-k-anc-004-dev`

Fixtures:
- pass: The Python binding forwards arguments to the Rust matcher and converts the Rust result to the public Python return shape.
- fail: The Rust matcher exists, but the Python binding independently reimplements the matching rule and can return a different result.

## ANC-005

Anchor: Phase K lint behavior must support consumer repositories beyond atm-core without hard-coded atm-core repository identities or policies.

> sc-lint is a general purpose linting tool that is used on many repos.

Said by Rand, Phase K planning, session 01a12885-fd21-7043-a747-0d6f9e4d9869 Rand approved the exact pass/fail examples in anchor-fixture-review.md after opening that file in VS Code (approval: anchor-fixture-approval.json). The examples clarify the previously approved intent, not new goals., 2026-10-12. Status: locked. Version 2.

Summary: General-purpose product

Scope: Adds consumer-facing lint rules, configuration or tool behavior.

Beads: anchor `lint-k-anc-005`, plan questions `lint-k-anc-005-plan`, dev questions `lint-k-anc-005-dev`

Fixtures:
- pass: Two repositories run the same rule with different configured allowlists; no rule checks the repository name.
- fail: The rule detects the repository name atm-core and silently enables an atm-core-only allowlist with no consumer configuration.

## ANC-006

Anchor: Optional consumer policy must be opt-in: sc-lint enforces it when supplied and preserves that policy's prior result when absent.

> I would make these optional if possible, if the manifest/allow list exists, it is used, if it is absent then current behavior.

Said by Rand, Session 01a12885-fd21-7043-a747-0d6f9e4d9869: Rand approved the four proposed constraints (optional policy manifests, compatibility, concrete gap closure, minimal scope) with: yes, those are all aligned with the goals. Pending live rubric, not gate-ready. Rand approved the exact pass/fail examples in anchor-fixture-review.md after opening that file in VS Code (approval: anchor-fixture-approval.json). The examples clarify the previously approved intent, not new goals., 2026-10-12. Status: locked. Version 3.

Summary: Optional policy manifests

Scope: Introduces optional manifests or allowlists.

Beads: anchor `lint-k-anc-006`, plan questions `lint-k-anc-006-plan`, dev questions `lint-k-anc-006-dev`

Fixtures:
- pass: Using the chosen supported form (a configuration table or a file), an optional allowlist is supplied and enforced; without that allowlist, the same input retains its legacy result for that policy.
- fail: No optional allowlist is supplied, yet the same input changes result solely because the new optional-policy mechanism exists.

## ANC-007

Anchor: Existing simpler consumers must continue to run sc-lint without atm-core-specific configuration.

> atm-core is the promary consumer of sc-lint-boundary.  other consumers use it, but they are much simpler boundary problems.

Said by Rand, Session 01a12885-fd21-7043-a747-0d6f9e4d9869: Rand approved the four proposed constraints (optional policy manifests, compatibility, concrete gap closure, minimal scope) with: yes, those are all aligned with the goals. Pending live rubric, not gate-ready. Rand approved the exact pass/fail examples in anchor-fixture-review.md after opening that file in VS Code (approval: anchor-fixture-approval.json). The examples clarify the previously approved intent, not new goals., 2026-10-12. Status: locked. Version 3.

Summary: Consumer compatibility

Scope: Changes consumer configuration or boundary checking.

Beads: anchor `lint-k-anc-007`, plan questions `lint-k-anc-007-plan`, dev questions `lint-k-anc-007-dev`

Fixtures:
- pass: A smaller consumer runs without an atm-core manifest and receives the corrected result of an intended boundary bug fix.
- fail: The same consumer cannot run the boundary check because sc-lint now requires an atm-core policy file.

## ANC-008

Anchor: Each Phase K gap-fix sprint must include an issue-linked regression test reproducing its reported consumer failure.

> yes, those are all aligned with the goals.

Said by Rand, Session 01a12885-fd21-7043-a747-0d6f9e4d9869: Rand approved the four proposed constraints (optional policy manifests, compatibility, concrete gap closure, minimal scope) with: yes, those are all aligned with the goals. Pending live rubric, not gate-ready. Rand approved the exact pass/fail examples in anchor-fixture-review.md after opening that file in VS Code (approval: anchor-fixture-approval.json). The examples clarify the previously approved intent, not new goals., 2026-10-12. Status: locked. Version 3.

Summary: Concrete gap closure

Scope: Defines a sprint addressing a reported consumer gap.

Beads: anchor `lint-k-anc-008`, plan questions `lint-k-anc-008-plan`, dev questions `lint-k-anc-008-dev`

Fixtures:
- pass: A gap-fix sprint cites the GitHub issue reporting a malformed-policy failure and includes a regression test that reproduces that failure.
- fail: A gap-fix sprint cites the issue but tests only a generic success case, leaving the reported failure untested.

## ANC-009

Anchor: Phase K sprints must keep changes within the planned consumer gaps, excluding unrelated tooling, skill changes and new process.

> fix the stack with ONLY changes that are in scope

Said by Rand, Session 01a12885-fd21-7043-a747-0d6f9e4d9869: Rand approved the four proposed constraints (optional policy manifests, compatibility, concrete gap closure, minimal scope) with: yes, those are all aligned with the goals. Pending live rubric, not gate-ready. Rand approved the exact pass/fail examples in anchor-fixture-review.md after opening that file in VS Code (approval: anchor-fixture-approval.json). The examples clarify the previously approved intent, not new goals., 2026-10-12. Status: locked. Version 2.

Summary: Minimal scope

Scope: Defines or delivers Phase K sprint changes.

Beads: anchor `lint-k-anc-009`, plan questions `lint-k-anc-009-plan`, dev questions `lint-k-anc-009-dev`

Fixtures:
- pass: A sprint fixes GitHub issue #197 with a lint-rule change and regression test and makes no unrelated tool or workflow changes.
- fail: A sprint fixes an in-scope issue but also redesigns an unrelated agent skill and adds a new team process.
