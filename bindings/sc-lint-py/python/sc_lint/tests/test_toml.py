from __future__ import annotations

import importlib
import unittest
from unittest import mock

from sc_lint import _toml


SAMPLE = """
name = "sc-lint"
count = 3
ratio = 1.5
enabled = true
tags = ["a", "b"]
limit = inf
missing = nan

[nested]
key = "value"
"""


def tomllib_or_none():
    try:
        return importlib.import_module("tomllib")
    except ImportError:
        return None


class FallbackPathTests(unittest.TestCase):
    def setUp(self):
        tomllib = tomllib_or_none()
        if tomllib is None:
            self.skipTest("tomllib needs Python 3.11 or newer")
        patches = [
            mock.patch.object(_toml, "_native_toml_loads", None),
            mock.patch.object(_toml, "_tomllib", tomllib),
        ]
        for patch in patches:
            patch.start()
            self.addCleanup(patch.stop)

    def test_parses_scalars_arrays_and_tables(self):
        parsed = _toml.loads(SAMPLE)
        self.assertEqual(parsed["name"], "sc-lint")
        self.assertEqual(parsed["count"], 3)
        self.assertEqual(parsed["tags"], ["a", "b"])
        self.assertEqual(parsed["nested"], {"key": "value"})

    def test_non_finite_floats_become_none(self):
        parsed = _toml.loads(SAMPLE)
        self.assertIsNone(parsed["limit"])
        self.assertIsNone(parsed["missing"])

    def test_datetimes_become_text(self):
        parsed = _toml.loads('when = 1979-05-27T07:32:00Z\nday = 1979-05-27\n')
        self.assertIsInstance(parsed["when"], str)
        self.assertTrue(parsed["when"].startswith("1979-05-27T07:32:00"))
        self.assertEqual(parsed["day"], "1979-05-27")

    def test_invalid_toml_raises_decode_error(self):
        with self.assertRaises(_toml.TOMLDecodeError):
            _toml.loads("name = ")


class NativePathTests(unittest.TestCase):
    @unittest.skipIf(_toml._native_toml_loads is None, "sc_lint._native is not built; run `just setup`")
    def test_native_parses_scalars_arrays_and_tables(self):
        parsed = _toml.loads(SAMPLE)
        self.assertEqual(parsed["name"], "sc-lint")
        self.assertEqual(parsed["tags"], ["a", "b"])
        self.assertEqual(parsed["nested"], {"key": "value"})
        self.assertIsNone(parsed["limit"])
        self.assertIsNone(parsed["missing"])

    @unittest.skipIf(_toml._native_toml_loads is None, "sc_lint._native is not built; run `just setup`")
    def test_native_datetimes_keep_the_source_text(self):
        parsed = _toml.loads("when = 1979-05-27T07:32:00Z\n")
        self.assertEqual(parsed["when"], "1979-05-27T07:32:00Z")

    @unittest.skipIf(_toml._native_toml_loads is None, "sc_lint._native is not built; run `just setup`")
    def test_native_invalid_toml_raises_decode_error_with_location(self):
        with self.assertRaises(_toml.TOMLDecodeError) as raised:
            _toml.loads("name = ")
        self.assertIn("invalid TOML", str(raised.exception))


class ParityTests(unittest.TestCase):
    @unittest.skipIf(_toml._native_toml_loads is None, "sc_lint._native is not built; run `just setup`")
    @unittest.skipIf(tomllib_or_none() is None, "tomllib needs Python 3.11 or newer")
    def test_both_paths_return_the_same_mapping_without_datetimes(self):
        native = _toml.loads(SAMPLE)
        with mock.patch.object(_toml, "_native_toml_loads", None), \
                mock.patch.object(_toml, "_tomllib", tomllib_or_none()):
            fallback = _toml.loads(SAMPLE)
        self.assertEqual(native, fallback)


class NoParserTests(unittest.TestCase):
    def test_neither_parser_available_raises_a_clear_error(self):
        with mock.patch.object(_toml, "_native_toml_loads", None), \
                mock.patch.object(_toml, "_tomllib", None):
            with self.assertRaises(ImportError) as raised:
                _toml.loads(SAMPLE)
        self.assertIn("sc_lint._native", str(raised.exception))
        self.assertIn("just setup", str(raised.exception))


if __name__ == "__main__":
    unittest.main()
