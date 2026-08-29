# Wyvern release-artifact smoke fixture

This fixture drives only the public `wyvern-host` HTTP protocol. It does not
implement wizard navigation in Python: the release artifact owns only
history/stack bookkeeping, terminal semantics, and the session timeout. The
client supplies page sequencing by choosing each next page ID and constructing
its descriptor on every `navigate` call; this harness does that explicitly for
the authoritative ten-page F.3a journey. The harness sends the same JSON that
a page would send with `fetch` and asserts the release's HTTP responses and
stdout result.

This division of responsibility is a launcher-design finding for F.3d: its
client adapter must own page sequencing and descriptor construction; it must
not assume that the released host selects the next page.

## Run

Extract a checksum-verified v0.6.0 archive, then pass its native binary:

```sh
python3 run_smoke.py --binary /path/to/wyvern
```

The command is local-only after extraction. It binds loopback, uses
`--viewer none`, and never contacts a browser, network service, or source
checkout. The timeout case intentionally waits for the released headless idle
budget (30 seconds); this is a real timeout assertion, not process cleanup.

The JSON report lists every F.3a host requirement and is stable apart from the
ephemeral loopback port. `--output results.json` writes the normalized report
for CI/archive review.
