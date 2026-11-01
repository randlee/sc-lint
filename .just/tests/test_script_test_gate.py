"""Keep all script tests in the aggregate gate, including newly added files."""
from pathlib import Path
import shlex

ROOT = Path(__file__).resolve().parents[2]


def uncovered_tests(root, arguments):
    tests = {path.relative_to(root).as_posix() for path in (root / '.github/scripts/tests').rglob('test_*.py')}
    paths = [root / arg for arg in arguments if not arg.startswith('-')]
    return {name for name in tests if not any((root / name) == path or (path.is_dir() and path in (root / name).parents) for path in paths)}


def test_every_script_test_is_in_the_aggregate_gate():
    recipe = (ROOT / 'Justfile').read_text().split('_source-repo-pytests:\n', 1)[1].split('\n\n', 1)[0]
    arguments = shlex.split(recipe)
    assert '.github/scripts/tests' in arguments
    assert not any(arg.startswith(('--ignore', '--deselect')) for arg in arguments)
    assert uncovered_tests(ROOT, arguments) == set()


def test_a_new_file_cannot_fall_outside_a_hand_curated_gate(tmp_path):
    directory = tmp_path / '.github/scripts/tests'
    directory.mkdir(parents=True)
    (directory / 'test_old.py').write_text('')
    (directory / 'test_new.py').write_text('')
    assert uncovered_tests(tmp_path, ['.github/scripts/tests/test_old.py']) == {'.github/scripts/tests/test_new.py'}
    assert uncovered_tests(tmp_path, ['.github/scripts/tests']) == set()


def test_renderer_test_dependencies_match_the_shipped_kit_pin():
    import ast
    import runpy
    import tomllib
    pin = runpy.run_path(str(ROOT / '.github/scripts/bootstrap_sc_compose.py'))['SC_COMPOSE_VERSION']
    expected = 'sc-compose==' + pin
    project = tomllib.loads((ROOT / 'bindings/sc-lint-py/pyproject.toml').read_text())
    requirements = project['project']['optional-dependencies']['test']
    assert [item for item in requirements if item.startswith('sc-compose')] == [expected]
    source = (ROOT / 'bindings/sc-lint-py/python/sc_lint/source_venv.py').read_text()
    requirements = [node.value for node in ast.walk(ast.parse(source)) if isinstance(node, ast.Constant) and isinstance(node.value, str) and node.value.startswith('sc-compose==')]
    assert requirements == [expected]
