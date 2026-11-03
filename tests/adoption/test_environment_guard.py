"""The adoption suite rejects environment leaks after fixture cleanup."""
import os
from pathlib import Path
import sys

import pytest
from install_helpers import run_subprocess


@pytest.mark.parametrize("body,leaks", [
    ('os.environ["SC_LINT_GUARD_ADDED"] = "leak"', True),
    ('os.environ["SC_LINT_GUARD_EXISTING"] = "changed"', True),
    ('del os.environ["SC_LINT_GUARD_EXISTING"]', True),
    ('monkeypatch.setenv("SC_LINT_GUARD_EXISTING", "temporary")', False),
])
def test_environment_guard_detects_leaks_after_cleanup(tmp_path, body, leaks):
    # Each probe runs alone: its result cannot depend on neighboring tests.
    (tmp_path / "conftest.py").write_text(Path(__file__).with_name("conftest.py").read_text())
    (tmp_path / "test_probe.py").write_text(
        "import os\ndef test_probe(monkeypatch):\n    " + body + "\n"
    )
    environment = {**os.environ, "SC_LINT_GUARD_EXISTING": "original"}
    environment.pop("SC_LINT_GUARD_ADDED", None)
    result = run_subprocess(
        [sys.executable, "-m", "pytest", "-q", "test_probe.py"],
        cwd=tmp_path, env=environment, text=True, capture_output=True,
    )
    output = result.stdout + result.stderr
    assert result.returncode == (1 if leaks else 0), output
    assert ("test leaked environment variables" in output) == leaks, output
