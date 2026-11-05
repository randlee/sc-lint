from __future__ import annotations

import re
import unittest


from sc_lint.tests.source_checkout import source_checkout
from sc_lint.print_help import SECTIONS, render_help


REPO_ROOT = source_checkout(__file__)


class PrintHelpTests(unittest.TestCase):
    def test_render_help_lists_the_real_recipes(self) -> None:
        output = render_help()
        for entry in (
            "version [current|latest]",
            "setup",
            "upgrade",
            "test-adoption",
            "fmt [check|write]",
            "lint <selector> <scope>",
        ):
            self.assertIn(entry, output)

    def test_render_help_omits_recipes_the_justfile_does_not_define(self) -> None:
        output = render_help()
        for stale in (
            "lint fast",
            "lint full",
            "lint sc-boundary",
            "lint modules",
            "lint pytests",
            "fmt apply",
            "daemon-singleton",
            "view boundaries",
        ):
            self.assertNotIn(stale, output)

    @unittest.skipUnless(REPO_ROOT is not None, "needs the source checkout")
    def test_every_listed_recipe_is_defined_in_the_justfile(self) -> None:
        recipes = set(re.findall(r"^([a-z][\w-]*)[^:\n]*:(?!=)", (REPO_ROOT / "Justfile").read_text(), re.M))
        for _, entries in SECTIONS:
            for name, _ in entries:
                self.assertIn(name.split()[0], recipes, name)


if __name__ == "__main__":
    unittest.main()
