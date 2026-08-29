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

## Architecting Stacks For A Phase

Do this before writing any sprint doc. The output is the phase plan's
`## Branch Stacks And Parallelism` section.

1. **List deliverables, not sprints.** Enumerate every committed deliverable
   with the repo paths it creates or modifies.
2. **Partition by path set.** Group deliverables whose path sets overlap.
   Each group with no overlap against any other group is a candidate stack.
   Overlapping groups must be in the same stack or joined by exactly one
   named reconciliation layer.
3. **Order within a stack by dependency.** Foundational work (types, schemas,
   bootstrap, CI actions) is the bottom layer; consumers of it are higher
   layers. A layer may only depend on layers below it in the same stack.
4. **One sprint per layer.** Each layer becomes one sprint doc, one branch,
   one worktree, one PR. Split a layer if it fails the Split Early rules.
5. **Root every stack on `develop`.** The bottom layer's base is `develop`
   unless the phase has a planning branch that must land first, in which case
   that planning branch is the base and is itself the bottom of the stack.
6. **Assign one owner per stack.** The number of stacks that can run at once
   is the number of available owners; do not plan more concurrent stacks than
   owners, and do not give one owner two concurrent stacks.
7. **Keep stacks short.** Prefer 2–3 layers per stack. A stack longer than
   four layers is a sign that a second stack should be split off.
8. **Name cross-stack touch points.** For every file two stacks must both
   change, state which stack merges first and which single higher layer in
   the other stack reconciles after that merge.
9. **Put external-repo work in non-branch sprints.** Consumer-repo PRs and
   rollouts are sprints with `branch: n/a`, sequenced after the stack whose
   release they depend on.
10. **Draw it.** Render the ASCII diagram and the parallel-vs-sequential
    table from the result; the diagram is normative and the sprint docs must
    match it.

Reference shape for a two-stack phase:

```text
develop
├── Stack A (owner: clint)
│   └── sprint/X.0-foundation        PR base: develop
│       └── sprint/X.1-kit           PR base: sprint/X.0-foundation
│           └── sprint/X.2-skill     PR base: sprint/X.1-kit
└── Stack B (owner: cfast)           runs in parallel with Stack A from day one
    └── sprint/X.3a-bindings         PR base: develop
        └── sprint/X.3b-release      PR base: sprint/X.3a-bindings
                                     reconciles touch point with A after A merges
```

## Plan For Parallel Implementation

Parallel sprints are the default, not an optimization. The phase plan must
explicitly define which sprints run in parallel; a plan that does not name
its parallel sprints is not hardened. Optimize for the largest number of
stacks that can be implemented at the same time without cross-stack
conflicts.

The parallel-vs-sequential table is mandatory and has one row per sprint:

| Sprint | Stack | Runs in parallel with | Waits for | Unblocked when |
|--------|-------|-----------------------|-----------|----------------|

"Unblocked when" names a commit event on a specific branch (for example
"X.0 committed on sprint/X.0-foundation"), never a merge to `develop`, unless
the dependency is genuinely on a released artifact.

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

### Sprint start conditions

Serial sprint plans are the primary cause of slow phases. Apply these rules
when deciding whether a sprint may start:

- a sprint may start the moment the layer below it is **committed** on its
  branch; it does not wait for that layer's CI, QA, review, or merge
- CI and QA of a lower layer run concurrently with implementation of the
  layer above; a failure below is fixed in the lower branch and merged
  forward, not by holding the upper sprint
- a sprint in a different stack never waits for anything in this stack unless
  the phase plan names the touch point
- "wait for CI green" or "wait for merge to `develop`" is a valid start
  condition only when the sprint consumes a **released artifact** (a
  published crate, wheel, tag, or another repo's PR); the plan must say which
  artifact
- the coordinator (`team-lead`) dispatches the next sprint on commit, not on
  CI pass; withholding dispatch until CI passes is a protocol violation, not
  caution

A plan where every sprint's start condition is the previous sprint's merge
or CI is a `STACK-SHAPE` finding and must be restructured.

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
