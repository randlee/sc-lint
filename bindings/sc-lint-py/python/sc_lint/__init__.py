"""sc-lint Python package: thin native bindings plus repository helper modules."""
from __future__ import annotations

import json

from sc_lint._native import run, version_json
from sc_lint._binary import binary_path

__all__ = ["__version__", "binary_path", "run", "version_json"]
__version__: str = json.loads(version_json())["version"]
