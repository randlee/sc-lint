# Sprint Planning Guidelines

Use these rules when hardening sprint plans.

## Core Rules

- The sprint plan is authoritative.
- Downstream prompts may carry structured projections of sprint-plan data, but
  they must not replace or narrow the sprint plan.
- If QA cannot review directly from the sprint doc, the sprint doc is not
  hardened.

## Split Early

Split a sprint immediately when any of these are true:

- there is credible doubt that every committed deliverable can land at a
  production-ready level in the same sprint
- the sprint mixes too many closure types
- the sprint touches too many modules, boundaries, or runtime paths for clear
  ownership
- acceptance criteria would allow one deliverable to slip while the sprint
  still claims success
- the same deliverable is being planned more than once across multiple sprints

Do not preserve an overloaded sprint just to keep the sprint count low.

## Branch Stacks Are How Sprints Are Planned

Every phase plan is planned as one or more `gh stack` branch stacks. A sprint
is a layer in a stack; the stack is the unit of parallelism.

Definitions:

- a **stack** is a strictly linear chain of branches rooted on `develop`
  (`gh stack link --base develop <bottom> ... <top>`)
- each sprint owns exactly one branch and one worktree; the PR base of a
  sprint is the layer directly below it (the bottom layer's base is
  `develop`)
- a sprint may start as soon as the layer below it is committed (not merged)
- stacks merge with `gh stack merge <pr> --yes --merge`; never `gh stack
  sync` or `gh stack rebase` in this repo, because the repo rule is
  merge-forward, never rebase

Required in every phase plan:

- a `## Branch Stacks And Parallelism` section containing an ASCII diagram of
  every stack, rooted on `develop`, listing each sprint branch in order
- one stack per owner (developer or agent) at any given time; a single owner
  never works two stacks concurrently
- a parallel-vs-sequential table: for every sprint, which sprints it can run
  alongside, which it must wait for, and the exact commit event that unblocks
  it (for example "G.1 may start when G.0 is committed on its branch")
- a `### Stack protocol` subsection stating how branches are created, linked,
  merged forward, and merged to `develop`

Required in every sprint doc frontmatter:

```yaml
branch: sprint/<id>-<slug>
worktree: ../<repo>-worktrees/sprint/<id>-<slug>
stack: <stack name>
stack_base: <branch directly below, or develop>
target: develop (via stack <name>, PR base <stack_base>)
owner: <teammate>
```

A sprint doc whose branch, stack, or stack base disagrees with the phase-plan
diagram is a structural finding.

## Plan For Parallel Implementation

Optimize the phase plan for the largest number of stacks that can be
implemented at the same time without cross-stack conflicts.

- partition deliverables into disjoint path sets before ordering sprints; each
  disjoint set becomes a candidate stack
- put a deliverable in the same stack as anything it depends on; a stack must
  never depend on a commit in another stack
- when two stacks must touch the same file, name the touch point explicitly in
  the phase plan and assign exactly one reconciliation layer (the higher
  numbered sprint) to resolve it after the other stack merges; do not let both
  stacks own the reconciliation
- prefer a shorter stack that can start on day one over a longer stack that
  waits on another stack's merge
- work that is external to the repo (PRs to other repos, rollouts) belongs in
  its own stack or in a non-branch sprint clearly marked as such
- state explicitly which sprints are sequential and why; an unexplained
  sequential dependency is a finding

A plan whose sprints form one single chain when the deliverables could be
partitioned is under-parallelized and must be restructured before hardening
continues.

## Sprint Doc Shape

Each sprint doc should have one authoritative list for:

- deliverables
- acceptance criteria
- paths to delete, when applicable
- required validation

Do not restate the same checklist item in multiple sections with different
wording.

## Production-Ready Expectation

Every listed deliverable must be expected to land at a production-ready level
for the scope that sprint claims.

Do not allow:

- shape-only completion
- test-only completion
- boundary-only completion when runtime behavior is still open
- silent carry-forward of a committed deliverable

If a sprint intentionally does not close something, state that explicitly under
non-closure or out-of-scope sections.

## Code Samples

Important traits, enums, protocol types, interfaces, and boundary contracts
must have explicit code samples or signatures in the sprint doc when prose
alone would leave implementation choices open.

## QA Consumption

Sprint docs must be short and structured enough that:

- `req-qa` can enumerate deliverables and acceptance criteria directly
- `arch-qa` can identify structural gate artifacts directly
- `quality-mgr` can route QA without copying scope by hand

If that is not true, shorten or tighten the sprint doc instead of adding more
prompt ceremony.

## Finding Classification

Classify each finding as either structural or wording before assigning
severity.

Structural findings:
- missing acceptance or validation gate
- incorrect command, test name, or grep gate
- uncovered call site, file, module, or runtime path
- missing type, trait, function, boundary contract, or ADR
- false-closure wording that hides still-open runtime or boundary work
- sprint branch, stack, or stack base that disagrees with the phase-plan stack diagram
- a sprint whose start condition or cross-stack touch point is unstated
- a plan that serializes sprints whose deliverables could be partitioned into parallel stacks

Structural findings always remain in the main `findings` array and must be
rated `Blocking` or `Important` when they affect implementability or closure.

Wording findings:
- prose ambiguity that does not change scope or closure meaning
- formatting cleanup
- non-normative wording polish

Wording findings belong in `minor_wording` and do not fail the round unless
the reviewer marks them `affects_ac: true`.
