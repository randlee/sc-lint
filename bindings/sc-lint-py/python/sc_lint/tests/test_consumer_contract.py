from __future__ import annotations

from sc_lint.lint_common import discover_repo_root

from pathlib import Path
import re
import subprocess
import sys
import unittest


ROOT = discover_repo_root()


class ConsumerContractTests(unittest.TestCase):
    def setUp(self) -> None:
        self.justfile = (ROOT / "Justfile").read_text(encoding="utf-8")
        self.config = (ROOT / "sc-lint.toml").read_text(encoding="utf-8")

    def test_root_model_has_four_public_product_recipes(self) -> None:
        for recipe in ("setup", "lint", "test", "upgrade"):
            self.assertIn(f"{recipe}: _source-build _source-venv", self.justfile)
            self.assertIn(f".sc-lint/bootstrap {recipe} --config sc-lint.toml", self.justfile)
        self.assertNotIn("_ensure-sc-lint", self.justfile)

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
        self.assertIn("-m sc_lint.run_pytests", self.justfile)
        self.assertIn("node --test action/test/action.test.cjs", self.justfile)

    def test_public_recipe_dry_runs_use_only_product_commands_after_build(self) -> None:
        for recipe, required in {
            "lint": "bootstrap lint --config sc-lint.toml",
            "test": "bootstrap test --config sc-lint.toml",
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
            self.assertIn("Invoke-VerifiedReleaseLauncher", contents)
            self.assertIn("CLI.SC_LINT_RELEASE_UNAVAILABLE", contents)
            self.assertIn("compatibility check", contents)

    def test_windows_bootstrap_consumes_gnu_style_flags_from_remaining_arguments(self) -> None:
        contents = (ROOT / "crates/sc-lint/assets/bootstrap.ps1").read_text(
            encoding="utf-8"
        )
        self.assertIn('[string[]]$Rest', contents)
        self.assertIn('[Array]::IndexOf($Rest, "--config")', contents)
        self.assertNotIn('[Array]::IndexOf($args, "--config")', contents)

    def test_posix_bootstrap_leaves_managed_activation_to_the_rust_installer(self) -> None:
        contents = (ROOT / "crates/sc-lint/assets/bootstrap").read_text(encoding="utf-8")
        self.assertIn('"$staging/extract/sc-lint" --config "$config" "$command" "$@"', contents)
        self.assertNotIn('mv "$staging/extract/sc-lint" "$install_dir/.sc-lint-new"', contents)

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
        for name in ("bootstrap", "bootstrap.ps1"):
            self.assertEqual(
                (ROOT / "packages/sc-lint-adoption/.sc-lint" / name).read_text(encoding="utf-8"),
                (ROOT / "crates/sc-lint/assets" / name).read_text(encoding="utf-8"),
                f"packages/sc-lint-adoption/.sc-lint/{name} drifted from the product asset",
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
            document = path.read_text(encoding="utf-8")
            just_blocks = re.findall(r"```just\n(.*?)\n```", document, flags=re.DOTALL)
            consumer_blocks = [
                block.strip() for block in just_blocks if "default: lint" in block
            ]
            self.assertEqual(consumer_blocks, [template], path)

    def test_agent_and_installed_guidance_use_the_two_completion_commands(self) -> None:
        agents = (ROOT / "AGENTS.md").read_text(encoding="utf-8")
        readme = (ROOT / "README.md").read_text(encoding="utf-8")
        guide = (ROOT / "docs-bundle/best-practices.md").read_text(encoding="utf-8")
        for content in (agents, readme, guide):
            self.assertIn("just lint", content)
            self.assertIn("just test", content)


class PosixBootstrapNonMutatingSetupTests(unittest.TestCase):
    """`setup --check`/`--dry-run` must never fetch a release, even when the
    managed binary fails its compatibility check (SC-QA-101)."""

    def _run_setup(self, flag: str, tmp: Path) -> tuple[subprocess.CompletedProcess[str], Path]:
        install_dir = tmp / "managed"
        install_dir.mkdir()
        log = tmp / "calls.log"
        fake_binary = install_dir / "sc-lint"
        fake_binary.write_text(
            "#!/bin/sh\n"
            f"printf '%s\\n' \"$*\" >> '{log}'\n"
            'for a in "$@"; do [ "$a" = "compatibility" ] && exit 1; done\n'
            "exit 0\n",
            encoding="utf-8",
        )
        fake_binary.chmod(0o755)
        tools = tmp / "tools"
        tools.mkdir()
        curl = tools / "curl"
        curl.write_text(f"#!/bin/sh\nprintf 'curl %s\\n' \"$*\" >> '{log}'\nexit 7\n", encoding="utf-8")
        curl.chmod(0o755)
        # A private config copy keeps the helper venv lookup inside `tmp`, so the
        # test never depends on (or mutates) this repository's own .sc-lint/venv.
        consumer = tmp / "consumer"
        consumer.mkdir()
        config = consumer / "sc-lint.toml"
        config.write_text((ROOT / "sc-lint.toml").read_text(encoding="utf-8"), encoding="utf-8")
        env = {
            "PATH": f"{tools}:/usr/bin:/bin",
            "HOME": str(tmp),
            "SC_LINT_INSTALL_DIR": str(install_dir),
        }
        result = subprocess.run(
            ["sh", str(ROOT / "crates/sc-lint/assets/bootstrap"), "setup", "--config", str(config), flag],
            cwd=tmp,
            env=env,
            text=True,
            capture_output=True,
            check=False,
        )
        return result, log

    def test_setup_check_and_dry_run_reuse_an_incompatible_managed_binary_without_fetching(self) -> None:
        import tempfile

        # With no helper venv present, --dry-run reports what it would do (exit 0)
        # while --check reports the missing venv (exit 4); neither may fetch.
        expected = {"--check": (4, "CLI.SC_LINT_PYTHON_UNAVAILABLE"), "--dry-run": (0, "")}
        for flag, (code, marker) in expected.items():
            with tempfile.TemporaryDirectory() as raw:
                result, log = self._run_setup(flag, Path(raw))
                calls = log.read_text(encoding="utf-8") if log.exists() else ""
                self.assertEqual(result.returncode, code, (flag, result.stdout, result.stderr, calls))
                self.assertIn(marker, result.stderr, (flag, result.stderr))
                self.assertNotIn("RELEASE_UNAVAILABLE", result.stderr, (flag, result.stderr))
                self.assertNotIn("curl", calls, (flag, calls))
                self.assertIn(f"setup {flag}", calls, (flag, calls))


if __name__ == "__main__":
    unittest.main()
