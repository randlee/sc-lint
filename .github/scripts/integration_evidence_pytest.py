"""Capture the count omitted from pytest's JUnit XML."""
import json
import os
from pathlib import Path


def pytest_configure(config):
    config._evidence_deselected = 0


def pytest_deselected(items):
    if items:
        items[0].config._evidence_deselected += len(items)


def pytest_sessionfinish(session, exitstatus):
    Path(os.environ["SC_LINT_EVIDENCE_DESELECTIONS"]).write_text(
        json.dumps({"deselected": session.config._evidence_deselected}), encoding="utf-8"
    )
