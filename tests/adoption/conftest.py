"""Adoption-suite options.

Tests marked `integration` execute the kit against a built product. They run
only with `--integration` (see `just test-adoption-integration`), and then
read the binary and wheel set passed on the command line, never the shared
`target/debug` or `.sc-lint/wheels`. Missing artifacts fail the run.
"""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[2]


@dataclass(frozen=True)
class ProductArtifacts:
    binary: Path
    wheel_dir: Path
    version: str


def pytest_addoption(parser: pytest.Parser) -> None:
    group = parser.getgroup("adoption")
    group.addoption("--integration", action="store_true", help="run tests that execute the kit against a built product")
    group.addoption("--sc-lint-bin", type=Path, help="sc-lint binary for --integration tests")
    group.addoption("--sc-lint-wheel-dir", type=Path, help="offline sc_lint wheel directory for --integration tests")


def pytest_configure(config: pytest.Config) -> None:
    config.addinivalue_line("markers", "integration: executes the kit against a built product; selected by --integration")


def pytest_collection_modifyitems(config: pytest.Config, items: list[pytest.Item]) -> None:
    if config.getoption("--integration"):
        return
    deselected = [item for item in items if item.get_closest_marker("integration")]
    if deselected:
        config.hook.pytest_deselected(items=deselected)
        items[:] = [item for item in items if not item.get_closest_marker("integration")]


def workspace_version() -> str:
    lines = (ROOT / "Cargo.toml").read_text().splitlines()
    return next(line.split('"')[1] for line in lines if line.startswith("version ="))


@pytest.fixture(scope="session")
def product_artifacts(request: pytest.FixtureRequest) -> ProductArtifacts:
    binary = request.config.getoption("--sc-lint-bin")
    wheel_dir = request.config.getoption("--sc-lint-wheel-dir")
    if binary is None or wheel_dir is None:
        pytest.fail("--integration needs --sc-lint-bin and --sc-lint-wheel-dir; run `just test-adoption-integration`")
    binary, wheel_dir = binary.resolve(), wheel_dir.resolve()
    version = workspace_version()
    if not binary.is_file():
        pytest.fail(f"sc-lint binary not found: {binary}")
    if not any(wheel_dir.glob(f"sc_lint-{version}-*.whl")):
        pytest.fail(f"no sc_lint-{version} wheel in {wheel_dir}")
    return ProductArtifacts(binary, wheel_dir, version)
