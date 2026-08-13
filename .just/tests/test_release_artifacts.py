from __future__ import annotations

import argparse
import importlib.util
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest


REPO_ROOT = Path(__file__).resolve().parents[2]
SCRIPT_PATH = REPO_ROOT / "scripts/release_artifacts.py"
MANIFEST_PATH = REPO_ROOT / "release/publish-artifacts.toml"

spec = importlib.util.spec_from_file_location("release_artifacts", SCRIPT_PATH)
assert spec is not None and spec.loader is not None
release_artifacts = importlib.util.module_from_spec(spec)
spec.loader.exec_module(release_artifacts)


class ReleaseArtifactsTests(unittest.TestCase):
    def setUp(self) -> None:
        self.tempdir = tempfile.TemporaryDirectory()
        self.root = Path(self.tempdir.name)
        self.manifest = release_artifacts.load_manifest(MANIFEST_PATH)
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

    def test_primary_formula_installs_docs_to_pkgshare_and_passes_ruby_syntax(self) -> None:
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

    def test_homebrew_layout_installs_bins_and_docs_under_formula_owned_paths(self) -> None:
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
