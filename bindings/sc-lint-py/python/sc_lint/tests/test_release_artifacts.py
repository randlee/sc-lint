from __future__ import annotations

from sc_lint.lint_common import discover_repo_root

import argparse
import io
import importlib.util
import json
import re
from pathlib import Path
import shutil
import subprocess
import sys
import tarfile
import tempfile
import tomllib
import unittest
import zipfile


REPO_ROOT = discover_repo_root()
SCRIPT_PATH = REPO_ROOT / "scripts/release_artifacts.py"
MANIFEST_PATH = REPO_ROOT / "release/sc-lint-release-checks.toml"
PUBLISH_MANIFEST_PATH = REPO_ROOT / "release/publish-artifacts.toml"
PUBLISH_KIT_SCRIPT = REPO_ROOT / ".github/scripts/release_artifacts.py"
RELEASE_SMOKE_PATH = REPO_ROOT / "scripts/release_smoke.py"

spec = importlib.util.spec_from_file_location("release_artifacts", SCRIPT_PATH)
assert spec is not None and spec.loader is not None
release_artifacts = importlib.util.module_from_spec(spec)
spec.loader.exec_module(release_artifacts)

smoke_spec = importlib.util.spec_from_file_location("release_smoke", RELEASE_SMOKE_PATH)
assert smoke_spec is not None and smoke_spec.loader is not None
release_smoke = importlib.util.module_from_spec(smoke_spec)
smoke_spec.loader.exec_module(release_smoke)


class ReleaseArtifactsTests(unittest.TestCase):
    def setUp(self) -> None:
        self.tempdir = tempfile.TemporaryDirectory()
        self.root = Path(self.tempdir.name)
        self.manifest = release_artifacts.load_manifest(MANIFEST_PATH, PUBLISH_MANIFEST_PATH)
        self.docs = self.root / "docs-bundle"
        self.docs.mkdir()
        for relative_path in release_artifacts.documentation_bundle_files(self.manifest):
            path = self.docs / relative_path
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(f"# {relative_path.name}\n", encoding="utf-8")
        shutil.copy2(REPO_ROOT / "docs-bundle" / "manifest.toml", self.docs / "manifest.toml")
        self.binaries = self.root / "binaries"
        self.binaries.mkdir()
        for name in release_artifacts.release_binary_names(self.manifest):
            (self.binaries / name).write_text(name, encoding="utf-8")

    def tearDown(self) -> None:
        self.tempdir.cleanup()

    def test_python_package_version_works_with_release_build_composite(self) -> None:
        workspace_toml = REPO_ROOT / "Cargo.toml"
        pyproject = REPO_ROOT / "bindings" / "sc-lint-py" / "pyproject.toml"
        version = tomllib.loads(workspace_toml.read_text(encoding="utf-8"))["workspace"]["package"]["version"]
        metadata = tomllib.loads(pyproject.read_text(encoding="utf-8"))["project"]

        self.assertEqual(metadata["version"], version)
        self.assertNotIn("version", metadata.get("dynamic", []))

        synced_pyproject = self.root / "pyproject.toml"
        synced_pyproject.write_text(pyproject.read_text(encoding="utf-8"), encoding="utf-8")
        sync = subprocess.run(
            [
                sys.executable,
                str(PUBLISH_KIT_SCRIPT),
                "sync-python-version",
                "--workspace-toml",
                "Cargo.toml",
                "--pyproject",
                str(synced_pyproject),
            ],
            cwd=REPO_ROOT,
            text=True,
            capture_output=True,
            check=False,
            timeout=30,
        )
        self.assertEqual(sync.returncode, 0, sync.stderr)
        self.assertIn(f"synced python package version to {version}", sync.stdout)

        verify = subprocess.run(
            [
                sys.executable,
                str(PUBLISH_KIT_SCRIPT),
                "verify-python-version",
                "--workspace-toml",
                "Cargo.toml",
                "--pyproject",
                str(synced_pyproject),
                "--version",
                version,
            ],
            cwd=REPO_ROOT,
            text=True,
            capture_output=True,
            check=False,
            timeout=30,
        )
        self.assertEqual(verify.returncode, 0, verify.stderr)
        self.assertIn("python version verification passed", verify.stdout)

    def test_python_release_verifier_requires_all_declared_wheel_platforms(self) -> None:
        assets = self.root / "python-release-assets"
        assets.mkdir()
        wheel_names = (
            "sc_lint-0.7.0-cp39-abi3-manylinux_2_28_x86_64.whl",
            "sc_lint-0.7.0-cp39-abi3-manylinux_2_28_aarch64.whl",
            "sc_lint-0.7.0-cp39-abi3-macosx_11_0_arm64.whl",
            "sc_lint-0.7.0-cp39-abi3-macosx_11_0_x86_64.whl",
            "sc_lint-0.7.0-cp39-abi3-win_amd64.whl",
            "sc_lint-0.7.0-cp39-abi3-win_arm64.whl",
        )
        wheel_metadata = "Metadata-Version: 2.1\nName: sc-lint\nVersion: 0.7.0\n\n"
        for wheel_name in wheel_names:
            with zipfile.ZipFile(assets / wheel_name, "w") as wheel:
                wheel.writestr("sc_lint-0.7.0.dist-info/METADATA", wheel_metadata)

        sdist_metadata = b"Metadata-Version: 2.1\nName: sc-lint\nVersion: 0.7.0\n\n"
        with tarfile.open(assets / "sc_lint-0.7.0.tar.gz", "w:gz") as sdist:
            package_info = tarfile.TarInfo("sc_lint-0.7.0/PKG-INFO")
            package_info.size = len(sdist_metadata)
            sdist.addfile(package_info, io.BytesIO(sdist_metadata))

        def verify(manifest: Path) -> subprocess.CompletedProcess[str]:
            return subprocess.run(
                [
                    sys.executable,
                    str(PUBLISH_KIT_SCRIPT),
                    "verify-python-release-assets",
                    "--manifest",
                    str(manifest),
                    "--asset-dir",
                    str(assets),
                ],
                cwd=REPO_ROOT,
                text=True,
                capture_output=True,
                check=False,
                timeout=30,
            )

        result = verify(PUBLISH_MANIFEST_PATH)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("'sc-lint': {'wheel': 6, 'sdist': 1}", result.stdout)
        for name in (wheel_names[1], wheel_names[3], wheel_names[5]):
            (assets / name).unlink()
        self.assertNotEqual(verify(PUBLISH_MANIFEST_PATH).returncode, 0)

        current_manifest = PUBLISH_MANIFEST_PATH.read_text(encoding="utf-8")
        current_wheels_line = next(
            line for line in current_manifest.splitlines() if line.startswith("wheels = ")
        )
        old_wheels_line = (
            'wheels = [{ id = "linux-x86_64", os = "ubuntu-latest", '
            'target = "x86_64-unknown-linux-gnu", platform = "manylinux_2_28_x86_64", '
            'manylinux = "2_28" }, "macos-latest", "windows-latest"]'
        )
        old_manifest = self.root / "old-publish-artifacts.toml"
        old_manifest.write_text(
            current_manifest.replace(current_wheels_line, old_wheels_line, 1),
            encoding="utf-8",
        )
        old_result = verify(old_manifest)
        self.assertNotEqual(old_result.returncode, 0)
        self.assertIn("Python wheel platforms do not match declared unique targets", old_result.stderr)

    def stage_archive(self, name: str = "archive") -> Path:
        staging = self.root / name
        release_artifacts.stage_release_archive(
            argparse.Namespace(
                manifest=str(MANIFEST_PATH),
                binaries_directory=str(self.binaries),
                documentation_directory=str(self.docs),
                staging_directory=str(staging),
                windows=False,
            )
        )
        return staging

    def test_staged_archive_contains_exactly_manifested_bundle_and_binaries(self) -> None:
        staging = self.stage_archive()
        release_artifacts.validate_staged_release_layout(self.manifest, staging, windows=False)
        self.assertTrue((staging / "sc-lint-docs" / "README.md").is_file())
        self.assertTrue((staging / "sc-lint-docs" / "packages" / "sc-lint.md").is_file())

    def test_archive_validation_rejects_missing_and_unexpected_documentation(self) -> None:
        self.docs.joinpath("packages/sc-lint.md").unlink()
        with self.assertRaisesRegex(SystemExit, "missing documentation files"):
            self.stage_archive("archive-missing")

        (self.docs / "packages/sc-lint.md").write_text("# sc-lint\n", encoding="utf-8")
        (self.docs / "surprise.md").write_text("# surprise\n", encoding="utf-8")
        with self.assertRaisesRegex(SystemExit, "unexpected documentation files"):
            self.stage_archive("archive-unexpected")

    def test_archive_validation_rejects_guides_not_recorded_in_package_manifest(self) -> None:
        package_manifest = self.docs / "manifest.toml"
        content = package_manifest.read_text(encoding="utf-8")
        package_manifest.write_text(
            content.replace(
                '[[guides]]\npath = "packages/sc-lint.md"\nkind = "package"\npackage = "sc-lint"\n\n',
                "",
            ),
            encoding="utf-8",
        )
        with self.assertRaisesRegex(SystemExit, "missing package-manifest guides"):
            self.stage_archive("archive-package-manifest")

    def test_archive_validation_rejects_broken_relative_documentation_links(self) -> None:
        overview = self.docs / "README.md"
        overview.write_text("[missing](missing.md)\n", encoding="utf-8")
        with self.assertRaisesRegex(SystemExit, "broken documentation link"):
            self.stage_archive("archive-broken-link")

    def test_windows_archive_layout_requires_executables_with_the_expected_suffix(self) -> None:
        windows_binaries = self.root / "windows-binaries"
        windows_binaries.mkdir()
        for name in release_artifacts.release_binary_names(self.manifest):
            (windows_binaries / f"{name}.exe").write_text(name, encoding="utf-8")
        staging = self.root / "windows-archive"
        release_artifacts.stage_release_archive(
            argparse.Namespace(
                manifest=str(MANIFEST_PATH),
                binaries_directory=str(windows_binaries),
                documentation_directory=str(self.docs),
                staging_directory=str(staging),
                windows=True,
            )
        )
        self.assertTrue((staging / "sc-lint.exe").is_file())
        release_artifacts.validate_staged_release_layout(self.manifest, staging, windows=True)

    def test_compressed_tar_and_zip_archives_reject_unexpected_content(self) -> None:
        archive = self.stage_archive()
        tar_path = Path(shutil.make_archive(str(self.root / "release"), "gztar", archive))
        release_artifacts.validate_release_archive(
            argparse.Namespace(manifest=str(MANIFEST_PATH), archive=str(tar_path), windows=False)
        )

        zip_path = Path(shutil.make_archive(str(self.root / "release"), "zip", archive))
        release_artifacts.validate_release_archive(
            argparse.Namespace(manifest=str(MANIFEST_PATH), archive=str(zip_path), windows=False)
        )
        (archive / "surprise.txt").write_text("unexpected\n", encoding="utf-8")
        unexpected_tar = Path(shutil.make_archive(str(self.root / "unexpected"), "gztar", archive))
        with self.assertRaisesRegex(SystemExit, "unexpected archive files"):
            release_artifacts.validate_release_archive(
                argparse.Namespace(manifest=str(MANIFEST_PATH), archive=str(unexpected_tar), windows=False)
            )

    def test_release_smoke_reads_the_pinned_kit_archive_layout_from_publish_manifest(self) -> None:
        manifest = tomllib.loads(PUBLISH_MANIFEST_PATH.read_text(encoding="utf-8"))
        workspace = tomllib.loads((REPO_ROOT / "Cargo.toml").read_text(encoding="utf-8"))
        version = workspace["workspace"]["package"]["version"]
        target = "x86_64-unknown-linux-gnu"
        config = json.loads(
            subprocess.check_output(
                [
                    sys.executable,
                    str(PUBLISH_KIT_SCRIPT),
                    "release-package-config",
                    "--manifest",
                    str(PUBLISH_MANIFEST_PATH),
                    "--target",
                    target,
                ],
                cwd=REPO_ROOT,
                text=True,
            )
        )

        stage = self.root / f"{config['project']['archive_prefix']}_{version}_{target}"
        binary_directory = stage / "bin"
        binary_directory.mkdir(parents=True)
        for binary in config["binaries"]:
            name = binary["name"]
            (binary_directory / name).write_text(name, encoding="utf-8")
            for bundled_path in binary.get("bundled_paths", []):
                shutil.copytree(
                    REPO_ROOT / bundled_path["source"],
                    stage / bundled_path["destination"],
                    dirs_exist_ok=True,
                )

        archive = self.root / f"{stage.name}.tar.gz"
        with tarfile.open(archive, "w:gz") as output:
            output.add(stage, arcname=stage.name)
        extracted = self.root / "extracted"
        binary_path, package_root, docs_bundle = release_smoke.extract_archive(
            archive,
            extracted,
            manifest=PUBLISH_MANIFEST_PATH,
            version=version,
            target=target,
        )

        sc_lint = next(entry for entry in manifest["release_binaries"] if entry["name"] == "sc-lint")
        docs_destination = next(
            entry["destination"]
            for entry in sc_lint["bundled_paths"]
            if entry["source"] == "docs-bundle"
        )
        self.assertEqual(binary_path, package_root / "bin" / "sc-lint")
        self.assertEqual(package_root, extracted / stage.name)
        self.assertEqual(docs_bundle, package_root / docs_destination)
        self.assertTrue((docs_bundle / "README.md").is_file())
        release_smoke.assert_documentation_integrity(docs_bundle, PUBLISH_MANIFEST_PATH)

    def test_release_smoke_rejects_missing_guide_in_extracted_bundle(self) -> None:
        docs_bundle = self.root / "missing-guide-bundle"
        shutil.copytree(REPO_ROOT / "docs-bundle", docs_bundle)
        guide = self.manifest["documentation_bundle"]["package_guides"][0]
        (docs_bundle / "packages" / guide).unlink()

        missing_guide = re.escape(str(Path("packages") / guide))
        with self.assertRaisesRegex(SystemExit, f"missing documentation files: {missing_guide}"):
            release_smoke.assert_documentation_integrity(docs_bundle, PUBLISH_MANIFEST_PATH)

    def test_release_smoke_rejects_broken_relative_link_in_extracted_bundle(self) -> None:
        docs_bundle = self.root / "broken-link-bundle"
        shutil.copytree(REPO_ROOT / "docs-bundle", docs_bundle)
        (docs_bundle / "README.md").write_text("[missing](missing.md)\n", encoding="utf-8")

        with self.assertRaisesRegex(SystemExit, "broken documentation link: README.md -> missing.md"):
            release_smoke.assert_documentation_integrity(docs_bundle, PUBLISH_MANIFEST_PATH)

    def test_legacy_formula_renderer_installs_docs_to_pkgshare_and_passes_ruby_syntax(self) -> None:
        # This local renderer is retained as legacy helper coverage. The release
        # workflow renders formulas through the pinned publish kit.
        sha_map = {target: "a" * 64 for target in release_artifacts.HOMEBREW_TARGETS}
        formula = release_artifacts.render_homebrew_formula_text(
            self.manifest,
            formula_name="sc-lint",
            version="0.4.0",
            tag="v0.4.0",
            sha_map=sha_map,
        )
        self.assertIn('pkgshare.install "sc-lint-docs"', formula)
        self.assertNotIn("README.md\"", formula)
        ruby = shutil.which("ruby")
        if ruby:
            formula_path = self.root / "sc-lint.rb"
            formula_path.write_text(formula, encoding="utf-8")
            completed = subprocess.run([ruby, "-c", str(formula_path)], capture_output=True, text=True)
            self.assertEqual(completed.returncode, 0, completed.stderr)

    def test_legacy_homebrew_layout_helper_stages_bins_and_docs(self) -> None:
        # This local staging helper does not model the current publish workflow.
        archive = self.stage_archive()
        prefix = self.root / "homebrew-prefix"
        release_artifacts.stage_homebrew_layout(
            argparse.Namespace(
                manifest=str(MANIFEST_PATH),
                archive_directory=str(archive),
                prefix=str(prefix),
            )
        )
        self.assertEqual(
            {path.name for path in (prefix / "bin").iterdir()},
            set(release_artifacts.release_binary_names(self.manifest)),
        )
        self.assertTrue(
            (prefix / "share" / "sc-lint" / "sc-lint-docs" / "just-setup.md").is_file()
        )


if __name__ == "__main__":
    unittest.main()
