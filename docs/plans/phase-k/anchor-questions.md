# Phase K accepted anchor questions

These are the independently tuned question sets installed in the question beads. Each choice rubric is included so the full decision criteria remain visible.

## ANC-001

When atm-core source implements a reported failure and can be adapted to a consumer-neutral sc-lint interface, Phase K must reuse its source logic.

### Planning gate

**relevant**

Does `item.text` work on a reported failure whose behavior could fall under the anchor’s conditional reuse duty? An item stays relevant when silent about where source comes from. The conditional duty is not triggered when the item explicitly establishes that no adaptable atm-core implementation exists. Judge applicability only, never whether reuse is planned.

- **yes:** The item implements, investigates, or changes a reported failure behavior and does not rule out applicable atm-core source.
- **no:** The item is unrelated, or expressly establishes that no atm-core source for this behavior can fit sc-lint.
- **unclear:** The item is too vague to identify whether it concerns a reported failure behavior.

**addressed**

Does `item.text` plan to reuse applicable atm-core source logic for the reported behavior, adapted to a reusable sc-lint interface? Judge the plan stated, not conflict.

- **yes:** It says to port, copy, fork, or otherwise reuse the existing source logic as the implementation starting point.
- **no:** It only studies or mentions atm-core, omits source reuse, or proposes a fresh implementation.
- **unclear:** It proposes a related implementation but leaves whether source logic is reused uncertain.

**contradicted**

Does `item.text` explicitly choose a fresh implementation rather than reuse applicable atm-core source for the reported behavior? Silence is not conflict.

- **yes:** It rejects, replaces, or rewrites from scratch adaptable existing atm-core source.
- **no:** It does not reject applicable source reuse, including when the item is silent or no applicable source exists.
- **unclear:** Its approach may bypass suitable source, but the text does not settle that.

### Development gate

**relevant**

Is the reported work within the conditional scope of a reported failure for which suitable atm-core source may exist? Omission of the required action does not make the item irrelevant; no adaptable atm-core source exists for the behavior makes it outside scope.

- **yes:** It concerns the governed behavior or investigates it and the trigger is not ruled out.
- **no:** It is unrelated or establishes that no adaptable atm-core source exists for the behavior.
- **unclear:** It does not identify enough to decide whether governed behavior is involved.

**implemented**

Read `item.text` for a report of completed source reuse. Did the delivered work actually use suitable atm-core source logic as the basis of consumer-neutral sc-lint behavior? Reporting only that the failure check was implemented, without saying source logic was reused, is no. Intentions and future plans are also no. Judge reported implementation, not proof.

- **yes:** The account explicitly reports completed reuse of suitable atm-core implementation source logic for this behavior.
- **no:** It reports only a failure check, investigation, future plan, or other work without saying the applicable source logic was reused.
- **unclear:** It mentions source reuse but leaves whether that reuse was completed unresolved.

**violated**

In completed work, was the incompatible alternative used: delivered the same applicable behavior independently instead of reusing suitable atm-core source? Only an explicit completed conflict qualifies.

- **yes:** The delivered work explicitly delivered the same applicable behavior independently instead of reusing suitable atm-core source.
- **no:** No completed incompatible choice is stated.
- **unclear:** There may be an incompatible completed choice, but it is not clear.

**evidenced**

Does the completed-compliance claim include a checkable reference to where the required behavior lives or how it was tested? Do not infer evidence from the claim itself.

- **yes:** A path, diff, specific test, output, or comparison can be checked and supports the claim.
- **no:** Only an assertion, intent, or uncheckable description is given.
- **unclear:** The cited artifact may be checkable but its support is uncertain.

## ANC-002

When atm-core already tests a reported failure being ported, Phase K regression coverage must reuse that test case through the sc-lint interface.

### Planning gate

**relevant**

Judge whether `item.text` itself adds or changes sc-lint test coverage for a reported failure with an existing applicable atm-core test, or investigates that upstream test as groundwork. A happy-path-only test that omits the upstream failure remains relevant because it changes coverage for the same reported behavior. A docs-only item that only describes or executes already-existing cases, without changing coverage, is outside this anchor. If no upstream test exists, the conditional duty does not apply.

- **yes:** The item adds or changes tests or regression coverage for the reported behavior, even with only a happy-path case that omits the upstream failure, or investigates the upstream test as groundwork.
- **no:** The item only documents or executes already-existing tests without adding or changing regression coverage, is unrelated, or expressly establishes no applicable upstream test exists.
- **unclear:** The item mentions failure tests but it is unclear whether it undertakes regression coverage work or only documents existing coverage.

**addressed**

Does `item.text` explicitly plan to reuse the upstream failing input and expected result in a sc-lint regression test through its interface? Judge presence of this specific obligation, not conflict.

- **yes:** It plans to reuse the upstream failing input and expected result in a sc-lint regression test through its interface in substance, even with different words.
- **no:** It is silent, only mentions the topic, or uses a different approach without including that duty.
- **unclear:** It alludes to the duty but does not establish whether the specific behavior is planned.

**contradicted**

Does `item.text` affirmatively reject, omit, or replace reuse of an available applicable atm-core failure test while adding or changing sc-lint regression coverage? Adding independent failure tests in its place conflicts. A docs-only file fence saying no test cases will be added or changed is not an affirmative rejection of upstream reuse in implementing coverage. Silence is not conflict.

- **yes:** The item undertakes regression coverage for a failure with an applicable upstream test, and explicitly rejects or replaces that upstream case.
- **no:** No incompatible coverage choice is stated, including a docs-only instruction not to edit tests or a case where no upstream test exists.
- **unclear:** The item may replace an applicable upstream case, but that coverage choice is unresolved.

### Development gate

**relevant**

Does `item.text` concern sc-lint regression coverage for a reported failure that an atm-core test may already cover? Investigation, omission, or an explicit deferral of this coverage remains relevant to the sprint even when no test was delivered. If the item expressly establishes that no atm-core test covers the failure, the conditional reuse duty does not apply. Judge applicability, not completion.

- **yes:** It describes, investigates, or defers sc-lint regression coverage for a reported failure without ruling out an applicable atm-core test.
- **no:** It is unrelated or expressly establishes that no atm-core test covers the reported failure.
- **unclear:** It is too vague to identify whether this reported-failure test subject is involved.

**implemented**

Does `item.text` report that the delivered work actually reused the existing applicable atm-core failure test case through the sc-lint interface? A future plan or intention is not implementation. Judge the reported completed action, not its proof.

- **yes:** The report states the completed work actually reused the existing applicable atm-core failure test case through the sc-lint interface.
- **no:** It reports only a plan, investigation, silence, or a different completed approach.
- **unclear:** It discusses the duty but does not settle what was delivered.

**violated**

Does `item.text` report that the delivered work delivered coverage that deliberately replaces or omits the available upstream failure case? This conflict is about completed work, not a future plan or silence.

- **yes:** It expressly reports that the completed work delivered coverage that deliberately replaces or omits the available upstream failure case.
- **no:** It states no incompatible delivered action; omission and plan-only text do not count.
- **unclear:** A conflicting delivered action is possible but not established.

**evidenced**

If `item.text` claims this anchor was implemented in delivered work, does it cite a concrete checkable artifact supporting that claim, such as a source path, diff, test file or named test output? A bare implementation assertion and plan-only fixture are not evidence. Judge cited support, not whether the claim is true.

- **yes:** A checkable artifact is identified and tied to the implementation claim.
- **no:** No checkable artifact supports a completed compliance claim.
- **unclear:** A possible artifact is cited but its connection to the claim is unclear.

## ANC-003

Phase K tool logic must use Rust when Rust fits the tool's sc-lint crate boundary.

### Planning gate

**relevant**

Does `item.text` concern implementation language of substantive Phase K lint tool logic when Rust fits the sc-lint crate boundary? The item remains relevant when it omits the required behavior. If it explicitly establishes that the work contains only packaging glue and no substantive lint tool logic suited to Rust, the conditional duty does not apply. Judge applicability only.

- **yes:** The item plans, investigates, or changes implementation language of substantive Phase K lint tool logic when Rust fits the sc-lint crate boundary and does not rule out the condition.
- **no:** The item is unrelated or explicitly establishes that the work contains only packaging glue and no substantive lint tool logic suited to Rust.
- **unclear:** The item may touch the governed subject, but its applicability is unresolved.

**addressed**

Does `item.text` explicitly plan to put substantive tool logic in Rust where the sc-lint crate boundary fits? Judge presence of this specific obligation, not conflict.

- **yes:** It plans to put substantive tool logic in Rust where the sc-lint crate boundary fits in substance, even with different words.
- **no:** It is silent, only mentions the topic, or uses a different approach without including that duty.
- **unclear:** It alludes to the duty but does not establish whether the specific behavior is planned.

**contradicted**

Does `item.text` explicitly put substantive lint tool logic in a non-Rust language while acknowledging that Rust fits the tool and sc-lint crate boundary? The reason or alternate language does not change this conflict. Silence about language is omission, and work for which Rust does not fit is outside the conditional duty. Judge explicit conflict only.

- **yes:** The item acknowledges Rust is appropriate for substantive tool logic but chooses a non-Rust implementation of that logic.
- **no:** The item states no incompatible language choice, or establishes that Rust is not appropriate for the work.
- **unclear:** The item may bypass an appropriate Rust implementation, but either suitability or language choice remains unresolved.

### Development gate

**relevant**

Does `item.text` concern substantive Phase K lint tool logic or investigation of whether that logic fits Rust? A report about a new lint rule remains relevant even if it omits implementation language; a plan or deferral of this subject remains relevant but is not delivered compliance. If it expressly contains only packaging metadata or other work with no substantive lint logic suited to Rust, this conditional anchor does not apply. Judge subject only.

- **yes:** The item describes, investigates, plans, or defers substantive lint tool logic and does not rule out a suitable Rust boundary.
- **no:** The item is unrelated or expressly establishes there is no substantive lint logic suited to Rust.
- **unclear:** The item is too vague to identify whether substantive lint tool logic is involved.

**implemented**

Does `item.text` report completed, delivered substantive lint tool logic in Rust when Rust fits the sc-lint crate boundary? A sentence explicitly about a tool plan, future intention, or investigation is not delivered implementation even when it says the plan places logic in Rust. Judge reported completion, not proof.

- **yes:** It reports that substantive logic was actually built in Rust for a tool whose Rust crate boundary fits.
- **no:** It reports only a tool plan, future intention, investigation, omission, or a non-Rust delivered implementation.
- **unclear:** It discusses the required Rust implementation but does not settle whether it was completed.

**violated**

Does `item.text` report that completed, delivered substantive lint tool logic was implemented outside Rust even though Rust fits the tool and sc-lint crate boundary? The alternative language and motive do not matter. A sentence explicitly about a tool plan or future choice is not a delivered violation. Judge completed conflict only.

- **yes:** It reports completed substantive non-Rust logic despite acknowledging a suitable Rust crate boundary.
- **no:** No incompatible completed implementation is stated, including plan-only choices, silence, and cases where Rust is unsuitable.
- **unclear:** It may report incompatible completed work, but suitability or completed language choice is unresolved.

**evidenced**

If `item.text` claims this anchor was implemented in delivered work, does it cite a concrete checkable artifact supporting that claim, such as a source path, diff, test file or named test output? A bare implementation assertion and plan-only fixture are not evidence. Judge cited support, not whether the claim is true.

- **yes:** A checkable artifact is identified and tied to the implementation claim.
- **no:** No checkable artifact supports a completed compliance claim.
- **unclear:** A possible artifact is cited but its connection to the claim is unclear.

## ANC-004

When a Phase K rule is implemented in Rust, its Python binding must not independently implement the same rule.

### Planning gate

**relevant**

Does `item.text` undertake, describe as its work, or investigate an added or modified Python binding for a Phase K rule implemented in Rust? A binding described as forwarding to or duplicating Rust is relevant. A docs-only item that merely describes previously delivered bindings or executes their existing examples, with no binding change, is outside this anchor even if it names Python bindings. Judge applicability only.

- **yes:** The item adds, changes, or investigates a Python binding for a Rust-implemented rule, or describes that binding behavior as the work being delivered.
- **no:** The item only updates documentation or verifies existing examples without creating/changing bindings, changes only Rust code, or is unrelated.
- **unclear:** The item mentions bindings but does not establish whether it undertakes binding work or only describes existing behavior.

**addressed**

Use the specific behavioral distinction in the criteria to judge `item.text`; do not infer missing behavior. Does the planned method state the complete obligation, including every conditional branch?

- **yes:** Classify yes when python delegates substantive decisions for the Rust-implemented rule to Rust without a second independent implementation; argument conversion, type checking, and result shaping may remain in Python. Equivalent substantive wording counts.
- **no:** Classify no when the behavior is absent, merely topical, incomplete, or replaced by an incompatible alternative.
- **unclear:** Classify unclear when the text alludes to the specific behavior but its meaning is unresolved.

**contradicted**

Use the specific behavioral distinction in the criteria to judge `item.text`; do not infer missing behavior. Is there an affirmative decision for an incompatible method? Silence alone says no.

- **yes:** Classify yes when python independently decides the same substantive rule already implemented in Rust.
- **no:** Classify no when no incompatible behavior is stated; silence and missing detail alone count no.
- **unclear:** Classify unclear when stated behavior might be incompatible but cannot be resolved.

### Development gate

**relevant**

Use the specific behavioral distinction in the criteria to judge `item.text`; do not infer missing behavior. Is the governed work present? Investigating it or omitting its constraint still touches its subject.

- **yes:** Classify yes when the work touches Python bindings for any Phase K rule implemented in Rust, including investigations and incomplete changes.
- **no:** Classify no when the work is unrelated or explicitly says no Python binding for a Rust-implemented rule is created or changed.
- **unclear:** Classify unclear when the text does not establish whether this subject is touched.

**implemented**

Use the specific behavioral distinction in the criteria to judge `item.text`; do not infer missing behavior. Does the described result implement the complete obligation, including every conditional branch? Evidence presence is separate.

- **yes:** Classify yes when python delegates substantive decisions for the Rust-implemented rule to Rust without a second independent implementation; argument conversion, type checking, and result shaping may remain in Python. Equivalent substantive wording counts.
- **no:** Classify no when the behavior is absent, merely topical, incomplete, or replaced by an incompatible alternative.
- **unclear:** Classify unclear when the text alludes to the specific behavior but its meaning is unresolved.

**violated**

Use the specific behavioral distinction in the criteria to judge `item.text`; do not infer missing behavior. Does the described result affirmatively break the obligation? Silence alone says no.

- **yes:** Classify yes when python independently decides the same substantive rule already implemented in Rust.
- **no:** Classify no when no incompatible behavior is stated; silence and missing detail alone count no.
- **unclear:** Classify unclear when stated behavior might be incompatible but cannot be resolved.

**evidenced**

Use the specific behavioral distinction in the criteria to judge `item.text`; do not infer missing behavior. Are concrete artifacts or observed checks supplied for the governed behavior? Evidence may demonstrate failure as well as success.

- **yes:** Classify yes when an inspectable code or diff excerpt, specific artifact content, or a concrete command/test observation substantiates the rule behavior.
- **no:** Classify no when only assertions, intentions, topic mentions, or unsubstantiated claims are supplied; no concrete behavioral evidence.
- **unclear:** Classify unclear when a cited artifact cannot be connected to the relevant behavior.

## ANC-005

Phase K lint behavior must support consumer repositories beyond atm-core without hard-coded atm-core repository identities or policies.

### Planning gate

**relevant**

Judge the work item, not the list of features it describes. If it exclusively updates documentation and executes examples of already-delivered lint behavior, without changing consumer-facing lint rules, configuration, or tools, answer no. Otherwise, an item that describes, adds, changes, or investigates consumer-facing Phase K lint behavior across repositories is relevant even when its method is missing or wrong.

- **yes:** The item undertakes or investigates consumer-facing lint behavior, configuration, or tool work across repositories, or describes the rule it is implementing.
- **no:** The item exclusively documents or checks examples of already-delivered behavior without changing consumer-facing behavior, is unrelated, or explicitly rules out a governed change.
- **unclear:** The item mentions such behavior but does not establish whether it is undertaking governed work or only documenting existing behavior.

**addressed**

Assess addressed for the single work item against this approved requirement: Phase K lint behavior must support consumer repositories beyond atm-core without hard-coded atm-core repository identities or policies. Classify whether its planned behavior explicitly carries the complete requirement. Mere topic naming or an incomplete requirement does not carry it.

- **yes:** behavior supports repositories beyond atm-core, with no hard-coded atm-core repository identity or private policy; consumer configuration may select policies and generic repository-neutral rules may need none. Equivalent substantive wording counts.
- **no:** The behavior is absent, merely topical, incomplete, or replaced by an incompatible alternative.
- **unclear:** The text alludes to the specific behavior but its meaning is unresolved.

**contradicted**

Assess contradicted for the single work item against this approved requirement: Phase K lint behavior must support consumer repositories beyond atm-core without hard-coded atm-core repository identities or policies. Classify only an explicit incompatible decision. Missing description is not an incompatible decision.

- **yes:** the behavior depends on hard-coded atm-core identity OR embeds atm-core-specific policy, even without an identity check.
- **no:** No incompatible behavior is stated; silence and missing detail alone count no.
- **unclear:** Stated behavior might be incompatible but cannot be resolved.

### Development gate

**relevant**

Use the specific behavioral distinction in the criteria to judge `item.text`; do not infer missing behavior. Is the governed work present? Investigating it or omitting its constraint still touches its subject.

- **yes:** Classify yes when the work touches consumer-facing Phase K lint behavior, configuration, or tools across repositories, including investigations and incomplete changes.
- **no:** Classify no when the work is unrelated or explicitly says no consumer-facing lint behavior, configuration, or tool behavior is touched.
- **unclear:** Classify unclear when the text does not establish whether this subject is touched.

**implemented**

Use the specific behavioral distinction in the criteria to judge `item.text`; do not infer missing behavior. Does the described result implement the complete obligation, including every conditional branch? Evidence presence is separate.

- **yes:** Classify yes when behavior supports repositories beyond atm-core, with no hard-coded atm-core repository identity or private policy; consumer configuration may select policies and generic repository-neutral rules may need none. Equivalent substantive wording counts.
- **no:** Classify no when the behavior is absent, merely topical, incomplete, or replaced by an incompatible alternative.
- **unclear:** Classify unclear when the text alludes to the specific behavior but its meaning is unresolved.

**violated**

Use the specific behavioral distinction in the criteria to judge `item.text`; do not infer missing behavior. Does the described result affirmatively break the obligation? Silence alone says no.

- **yes:** Classify yes when the behavior depends on hard-coded atm-core identity OR embeds atm-core-specific policy, even without an identity check.
- **no:** Classify no when no incompatible behavior is stated; silence and missing detail alone count no.
- **unclear:** Classify unclear when stated behavior might be incompatible but cannot be resolved.

**evidenced**

Use the specific behavioral distinction in the criteria to judge `item.text`; do not infer missing behavior. Are concrete artifacts or observed checks supplied for the governed behavior? Evidence may demonstrate failure as well as success.

- **yes:** Classify yes when an inspectable code or diff excerpt, specific artifact content, or a concrete command/test observation substantiates the rule behavior.
- **no:** Classify no when only assertions, intentions, topic mentions, or unsubstantiated claims are supplied; no concrete behavioral evidence.
- **unclear:** Classify unclear when a cited artifact cannot be connected to the relevant behavior.

## ANC-006

Optional consumer policy must be opt-in: sc-lint enforces it when supplied and preserves that policy's prior result when absent.

### Planning gate

**relevant**

Judge the activity in `item.text`, not the subject words it mentions. Is this item adding, changing, or investigating introduction or change of optional consumer-policy behavior, including supported manifests, allowlists, or configuration tables? An item that only updates documentation or runs examples of already delivered behavior, with no scoped implementation/test/configuration change, is outside this anchor. Omission within scoped work remains relevant. An express statement that no optional consumer-policy mechanism or behavior is introduced or changed is also outside scope.

- **yes:** It adds, changes, or investigates introduction or change of optional consumer-policy behavior, including supported manifests, allowlists, or configuration tables, including work whose required method is omitted.
- **no:** It only documents or executes examples of already delivered behavior without a scoped change, is unrelated, or expressly establishes that no optional consumer-policy mechanism or behavior is introduced or changed.
- **unclear:** The text does not establish whether it undertakes scoped work or only describes existing behavior.

**addressed**

Compare the work with the approved anchor scope. Does `item.text` explicitly commit to behavior satisfying: supplied optional policy is enforced AND absence preserves that policy’s prior result on identical input; both branches are necessary? Judge specific compliance only.

- **yes:** supplied optional policy is enforced AND absence preserves that policy’s prior result on identical input; both branches are necessary. Equivalent substantive wording counts.
- **no:** The behavior is absent, merely topical, incomplete, or replaced by an incompatible alternative.
- **unclear:** The text alludes to the specific behavior but its meaning is unresolved.

**contradicted**

Compare the work with the approved anchor scope. Does `item.text` explicitly choose behavior that conflicts because a supplied optional policy is ignored OR absence changes that policy’s prior result solely due to the new optional mechanism? Omission, investigation, and topical mention alone do not conflict.

- **yes:** a supplied optional policy is ignored OR absence changes that policy’s prior result solely due to the new optional mechanism.
- **no:** No incompatible behavior is stated; silence and missing detail alone count no.
- **unclear:** Stated behavior might be incompatible but cannot be resolved.

### Development gate

**relevant**

Use the specific behavioral distinction in the criteria to judge `item.text`; do not infer missing behavior. Is the governed work present? Investigating it or omitting its constraint still touches its subject.

- **yes:** Classify yes when the work touches optional consumer policies, including manifests, allowlists, configuration tables, and other supported forms, including investigations and incomplete changes.
- **no:** Classify no when the work is unrelated or explicitly says no optional consumer policy mechanism is introduced or changed.
- **unclear:** Classify unclear when the text does not establish whether this subject is touched.

**implemented**

Use the specific behavioral distinction in the criteria to judge `item.text`; do not infer missing behavior. Does the described result implement the complete obligation, including every conditional branch? Evidence presence is separate.

- **yes:** Classify yes when supplied optional policy is enforced AND absence preserves that policy’s prior result on identical input; both branches are necessary. Equivalent substantive wording counts.
- **no:** Classify no when the behavior is absent, merely topical, incomplete, or replaced by an incompatible alternative.
- **unclear:** Classify unclear when the text alludes to the specific behavior but its meaning is unresolved.

**violated**

Use the specific behavioral distinction in the criteria to judge `item.text`; do not infer missing behavior. Does the described result affirmatively break the obligation? Silence alone says no.

- **yes:** Classify yes when a supplied optional policy is ignored OR absence changes that policy’s prior result solely due to the new optional mechanism.
- **no:** Classify no when no incompatible behavior is stated; silence and missing detail alone count no.
- **unclear:** Classify unclear when stated behavior might be incompatible but cannot be resolved.

**evidenced**

Use the specific behavioral distinction in the criteria to judge `item.text`; do not infer missing behavior. Are concrete artifacts or observed checks supplied for the governed behavior? Evidence may demonstrate failure as well as success.

- **yes:** Classify yes when an inspectable code or diff excerpt, specific artifact content, or a concrete command/test observation substantiates the rule behavior.
- **no:** Classify no when only assertions, intentions, topic mentions, or unsubstantiated claims are supplied; no concrete behavioral evidence.
- **unclear:** Classify unclear when a cited artifact cannot be connected to the relevant behavior.

## ANC-007

Existing simpler consumers must continue to run sc-lint without atm-core-specific configuration.

### Planning gate

**relevant**

Does `item.text` address compatibility of sc-lint consumer configuration, CLI operation, or boundary checking for existing simpler consumers? A statement that a simpler consumer remains runnable with ordinary settings is relevant preservation even without an explicit code-change verb. Investigation, omission, or conflict in that subject remains relevant. Answer no if the item expressly states consumer configuration and boundary checking are untouched, or limits work to documenting/verifying already-delivered compatibility.

- **yes:** The item states, plans, changes, preserves, or investigates simpler-consumer configuration, CLI, or boundary-check compatibility as governed work, including a statement that the CLI remains runnable without atm-core settings.
- **no:** The item expressly leaves consumer configuration and boundary checking untouched, limits its work to documentation or execution of existing examples without such changes, or is unrelated.
- **unclear:** It mentions simpler consumers but does not resolve whether it addresses governed compatibility or only describes existing behavior in documentation.

**addressed**

Does `item.text` state as part of a changed sc-lint behavior that existing simpler consumers can run without atm-core-specific configuration? Merely investigating how they invoke the tool, or documenting already-delivered compatibility, does not address a new compatibility obligation. Ordinary optional settings and intended bug-fix result changes are allowed.

- **yes:** The item carries forward simpler-consumer operation without atm-core-specific configuration as part of the configuration, boundary, or CLI behavior it changes or preserves.
- **no:** The item only investigates simpler-consumer use, only documents already-delivered compatibility, omits preservation in governed work, or requires atm-core-specific configuration.
- **unclear:** The item concerns a changed consumer behavior but leaves whether atm-core-specific configuration is required uncertain.

**contradicted**

Does `item.text` state an incompatible planned choice: make running sc-lint by an existing simpler consumer depend on any atm-core-specific configuration? Silence or incomplete discussion is not conflict.

- **yes:** It explicitly commits to make running sc-lint by an existing simpler consumer depend on any atm-core-specific configuration.
- **no:** It contains no incompatible action; silence alone is not a violation.
- **unclear:** A conflicting action is possible but cannot be established.

### Development gate

**relevant**

Read `item.text` for applicability to consumer configuration or boundary-check compatibility for existing simpler sc-lint consumers; compliance is a separate judgment. Omission or conflict remains relevant. Work explicitly outside the governing subject is not relevant. Decide from the described action and its scope; do not infer missing commitments.

- **yes:** The subject consumer configuration or boundary-check compatibility for existing simpler sc-lint consumers is involved, regardless of whether the duty is stated or obeyed.
- **no:** It is unrelated, or explicitly establishes that the item does not change consumer configuration or boundary checking.
- **unclear:** The governed subject cannot be determined.

**implemented**

Does `item.text` report completed behavior in which existing simpler consumers can run the sc-lint behavior under discussion without atm-core-specific configuration? Judge what the account claims, separately from supporting evidence. A report explicitly identifying its behavior as completed counts regardless of local verb tense. Future-only intentions, topical mentions, and unrelated work do not count.

- **yes:** The account reports that a simpler consumer runs, remains able to run, or has retained operation of a sc-lint check or CLI without an atm-core manifest or other atm-core-specific configuration. A stated completed-behavior report counts even without evidence.
- **no:** That compatibility content is absent, only planned for the future, replaced by a requirement for atm-core configuration, or inferred merely from unrelated work.
- **unclear:** The account discusses operation but does not establish whether compatibility is completed or atm-core-specific configuration is required.

**violated**

Does `item.text` state an incompatible completed action: make running sc-lint by an existing simpler consumer depend on any atm-core-specific configuration? Silence or incomplete discussion is not conflict. Decide from the described action and its scope; do not infer missing commitments.

- **yes:** The described delivered work requires the incompatible behavior: make running sc-lint by an existing simpler consumer depend on any atm-core-specific configuration.
- **no:** It contains no incompatible action; silence alone is not a violation.
- **unclear:** A conflicting action is possible but cannot be established.

**evidenced**

Does `item.text` cite a concrete checkable artifact directly supporting completed compliance: preserve simpler consumers’ ability to run sc-lint without any atm-core-specific configuration? Judge support, not whether the behavior is asserted. Bare claims, future plans, or unrelated artifacts are insufficient. Decide from the described action and its scope; do not infer missing commitments.

- **yes:** A named test, command and observation, code path or diff directly supports the specific completed obligation.
- **no:** No direct supporting artifact is cited, no compliance claim is made, or the artifact is unrelated.
- **unclear:** An artifact is mentioned but its support for the obligation cannot be determined.

## ANC-008

Each Phase K gap-fix sprint must include an issue-linked regression test reproducing its reported consumer failure.

### Planning gate

**relevant**

Read `item.text` for applicability to a Phase K sprint that fixes a reported consumer gap; compliance is a separate judgment. Omission or conflict remains relevant. Work explicitly outside the governing subject is not relevant. Decide from the described action and its scope; do not infer missing commitments.

- **yes:** The subject a Phase K sprint that fixes a reported consumer gap is involved, regardless of whether the duty is stated or obeyed.
- **no:** It is unrelated, or explicitly establishes that the item is not a sprint fixing a reported consumer gap.
- **unclear:** The governed subject cannot be determined.

**addressed**

Is the anchor-specific behavior present in the planned method or constraint: cite the reporting GitHub issue and include an issue-linked regression test reproducing its reported failure? Topic labels and incomplete descriptions do not carry this duty. Decide from the described action and its scope; do not infer missing commitments.

- **yes:** The text explicitly states cite the reporting GitHub issue and include an issue-linked regression test reproducing its reported failure as a planned action or constraint.
- **no:** The obligation is omitted, only its topic is mentioned, or an incompatible approach replaces it.
- **unclear:** The text alludes to compliance but leaves the required behavior uncertain.

**contradicted**

Does `item.text` explicitly reject or defer an issue-linked regression test that reproduces the reported consumer failure within its Phase K gap-fix sprint? Judge incompatible choices only. Omitting discussion of issues or tests is silence, not conflict.

- **yes:** The item deliberately omits the reporting GitHub issue link, deliberately skips or defers reproduction of the reported failure beyond this sprint, or substitutes only a generic success test. Any such explicit incompatible decision counts.
- **no:** There is no explicit incompatible decision; a missing issue/test discussion by itself does not count.
- **unclear:** A possibly incompatible decision is described, but it is uncertain whether the required issue-linked failure reproduction is rejected.

### Development gate

**relevant**

Judge only whether `item.text` touches a Phase K sprint that fixes a reported consumer gap. Omission or conflict remains relevant. Work explicitly outside the governing subject is not relevant.

- **yes:** It changes, investigates, or defines a Phase K sprint that fixes a reported consumer gap.
- **no:** It is unrelated, or explicitly establishes that the item is not a sprint fixing a reported consumer gap.
- **unclear:** The governed subject cannot be determined.

**implemented**

Does the delivered account state it actually did cite the reporting GitHub issue and include an issue-linked regression test reproducing its reported failure? Judge the specific obligation in `item.text`, not topic mention or conflict. Future intentions are not completed behavior; evidence is judged separately.

- **yes:** The text explicitly states cite the reporting GitHub issue and include an issue-linked regression test reproducing its reported failure as completed behavior.
- **no:** The obligation is omitted, only its topic is mentioned, or an incompatible approach replaces it. A plan to do it later also counts no.
- **unclear:** The text alludes to compliance but leaves the required behavior uncertain.

**violated**

Does `item.text` report a completed Phase K gap-fix sprint that explicitly rejected or deferred its issue-linked regression reproduction? Judge incompatible choices only. Missing discussion alone is not violation.

- **yes:** The completed sprint deliberately omitted the reporting GitHub issue link, skipped or deferred reproduction of the reported consumer failure beyond the sprint, or substituted only a generic success test.
- **no:** No incompatible completed decision is reported; absent issue or test discussion is silence.
- **unclear:** Whether the delivered sprint rejected the required issue-linked failure reproduction cannot be determined.

**evidenced**

Does `item.text` cite a checkable artifact that directly supports completed delivery of an issue-linked regression reproducing the reported consumer failure? A bare assertion that a test exists, even with an issue number and described input, is not an artifact citation. Require a test name/path, code/diff reference, command/output, or before/after observation tied to that regression. Judge evidence only.

- **yes:** It gives a concrete locator, test name, code/diff reference, executed command/output, or before/after observation directly supporting the issue-linked failure reproduction.
- **no:** It merely asserts a regression test exists or describes what it covers without citing a checkable artifact; no compliance claim is made; or only unrelated evidence is cited.
- **unclear:** A possible artifact citation is present but its connection to the issue-linked failure reproduction is uncertain.

## ANC-009

Phase K sprints must keep changes within the planned consumer gaps, excluding unrelated tooling, skill changes and new process.

### Planning gate

**relevant**

Does `item.text` concern a sprint’s proposed or delivered changes or their scope in this Phase K review? A sprint gap fix, an investigation of sprint scope, and an unrelated addition proposed for a sprint all touch this constraint. It also applies when the proposed sprint contains only unrelated work. An activity explicitly outside Phase K is not governed. Judge applicability only.

- **yes:** It describes a sprint gap fix, its change scope, or additions to sprint work. A sprint in this review is relevant unless the text explicitly places it outside Phase K.
- **no:** It describes a different activity explicitly outside the Phase K sprint or unrelated system work with no Phase K scope connection.
- **unclear:** Whether the activity belongs to a Phase K sprint cannot be determined.

**addressed**

Does `item.text` explicitly limit the sprint to its planned consumer-gap work? An in-scope lint-rule fix and regression test together with an explicit exclusion of unrelated changes carries this constraint. Equivalent scope language counts; the item need not repeat every excluded category verbatim. Judge inclusion of the scope limit only.

- **yes:** The sprint includes its gap fix or direct supporting work and explicitly limits changes to that work, or says no unrelated tool or workflow changes. Phrases such as only in-scope changes, nothing outside the planned gaps, and no unrelated tooling, skills, or process carry the same scope limit.
- **no:** It only mentions or fixes a gap without an explicit scope limit, merely investigates scope, describes unrelated work without the constraint, or adds changes outside the planned gaps.
- **unclear:** It mentions limiting scope but does not establish whether unrelated changes are excluded.

**contradicted**

Does `item.text` explicitly include changes outside the planned consumer gaps within a Phase K sprint? This includes unrelated tooling, skill changes, or new process, whether accompanying a gap fix or proposed alone as Phase K work. Judge incompatible scope decisions only. A separate activity explicitly outside Phase K cannot violate this sprint scope constraint; silence about scope is not conflict.

- **yes:** A Phase K sprint includes work outside its planned consumer gaps, such as unrelated tooling, skill changes, or new process.
- **no:** No incompatible Phase K scope decision is stated, the description is silent, or the activity is explicitly outside Phase K. Tool/test support directly needed for the planned gap is allowed.
- **unclear:** It is uncertain whether the proposed extra changes belong to the Phase K sprint or directly support its planned consumer gap.

### Development gate

**relevant**

Does `item.text` define, investigate, or deliver any changes in a Phase K sprint? The scope constraint applies to every proposed Phase K sprint change, including unrelated additions or a sprint containing only unrelated work. No in-scope gap fix is required for relevance. Work explicitly outside Phase K is not governed. Judge applicability only. Applicability concerns subject matter; a future-only intention about that subject remains relevant even though it is not completed implementation.

- **yes:** It concerns the content or scope of a Phase K sprint, including a proposal that adds only unrelated tooling, skills, or process.
- **no:** It describes a different activity explicitly outside the Phase K sprint or unrelated system work with no Phase K scope connection.
- **unclear:** Whether the activity belongs to a Phase K sprint cannot be determined.

**implemented**

Does the delivered account state it actually did limit the sprint to planned consumer-gap fixes and exclude unrelated tooling, agent skills, and new team process? Judge the specific obligation in `item.text`, not topic mention or conflict. Future intentions are not completed behavior; evidence is judged separately.

- **yes:** The text explicitly states limit the sprint to planned consumer-gap fixes and exclude unrelated tooling, agent skills, and new team process as completed behavior.
- **no:** The obligation is omitted, only its topic is mentioned, or an incompatible approach replaces it. A plan to do it later also counts no.
- **unclear:** The text alludes to compliance but leaves the required behavior uncertain.

**violated**

Does `item.text` report completed changes outside the planned consumer gaps within a Phase K sprint? This includes unrelated tooling, skill changes, or new process, whether accompanying a gap fix or delivered alone as Phase K work. Judge incompatible completed scope decisions only. A separate activity explicitly outside Phase K cannot violate this sprint scope constraint; silence about scope is not conflict.

- **yes:** A Phase K sprint includes work outside its planned consumer gaps, such as unrelated tooling, skill changes, or new process.
- **no:** No incompatible Phase K scope decision is stated, the description is silent, or the activity is explicitly outside Phase K. Tool/test support directly needed for the planned gap is allowed.
- **unclear:** It is uncertain whether the proposed extra changes belong to the Phase K sprint or directly support its planned consumer gap.

**evidenced**

Does `item.text` cite a concrete checkable artifact directly supporting completed compliance: limit the sprint to planned consumer-gap fixes and exclude unrelated tooling, agent skills, and new team process? Judge support, not whether the behavior is asserted. Bare claims, future plans, or unrelated artifacts are insufficient.

- **yes:** A named test, command and observation, code path or diff directly supports the specific completed obligation.
- **no:** No direct supporting artifact is cited, no compliance claim is made, or the artifact is unrelated.
- **unclear:** An artifact is mentioned but its support for the obligation cannot be determined.
