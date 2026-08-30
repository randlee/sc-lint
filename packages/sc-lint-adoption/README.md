# sc-lint adoption kit

Install a generic, drift-checked `sc-lint` consumer contract with:

```text
python3 plugins/sc-lint/install.py --input install.json .
```

Use `--dry-run` to report drift without writing.

Kit-rendered steps name only shipped binaries or `sc_lint` modules; profile and
test-layer commands are consumer-owned and rendered verbatim.
