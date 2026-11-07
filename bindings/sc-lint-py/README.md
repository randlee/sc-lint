# sc-lint for Python

`sc-lint` on PyPI provides Python bindings for the Rust `sc-lint` CLI and the
repository-maintenance helper modules used by the project. It does not install
the Rust CLI executables. Install those separately with Homebrew or from
[GitHub Releases](https://github.com/randlee/sc-lint/releases).

Current release: **0.8.6**. Python 3.9 or newer is required (3.11 or newer on
Windows ARM64).

## Install

```bash
python -m pip install sc-lint
```

The Python package includes the native extension and the `sc_lint` Python
package. Its runtime dependencies are `codespell>=2.2` for the repository
spell-check helper and `packaging>=22` for Python dependency checks. The
package does not expose a `sc-lint` console script.

## Python API

The native module exposes three public helpers:

```python
import sc_lint

# JSON string with the bound sc-lint version.
version = sc_lint.version_json()

# Run the CLI in-process; argv[0] is the program name.
exit_code = sc_lint.run(["sc-lint", "version"])

# Resolve the product executable from SC_LINT_BIN, the managed install
# directory, or PATH. Raises FileNotFoundError if no executable is found.
binary = sc_lint.binary_path()
```

`run(argv)` returns the CLI exit code instead of terminating the Python
interpreter. It does not add a command-line entry point to the wheel.

The native extension also provides `toml_loads(text)`, which the helper modules
use so TOML is parsed by the same Rust parser as the CLI on every supported
Python version.

## Links

- [Python bindings contract](https://github.com/randlee/sc-lint/blob/main/docs/sc-lint/python-bindings.md)
- [sc-lint operator manual](https://github.com/randlee/sc-lint/tree/main/docs-bundle)
- [Source repository](https://github.com/randlee/sc-lint)
- [Issue tracker](https://github.com/randlee/sc-lint/issues)
- [Changelog](https://github.com/randlee/sc-lint/blob/main/CHANGELOG.md)
- [MIT license](https://github.com/randlee/sc-lint/blob/main/bindings/sc-lint-py/LICENSE)
