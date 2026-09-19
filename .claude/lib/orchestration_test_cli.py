"""External-CLI policy shared by orchestration tests."""
<<<<<<< HEAD
from __future__ import annotations
import os
import shutil
import pytest

def require_dev_cli(name: str, minimum: str, install: str) -> None:
    if shutil.which(name):
        return
    if os.getenv("CI"):
=======

from __future__ import annotations

import os
import shutil

import pytest


def require_dev_cli(name: str, minimum: str, install: str) -> None:
    """Require a dev-host CLI; CI intentionally skips unavailable agent tooling."""
    if shutil.which(name):
        return
    if os.environ.get("CI"):
>>>>>>> e4b929c (test: enforce dev-host CLI policy)
        pytest.skip(f"dev-host-only check: {name} not installed in CI")
    pytest.fail(f"{name} CLI is required ({minimum}); install it with: {install}")
