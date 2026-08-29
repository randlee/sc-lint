# Configure apply and Just fixtures

These are consumer-owned root-Justfile inputs. The external configure tests
copy them into temporary Rust repositories before planning and applying.

- `existing-just` exercises comments, a consumer import, and an unrelated
  consumer recipe. Its bytes are converted to CRLF by the test to verify that
  insertion preserves Windows line endings.
- `malformed-marker`, `duplicate-marker`, `moved-marker`, and
  `modified-marker` must be no-write conflicts.
- `reserved-*` covers every product-owned recipe name. None may be shadowed
  through a Just import.
- `legacy-near-miss` is a similarly named old-action path with different
  bytes. It must never yield a removal operation or a deletion during apply.

The generated managed Justfile is checked against its Windows bootstrap form
(`& .\\.sc-lint\\bootstrap.ps1`) by the same external test; this fixture set
therefore has no platform-specific executable file to maintain.
