# Anchors - phase-j

Constraints captured during planning. Each anchor is a testable statement written by the orchestrator and approved by the user; the user's verbatim words are kept beneath it as provenance. Every anchor is judged at the planning gate and again at the development gate. Edit `anchors.json`, not this file; this file is rendered.

Intent: `docs/plans/phase-j/intent.md`

| ID | Status | Summary | Stated |
| --- | --- | --- | --- |
| ANC-001 | locked | No input from Rand | 2026-10-13 |
| ANC-002 | locked | Only the needed file changes | 2026-10-13 |
| ANC-003 | locked | Test that it works | 2026-10-13 |
| ANC-004 | locked | No edits to other skills | 2026-10-13 |
| ANC-005 | locked | Read the changelog | 2026-10-13 |
| ANC-006 | locked | Handle deprecations | 2026-10-13 |
| ANC-007 | locked | Execute formula steps in order | 2026-10-13 |
| ANC-008 | locked | Breaking: stop at ready develop PR | 2026-10-13 |
| ANC-009 | locked | Breaking: impact assessment in PR | 2026-10-13 |
| ANC-010 | locked | No WIP: PR to main passed preflight | 2026-10-13 |
| ANC-011 | locked | No WIP: patch version bump | 2026-10-13 |
| ANC-012 | locked | Decision points message omega-prime | 2026-10-13 |
| ANC-013 | locked | Escalations go to omega-prime | 2026-10-13 |
| ANC-014 | locked | Green protected PR: notify omega-prime | 2026-10-13 |
| ANC-015 | locked | rand-lee reviews develop/main PRs | 2026-10-13 |
| ANC-016 | locked | Floors move to the new version | 2026-10-13 |
| ANC-017 | locked | Minor break: fix and announce | 2026-10-13 |

## ANC-001

Anchor: An sc-compose version upgrade run must complete without any step that waits for input from Rand Lee, the repository owner, other than his merges of the PRs to develop and main.

> I want upgrades of dependent libraries to be completed atonamously with no input from me.

Said by Rand Lee, phase-j sc-compose upgrade formula, 2026-10-13. Status: locked. Version 3.

Summary: No input from Rand

Scope: the upgrade-sc-compose formula (.atm-bd/formula/upgrade-sc-compose.formula.toml.j2) and every bead poured from it

Fixtures:
- pass: Every step of the formula is dispatched to an agent role (dev, qa, lead, publisher); decisions and stops are sent to omega-prime@hermes and no step waits for a reply from Rand.
- fail: The release step pauses and asks Rand to confirm whether to cut a patch release before it continues.

## ANC-002

Anchor: An sc-compose version upgrade must change only the repository files that reference the sc-compose version or that the new version's changelog requires changing.

> the responsibliity is to make the 2-3 file changes necessary and be done

Said by Rand Lee, phase-j sc-compose upgrade formula, 2026-10-13. Status: locked. Version 2.

Summary: Only the needed file changes

Scope: the upgrade-sc-compose formula (.atm-bd/formula/upgrade-sc-compose.formula.toml.j2) and every bead poured from it

Fixtures:
- pass: Deliverable 1 bumps SC_COMPOSE_WHEEL_PIN in .claude/lib/sc_compose_dependency.py; deliverable 2 edits only the repo-owned files the changelog requires; owned_paths lists only these.
- fail: The upgrade step also refactors the bead templates, rewrites the README and reorganises .claude/lib while bumping the pin.

## ANC-003

Anchor: An sc-compose version upgrade must run the repository's test suite after its edits and record whether the tests pass.

> you are writing a formula that likely needs to edit 3-4 lines in the repo. and test that it works

Said by Rand Lee, phase-j sc-compose upgrade formula, 2026-10-13. Status: locked. Version 2.

Summary: Test that it works

Scope: the upgrade-sc-compose formula (.atm-bd/formula/upgrade-sc-compose.formula.toml.j2) and every bead poured from it

Fixtures:
- pass: After the pin bump the dev runs just test and the pytest suites with the new sc-compose and records each test's pass/fail in the bead notes.
- fail: The step bumps the pin and opens the PR; no tests are run against the new sc-compose version.

## ANC-004

Anchor: An sc-compose version upgrade must not change any file under another skill's directory in .claude/skills/.

> we can not micromanage other skills.

Said by Rand Lee, phase-j sc-compose upgrade formula, 2026-10-13. Status: locked. Version 3.

Summary: No edits to other skills

Scope: the upgrade-sc-compose formula (.atm-bd/formula/upgrade-sc-compose.formula.toml.j2) and every bead poured from it

Fixtures:
- pass: The upgrade step's owned_paths is only .claude/lib/sc_compose_dependency.py; it changes no file under another skill's directory in .claude/skills/, and a deprecation found in a skill-owned file is filed as an issue against that skill's repo instead.
- fail: The upgrade step rewrites .claude/skills/atm-beads/templates/*.j2 to the new sc-compose syntax.

## ANC-005

Anchor: The sc-compose version upgrade agent must read the sc-compose CHANGELOG.md or migration documentation for the new version before editing.

> the agent should read the change-log or other appropriate documentation.

Said by Rand Lee, phase-j sc-compose upgrade formula, 2026-10-13. Status: locked. Version 2.

Summary: Read the changelog

Scope: the upgrade-sc-compose formula (.atm-bd/formula/upgrade-sc-compose.formula.toml.j2) and every bead poured from it

Fixtures:
- pass: Deliverable 2 reads randlee/sc-compose CHANGELOG.md and docs/migration-notes.md at tag v<to_version> before any edit.
- fail: The dev bumps the pin and runs tests; the sc-compose changelog is not consulted.

## ANC-006

Anchor: For each deprecation or recommended change listed in the new sc-compose version's changelog, the upgrade agent must apply it or record why it does not apply.

> if there are recommended changes, deprrecations, those should be handled.

Said by Rand Lee, phase-j sc-compose upgrade formula, 2026-10-13. Status: locked. Version 2.

Summary: Handle deprecations

Scope: the upgrade-sc-compose formula (.atm-bd/formula/upgrade-sc-compose.formula.toml.j2) and every bead poured from it

Fixtures:
- pass: Each deprecation and recommended change in the changelog is listed in the bead notes and either applied with the changed path or recorded as not used with the git grep that shows it.
- fail: Deprecations listed in the sc-compose changelog are noted as out of scope and left for a later phase.

## ANC-007

Anchor: An sc-compose version upgrade must execute only the steps poured from the upgrade-sc-compose formula, in their dependency order.

> The purpose of using a formula is to have the planning work done ahead of time.  we can directly build and execute the sprint steps in order.

Said by Rand Lee, phase-j sc-compose upgrade formula, 2026-10-13. Status: locked. Version 2.

Summary: Execute formula steps in order

Scope: the upgrade-sc-compose formula (.atm-bd/formula/upgrade-sc-compose.formula.toml.j2) and every bead poured from it

Fixtures:
- pass: A run pours the formula with sc-compose bead attach and dispatches the poured beads as bd ready releases them along their needs edges; no new planning bead is created.
- fail: After pouring, the lead writes a fresh sprint plan for the upgrade and adds new sprint beads before dispatching.

## ANC-008

Anchor: When an sc-compose version upgrade has breaking changes, it must not open a release PR to main and must end with its PR to develop ready to merge.

> no if there are breaking changes, proceed with completing the work and STOP with pr ready to merge to develop AND an impace accessment.

Said by Rand Lee, phase-j sc-compose upgrade formula, 2026-10-13. Status: locked. Version 2.

Summary: Breaking: stop at ready develop PR

Scope: the upgrade-sc-compose formula (.atm-bd/formula/upgrade-sc-compose.formula.toml.j2) and every bead poured from it

Fixtures:
- pass: On a BREAKING verdict the pr_develop step leaves the develop PR ready to merge, closes the release bead as not planned and opens no PR to main.
- fail: On a BREAKING verdict the run abandons the branch without opening a PR to develop.

## ANC-009

Anchor: When an sc-compose upgrade has breaking changes, its PR to develop must include an impact assessment of those breaking changes.

> no if there are breaking changes, proceed with completing the work and STOP with pr ready to merge to develop AND an impace accessment.

Said by Rand Lee, phase-j sc-compose upgrade formula, 2026-10-13. Status: locked. Version 2.

Summary: Breaking: impact assessment in PR

Scope: the upgrade-sc-compose formula (.atm-bd/formula/upgrade-sc-compose.formula.toml.j2) and every bead poured from it

Fixtures:
- pass: On a BREAKING verdict the PR to develop description lists the verdict, each failing test and each rendered diff as an impact assessment.
- fail: On a BREAKING verdict the PR to develop is opened with only the pin change in its description.

## ANC-010

Anchor: When no other work is in progress, an sc-compose version upgrade must end in a PR to main whose publish-preflight run passed.

> if there is NOT other work in progress, I expect this to end up in a pr to main that has passed publish-preflight and has a patch++ version bump ready to publish as soon as I approve the pr to main.

Said by Rand Lee, phase-j sc-compose upgrade formula, 2026-10-13. Status: locked. Version 2.

Summary: No WIP: PR to main passed preflight

Scope: the upgrade-sc-compose formula (.atm-bd/formula/upgrade-sc-compose.formula.toml.j2) and every bead poured from it

Fixtures:
- pass: With no other work in progress the publisher runs publish-preflight to GO and only then opens the develop -> main PR.
- fail: With no other work in progress the publisher opens the PR to main and leaves publish-preflight for after Rand merges it.

## ANC-011

Anchor: When no other work is in progress, the sc-compose version upgrade's PR to main must increase the patch version number.

> if there is NOT other work in progress, I expect this to end up in a pr to main that has passed publish-preflight and has a patch++ version bump ready to publish as soon as I approve the pr to main.

Said by Rand Lee, phase-j sc-compose upgrade formula, 2026-10-13. Status: locked. Version 2.

Summary: No WIP: patch version bump

Scope: the upgrade-sc-compose formula (.atm-bd/formula/upgrade-sc-compose.formula.toml.j2) and every bead poured from it

Fixtures:
- pass: With no other work in progress the publisher bumps the patch version (0.7.4 -> 0.7.5) before the PR to main.
- fail: With no other work in progress the PR to main ships with the existing version number unchanged.

## ANC-012

Anchor: Every decision point in the upgrade-sc-compose formula must contain an explicit `atm send omega-prime@hermes` instruction.

> at every decision point, there should be explicit instruction to send to omega-prime@hermes.

Said by Rand Lee, phase-j sc-compose upgrade formula, 2026-10-13. Status: locked. Version 2.

Summary: Decision points message omega-prime

Scope: the upgrade-sc-compose formula (.atm-bd/formula/upgrade-sc-compose.formula.toml.j2) and every bead poured from it

Fixtures:
- pass: Decision point 1 sends `atm send omega-prime@hermes` with the BREAKING or CI-green message; decision point 2 sends `atm send omega-prime@hermes` when the release is deferred.
- fail: Decision point 2 checks for other work in progress and silently closes the release bead when release is deferred.

## ANC-013

Anchor: Every escalation from an sc-compose version upgrade run must be sent with `atm send omega-prime@hermes`.

> Since the entire process will run unattended, all escelation will be to omega-prime@hermes

Said by Rand Lee, phase-j sc-compose upgrade formula, 2026-10-13. Status: locked. Version 2.

Summary: Escalations go to omega-prime

Scope: the upgrade-sc-compose formula (.atm-bd/formula/upgrade-sc-compose.formula.toml.j2) and every bead poured from it

Fixtures:
- pass: If an environment will not install, the dev records the refusal and sends the reason with `atm send omega-prime@hermes`.
- fail: If an environment will not install, the dev messages the team-lead and waits.

## ANC-014

Anchor: When CI is green on an sc-compose version upgrade PR to develop or main, the agent must send `atm send omega-prime@hermes` a message asking Rand Lee to complete the PR.

> regarding protected branch merge to develop AND to master, when ci is green (after preflight and release-candidate tagging is done), notify omega-prime@hermes to notify me to complete the pr.

Said by Rand Lee, phase-j sc-compose upgrade formula, 2026-10-13. Status: locked. Version 2.

Summary: Green protected PR: notify omega-prime

Scope: the upgrade-sc-compose formula (.atm-bd/formula/upgrade-sc-compose.formula.toml.j2) and every bead poured from it

Fixtures:
- pass: When CI on the develop PR is green, the lead sends omega-prime@hermes "CI green. Rand, please complete <PR link>."
- fail: When CI on the develop PR is green, the lead merges the PR to develop itself.

## ANC-015

Anchor: Every PR to develop or main must have GitHub user rand-lee added as a reviewer.

> add github/rand-lee as reviewer for any pr to develop or main

Said by Rand Lee, phase-j sc-compose upgrade formula, 2026-10-13. Status: locked. Version 2.

Summary: rand-lee reviews develop/main PRs

Scope: the upgrade-sc-compose formula (.atm-bd/formula/upgrade-sc-compose.formula.toml.j2) and every bead poured from it

Fixtures:
- pass: The develop PR is opened and rand-lee is added with gh api .../requested_reviewers -f 'reviewers[]=rand-lee'.
- fail: The PR to main is opened with no reviewer requested.

## ANC-016

Anchor: An sc-compose version upgrade must raise the repository's minimum required sc-compose version to the new version.

> I would require 1.7.1 sc-compose for floor.

Said by Rand Lee, AskUserQuestion answer 2026-10-13 on #342 floor pins, 2026-10-13. Status: locked. Version 2.

Summary: Floors move to the new version

Scope: the upgrade-sc-compose formula (.atm-bd/formula/upgrade-sc-compose.formula.toml.j2) and every bead poured from it

Fixtures:
- pass: Deliverable 1 sets SC_COMPOSE_WHEEL_PIN and the MIN_SC_COMPOSE floors and install strings to the new version 1.7.1.
- fail: Deliverable 1 bumps SC_COMPOSE_WHEEL_PIN only; the minimum-version floors stay at 1.6.1.

## ANC-017

Anchor: When an sc-compose upgrade's breaking change is minor, the upgrade must fix it and announce the fix in its PR to develop.

> If the breaking change is minor, we should fix and announce in pr.

Said by Rand Lee, AskUserQuestion answer 2026-10-13 on BREAKING CI, 2026-10-13. Status: locked. Version 2.

Summary: Minor break: fix and announce

Scope: the upgrade-sc-compose formula (.atm-bd/formula/upgrade-sc-compose.formula.toml.j2) and every bead poured from it

Fixtures:
- pass: A minor break found in B is fixed in a repo-owned file in this layer, and the PR description lists each break and its fix.
- fail: A minor break found in B is left failing and the PR to develop is opened with CI red and no mention of it.
