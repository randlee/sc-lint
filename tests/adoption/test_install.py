"""Integration tests for the vendorable adoption kit."""
from __future__ import annotations

import ast
import hashlib
import importlib.util
import json
import os
import re
import runpy
import shutil
import subprocess
import sys
import tempfile
from textwrap import dedent
from pathlib import Path
from typing import Any

import pytest
from conftest import ProductArtifacts
from install_helpers import FIXTURES
from install_helpers import INSTALL
from install_helpers import KIT
from install_helpers import ROOT
from install_helpers import SUBPROCESS_TIMEOUT_SECONDS
from install_helpers import WORKED_EXAMPLE_TIMEOUT_SECONDS
from install_helpers import copy_fixture
from install_helpers import run
from install_helpers import run_subprocess
from install_helpers import write_input


def test_run_subprocess_stdin_is_closed(monkeypatch: pytest.MonkeyPatch) -> None:
    command = ["python3", "-c", "import sys; print(repr(sys.stdin.read()))"]
    actual_run = subprocess.run
    observed_options: dict[str, Any] = {}

    def record_options(*arguments: Any, **options: Any) -> subprocess.CompletedProcess[str]:
        observed_options.update(options)
        return actual_run(*arguments, **options)

    monkeypatch.setattr(subprocess, "run", record_options)
    result = run_subprocess(command, text=True, capture_output=True, check=False)

    assert result.returncode == 0
    assert result.stdout == "''\n"
    assert observed_options["stdin"] is subprocess.DEVNULL


def test_run_subprocess_timeout_reports_command_and_output(monkeypatch: pytest.MonkeyPatch) -> None:
    command = ["python3", "-c", "print('ready')"]

    def raise_timeout(*arguments: Any, **options: Any) -> subprocess.CompletedProcess[str]:
        raise subprocess.TimeoutExpired(
            arguments[0], options["timeout"], output="ready\n", stderr="warning\n"
        )

    monkeypatch.setattr(subprocess, "run", raise_timeout)
    with pytest.raises(AssertionError) as error:
        run_subprocess(command, timeout=0.1, text=True, capture_output=True, check=False)

    message = str(error.value)
    assert "timed out after 0.1s" in message
    assert repr(command) in message
    assert "ready" in message
    assert "warning" in message


def test_empty_workspace_installs_converges_and_exposes_consumer_recipes() -> None:
    consumer = copy_fixture("empty-workspace")
    install = run("--input", str(FIXTURES / "install.json"), str(consumer))
    assert install.returncode == 0, install.stderr
    clean = run("--dry-run", "--input", str(FIXTURES / "install.json"), str(consumer))
    assert clean.returncode == 0, clean.stdout
    assert (consumer / ".sc-lint" / "bootstrap.py").is_file()
    recipes = run_subprocess(
        ["just", "--list"], cwd=consumer, text=True, capture_output=True, check=False
    )
    assert recipes.returncode == 0, recipes.stderr
    for recipe in ("default", "setup", "lint", "test", "upgrade"):
        assert f"    {recipe}" in recipes.stdout


def test_retired_managed_bootstraps_are_removed_only_if_unmodified() -> None:
    consumer = copy_fixture("empty-workspace")
    args = ("--input", str(FIXTURES / "install.json"), str(consumer))
    assert run(*args).returncode == 0
    manifest_path = consumer / ".sc-lint" / "installed.json"
    manifest = json.loads(manifest_path.read_text())
    for name in (".sc-lint/bootstrap", ".sc-lint/bootstrap.ps1"):
        path = consumer / name
        payload = ("retired " + name + "\n").encode()
        path.write_bytes(payload)
        manifest["managed"][name] = hashlib.sha256(payload).hexdigest()
    manifest_path.write_text(json.dumps(manifest))
    before = {name: (consumer / name).read_bytes() for name in manifest["managed"] if (consumer / name).is_file()}
    manifest_before = manifest_path.read_bytes()

    dry_run = run("--dry-run", *args)
    assert dry_run.returncode == 1, dry_run.stderr
    assert ".sc-lint/bootstrap" in dry_run.stdout
    assert (consumer / ".sc-lint/bootstrap").is_file()
    assert (consumer / ".sc-lint/bootstrap.ps1").is_file()
    assert manifest_path.read_bytes() == manifest_before
    assert all((consumer / name).read_bytes() == content for name, content in before.items())

    modified = consumer / ".sc-lint/bootstrap"
    modified.write_text("consumer modification\n")
    conflict = run(*args)
    assert conflict.returncode == 2
    assert "conflict: .sc-lint/bootstrap was modified outside sc-lint" in conflict.stderr
    assert modified.read_text() == "consumer modification\n"
    assert (consumer / ".sc-lint/bootstrap.ps1").is_file()

    modified.write_text("retired .sc-lint/bootstrap\n")
    installed = run(*args)
    assert installed.returncode == 0, installed.stderr
    assert not modified.exists()
    assert not (consumer / ".sc-lint/bootstrap.ps1").exists()
    assert (consumer / ".sc-lint/bootstrap.py").is_file()


def test_established_justfile_preserves_unmanaged_recipe() -> None:
    consumer = copy_fixture("established-workspace")
    before = (consumer / "Justfile").read_text()
    result = run("--input", str(FIXTURES / "install.json"), str(consumer))
    assert result.returncode == 0, result.stderr
    rendered = (consumer / "Justfile").read_text()
    assert before in rendered
    assert rendered.count("# >>> sc-lint managed integration >>>") == 1


def test_established_test_recipe_is_a_declared_layer_after_migration() -> None:
    consumer = copy_fixture("established-workspace")
    input_path = consumer / "install.json"
    result = run("--input", str(input_path), str(consumer))
    assert result.returncode == 0, result.stderr
    config = (consumer / "sc-lint.toml").read_text()
    assert 'name = "integration"' in config
    assert 'command = ["cargo", "test", "--test", "integration"]' in config
    justfile = (consumer / "Justfile").read_text()
    assert "test-integration:\n    just test integration" in justfile
    assert "hello:" in justfile


def test_dry_run_reports_modified_managed_asset() -> None:
    consumer = copy_fixture("empty-workspace")
    assert run("--input", str(FIXTURES / "install.json"), str(consumer)).returncode == 0
    managed = consumer / ".sc-lint" / "justfile"
    managed.write_text(managed.read_text() + "# consumer edit\n")
    result = run("--dry-run", "--input", str(FIXTURES / "install.json"), str(consumer))
    assert result.returncode == 1
    assert str(managed) in result.stdout


def test_install_rejects_a_modified_managed_asset_without_writing() -> None:
    consumer = copy_fixture("empty-workspace")
    assert run("--input", str(FIXTURES / "install.json"), str(consumer)).returncode == 0
    managed = consumer / ".sc-lint" / "justfile"
    managed.write_text(managed.read_text() + "# consumer edit\n")
    before = managed.read_bytes()
    result = run("--input", str(FIXTURES / "install.json"), str(consumer))
    assert result.returncode == 2
    assert str(managed) in result.stderr
    assert managed.read_bytes() == before


def test_first_install_refuses_pre_existing_kit_file_without_writing() -> None:
    for dry_run in (False, True):
        consumer = copy_fixture("empty-workspace")
        existing = [consumer / ".sc-lint" / "justfile", consumer / "README.sc-lint.md"]
        existing[0].parent.mkdir()
        for path in existing:
            path.write_text("consumer-owned content\n")
        arguments = ["--input", str(FIXTURES / "install.json"), str(consumer)]
        if dry_run:
            arguments.insert(0, "--dry-run")

        result = run(*arguments)
        assert result.returncode == 2
        assert all(str(path) in result.stderr for path in existing)
        assert all(path.read_text() == "consumer-owned content\n" for path in existing)
        assert not (consumer / ".sc-lint" / "installed.json").exists()
        assert not (consumer / "sc-lint.toml").exists()
        assert not (consumer / "plugins" / "sc-lint").exists()


def test_rerun_refuses_pre_existing_new_kit_file_without_writing() -> None:
    consumer = copy_fixture("empty-workspace")
    input_path = FIXTURES / "install.json"
    assert run("--input", str(input_path), str(consumer)).returncode == 0

    relative = "plugins/sc-lint/README.md"
    manifest_path = consumer / ".sc-lint" / "installed.json"
    manifest = json.loads(manifest_path.read_text())
    assert relative in manifest["managed"]
    del manifest["managed"][relative]
    manifest_path.write_text(json.dumps(manifest))

    new_kit_file = consumer / relative
    new_kit_file.write_text("consumer-owned content\n")
    before_manifest = manifest_path.read_bytes()

    result = run("--input", str(input_path), str(consumer))

    assert result.returncode == 2
    assert str(new_kit_file) in result.stderr
    assert new_kit_file.read_text() == "consumer-owned content\n"
    assert manifest_path.read_bytes() == before_manifest


def test_workflow_uses_vendored_action_and_root_has_no_action() -> None:
    consumer = copy_fixture("empty-workspace")
    assert run("--input", str(FIXTURES / "install.json"), str(consumer)).returncode == 0
    workflow = (consumer / ".github" / "workflows" / "sc-lint.yml").read_text()
    assert "runs-on: ${{ matrix.os }}" in workflow
    assert "uses: ./plugins/sc-lint/.github/actions/setup-sc-lint\n" in workflow
    assert (consumer / "plugins/sc-lint/.github/actions/setup-sc-lint/action.yml").is_file()
    assert not (consumer / ".github" / "actions").exists()


@pytest.mark.parametrize(
    ("replacement", "error_fragment"),
    [
        ("{{ unknown_setting }}", "unknown template variables: ['unknown_setting']"),
        ("", "missing template variables: ['analyzers']"),
    ],
)
def test_unresolved_template_variables_fail_before_writing(
    tmp_path: Path, replacement: str, error_fragment: str
) -> None:
    kit = tmp_path / "kit"
    shutil.copytree(KIT, kit)
    template = kit / "templates" / "sc-lint.toml.j2"
    template.write_text(template.read_text().replace("{{ analyzers }}", replacement))
    consumer = tmp_path / "consumer"
    consumer.mkdir()

    result = run_subprocess(
        ["python3", str(kit / "install.py"), "--input", str(FIXTURES / "install.json"), str(consumer)],
        text=True,
        capture_output=True,
        check=False,
    )

    assert result.returncode == 2
    assert error_fragment in result.stderr
    assert list(consumer.iterdir()) == []


def test_unspaced_template_variable_is_substituted_by_installer(tmp_path: Path) -> None:
    kit = tmp_path / "kit"
    shutil.copytree(KIT, kit)
    template = kit / "templates" / "sc-lint.toml.j2"
    template.write_text(template.read_text().replace("{{ minimum_version }}", "{{minimum_version}}"))
    consumer = tmp_path / "consumer"
    consumer.mkdir()

    result = run_subprocess(
        ["python3", str(kit / "install.py"), "--input", str(FIXTURES / "install.json"), str(consumer)],
        text=True,
        capture_output=True,
        check=False,
    )

    assert result.returncode == 0, result.stderr
    rendered = (consumer / "sc-lint.toml").read_text()
    assert 'minimum_version = "0.5.0"' in rendered
    assert "{{minimum_version}}" not in rendered


def test_unspaced_windows_shell_variable_is_substituted_in_second_pass(tmp_path: Path) -> None:
    kit = tmp_path / "kit"
    shutil.copytree(KIT, kit)
    template = kit / "templates" / "Justfile.import.j2"
    template.write_text(template.read_text().replace("{{ windows_shell }}", "{{windows_shell}}"))
    consumer = tmp_path / "consumer"
    consumer.mkdir()

    result = run_subprocess(
        ["python3", str(kit / "install.py"), "--input", str(FIXTURES / "install.json"), str(consumer)],
        text=True,
        capture_output=True,
        check=False,
    )

    assert result.returncode == 0, result.stderr
    rendered = (consumer / "Justfile").read_text()
    assert "set windows-shell :=" in rendered
    assert "{{windows_shell}}" not in rendered


def test_ci_disabled_writes_no_workflow_or_action_files() -> None:
    consumer = copy_fixture("empty-workspace")
    input_path = write_input(consumer, ci={"os": ["linux"], "enabled": False})
    assert run("--input", str(input_path), str(consumer)).returncode == 0
    assert not (consumer / ".github").exists()
    assert not (consumer / "plugins" / "sc-lint" / ".github").exists()


def test_vendored_kit_can_recheck_and_schema_stays_vendored() -> None:
    consumer = copy_fixture("empty-workspace")
    input_path = FIXTURES / "install.json"
    assert run("--input", str(input_path), str(consumer)).returncode == 0
    installer = consumer / "plugins" / "sc-lint" / "install.py"
    assert installer.is_file()
    assert not (consumer / "install.schema.json").exists()
    result = run_subprocess(
        ["python3", "plugins/sc-lint/install.py", "."],
        cwd=consumer,
        text=True,
        capture_output=True,
        check=False,
    )
    assert result.returncode == 0, result.stderr
    result = run_subprocess(
        ["python3", "plugins/sc-lint/install.py", "--dry-run", "."],
        cwd=consumer,
        text=True,
        capture_output=True,
        check=False,
    )
    assert result.returncode == 0, result.stderr
    assert (consumer / ".sc-lint" / "install.json").is_file()


def test_bare_rerun_reports_a_missing_persisted_input() -> None:
    consumer = copy_fixture("empty-workspace")
    assert run("--input", str(FIXTURES / "install.json"), str(consumer)).returncode == 0
    (consumer / ".sc-lint" / "install.json").unlink()
    result = run_subprocess(
        ["python3", "plugins/sc-lint/install.py", "."],
        cwd=consumer,
        text=True,
        capture_output=True,
        check=False,
    )
    assert result.returncode == 2
    assert ".sc-lint" in result.stderr and "install.json" in result.stderr


def test_custom_input_refuses_to_overwrite_pre_existing_consumer_manifest() -> None:
    consumer = copy_fixture("empty-workspace")
    input_path = write_input(consumer, minimum_version="9.9.9")
    existing = consumer / ".sc-lint" / "install.json"
    existing.parent.mkdir(parents=True)
    existing.write_text('{"consumer": true}\n')
    before = existing.read_bytes()

    result = run("--input", str(input_path), str(consumer))

    assert result.returncode == 2
    assert str(existing) in result.stderr
    assert existing.read_bytes() == before
    assert not (consumer / "sc-lint.toml").exists()
    assert not (consumer / "plugins" / "sc-lint").exists()


def test_consumer_manifest_is_allowed_when_it_is_the_input() -> None:
    consumer = copy_fixture("empty-workspace")
    input_path = consumer / ".sc-lint" / "install.json"
    input_path.parent.mkdir(parents=True)
    input_path.write_bytes((FIXTURES / "install.json").read_bytes())

    result = run("--input", str(input_path), str(consumer))

    assert result.returncode == 0, result.stderr
    assert input_path.is_file()
    assert (consumer / ".sc-lint" / "installed.json").is_file()


def test_rerun_accepts_updated_input_manifest() -> None:
    consumer = copy_fixture("empty-workspace")
    input_path = consumer / ".sc-lint" / "install.json"
    input_path.parent.mkdir(parents=True)
    input_path.write_bytes((FIXTURES / "install.json").read_bytes())
    assert run("--input", str(input_path), str(consumer)).returncode == 0
    data = json.loads(input_path.read_text())
    data["minimum_version"] = "9.9.9"
    input_path.write_text(json.dumps(data))

    result = run("--input", str(input_path), str(consumer))

    assert result.returncode == 0, result.stderr
    assert 'minimum_version = "9.9.9"' in (consumer / "sc-lint.toml").read_text()


def test_rerun_preserves_unmanaged_justfile_edits() -> None:
    consumer = copy_fixture("empty-workspace")
    input_path = FIXTURES / "install.json"
    assert run("--input", str(input_path), str(consumer)).returncode == 0
    justfile = consumer / "Justfile"
    justfile.write_text(justfile.read_text() + "\nconsumer-task:\n    echo kept\n")

    result = run("--input", str(input_path), str(consumer))

    assert result.returncode == 0, result.stderr
    assert "consumer-task:\n    echo kept\n" in justfile.read_text()


def test_rerun_repairs_merged_justfile_and_keeps_consumer_edits() -> None:
    consumer = copy_fixture("empty-workspace")
    input_path = FIXTURES / "install.json"
    assert run("--input", str(input_path), str(consumer)).returncode == 0
    justfile = consumer / "Justfile"
    content = justfile.read_text()
    justfile.write_text(content.replace("import '.sc-lint/justfile'", "import 'broken'") + "\nconsumer-task:\n    echo kept\n")

    result = run("--input", str(input_path), str(consumer))

    assert result.returncode == 0, result.stderr
    repaired = justfile.read_text()
    assert "import '.sc-lint/justfile'" in repaired
    assert "import 'broken'" not in repaired
    assert "consumer-task:\n    echo kept\n" in repaired


def test_duplicate_markers_are_conflicts_without_writes() -> None:
    consumer = copy_fixture("empty-workspace")
    justfile = consumer / "Justfile"
    justfile.write_text("# >>> sc-lint managed integration >>>\n# <<< sc-lint managed integration <<<\n" * 2)
    before = justfile.read_bytes()
    result = run("--input", str(FIXTURES / "install.json"), str(consumer))
    assert result.returncode == 2
    assert str(justfile) in result.stderr or "marker conflict" in result.stderr
    assert justfile.read_bytes() == before


def test_analyzer_worked_example_is_declarative() -> None:
    consumer = copy_fixture("analyzer-worked-example")
    result = run("--input", str(FIXTURES / "analyzer-worked-example" / "install.json"), str(consumer))
    assert result.returncode == 0, result.stderr
    config = (consumer / "sc-lint.toml").read_text()
    assert 'reason = "no async runtime"' in config
    assert '"linux"' in config
    assert "[tool.sc-lint.analyzers.runtime]\nenabled = false" in config
    assert 'name = "unit"' in config and 'name = "integrate"' in config


def test_unknown_analyzer_key_is_rejected(tmp_path: Path) -> None:
    data = json.loads((FIXTURES / "install.json").read_text())
    data["analyzers"]["portability"]["severity"] = "warn"
    (tmp_path / "in.json").write_text(json.dumps(data))
    result = run("--input", str(tmp_path / "in.json"), str(copy_fixture("empty-workspace")))
    assert result.returncode == 2
    assert result.stderr == "conflict: analyzers.portability has unknown fields: ['severity']\n"


def test_portability_keys_render_into_own_table(tmp_path: Path) -> None:
    data = json.loads((FIXTURES / "install.json").read_text())
    data["analyzers"]["portability"] = {"unix_path_prefixes": ["/opt/"], "fail_rules": ["PORT-001"]}
    (tmp_path / "in.json").write_text(json.dumps(data))
    consumer = copy_fixture("empty-workspace")
    assert run("--input", str(tmp_path / "in.json"), str(consumer)).returncode == 0
    assert '[tool.sc-lint.analyzers.portability]\nunix_path_prefixes = ["/opt/"]\nfail_rules = ["PORT-001"]' in (
        consumer / "sc-lint.toml"
    ).read_text()


def test_issue385_installer_roundtrip(tmp_path: Path) -> None:
    data = json.loads((FIXTURES / "install.json").read_text())
    data["analyzers"]["portability"] = {
        "exclude_paths_by_rule": {"PORT-007": ["src/lib.rs"]}
    }
    input_path = tmp_path / "in.json"
    input_path.write_text(json.dumps(data))
    consumer = copy_fixture("empty-workspace")

    result = run("--input", str(input_path), str(consumer))

    assert result.returncode == 0, result.stderr
    assert 'exclude_paths_by_rule = { "PORT-007" = ["src/lib.rs"] }' in (
        consumer / "sc-lint.toml"
    ).read_text()
    (consumer / "src" / "lib.rs").write_text(
        (ROOT / "crates" / "sc-lint-portability" / "tests" / "fixtures" / "issue385.rs").read_text()
    )

    build = run_subprocess(
        [
            "cargo",
            "build",
            "--quiet",
            "--message-format=json-render-diagnostics",
            "-p",
            "sc-lint-portability",
        ],
        cwd=ROOT,
        text=True,
        capture_output=True,
        check=False,
        timeout=WORKED_EXAMPLE_TIMEOUT_SECONDS,
    )
    assert build.returncode == 0, build.stderr

    artifacts = []
    for line in build.stdout.splitlines():
        message = json.loads(line)
        target = message.get("target", {})
        if (
            message.get("reason") == "compiler-artifact"
            and target.get("name") == "sc-lint-portability"
            and "bin" in target.get("kind", [])
            and message.get("executable")
        ):
            artifacts.append(message)
    assert len(artifacts) == 1, build.stdout
    analyzer_path = artifacts[0]["executable"]

    analyzer = run_subprocess(
        [
            analyzer_path,
            "analyze",
            "--root",
            str(consumer),
            "--config",
            str(consumer / "sc-lint.toml"),
            "--format",
            "json",
        ],
        cwd=ROOT,
        text=True,
        capture_output=True,
        check=False,
        timeout=SUBPROCESS_TIMEOUT_SECONDS,
    )
    assert analyzer.returncode == 0, analyzer.stderr
    report = json.loads(analyzer.stdout)
    assert [finding["rule_id"] for finding in report["findings"]] == ["PORT-006"]

    data["analyzers"]["portability"] = {
        "exclude_paths_by_rule": {"PORT-007": "src/lib.rs"}
    }
    input_path.write_text(json.dumps(data))
    result = run("--input", str(input_path), str(copy_fixture("empty-workspace")))
    assert result.returncode == 2
    assert "exclude_paths_by_rule" in result.stderr

    data["analyzers"]["portability"] = {
        "exclude_paths_by_rule": {"portability": ["src/lib.rs"]}
    }
    input_path.write_text(json.dumps(data))
    result = run("--input", str(input_path), str(copy_fixture("empty-workspace")))
    assert result.returncode == 2
    assert "unknown exact rules" in result.stderr


def test_generated_catalogs_are_current_and_vendored_only() -> None:
    result = run_subprocess(
        ["python3", str(ROOT / "tools" / "generate_installer_catalog.py"), "--check"],
        cwd=ROOT,
        text=True,
        capture_output=True,
        check=False,
        timeout=WORKED_EXAMPLE_TIMEOUT_SECONDS,
    )
    assert result.returncode == 0, result.stderr

    consumer = copy_fixture("empty-workspace")
    assert run("--input", str(FIXTURES / "install.json"), str(consumer)).returncode == 0
    assert not (consumer / "analyzer-catalog").exists()
    assert not (consumer / "schema.d").exists()
    plugin = consumer / "plugins" / "sc-lint"
    assert (plugin / "analyzer-catalog" / "portability.json").is_file()
    assert (plugin / "schema.d" / "tools.json").is_file()
    assert (plugin / "install.schema.json").is_file()


def test_install_omits_internal_schema_and_catalog(tmp_path: Path) -> None:
    kit = tmp_path / "kit"
    shutil.copytree(KIT, kit)
    fragment = kit / "schema.d" / "phase-k.json"
    fragment.parent.mkdir(exist_ok=True)
    fragment.write_text(json.dumps({"properties": {"phase_k_fixture": {"type": "string"}}}))
    consumer = tmp_path / "consumer"
    consumer.mkdir()

    result = run_subprocess(
        ["python3", str(kit / "install.py"), "--input", str(FIXTURES / "install.json"), str(consumer)],
        text=True,
        capture_output=True,
        check=False,
    )

    assert result.returncode == 0, result.stderr
    assert not (consumer / "analyzer-catalog").exists()
    assert not (consumer / "schema.d").exists()
    plugin = consumer / "plugins" / "sc-lint"
    assert (plugin / "analyzer-catalog" / "portability.json").is_file()
    assert (plugin / "schema.d" / "phase-k.json").is_file()


def test_schema_merge_conflict_names_fragment_and_property(tmp_path: Path) -> None:
    kit = tmp_path / "kit"
    shutil.copytree(KIT, kit)
    overlay = kit / "schema.d" / "zzz-conflict.json"
    overlay.parent.mkdir(exist_ok=True)
    overlay.write_text(json.dumps({"properties": {"minimum_version": {"type": "integer"}}}))
    consumer = tmp_path / "consumer"
    consumer.mkdir()

    result = run_subprocess(
        ["python3", str(kit / "install.py"), "--input", str(FIXTURES / "install.json"), str(consumer)],
        text=True,
        capture_output=True,
        check=False,
    )

    assert result.returncode == 2
    assert "zzz-conflict.json" in result.stderr
    assert "properties.minimum_version.type" in result.stderr


def test_schema_fragments_merge_in_sorted_order(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    earlier = tmp_path / "aaa-conflict.json"
    later = tmp_path / "zzz-conflict.json"
    earlier.write_text(json.dumps({"properties": {"minimum_version": {"type": "integer"}}}))
    later.write_text(json.dumps({"properties": {"minimum_version": {"type": "boolean"}}}))

    module_name = "sc_lint_adoption_installer_under_test"
    module_spec = importlib.util.spec_from_file_location(module_name, INSTALL)
    assert module_spec is not None
    assert module_spec.loader is not None
    installer = importlib.util.module_from_spec(module_spec)
    monkeypatch.setitem(sys.modules, module_name, installer)
    module_spec.loader.exec_module(installer)

    class ReverseSchemaDirectory:
        def exists(self) -> bool:
            return True

        def glob(self, _pattern: str) -> list[Path]:
            return [later, earlier]

    monkeypatch.setattr(installer, "SCHEMA_DIRECTORY", ReverseSchemaDirectory())
    with pytest.raises(ValueError, match=r"aaa-conflict\.json at properties\.minimum_version\.type"):
        installer.load_schema()


def test_baseline_sc_lint_config_rendering_is_unchanged() -> None:
    consumer = copy_fixture("empty-workspace")
    result = run("--input", str(FIXTURES / "install.json"), str(consumer))
    assert result.returncode == 0, result.stderr

    expected = dedent(
        '''\
        [tool.sc-lint]
        minimum_version = "0.5.0"

        [tool.sc-lint.analyzers.runtime]
        enabled = false
        reason = "non-tokio"

        [tool.sc-lint.analyzers.portability]
        enabled = true
        targets = ["linux"]

        [[tool.sc-lint.lint]]
        name = "ci"
        command = ["cargo", "check"]

        [[tool.sc-lint.test]]
        name = "unit"
        command = ["cargo", "test"]
        '''
    )
    assert (consumer / "sc-lint.toml").read_text() == expected


def test_installer_has_no_rule_constants() -> None:
    catalog_ids = {
        rule_id
        for catalog_path in (KIT / "analyzer-catalog").glob("*.json")
        for catalog in [json.loads(catalog_path.read_text())]
        for rule_id in catalog["rules"]
    }
    source_constants = {
        node.value
        for node in ast.walk(ast.parse(INSTALL.read_text()))
        if isinstance(node, ast.Constant) and isinstance(node.value, str)
    }

    assert catalog_ids
    assert not (catalog_ids & source_constants), (
        "executable install.py contains analyzer catalog IDs; generated catalog data "
        "and test fixtures are intentionally not scanned"
    )


def test_schema_analyzer_keys_match_rust_catalogs() -> None:
    schema = runpy.run_path(str(INSTALL), run_name="sc_lint_adoption_install")["load_schema"]()
    tables = schema["properties"]["analyzers"]["properties"]
    common = {"enabled", "reason", "exclude_rules", "fail_rules", "exclude_paths", "exclude_paths_by_rule"}
    for name, crate in [
        ("boundary", "sc-lint-boundary"),
        ("portability", "sc-lint-portability"),
        ("runtime", "sc-lint-runtime"),
    ]:
        source = (ROOT / "crates" / crate / "src" / "lib.rs").read_text()
        own = set(re.findall(r'"([a-z_]+)"', re.search(r"own_keys: &\[([^\]]*)\]", source).group(1)))
        assert set(tables[name]["properties"]) == common | own, name


LAYER_INVOCATIONS = (("test",), ("test", "all"), ("test", "integrate"), ("lint",))


def install_worked_example(version: str | None = None) -> Path:
    consumer = copy_fixture("analyzer-worked-example")
    input_data = json.loads((FIXTURES / "analyzer-worked-example" / "install.json").read_text())
    if version is not None:
        input_data["minimum_version"] = version
    input_path = consumer.parent / "install.json"
    input_path.write_text(json.dumps(input_data))
    result = run("--input", str(input_path), str(consumer))
    assert result.returncode == 0, result.stderr
    return consumer


def test_analyzer_worked_example_declares_all_just_layers() -> None:
    consumer = install_worked_example()
    for arguments in LAYER_INVOCATIONS:
        command = run_subprocess(
            ["just", "--dry-run", *arguments],
            cwd=consumer,
            text=True,
            capture_output=True,
            check=False,
        )
        assert command.returncode == 0, command.stderr
        forwarded = " ".join([arguments[0], "--config", "sc-lint.toml", *arguments[1:]])
        assert command.stderr.rstrip().endswith(forwarded), command.stderr


@pytest.mark.integration
def test_analyzer_worked_example_runs_all_declared_just_layers(product_artifacts: ProductArtifacts) -> None:
    consumer = install_worked_example(product_artifacts.version)
    environment = {
        **os.environ,
        "SC_LINT_BIN": str(product_artifacts.binary),
        "SC_LINT_WHEEL_DIR": str(product_artifacts.wheel_dir),
    }
    for arguments in LAYER_INVOCATIONS:
        command = run_subprocess(
            ["just", *arguments],
            cwd=consumer,
            env=environment,
            text=True,
            capture_output=True,
            check=False,
            timeout=WORKED_EXAMPLE_TIMEOUT_SECONDS,
        )
        assert command.returncode == 0, command.stderr


def test_kit_justfile_declares_no_set_default_or_test() -> None:
    text = (KIT / ".sc-lint" / "justfile").read_text()
    assert not any(line.startswith(("set ", "default", "test")) for line in text.splitlines())


def test_new_repository_gets_the_default_justfile() -> None:
    consumer = copy_fixture("empty-workspace")
    assert run("--input", str(FIXTURES / "install.json"), str(consumer)).returncode == 0
    text = (consumer / "Justfile").read_text()
    assert text.startswith((KIT / "templates" / "Justfile.default").read_text().rstrip())
    assert (KIT / "templates" / "test.just").read_text() in text
    assert text.count("set windows-shell") == 1


def test_existing_justfile_without_test_gets_the_recommended_recipe() -> None:
    consumer = copy_fixture("empty-workspace")
    (consumer / "Justfile").write_text("hello:\n    @echo hi\n")
    assert run("--input", str(FIXTURES / "install.json"), str(consumer)).returncode == 0
    assert run("--dry-run", "--input", str(FIXTURES / "install.json"), str(consumer)).returncode == 0
    assert (consumer / "Justfile").read_text().count("test *layer:") == 1


def test_consumer_test_recipe_is_kept() -> None:
    consumer = copy_fixture("empty-workspace")
    (consumer / "Justfile").write_text("test:\n    cargo nextest run\n")
    assert run("--dry-run", "--input", str(FIXTURES / "install.json"), str(consumer)).returncode == 1
    assert run("--input", str(FIXTURES / "install.json"), str(consumer)).returncode == 0
    assert (consumer / "Justfile").read_text().startswith("test:\n    cargo nextest run\n")


def test_windows_shell_rendered_in_managed_block_when_consumer_has_none() -> None:
    consumer = copy_fixture("empty-workspace")
    (consumer / "Justfile").write_text("hello:\n    @echo hi\n")
    assert run("--input", str(FIXTURES / "install.json"), str(consumer)).returncode == 0
    assert "managed integration >>>\nset windows-shell := " in (consumer / "Justfile").read_text()


def test_consumer_windows_shell_is_not_duplicated() -> None:
    consumer = copy_fixture("empty-workspace")
    (consumer / "Justfile").write_text('set windows-shell := ["cmd", "/c"]\n\nhello:\n    @echo hi\n')
    assert run("--input", str(FIXTURES / "install.json"), str(consumer)).returncode == 0
    assert (consumer / "Justfile").read_text().count("set windows-shell") == 1
    assert run_subprocess(
        ["just", "--list"], cwd=consumer, capture_output=True, check=False
    ).returncode == 0


def test_windows_shell_false_omits_setting() -> None:
    consumer = copy_fixture("empty-workspace")
    (consumer / "Justfile").write_text("hello:\n    @echo hi\n")
    input_path = write_input(consumer, justfile={"windows_shell": False})
    assert run("--input", str(input_path), str(consumer)).returncode == 0
    assert "windows-shell" not in (consumer / "Justfile").read_text()


def test_unknown_justfile_key_is_rejected() -> None:
    consumer = copy_fixture("empty-workspace")
    result = run("--input", str(write_input(consumer, justfile={"shell": True})), str(consumer))
    assert result.returncode == 2
    assert result.stderr == "conflict: justfile has unknown fields: ['shell']\n"


def test_non_boolean_windows_shell_is_rejected() -> None:
    consumer = copy_fixture("empty-workspace")
    result = run("--input", str(write_input(consumer, justfile={"windows_shell": "yes"})), str(consumer))
    assert result.returncode == 2
    assert result.stderr == "conflict: justfile.windows_shell must be a boolean\n"


def test_dry_run_names_colliding_recipes_without_writing() -> None:
    consumer = copy_fixture("empty-workspace")
    original = "lint:\n    echo mine\n\nupgrade:\n    echo mine\n"
    (consumer / "Justfile").write_text(original)
    result = run("--dry-run", "--input", str(FIXTURES / "install.json"), str(consumer))
    assert result.returncode == 2
    assert result.stderr == "conflict: Justfile collisions: lint, upgrade\n"
    assert (consumer / "Justfile").read_text() == original
    assert not (consumer / ".sc-lint").exists()


def test_dry_run_without_just_on_path_exits_2_and_says_so() -> None:
    consumer = copy_fixture("empty-workspace")
    environment = {**os.environ, "PATH": str(consumer / "src")}
    python = shutil.which("python3")
    assert python is not None
    result = run_subprocess(
        [
            python,
            str(INSTALL),
            "--dry-run",
            "--input",
            str(FIXTURES / "install.json"),
            str(consumer),
        ],
        env=environment,
        text=True,
        capture_output=True,
        check=False,
    )
    assert result.returncode == 2
    assert "--dry-run needs `just` on PATH" in result.stderr


def test_hyphenated_test_recipe_still_gets_the_recommended_recipe() -> None:
    consumer = copy_fixture("empty-workspace")
    (consumer / "Justfile").write_text("test-unit:\n    echo unit\n")
    assert run("--input", str(FIXTURES / "install.json"), str(consumer)).returncode == 0
    text = (consumer / "Justfile").read_text()
    assert "test-unit:" in text
    assert text.count("\ntest *layer:") == 1
