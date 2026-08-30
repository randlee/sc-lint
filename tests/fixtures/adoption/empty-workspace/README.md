# empty-workspace

Consumer fixture for `scripts/release_smoke.py`: the released `sc-lint`
archive is installed here (in a temporary copy) via `sc-lint init --just`, and
`just setup`, `just lint`, `just test`, and the bootstrap `upgrade --check`
path must all pass with `SC_LINT_SOURCE_ROOT` unset and no `.just/` directory.
