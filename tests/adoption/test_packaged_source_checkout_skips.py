from __future__ import annotations

import os
from pathlib import Path
import shutil
import subprocess
import sys

from install_helpers import ROOT


def test_packaged_source_only_tests_skip_inside_unrelated_consumer(tmp_path: Path) -> None:
    consumer = tmp_path / "consumer"
    consumer.mkdir()
    (consumer / "Justfile").write_text("hello:\n    @echo unrelated consumer\n", encoding="utf-8")
    (consumer / "Cargo.toml").write_text(
        '[package]\nname = "unrelated-consumer"\nversion = "0.1.0"\n', encoding="utf-8"
    )

    package_source = ROOT / "bindings/sc-lint-py/python/sc_lint"
    site_packages = consumer / "venv/site-packages"
    site_packages.mkdir(parents=True)
    shutil.copytree(
        package_source,
        site_packages / "sc_lint",
        ignore=shutil.ignore_patterns(
            "__pycache__", "*.pyc", "test_consumer_contract.py", "test_consumer_lifecycle.py",
            "test_orchestration_contracts.py", "test_release_artifacts.py",
        ),
    )

    environment = os.environ.copy()
    environment.update({"PYTHONPATH": str(site_packages), "PYTHONNOUSERSITE": "1"})
    tests = [
        "sc_lint.tests.test_print_help.PrintHelpTests.test_every_listed_recipe_is_defined_in_the_justfile",
        "sc_lint.tests.test_lint_function_length.RepositoryTests.test_current_tree_passes_the_recorded_baseline",
    ]
    result = subprocess.run(
        [sys.executable, "-m", "unittest", "-v", *tests],
        cwd=consumer,
        env=environment,
        text=True,
        capture_output=True,
        timeout=30,
        check=False,
    )

    assert result.returncode == 0, result.stdout + result.stderr
    output = result.stdout + result.stderr
    assert all(test.rsplit(".", 1)[-1] in output for test in tests)
    assert output.count("skipped 'needs the source checkout'") == 2
    assert "OK (skipped=2)" in output
