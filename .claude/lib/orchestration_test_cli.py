"""External-CLI policy shared by orchestration tests."""
from __future__ import annotations
import os
import shutil
import pytest

def require_dev_cli(name: str, minimum: str, install: str) -> None:
    if shutil.which(name):
        return
    if os.getenv("CI"):
        pytest.skip(f"dev-host-only check: {name} not installed in CI")
    pytest.fail(f"{name} CLI is required ({minimum}); install it with: {install}")
