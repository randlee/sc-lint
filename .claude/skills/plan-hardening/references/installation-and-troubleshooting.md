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
