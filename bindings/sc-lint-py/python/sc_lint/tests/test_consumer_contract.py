from __future__ import annotations

from sc_lint.lint_common import discover_repo_root

import json
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile
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
            try:
                result = subprocess.run(
                    ["just", "--dry-run", recipe],
                    cwd=ROOT,
                    check=True,
                    capture_output=True,
                    text=True,
                    timeout=30,
                )
            except subprocess.TimeoutExpired as error:
                self.fail(
                    f"just --dry-run {recipe} timed out after {error.timeout}s; "
                    f"stdout={error.stdout!r}; stderr={error.stderr!r}"
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
        self.assertIn("wheel-runtime:", workflow)
        self.assertIn('python: ["3.11", "3.14"]', workflow)
        self.assertIn("github.base_ref == 'develop'", workflow)
        self.assertIn("github.base_ref == 'main'", workflow)
        self.assertIn("sc_lint.run", workflow)
        self.assertIn("Run adoption kit integration tests", workflow)
        self.assertIn("Run archive and docs smoke", workflow)
        self.assertIn("Run full release-archive smoke test", workflow)
        self.assertIn("offline native artifact", workflow)
        self.assertGreaterEqual(workflow.count("github.event_name == 'push'"), 4)

    def test_windows_companion_is_a_managed_part_of_the_model_contract(self) -> None:
        # This is a content guard; the PowerShell integration path is not run here.
        for path in (
            ROOT / ".sc-lint/bootstrap.ps1",
            ROOT / "crates/sc-lint/assets/bootstrap.ps1",
        ):
            contents = path.read_text(encoding="utf-8")
            self.assertIn('"--config"', contents)
            self.assertIn("$InvocationArguments = @($args)", contents)
            self.assertIn("$Rest = if ($InvocationArguments.Count -gt 1)", contents)
            self.assertNotIn("[CmdletBinding()]", contents)
            self.assertIn("Invoke-VerifiedReleaseLauncher", contents)
            self.assertIn("CLI.SC_LINT_RELEASE_UNAVAILABLE", contents)
            self.assertIn("compatibility check", contents)
            self.assertIn("function Enter-ProductEnvironment {", contents)
            self.assertEqual(contents.count("        Enter-ProductEnvironment\n"), 2)
            self.assertIn("$env:SC_LINT_BIN = $productBinary", contents)
            self.assertIn(
                "$env:PATH = (Split-Path -Parent $productBinary) + [IO.Path]::PathSeparator + $env:PATH",
                contents,
            )

    @unittest.skipUnless(shutil.which("pwsh"), "PowerShell Core is required")
    def test_windows_managed_binary_probe_is_quiet_when_managed_path_is_missing(self) -> None:
        contents = (ROOT / "crates/sc-lint/assets/bootstrap.ps1").read_text(
            encoding="utf-8"
        )
        match = re.search(r"(?ms)^function Test-ManagedBinary \{\n.*?^\}", contents)
        self.assertIsNotNone(match, "Test-ManagedBinary function must remain available")

        script = f'''$ErrorActionPreference = "Continue"
$root = Join-Path ([IO.Path]::GetTempPath()) ("sc-lint-managed-guard-" + [guid]::NewGuid().ToString("N"))
$managedDirectory = Join-Path $root "managed"
$externalDirectory = Join-Path $root "external"
New-Item -ItemType Directory -Path $managedDirectory, $externalDirectory -Force | Out-Null
$managedBinary = Join-Path $managedDirectory "sc-lint.exe"
$externalBinary = Join-Path $externalDirectory "sc-lint.exe"
Set-Content -LiteralPath $externalBinary -Value "external"
{match.group(0)}
$Error.Clear()
$missingResult = Test-ManagedBinary $externalBinary
$missingErrors = @($Error)
Set-Content -LiteralPath $managedBinary -Value "managed"
$Error.Clear()
$existingResult = Test-ManagedBinary $managedBinary
$existingErrors = @($Error)
Write-Output "$missingResult|$existingResult|$($missingErrors.Count)|$($existingErrors.Count)"
Remove-Item -LiteralPath $root -Recurse -Force
if ($missingResult -ne $false -or $existingResult -ne $true -or $missingErrors.Count -gt 0 -or $existingErrors.Count -gt 0) {{ exit 1 }}
'''
        with tempfile.TemporaryDirectory(prefix="sc-lint-pwsh-probe-") as temporary:
            script_path = Path(temporary) / "managed_binary_probe.ps1"
            script_path.write_text(script, encoding="utf-8")
            result = subprocess.run(
                ["pwsh", "-NoLogo", "-NoProfile", "-NonInteractive", "-File", str(script_path)],
                capture_output=True,
                text=True,
                check=False,
                timeout=30,
            )

        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(result.stdout.strip(), "False|True|0|0")
        self.assertEqual(result.stderr, "")

    def test_posix_lint_and_test_share_one_product_environment_helper(self) -> None:
        contents = (ROOT / "crates/sc-lint/assets/bootstrap").read_text(encoding="utf-8")
        self.assertEqual(contents.count("activate_product_env() {"), 1)
        self.assertEqual(contents.count("        activate_product_env\n"), 2)
        self.assertIn('export SC_LINT_BIN="$product_binary"', contents)
        self.assertIn('PATH="$(dirname "$product_binary"):$PATH"; export PATH', contents)

    def test_windows_bootstrap_consumes_gnu_style_flags_from_remaining_arguments(self) -> None:
        contents = (ROOT / "crates/sc-lint/assets/bootstrap.ps1").read_text(
            encoding="utf-8"
        )
        self.assertIn("$InvocationArguments = @($args)", contents)
        self.assertIn("$Rest = if ($InvocationArguments.Count -gt 1)", contents)
        self.assertIn('$execArguments = @($Rest[($index + 1)..($Rest.Count - 1)])', contents)
        self.assertIn('[Array]::IndexOf($Rest, "--config")', contents)
        self.assertNotIn("[CmdletBinding()]", contents)
        self.assertIn('$candidate = Join-Path $extract "sc-lint.exe"', contents)
        self.assertIn(
            'Join-Path (Join-Path (Join-Path $extract "sc-lint_${version}_x86_64-pc-windows-msvc") "bin") "sc-lint.exe"',
            contents,
        )
        self.assertLess(
            contents.index('$candidate = Join-Path $extract "sc-lint.exe"'),
            contents.index(
                'Join-Path (Join-Path (Join-Path $extract "sc-lint_${version}_x86_64-pc-windows-msvc") "bin") "sc-lint.exe"'
            ),
        )

    def test_posix_bootstrap_leaves_managed_activation_to_the_rust_installer(self) -> None:
        contents = (ROOT / "crates/sc-lint/assets/bootstrap").read_text(encoding="utf-8")
        self.assertIn('launcher="$staging/extract/sc-lint"', contents)
        self.assertIn('launcher="$staging/extract/sc-lint_${version}_${target}/bin/sc-lint"', contents)
        self.assertLess(
            contents.index('launcher="$staging/extract/sc-lint"'),
            contents.index('launcher="$staging/extract/sc-lint_${version}_${target}/bin/sc-lint"'),
        )
        self.assertIn('"$launcher" --config "$config" "$command" "$@"', contents)
        self.assertNotIn('mv "$staging/extract/sc-lint" "$install_dir/.sc-lint-new"', contents)

    def test_only_bootstrap_setup_hands_declared_tools_to_the_managed_helper(self) -> None:
        posix = (ROOT / "crates/sc-lint/assets/bootstrap").read_text(encoding="utf-8")
        self.assertIn('install_input="$(cd "$(dirname "$config")" && pwd)/.sc-lint/install.json"', posix)
        self.assertIn('"$venv_python" -m sc_lint.setup_tools --config "$config"\n', posix)
        self.assertIn('"$venv_python" -m sc_lint.setup_tools --config "$config" --check\n', posix)
        self.assertEqual(posix.count('provision_tools "$setup_mode"'), 1)
        self.assertLess(
            posix.index('provision_venv "$setup_mode" || exit $?'),
            posix.index('provision_tools "$setup_mode"'),
        )
        self.assertLess(posix.index("    setup)\n"), posix.index('provision_tools "$setup_mode"'))
        self.assertLess(posix.index('provision_tools "$setup_mode"'), posix.index("    lint)\n"))

        windows = (ROOT / "crates/sc-lint/assets/bootstrap.ps1").read_text(encoding="utf-8")
        self.assertIn('$installInput = Join-Path (Split-Path -Parent (Resolve-Path -LiteralPath $Config).Path) ".sc-lint\\install.json"', windows)
        self.assertIn('$helperArguments = @("-m", "sc_lint.setup_tools", "--config", $Config)', windows)
        self.assertIn('if ($Mode -ne "install") { $helperArguments += "--check" }', windows)
        self.assertEqual(windows.count("        Install-DeclaredTools $setupMode\n"), 1)
        self.assertLess(windows.index("        Install-Venv $setupMode\n"), windows.index("        Install-DeclaredTools $setupMode\n"))
        self.assertLess(windows.index('    "setup" {'), windows.index("        Install-DeclaredTools $setupMode\n"))
        self.assertLess(windows.index("        Install-DeclaredTools $setupMode\n"), windows.index('    "lint" {'))
        for contents in (posix, windows):
            self.assertNotIn("cargo install", contents)
            self.assertNotIn("rustup target", contents)

    @unittest.skipUnless(shutil.which("pwsh"), "PowerShell Core is required")
    def test_windows_tools_probe_passes_through_powershell_to_the_seed_python(self) -> None:
        contents = (ROOT / "crates/sc-lint/assets/bootstrap.ps1").read_text(encoding="utf-8")
        match = re.search(r"(?ms)^\$toolsProbe = @'\n.*?^'@$", contents)
        self.assertIsNotNone(match, "the PowerShell tools probe must remain a literal here-string")
        cases = {
            "absent": {"minimum_version": "0.7.0"},
            "empty": {"tools": {"cargo": {}, "rustup_targets": []}},
            "declared": {"tools": {"cargo": {"cargo-xwin": "0.18.4"}}},
            "malformed": {"tools": []},
        }
        with tempfile.TemporaryDirectory(prefix="sc-lint-pwsh-tools-probe-") as temporary:
            root = Path(temporary)
            for name, data in cases.items():
                (root / f"{name}.json").write_text(json.dumps(data), encoding="utf-8")
            calls = "\n".join(
                f"& $python -c $toolsProbe '{root / name}.json'; $codes += $LASTEXITCODE" for name in cases
            )
            script = root / "tools_probe.ps1"
            script.write_text(
                f"$python = '{sys.executable}'\n{match.group(0)}\n$codes = @()\n{calls}\n"
                "Write-Output ($codes -join '|')\n",
                encoding="utf-8",
            )
            result = subprocess.run(
                ["pwsh", "-NoLogo", "-NoProfile", "-NonInteractive", "-File", str(script)],
                capture_output=True,
                text=True,
                check=False,
                timeout=60,
            )
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(result.stdout.strip(), "0|0|3|3", result.stderr)

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
        try:
            result = subprocess.run(
                ["sh", str(ROOT / "crates/sc-lint/assets/bootstrap"), "setup", "--config", str(config), flag],
                cwd=tmp,
                env=env,
                text=True,
                capture_output=True,
                check=False,
                timeout=30,
            )
        except subprocess.TimeoutExpired as error:
            self.fail(
                f"bootstrap setup {flag} timed out after {error.timeout}s; "
                f"stdout={error.stdout!r}; stderr={error.stderr!r}"
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


class PosixBootstrapConsumerStepEnvironmentTests(unittest.TestCase):
    @unittest.skipIf(sys.platform == "win32", "POSIX bootstrap")
    def test_lint_and_test_export_managed_binary_to_consumer_steps(self) -> None:
        import tempfile

        for operation in ("lint", "test"):
            with self.subTest(operation=operation), tempfile.TemporaryDirectory() as raw:
                tmp = Path(raw)
                install_dir = tmp / "managed"
                install_dir.mkdir()
                log = tmp / "env.log"
                fake = install_dir / "sc-lint"
                fake.write_text(
                    "#!/bin/sh\n"
                    f"printf 'bin=%s path=%s\\n' \"$SC_LINT_BIN\" \"$(command -v sc-lint)\" >> '{log}'\n"
                    "exit 0\n",
                    encoding="utf-8",
                )
                fake.chmod(0o755)
                consumer = tmp / "consumer"
                venv_bin = consumer / ".sc-lint/venv/bin"
                venv_bin.mkdir(parents=True)
                (venv_bin / "python3").write_text("#!/bin/sh\nexit 0\n", encoding="utf-8")
                (venv_bin / "python3").chmod(0o755)
                config = consumer / "sc-lint.toml"
                config.write_text((ROOT / "sc-lint.toml").read_text(encoding="utf-8"), encoding="utf-8")
                try:
                    result = subprocess.run(
                        ["sh", str(ROOT / "crates/sc-lint/assets/bootstrap"), operation, "--config", str(config)],
                        cwd=tmp,
                        env={
                            "PATH": "/usr/bin:/bin",
                            "HOME": str(tmp),
                            "SC_LINT_INSTALL_DIR": str(install_dir),
                        },
                        text=True,
                        capture_output=True,
                        check=False,
                        timeout=30,
                    )
                except subprocess.TimeoutExpired as error:
                    self.fail(
                        f"bootstrap {operation} timed out after {error.timeout}s; "
                        f"stdout={error.stdout!r}; stderr={error.stderr!r}"
                    )
                self.assertEqual(result.returncode, 0, result.stderr)
                last = log.read_text(encoding="utf-8").splitlines()[-1]
                self.assertEqual(last, f"bin={fake} path={fake}")


if __name__ == "__main__":
    unittest.main()
