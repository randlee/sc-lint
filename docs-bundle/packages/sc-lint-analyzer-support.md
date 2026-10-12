# sc-lint-analyzer-support

`sc-lint-analyzer-support` is the internal shared library for AST source
discovery, scope classification, concise text-report rendering, and the
rule-neutral parser for analyzer configuration tables and `--rule-filter`
selections used by the analyzers.

It has no standalone command or lint rules. Install it only when building an
integration that needs these Rust support APIs:

```toml
sc-lint-analyzer-support = "0.7.2"
```

The product-facing commands remain:

```sh
sc-lint lint sc-portability
sc-lint lint sc-runtime
```

See [sc-lint-portability](./sc-lint-portability.md) and
[sc-lint-runtime](./sc-lint-runtime.md) for user-facing analyzer guidance.

