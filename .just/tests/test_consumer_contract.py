from __future__ import annotations

from pathlib import Path
import subprocess
import sys
import unittest


ROOT = Path(__file__).resolve().parents[2]


class ConsumerContractTests(unittest.TestCase):
    def setUp(self) -> None:
        self.justfile = (ROOT / "Justfile").read_text(encoding="utf-8")
        self.config = (ROOT / "sc-lint.toml").read_text(encoding="utf-8")

    def test_root_model_has_four_public_product_recipes(self) -> None:
        for recipe in ("setup", "lint", "test", "upgrade"):
            self.assertIn(f"{recipe}: _ensure-sc-lint", self.justfile)
        self.assertIn("[private]\n_ensure-sc-lint: _source-build", self.justfile)
        self.assertIn(".sc-lint/bootstrap ensure --config sc-lint.toml", self.justfile)
        self.assertIn("lint --consumer --config sc-lint.toml ci", self.justfile)
        self.assertIn("--config sc-lint.toml test", self.justfile)

    def test_root_model_exports_the_product_binary_without_shell_specific_assignments(self) -> None:
        self.assertIn("export SC_LINT_BIN := sc_lint_binary", self.justfile)
        for line in self.justfile.splitlines():
            self.assertFalse(
                line.lstrip().startswith("SC_LINT_BIN="),
                f"shell-specific inline environment assignment: {line}",
            )

    def test_root_profiles_retain_complete_source_gates(self) -> None:
        self.assertIn('command = ["just", "_source-lint", "full"]', self.config)
        self.assertIn('command = ["just", "_source-test"]', self.config)
        self.assertIn("cargo test --workspace", self.justfile)
        self.assertIn("run_pytests.py", self.justfile)
        self.assertIn("node --test action/test/action.test.cjs", self.justfile)

    def test_public_recipe_dry_runs_use_only_product_commands_after_build(self) -> None:
        for recipe, required in {
            "lint": "lint --consumer --config sc-lint.toml ci",
            "test": " test",
            "setup": "bootstrap setup --config sc-lint.toml --dry-run",
            "upgrade": "bootstrap upgrade --config sc-lint.toml --check --dry-run",
        }.items():
            result = subprocess.run(
                ["just", "--dry-run", recipe],
                cwd=ROOT,
                check=True,
                capture_output=True,
                text=True,
            )
            rendered = result.stdout + result.stderr
            self.assertIn(required, rendered, recipe)
            self.assertIn(".sc-lint/bootstrap", rendered, recipe)
            self.assertNotIn("cargo run", rendered, recipe)

    def test_ci_dogfoods_aggregate_just_commands_on_every_platform(self) -> None:
        workflow = (ROOT / ".github/workflows/ci.yml").read_text(encoding="utf-8")
        for runner in ("ubuntu-latest", "macos-latest", "windows-latest"):
            self.assertIn(runner, workflow)
        self.assertIn("run: just lint", workflow)
        self.assertIn("run: just setup", workflow)
        self.assertIn("run: just test", workflow)
        self.assertNotIn("run: cargo test --verbose", workflow)

    def test_windows_companion_is_a_managed_part_of_the_model_contract(self) -> None:
        for path in (
            ROOT / ".sc-lint/bootstrap.ps1",
            ROOT / "crates/sc-lint/assets/bootstrap.ps1",
        ):
            contents = path.read_text(encoding="utf-8")
            self.assertIn('"--config"', contents)
            self.assertIn("ValueFromRemainingArguments = $true", contents)
            self.assertIn("CLI.SC_LINT_BINARY_NOT_FOUND", contents)
            self.assertIn("compatibility check", contents)

    def test_windows_bootstrap_consumes_gnu_style_flags_from_remaining_arguments(self) -> None:
        contents = (ROOT / "crates/sc-lint/assets/bootstrap.ps1").read_text(
            encoding="utf-8"
        )
        self.assertIn('[string[]]$Rest', contents)
        self.assertIn('[Array]::IndexOf($Rest, "--config")', contents)
        self.assertNotIn('[Array]::IndexOf($args, "--config")', contents)

    def test_root_helpers_match_the_generated_product_assets(self) -> None:
        generated_posix = (
            "#!/bin/sh\n"
            "# Managed by sc-lint; regenerate with `sc-lint init --just`.\n"
            + (ROOT / "crates/sc-lint/assets/bootstrap").read_text(encoding="utf-8").removeprefix("#!/bin/sh\n")
        )
        generated_windows = (
            "# Managed by sc-lint; regenerate with `sc-lint init --just`.\n"
            + (ROOT / "crates/sc-lint/assets/bootstrap.ps1").read_text(encoding="utf-8")
        )
        self.assertEqual(
            (ROOT / ".sc-lint/bootstrap").read_text(encoding="utf-8"),
            generated_posix,
        )
        self.assertEqual(
            (ROOT / ".sc-lint/bootstrap.ps1").read_text(encoding="utf-8"),
            generated_windows,
        )

    def test_embedded_consumer_templates_match_the_shipped_template(self) -> None:
        template = (ROOT / "crates/sc-lint/assets/consumer-Justfile").read_text(
            encoding="utf-8"
        ).strip()
        for path in (
            ROOT / "docs-bundle/just-setup.md",
            ROOT / "docs/phase-E/sprint-E3.md",
            ROOT / "docs/phase-E/sprint-E7.md",
            ROOT / "docs/sc-lint/adr/ADR-012-consumer-adoption-and-just-contract.md",
        ):
            self.assertIn(template, path.read_text(encoding="utf-8"), path)

    def test_agent_and_installed_guidance_use_the_two_completion_commands(self) -> None:
        agents = (ROOT / "AGENTS.md").read_text(encoding="utf-8")
        readme = (ROOT / "README.md").read_text(encoding="utf-8")
        guide = (ROOT / "docs-bundle/best-practices.md").read_text(encoding="utf-8")
        for content in (agents, readme, guide):
            self.assertIn("just lint", content)
            self.assertIn("just test", content)


if __name__ == "__main__":
    unittest.main()
