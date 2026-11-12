"""Tests for translating pytest evidence without inventing results."""
import importlib.util
import json
import os
import subprocess
import sys
from pathlib import Path

import pytest

SCRIPTS = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("integration_evidence", SCRIPTS / "integration_evidence.py")
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


def test_normalize_reports_each_outcome_and_deselection(tmp_path):
    xml = tmp_path / "results.xml"
    xml.write_text('''<testsuites><testsuite><testcase classname="a" name="ok"/><testcase classname="a" name="bad"><failure message="bad &amp; worse"/></testcase><testcase classname="a" name="skip"><skipped message="reason"/></testcase><testcase classname="a" name="error"><error message="setup"/></testcase></testsuite></testsuites>''')
    result = module.normalize(xml, 7, "0.8.8", "abc", "2026-01-01T00:00:00Z")
    assert result["counts"] == {"passed": 1, "failed": 1, "error": 1, "skipped": 1, "deselected": 7}
    assert result["tests"][1] == {"name": "a::bad", "outcome": "failed", "detail": "bad & worse"}


def test_empty_results_refused(tmp_path):
    xml = tmp_path / "empty.xml"
    xml.write_text("<testsuites/>")
    with pytest.raises(ValueError, match="no test cases"):
        module.normalize(xml, 0, "0.8.8", "abc", "now")


def test_real_pytest_reports_deselected_items(tmp_path, monkeypatch):
    (tmp_path / "test_sample.py").write_text("def test_keep(): pass\ndef test_drop(): pass\n")
    result_path = tmp_path / "deselected.json"
    monkeypatch.setenv("PYTEST_ADDOPTS", "--not-a-real-option")
    monkeypatch.setenv("PYTEST_PLUGINS", "not_a_real_plugin")
    env = os.environ.copy()
    env.pop("PYTEST_ADDOPTS", None)
    env.pop("PYTEST_PLUGINS", None)
    env["PYTEST_DISABLE_PLUGIN_AUTOLOAD"] = "1"
    env["PYTHONPATH"] = str(SCRIPTS)
    env["SC_LINT_EVIDENCE_DESELECTIONS"] = str(result_path)
    result = subprocess.run(
        [sys.executable, "-m", "pytest", "-q", "--rootdir=.", "-p", "integration_evidence_pytest", "-k", "keep", "test_sample.py"],
        cwd=tmp_path, env=env, capture_output=True, text=True, check=False,
    )
    assert result.returncode == 0, result.stdout + result.stderr
    assert json.loads(result_path.read_text()) == {"deselected": 1}


def test_strict_panel_render_escapes_test_content(tmp_path):
    import xml.etree.ElementTree as ET

    data = {"version": "0.8.8", "commit": "abc", "timestamp": "now", "exit_code": 1, "status": "FAIL", "source_status": [], "counts": {"passed": 0, "failed": 1, "error": 0, "skipped": 0, "deselected": 0}, "tests": [{"name": "a<&>", "outcome": "failed", "detail": "bad <script> & more"}]}
    variables = tmp_path / "result.json"
    variables.write_text(json.dumps(data))
    output = tmp_path / "panel.xhtml"
    module.render("panel.xhtml.j2", variables, output)
    root = ET.parse(output).getroot()
    assert "a<&>" in "".join(root.itertext())
    assert "bad <script> & more" in "".join(root.itertext())
    assert "FAIL" in "".join(root.itertext())
