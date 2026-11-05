"""Locate the sc-lint checkout from an installed test module's location."""
from pathlib import Path
from typing import Optional


def source_checkout(module_file: str) -> Optional[Path]:
    path = Path(module_file).resolve()
    for root in path.parents:
        if (root / "bindings/sc-lint-py/python/sc_lint/tests" == path.parent
                and (root / "Cargo.toml").is_file()
                and (root / "bindings/sc-lint-py/pyproject.toml").is_file()
                and (root / "Justfile").is_file()):
            return root
    return None
