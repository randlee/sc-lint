"""The local ARM smoke recipe must not manage a shared Colima VM."""

import os
from pathlib import Path
import subprocess

import pytest

ROOT = Path(__file__).resolve().parents[3]
SCRIPT = ROOT / ".just/test_linux_arm_release.sh"


@pytest.mark.parametrize("running", [True, False])
def test_recipe_requires_running_colima_without_changing_its_lifecycle(tmp_path, running):
    bindir = tmp_path / "bin"
    bindir.mkdir()
    log = tmp_path / "calls"
    colima = bindir / "colima"
    colima.write_text('#!/bin/sh\nprintf "colima %s\\n" "$*" >> "$CALL_LOG"\nexit "$COLIMA_STATUS"\n')
    colima.chmod(0o755)
    docker = bindir / "docker"
    docker.write_text('#!/bin/sh\nprintf "docker %s\\n" "$*" >> "$CALL_LOG"\nexit 0\n')
    docker.chmod(0o755)
    atm = bindir / "atm"
    atm.write_text('#!/bin/sh\nprintf "atm %s\\n" "$*" >> "$CALL_LOG"\nexit 0\n')
    atm.chmod(0o755)
    archive = tmp_path / "archive.tar.gz"
    wheel = tmp_path / "package.whl"
    archive.touch()
    wheel.touch()
    result = subprocess.run(
        ["bash", str(SCRIPT), str(archive), str(wheel)],
        env={**os.environ, "PATH": f"{bindir}{os.pathsep}{os.environ['PATH']}",
             "CALL_LOG": str(log), "COLIMA_STATUS": "0" if running else "1"},
        text=True, capture_output=True, check=False, timeout=30,
    )
    calls = log.read_text().splitlines()
    assert [line for line in calls if line.startswith("colima ")] == ["colima status"]
    assert not any(line.startswith("atm ") for line in calls)
    assert result.returncode == (0 if running else 1), result.stderr
    if running:
        assert sum(" run " in line for line in calls) == 2
    else:
        assert "must already be running" in result.stderr
        assert not any(line.startswith("docker ") for line in calls)
