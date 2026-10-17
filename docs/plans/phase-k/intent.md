# Phase K intent

Close the sc-lint gaps identified by fenix@atm-dev while preserving sc-lint as a general-purpose linting tool for multiple repositories. atm-core provides concrete consumer cases and reusable implementation and test evidence; its repository-specific policies must remain consumer configuration.

Reuse applicable atm-core code and tests directly, adapting them to sc-lint boundaries. Implement tools in Rust where appropriate, keeping Python bindings minimal. The sprint plan and dependencies remain authoritative in beads under the atm-beads workflow.

The anchor intent was confirmed by Rand. Live rubric runs and independent calibration are recorded under anchor-runs; the approved statements are locked for per-anchor tuning following Rand’s direction to test drift prevention. Generic rubric calibration did not pass; this lifecycle change does not claim it did.

Use atm-core’s reported gaps and proven implementations to improve sc-lint for all consumers. Preserve upstream test coverage when porting behavior into Rust, while keeping repository-specific policy configurable.

Optional policy manifests activate their rules when present; absent manifests preserve existing behavior. Existing simpler consumers do not need atm-core configuration. Each gap-fix sprint references its GitHub issues and tests the reported consumer failure. Reuse existing configuration and extension points; exclude unrelated tooling, skill changes and new process.

Rand confirmed these additions: “yes, those are all aligned with the goals.” This confirms the intended constraints. Rand subsequently approved all nine example pairs in anchor-fixture-review.md; anchor-fixture-approval.json records that approval. The earlier generic rubric calibration remains incomplete. Subsequent independent per-anchor tuning is accepted, all nine approved planning fixture pairs pass, and all seven then-current sprint descriptions passed the planning anchor gate; see anchor-review.md for evidence.

## Approved additions

Rand approved ADR-017 and requested: “approved. Add a sprint at the end to make sure all documents and user facind documentation is updated w/ the advanced features addedin 0.7.* so far”. Source: session `01a12885-fd21-7043-a747-0d6f9e4d9869`, 2026-10-12. Final sprint k-10 reconciles current user-facing and architecture documentation with shipped 0.7.* behavior and completed Phase K contracts. It follows both implementation tails, k-9 and k-8. It adds no product behavior or new documentation framework.

The expanded plan is undergoing fresh planning-gate validation and quality-mgr review; historical passing runs do not imply that those gates have completed.

Rand directed replacing the per-platform bootstrap scripts with one Python bootstrap: “the design is python”; “pwsh fails on mac erratically. you can't test scripts that are different between platforms.” He approved adding it to this phase: “ok, that is reasonable.” Source: session `ebafda5d-6884-401e-a56f-0dacdb3aaa49`, 2026-10-16. Sprint k-11 replaces `.sc-lint/bootstrap` and `.sc-lint/bootstrap.ps1` with one `.sc-lint/bootstrap.py` that runs identically on Linux, macOS and Windows, keeps the existing operations and exit codes, and removes pwsh from recipes, actions and tests. It fixes the Windows adoption failure blocking #541 and adds no product behavior.
