# sc-lint configure wizard UX contract

## Authority and scope

This is the authoritative UX handoff for the optional sc-lint configure wizard. It specifies the ten pages, all visible fields, JSON mappings, defaults, validation and recovery copy, enabled conditions, navigation, and terminal outcomes. It creates no launcher, page asset, consumer probe, or repository write.

The F.1 [context](../../schemas/sc-lint-configure-context.schema.json), [request](../../schemas/sc-lint-configure-request.schema.json), [plan](../../schemas/sc-lint-configure-plan.schema.json), and [result](../../schemas/sc-lint-configure-result.schema.json) schemas are the only public data authority. F.2 supplies bounded context and the no-write plan. The adapter may render this document and submit the normalized request; it may not parse a consumer Justfile/workflow, invent policy, execute a command, or own navigation state.

Context pointers are relative to the F.1 context payload. Request pointers are relative to the F.1 request. Fixture provenance is never sent to or rendered by the wizard. The initial draft is [request-recommended.json](./configure-wizard-fixtures/request-recommended.json). Commands are argv token arrays; no field accepts shell text.

| UI choice | JSON value |
| --- | --- |
| Recommended | state recommended; decision accept_recommendation |
| Enabled | state enabled; decision modify and required settings |
| Disabled | state disabled; decision disable |
| Keep existing | integration mode keep_existing |
| Review patch | integration mode review_patch; never permission to write |

## Shared layout, state, and routes

Every page has, in this order: programmatic progress text Step N of 10 — title; title; explanation; selection controls; read-only **What sc-lint found**; inline **Fix this before continuing** validation; read-only **Pending choices**; footer buttons Back, forward, Cancel setup. Facts render only F.2 conventional facts. A present Justfile or workflow must say **Present — not inspected**, never compatible or safe.

Cancel setup says: **Cancel setup? No request, plan, or repository change will be made.** Confirmed cancel returns status cancelled. Browser dismissal returns dismissed and timeout returns timeout. All three produce no request, plan, or write. Back restores all control values, argv tokens, and error messages. Changing an earlier answer truncates every later history frame before forward movement. An invalid or unselected required field disables forward and shows the stated recovery.

~~~
Overview → Baseline → Boundary → Portability → Runtime
         → Attributes/directives → Command groups → Just integration
         → CI integration → Final review
~~~

There is no skip route. Valid forward navigation always enters the next named page. Planning conflicts are visible only on final review; they never create a hidden branch or permit a rewrite.

## 1. Overview

**Title:** What sc-lint will set up

**Explanation:** sc-lint will collect JSON choices and generate a reviewable no-write plan. It checked conventional file presence only; it has not read or approved existing integration files.

**Footer:** Back disabled with announcement There is no previous step; forward **Review baseline**; Cancel setup enabled. Review baseline always enters Baseline.

| Visible field | Pointer | Default or display | Help, validation, recovery | Enabled condition |
| --- | --- | --- | --- | --- |
| **Standard developer commands** | Context /explanation/developer_contract | just setup, just lint, just test, just upgrade, schema order | These are the developer-facing commands sc-lint is helping configure. Invalid context prevents launch; recovery: Use the JSON configure request after repairing the context payload. | Read-only |
| **What sc-lint found** | Context /context/cargo_toml, /context/sc_lint_toml, /context/justfile, /context/github_workflows, /context/sc_lint_directory | One row per fact | Present is not compatibility. Absent is Not present. Never display local path or file contents. | Read-only |
| **Existing integration not inspected** | Context /explanation/uninspected_existing_integration | Empty, Justfile, and/or .github/workflows/ | sc-lint did not parse this integration and will not infer migration. Presence disagreement recovery: Regenerate bounded context. | Only if non-empty |
| **Proposed files and reasons** | Plan /operations/*/path and /operations/*/reason | Ordered no-write operations from the initial recommended draft | This is a preliminary no-write preview and is recalculated after each changed choice. Existing paths remain uninspected; no operation claims a rewrite is safe. Planning failure recovery: Repair the request field named by its pointer, then regenerate the preview. | Read-only after the initial draft validates |
| **What happens next** | None | Choose coverage and integration posture, then review a no-write plan. | Informational only; no promise of apply availability or safety. | Read-only |

## 2. Baseline

**Title:** Baseline lint and test profile

**Explanation:** Choose the baseline argv profile. Editing a command edits individual argv tokens; sc-lint never executes entered text.

The F.1 authority is [recommended-profile.toml](../../tests/fixtures/configure/contracts/recommended-profile.toml): fmt argv cargo/fmt/--all/--check; clippy argv cargo/clippy/--workspace/--all-targets/--/-D/warnings; workspace argv cargo/test/--workspace. The wizard consumes that fixture and does not substitute another profile.

**Footer:** Back to overview; **Continue to boundary**; Cancel setup. Valid choice enters Boundary.

| Visible field | Pointer | Default | Help, validation, recovery | Enabled condition |
| --- | --- | --- | --- | --- |
| **Minimum compatible sc-lint version** | Request /request/minimum_version | 0.5.0 | This is the one product compatibility floor later written to sc-lint.toml; neither a workflow nor installer selects another version. It must match the F.1 SemVer pattern. Recovery: Enter a complete SemVer value such as 0.5.0. | Always |
| **Use recommended baseline profile** | Request /request/lint_families/baseline | selected; recommended and accept_recommendation | Use documented fmt, clippy, workspace-test argv arrays. No consumer_profiles value is emitted. | Always |
| **Modify baseline argv arrays** | Request /request/lint_families/baseline and /request/consumer_profiles | not selected | Emits enabled, modify, settings.profile custom; opens three fixed cards. | Always |
| **Disable baseline profile** | Request /request/lint_families/baseline | not selected | Emits disabled and disable; removes baseline profiles. Recovery: Select Recommended or Modify to re-enable it. | Always |
| **fmt argv** | Request /request/consumer_profiles/0 | kind lint, name fmt, fixture argv | One token per item, argv not shell. At least one non-empty token. Recovery: Add executable token or choose Recommended. | Modify only |
| **clippy argv** | Request /request/consumer_profiles/1 | kind lint, name clippy, fixture argv | Same token validation. Recovery: Restore non-empty argv or choose Recommended. | Modify only |
| **workspace argv** | Request /request/consumer_profiles/2 | kind test, name workspace, fixture argv | Same token validation. Recovery: Restore non-empty argv or choose Recommended. | Modify only |

## 3. Boundary

**Title:** Boundary lint coverage

**Explanation:** Boundary linting can use a structured inventory setting. This wizard performs no source scan or inventory-file parse.

**Footer:** Back to baseline; **Continue to portability**; Cancel setup. Valid choice enters Portability.

| Visible field | Pointer | Default | Help, validation, recovery | Enabled condition |
| --- | --- | --- | --- | --- |
| **Disabled** | Request /request/lint_families/boundary | not selected | Emits disabled and disable. | Always |
| **Recommended** | Request /request/lint_families/boundary | not selected | Emits recommended and accept_recommendation. | Always |
| **Enabled with inventory setting** | Request /request/lint_families/boundary | selected; enabled, modify, settings.inventory detect | Select explicit structured inventory setting. Recovery: Choose Disabled, Recommended, or Enabled with inventory detect. | Always |
| **Inventory** | Request /request/lint_families/boundary/settings/inventory | detect | Must be non-empty string detect. Recovery: Select detect or return to Recommended. | Enabled setting only |

## 4. Portability

**Title:** Portability lint coverage

**Explanation:** Choose whether the installed sc-lint portability family is requested. This page does not probe operating systems, paths, or shell use.

**Footer:** Back to boundary; **Continue to runtime**; Cancel setup. Valid choice enters Runtime.

| Visible field | Pointer | Default | Help, validation, recovery | Enabled condition |
| --- | --- | --- | --- | --- |
| **Disabled** | Request /request/lint_families/portability | not selected | Emits disabled and disable. | Always |
| **Recommended** | Request /request/lint_families/portability | not selected | Emits recommended and accept_recommendation. | Always |
| **Enabled** | Request /request/lint_families/portability | selected; enabled and accept_recommendation | Request installed portability family, not Cargo package or source script. Recovery: Choose Disabled, Recommended, or Enabled. | Always |

## 5. Runtime

**Title:** Runtime lint coverage

**Explanation:** Choose whether to request installed sc-lint runtime coverage. This page does not classify a runtime or inspect source.

**Footer:** Back to portability; **Continue to attributes**; Cancel setup. Valid choice enters Attributes/directives.

| Visible field | Pointer | Default | Help, validation, recovery | Enabled condition |
| --- | --- | --- | --- | --- |
| **Disabled** | Request /request/lint_families/runtime | selected; disabled and disable | Do not request runtime lint coverage. | Always |
| **Recommended** | Request /request/lint_families/runtime | not selected | Emits recommended and accept_recommendation. | Always |
| **Enabled** | Request /request/lint_families/runtime | not selected | Emits enabled and accept_recommendation. Recovery: Choose Disabled, Recommended, or Enabled. | Always |

## 6. Attributes/directives

**Title:** Declarative attributes and directives

**Explanation:** Attributes and directives describe source intent. They are not an executable analyzer profile, and this page adds no argv command.

**Footer:** Back to runtime; **Continue to command groups**; Cancel setup. Valid choice enters Command groups.

| Visible field | Pointer | Default | Help, validation, recovery | Enabled condition |
| --- | --- | --- | --- | --- |
| **Disabled** | Request /request/lint_families/attributes | not selected | Emits disabled and disable. | Always |
| **Recommended** | Request /request/lint_families/attributes | selected; recommended and accept_recommendation | Record recommended declarative source intent. No command is added. | Always |
| **Enabled** | Request /request/lint_families/attributes | not selected | Emits enabled and accept_recommendation but no profile command. Recovery: Choose Disabled, Recommended, or Enabled. | Always |

## 7. Command groups

**Title:** Developer command groups

**Explanation:** Decide the four named developer command groups. A group is enabled or disabled before continuing; unselected is invalid.

The rows are fixed by Context /explanation/developer_contract: setup, lint, test, upgrade. Enabled adds bare group name to Request /request/consumer_command_groups; Disabled removes it. Unselected has no JSON representation because it cannot advance. The initial request omits this optional field; a host opening this page initializes all four rows Enabled and emits listed-order array when advancing.

**Footer:** Back to attributes; **Continue to Just integration**; Cancel setup. Unselected row message: Select Enabled or Disabled for group before continuing. Valid choice enters Just integration.

| Visible field | Pointer | Default | Help, validation, recovery | Enabled condition |
| --- | --- | --- | --- | --- |
| **setup** | Request /request/consumer_command_groups | Enabled | Include setup in desired developer contract. Recovery: Choose Enabled or Disabled for setup. | Always |
| **lint** | Request /request/consumer_command_groups | Enabled | Include lint in desired developer contract. Recovery: Choose Enabled or Disabled for lint. | Always |
| **test** | Request /request/consumer_command_groups | Enabled | Include test in desired developer contract. Recovery: Choose Enabled or Disabled for test. | Always |
| **upgrade** | Request /request/consumer_command_groups | Enabled | Include upgrade in desired developer contract. Recovery: Choose Enabled or Disabled for upgrade. | Always |

## 8. Just integration

**Title:** Just integration

**Explanation:** Choose a posture for Just integration. A present Justfile is not inspected, parsed, or classified by this wizard.

Facts always include Context /context/justfile and display Present — not inspected or Not present.

**Footer:** Back to command groups; **Continue to CI integration**; Cancel setup. Valid choice enters CI integration. Later manual collision appears at final review and blocks future apply confirmation, never a hidden rewrite.

| Visible field | Pointer | Default | Help, validation, recovery | Enabled condition |
| --- | --- | --- | --- | --- |
| **Keep existing** | Request /request/just/mode | selected when Justfile present | Preserve existing uninspected Justfile. Emits keep_existing. | Always |
| **Generate managed import** | Request /request/just/mode | selected when Justfile absent | Request later product-managed import posture; exact marked-block eligibility is later plan/apply work. Emits generate_managed_import; no write. | Always |
| **Supported migration** | Request /request/just/mode | not selected | Available only after fixture-proven migration contract identifies shape. F.3a has no proof. Recovery: Keep existing or review a patch. | Disabled in F.3a |
| **Review conflict or patch** | Request /request/just/mode | not selected | Request advisory review data. Emits review_patch; never authorizes Justfile modification. | Only when Justfile present |
| **Disable Just integration** | Request /request/just/mode | not selected | Do not request Just integration. Emits disabled. | Always |

## 9. CI integration

**Title:** CI integration

**Explanation:** Choose a posture for GitHub Actions integration. An existing workflow directory is not inspected, parsed, or classified by this wizard.

Facts always include Context /context/github_workflows and display Present — not inspected or Not present.

**Footer:** Back to Just integration; **Review no-write plan**; Cancel setup. Valid choice enters Final review. The F.3a fixture pack covers Just integration collisions only; CI conflicts remain uninspected and never authorize workflow rewrite.

| Visible field | Pointer | Default | Help, validation, recovery | Enabled condition |
| --- | --- | --- | --- | --- |
| **Keep existing** | Request /request/ci/mode | selected when workflow directory present | Preserve existing uninspected workflow directory. Emits keep_existing. | Always |
| **Generate sc-lint Action workflow** | Request /request/ci/mode | selected when workflow directory absent | Request later product-managed workflow. Version derives from sc-lint.toml. Emits generate_managed_workflow; no write. | Always |
| **Supported patch** | Request /request/ci/mode | not selected | Available only after fixture-proven workflow transformer recognizes shape. F.3a has no proof. Recovery: Keep existing or review a patch. | Disabled in F.3a |
| **Review conflict or patch** | Request /request/ci/mode | not selected | Request advisory review data. Emits review_patch; never authorizes workflow modification. | Only when workflow directory present |
| **Disable CI integration** | Request /request/ci/mode | not selected | Do not request CI integration. Emits disabled. | Always |

## 10. Final review

**Title:** Review your no-write configuration plan

**Explanation:** Review normalized request and ordered plan. Confirming generates data only; it does not apply, write, commit, or dispatch anything.

This page replaces selection controls with three full-width read-only sections in order: **Normalized request**, **Ordered no-write plan**, **Conflicts and manual steps**. It retains Pending choices. Request is formatted JSON. Plan displays operation ID, relative path, kind, reason/choices, conflict, exportable patch, and manual step. Never display absolute path.

**Footer:** Back to CI integration; **Confirm and generate no-write plan**; Cancel setup. Confirm is enabled only when request validates against F.1 request schema and every prior page is valid. Confirm returns complete page stack, normalized request, and F.2 plan; it never calls apply. If manual conflict exists, Confirm stays enabled so user obtains plan. Separate **Apply confirmed plan** is visible but disabled: Apply is unavailable because this plan has unresolved user-owned conflicts. Review the exportable patch and manual steps; no file was modified.

| Visible field | Pointer | Displayed value | Help, validation, recovery | Enabled condition |
| --- | --- | --- | --- | --- |
| **Normalized request** | Entire request | Schema-valid formatted JSON | This is exact agent-compatible request. Invalid request disables Confirm. Recovery: Go Back and repair field named by validation pointer. | Read-only |
| **Ordered no-write plan** | Plan /operations | Operation rows in source order | Preview, not write. Planning error renders message, cause, pointer, recovery, recovery_description, docs_ref. Recovery: Return to named page, repair field, regenerate plan. | When request valid |
| **Conflicts and manual steps** | Plan /conflicts, /manual_steps, /operations/*/conflict, /operations/*/exportable_patch | Empty state or typed rows | Unresolved conflicts are not overwritten. CLI.CONFIGURE_UNMANAGED_COLLISION says: Review the exported patch; no user-owned file was modified. | Only when non-empty |
| **Confirm and generate no-write plan** | Terminal stack | No default | Returns request and plan only. Recovery: Go Back and correct field shown above. | Valid request and plan |
| **Apply confirmed plan** | No F.3a write action | Disabled | F.3a never applies a plan. Without conflict: Apply belongs to later transactional contract. | Always disabled |

## Terminal result and Wyvern capability gate

Host returns one terminal result with complete page stack, not only final value. Successful completion has status finished and contains normalized F.1 request plus F.2 no-write plan. Adapter validates request before planning and plan/result after planning. Malformed terminal data is rejected; adapter never repairs or infers selections.

Released Wyvern 0.1.0 single-dialog commands do not meet this contract. Missing capability is an upstream blocking finding, never reason for a Python state machine, custom browser application, or second UI contract. F.3b owns released-artifact qualification.

| Required host capability | Required behavior | Headless acceptance case | Failure disposition |
| --- | --- | --- | --- |
| Multi-page descriptors | Ten stable descriptors: overview through final-review, title/data/controls/footer | Submit ten descriptors; assert ordered IDs without dialog collapse | Block F.3d; report fixture to Wyvern |
| Browser-history restoration | Back restores exact radio/token/error state | Edit baseline argv, advance twice, return twice, assert token array | Block F.3d; no Python state cache |
| Conditional next-page branching | Earlier edit truncates stale forward history | Reach CI, return to baseline, change selection, advance, assert old CI/final frames absent | Block F.3d; no launcher emulation |
| Opaque per-page data | Facts and request values round-trip without reinterpretation | Supply known pointer value; assert identical terminal value | Block F.3d; no page policy serialization |
| Cancel and dismiss | Cancelled/dismissed outcomes make no write | Trigger each at page 8; assert no finished stack, request emission, plan, mutation | Block F.3d; preserve JSON-only recovery |
| Finish with full stack | Final confirmation returns every frame plus terminal data | Finish recommended request; assert ten frames and exact request | Block F.3d; final-value-only API insufficient |
| Local-only serving | Released local artifact needs no network/source checkout | Start pinned artifact offline and load fixture flow | Block F.3d; no hosted fallback |
| Deterministic headless tests | Navigation, timeout, result scriptable | Run fixture twice; compare normalized terminal JSON byte-for-byte | Block F.3d; manual clicking is not evidence |

## Handoff and non-goals

The [fixture pack](./configure-wizard-fixtures/README.md) supplies empty Rust, sc-compose, atm-core, recommended-request, and no-write-conflict scenarios. Consumer fixtures are UX inputs only, never Phase P qualification evidence.

This does not close a released multi-page Wyvern host, launcher, HTML/CSS/JS assets, target mutation, transactional apply, workflow generation, or consumer conversion. Those are later-sprint work after dependencies pass.
