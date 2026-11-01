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
