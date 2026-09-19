# Function-Length Lint Plan

## Goal

Keep production Rust functions reviewable by enforcing a code-only line budget
through the stable `sc-lint lint function-length` command.

## Policy

- Count non-whitespace, non-comment Rust code lines from a function signature
  through its closing brace.
- Emit an advisory at 70 code lines and fail at 80 code lines, including
  exactly 80.
- Exempt test-only paths and functions directly marked by a test attribute.
- Do not use Git-diff or baseline grandfathering. A necessary exception is
  explicit in the source: `#[sc_lint(function_length.fail_at(N))]`.
- The exception has one positive integer limit, may be combined with other
  `sc_lint` directives, and is validated by the shared Rust attribute parser.

## Delivery and acceptance

1. Expose the command through the CLI, Python adapter, `just lint`, and
   findings artifacts.
2. Keep Python source counting as the runtime implementation, with Rust syn
   parsing as the compile-time attribute authority.
3. Cover code-only counting, threshold edges, test exemptions, malformed and
   duplicate exceptions, and combined/multiline attributes in tests.
4. Refactor newly discovered oversized production functions below the default
   limit instead of using broad exceptions.
5. Validate the complete aggregate gates: `just lint` and `just test`.

## Traceability

This plan implements `REQ-PRODUCT-006AB`; the architecture split is documented
in [docs/architecture.md](../architecture.md).
