import importlib.util
import json
import sys
from pathlib import Path

SCRIPT = Path(__file__).parents[1] / "scripts" / "check_dependencies.py"
CLAUDE_DIR = SCRIPT.parents[3]
if str(CLAUDE_DIR) not in sys.path:
    sys.path.insert(0, str(CLAUDE_DIR))

from lib.sc_compose_dependency import SC_COMPOSE_BINDING_INSTALL


spec = importlib.util.spec_from_file_location("check_dependencies", SCRIPT)
check_dependencies = importlib.util.module_from_spec(spec)
assert spec.loader is not None
spec.loader.exec_module(check_dependencies)


def test_version_parser():
    assert check_dependencies._version("sc-compose 1.2.0") == (1, 2, 0)
    assert check_dependencies._version("oxigraph 0.5.7") == (0, 5, 7)
    assert check_dependencies._version("missing") is None


def _binding(version: str) -> dict[str, object]:
    return {"name": "sc_compose", "version": version, "ok": True}


def test_preflight_accepts_newer_compatible_cli_and_binding(monkeypatch):
    monkeypatch.setattr(check_dependencies, "_find", lambda name: Path("/bin/true"))
    monkeypatch.setattr(check_dependencies, "_run_version", lambda path: ("tool 1.8.2", "tool 1.8.2"))
    monkeypatch.setattr(check_dependencies, "_python_binding_entry", lambda: _binding("1.8.2"))
    result = check_dependencies.run()
    assert result["success"] is True
    assert result["error"] is None


def test_old_sc_compose_is_a_structured_failure(monkeypatch):
    monkeypatch.setattr(check_dependencies, "_find", lambda name: Path("/bin/true"))
    monkeypatch.setattr(check_dependencies, "_run_version", lambda path: ("sc-compose 1.0.1", "sc-compose 1.0.1"))
    monkeypatch.setattr(check_dependencies, "_python_binding_entry", lambda: _binding("1.6.1"))
    result = check_dependencies.run()
    assert result["success"] is False
    assert result["error"]["code"] == "EXECUTION.DEPENDENCY"
    assert any(item["name"] == "sc-compose" for item in result["error"]["failures"])


def test_missing_python_binding_has_actionable_install_hint(monkeypatch):
    def missing(_name):
        raise check_dependencies.importlib.metadata.PackageNotFoundError("sc-compose")

    monkeypatch.setattr(check_dependencies.importlib.metadata, "version", missing)
    result = check_dependencies._python_binding_entry()
    assert result["ok"] is False
    assert SC_COMPOSE_BINDING_INSTALL in result["error"]
    assert ">=1.6.1" in SC_COMPOSE_BINDING_INSTALL


def test_python_binding_accepts_newer_compatible_version(monkeypatch):
    monkeypatch.setitem(sys.modules, "sc_compose", object())
    monkeypatch.setattr(check_dependencies.importlib.metadata, "version", lambda _name: "1.7.1")

    result = check_dependencies._python_binding_entry()

    assert result["ok"] is True
    assert result["version"] == "1.7.1"
    assert result["required"] == "Python binding >= 1.6.1"


def test_cli_emits_json_success_contract(monkeypatch, capsys):
    monkeypatch.setattr(check_dependencies, "_find", lambda name: Path("/bin/true"))
    monkeypatch.setattr(check_dependencies, "_run_version", lambda path: ("tool 1.6.1", "tool 1.6.1"))
    monkeypatch.setattr(check_dependencies, "_python_binding_entry", lambda: _binding("1.6.1"))
    assert check_dependencies.main() == 0
    payload = json.loads(capsys.readouterr().out)
    assert payload["success"] is True
    assert payload["error"] is None
