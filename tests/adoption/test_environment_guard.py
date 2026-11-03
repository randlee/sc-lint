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
def test_environment_guard_detects_leaks_after_cleanup(tmp_path, monkeypatch, body, leaks):
    # Host pytest/coverage configuration must not affect the nested runner.
    inherited = {
        "PYTEST_ADDOPTS": "--invalid-host-option",
        "PYTEST_PLUGINS": "nonexistent_host_plugin",
        "PYTHONPATH": str(tmp_path / "host-python-path"),
        "PYTHONHOME": str(tmp_path / "invalid-python-home"),
        "COV_CORE_SOURCE": "host-coverage-source",
        "COVERAGE_PROCESS_START": str(tmp_path / "host-coverage-config"),
    }
    for key, value in inherited.items():
        monkeypatch.setenv(key, value)
    # Each probe runs alone: its result cannot depend on neighboring tests.
    (tmp_path / "conftest.py").write_text(Path(__file__).with_name("conftest.py").read_text())
    (tmp_path / "test_probe.py").write_text(
        "import os\ndef test_probe(monkeypatch):\n"
        f"    assert not set({list(inherited)!r}) & os.environ.keys()\n"
        "    assert os.environ['PYTEST_DISABLE_PLUGIN_AUTOLOAD'] == '1'\n"
        "    " + body + "\n"
    )
    environment = {
        key: value for key, value in os.environ.items()
        if not key.startswith(("PYTEST_", "COV_", "COVERAGE_"))
        and key not in {"PYTHONPATH", "PYTHONHOME", "SC_LINT_GUARD_ADDED"}
    }
    environment.update(PYTEST_DISABLE_PLUGIN_AUTOLOAD="1", SC_LINT_GUARD_EXISTING="original")
    result = run_subprocess(
        [sys.executable, "-m", "pytest", "-q", "test_probe.py"],
        cwd=tmp_path, env=environment, text=True, capture_output=True,
    )
    output = result.stdout + result.stderr
    assert result.returncode == (1 if leaks else 0), output
    assert ("test leaked environment variables" in output) == leaks, output
