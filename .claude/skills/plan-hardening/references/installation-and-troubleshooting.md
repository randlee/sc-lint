# Installation And Troubleshooting

## gh stack

`plan-hardening` requires the GitHub CLI (`gh` v2.0+) with the `gh-stack`
extension, because sprints are planned as `gh stack` layers and the stack
protocol in every phase plan is executed with it.

Install:

```bash
gh extension install github/gh-stack
git config rerere.enabled true
git config remote.pushDefault origin
```

Verify:

```bash
gh --version && gh stack --version
```

Repo rule: use `gh stack link --base develop ...` and
`gh stack merge <pr> --yes --merge` only. Never run `gh stack sync` or
`gh stack rebase` in this repo (merge-forward, never rebase).

If `gh` is on PATH interactively but not here, Claude Code's bash inherits a
minimal PATH; check `/opt/homebrew/bin/gh` and `$HOME/.local/bin/gh` and
export the directory for the session.

## Minimum Version

- `gh` ≥ 2.0.0
- `gh-stack` ≥ 0.1.0 (`gh stack --version`); the skill uses only `link`,
  `merge`, `view --json`, and `unstack --local`, all present in 0.1.0

If the reported version is lower, upgrade with `gh extension upgrade
github/gh-stack` and re-run the Step 0 check before continuing.

## Validation

Run from any worktree of the repository:

```bash
gh stack --version
gh stack view --json >/dev/null 2>&1 || echo "no stack on this branch (expected on develop)"
git config --get rerere.enabled       # must print true
git config --get remote.pushDefault   # must print origin
```

All four succeed → the environment is ready. A missing config line is fixed
with the `git config` commands under "gh stack" above.

## Known Issues

- `gh stack view` without `--json` opens a TUI and hangs an agent; always
  pass `--json`.
- `gh stack checkout <pr>` when a different local stack already covers those
  branches prompts interactively; run `gh stack unstack --local` first.
- A branch that belongs to two stacks makes every command exit 6; check out
  a non-shared branch before retrying.
- `gh stack sync` aborts silently (`ℹ Sync aborted`) when local and remote
  stacks diverge — irrelevant here because `sync` is forbidden in this repo;
  merge forward instead.
- GitHub API rate limiting surfaces as `gh: HTTP 403`; wait for the reset
  reported by `gh api rate_limit` rather than retrying in a loop.
