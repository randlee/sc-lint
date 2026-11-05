from pathlib import Path
import tempfile
import unittest

from sc_lint.tests.source_checkout import source_checkout


class SourceCheckoutTests(unittest.TestCase):
    def test_source_layout_markers(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            module = root / "bindings/sc-lint-py/python/sc_lint/tests/test_example.py"
            module.parent.mkdir(parents=True)
            module.touch()
            self.assertIsNone(source_checkout(str(module)))
            for name in ("Cargo.toml", "Justfile", "bindings/sc-lint-py/pyproject.toml"):
                (root / name).touch()
            self.assertEqual(source_checkout(str(module)), root.resolve())
