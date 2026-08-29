# Released Wyvern capability matrix

This is the F.3b qualification record for the immutable Wyvern `v0.6.0`
release. The smoke runner is an HTTP client of the released host; it does not
implement a wizard, emulate history, or substitute for a missing host feature.

## Release and checksum record

The release is `v0.6.0`, published 2026-08-28, from the upstream
[Wyvern v0.6.0 release](https://github.com/randlee/wyvern/releases/tag/v0.6.0).
Each SHA-256 below was checked independently with `shasum -a 256` and then
verified again with `shasum -a 256 -c checksums.txt` after a fresh download.

| Host | Exact archive | SHA-256 | Launch command |
| --- | --- | --- | --- |
| macOS arm64 | `wyvern_0.6.0_aarch64-apple-darwin.tar.gz` | `b5f5b986868d65b37d39966d7e9fa0c2bb6fd35fd0675397cbe3b4f77dc6b9dc` | `wyvern wizard.json --viewer none` |
| macOS x86_64 | `wyvern_0.6.0_x86_64-apple-darwin.tar.gz` | `325880ca0edf0d2e0afda4cf2ef4818f1dc2c3d7fe3d5811cf55a3893bc43cc6` | `wyvern wizard.json --viewer none` |
| Linux x86_64 | `wyvern_0.6.0_x86_64-unknown-linux-gnu.tar.gz` | `549d7898e717475cbb0a1412cb768f6d0cb08053acc461dccb8e4fe66854315c` | `wyvern wizard.json --viewer none` |
| Windows x86_64 | `wyvern_0.6.0_x86_64-pc-windows-msvc.zip` | `d330f7f7903a738640613a497975b88fcdeab3fbb03ca4b27619dbc138ec0835` | `wyvern.exe wizard.json --viewer none` |

Every row must print exactly `wyvern 0.6.0` for `wyvern --version`. The
repository workflow [wyvern-f3b-smoke.yml](../../../.github/workflows/wyvern-f3b-smoke.yml)
downloads the named release asset, verifies its matching line in `checksums.txt`,
and runs the same fixture on native Linux, macOS arm64, macOS x86_64, and
Windows runners. This keeps Windows evidence reproducible even when the
qualification operator is not on a Windows host.

## Protocol and fixture

The qualified protocol version is `wizard-http-v1`. With `--viewer none`, the
host publishes `WYVERN_DIALOG_URL=...` on stderr, serves the loopback HTTP
endpoints `/api/wizard/state`, `/api/wizard/navigate`, and
`/api/wizard/finish`, and writes one terminal JSON object to stdout. The
fixture starts the released binary from its extracted archive and talks only
to that loopback URL. It does not require a browser, network access after the
asset download, or a Wyvern source checkout.

`wizard.json` supplies the single initial descriptor. Every subsequent
descriptor is supplied by the client in `navigate.next`; the v0.6.0 protocol
has no `pages` array and no declarative `when`/`pointer`/`equals` branch
conditions:

```json
{
  "action": "next",
  "page_id": "baseline",
  "data": {"page": "baseline", "step": 2},
  "next": {
    "id": "baseline",
    "title": "Baseline",
    "html": "pages/baseline.html"
  }
}
```

Wyvern v0.6.0's wire schema uses one initial `page` descriptor and accepts
subsequent descriptors in `navigate.next`; page sequencing is client-supplied
while the host preserves history/stack bookkeeping and terminal semantics.

Run one platform qualification locally with:

```sh
python3 docs/sc-lint/configure-wizard-fixtures/wyvern-smoke/run_smoke.py \
  --binary /path/to/extracted/bin/wyvern \
  --output wyvern-f3b-results.json
```

The runner requires `wyvern 0.6.0`, records `protocol_version`, and fails on
any unexpected response. The timeout case intentionally waits for the
released 30-second idle timeout and records the non-zero exit plus
`SESSION_TIMEOUT_ERROR`; it is not a test-process cleanup timeout.

## Host-capability verdict

| F.3a requirement | Fixture assertion | Verdict |
| --- | --- | --- |
| Multi-page descriptors and forward navigation | `forward_back_restore_branch` submits all ten stable F.3a descriptors (overview through final-review) and asserts their ordered IDs | PASS |
| Browser-history back navigation | The same case returns from final-review through the full stack to overview | PASS |
| Opaque page-data restoration | Back from final-review to CI integration restores the submitted final-review `page_data` exactly | PASS |
| Changed-branch navigation | A new overview → final-review branch leaves only overview in history; all stale forward frames are removed | PASS |
| First-page back disabled | `action: back` on the initial page returns HTTP 400 and `WIZARD_AT_FIRST_PAGE` | PASS |
| Finish result delivery | `finish_full_stack` returns `button: finish`, confirmation data, and all ten ordered F.3a frames | PASS |
| Cancel result delivery | `cancel` returns `button: cancel`, empty data, and an empty stack | PASS |
| Dismiss result delivery | `dismissed` returns `button: dismissed`, empty data, and the visited stack | PASS |
| Idle timeout | An undriven session exits non-zero after 30 seconds with stable `SESSION_TIMEOUT_ERROR` evidence | PASS |
| Local-only host boundary | Every request is loopback HTTP; no browser or external network is used by the fixture | PASS |
| Deterministic headless execution | The same Python runner and fixture produce normalized JSON on all four release artifacts | PASS |

The release therefore meets the F.3a host contract on Linux x86_64, macOS
arm64, macOS x86_64, and Windows x86_64. Any future release replacement must
repeat checksum verification and this complete matrix; a missing platform or
feature is a blocking upstream finding, never something for sc-lint to emulate.
