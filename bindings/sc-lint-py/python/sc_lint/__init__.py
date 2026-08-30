"""sc-lint Python package: thin native bindings plus repository helper modules."""
from __future__ import annotations

import json

from sc_lint._binary import binary_path

try:
    from sc_lint._native import run, version_json
except ImportError:  # source checkout without a built extension module
    _NATIVE_MISSING = "sc_lint._native is not built; install the sc-lint wheel or run `maturin develop`"

    def version_json() -> str:  # type: ignore[misc]
        raise RuntimeError(_NATIVE_MISSING)

    def run(argv: list[str]) -> int:  # type: ignore[misc]
        raise RuntimeError(_NATIVE_MISSING)

    __version__ = "0.0.0+source"
else:
    __version__ = json.loads(version_json())["version"]

__all__ = ["__version__", "binary_path", "run", "version_json"]
