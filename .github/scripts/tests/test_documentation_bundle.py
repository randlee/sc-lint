"""Tests for the optional [documentation_bundle] release-archive contract."""

from __future__ import annotations

import io
import json
import sys
import tarfile
import tempfile
import tomllib
import unittest
import zipfile
from pathlib import Path

SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))

import release_documentation as DOCS  # noqa: E402
from test_install import INSTALL, InstallValuesTests  # noqa: E402

BUNDLE = {
    "source_directory": "docs/bundle",
    "package_directory": "share/docs",
    "required_files": ["README.md", "guides/start.md"],
}
BINARIES = [{"name": "example"}]


def manifest(declared: bool = True) -> dict:
    return {"documentation_bundle": dict(BUNDLE)} if declared else {}


def write_tree(root: Path, files: list[str]) -> None:
    for name in files:
        path = root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(name, encoding="utf-8")


def make_tar(path: Path, names: list[str]) -> None:
    with tarfile.open(path, "w:gz") as archive:
        for name in names:
            data = name.encode()
            info = tarfile.TarInfo(name)
            info.size = len(data)
            archive.addfile(info, io.BytesIO(data))


def make_zip(path: Path, names: list[str]) -> None:
    with zipfile.ZipFile(path, "w") as archive:
        for name in names:
            archive.writestr(name, name)


class BundleContractTests(unittest.TestCase):
    def test_absent_bundle_is_a_no_op(self) -> None:
        self.assertIsNone(DOCS.documentation_bundle({}))
        with tempfile.TemporaryDirectory() as directory:
            stage = Path(directory) / "stage"
            stage.mkdir()
            DOCS.validate_documentation_bundle({}, Path(directory) / "missing")
            DOCS.stage_documentation_bundle({}, stage)
            self.assertEqual(list(stage.iterdir()), [])
            archive = Path(directory) / "pkg.tar.gz"
            make_tar(archive, ["pkg/bin/example"])
            DOCS.validate_release_archive({}, BINARIES, archive)
        config = {"a": 1}
        self.assertIs(DOCS.package_config_with_bundle({}, config), config)

    def test_declared_bundle_validates_shape(self) -> None:
        self.assertEqual(DOCS.documentation_bundle(manifest()), BUNDLE)
        for key in BUNDLE:
            broken = {"documentation_bundle": {k: v for k, v in BUNDLE.items() if k != key}}
            with self.assertRaises(SystemExit):
                DOCS.documentation_bundle(broken)
        for bad in ("/abs", "../up", "a\\b", ""):
            with self.assertRaises(SystemExit):
                DOCS.documentation_bundle(
                    {"documentation_bundle": {**BUNDLE, "source_directory": bad}}
                )
        with self.assertRaises(SystemExit):
            DOCS.documentation_bundle({"documentation_bundle": {**BUNDLE, "required_files": []}})
        with self.assertRaises(SystemExit):
            DOCS.documentation_bundle(
                {"documentation_bundle": {**BUNDLE, "required_files": ["a", "a"]}}
            )

    def test_package_config_carries_declared_bundle(self) -> None:
        config = DOCS.package_config_with_bundle(manifest(), {"a": 1})
        self.assertEqual(config["documentation_bundle"], BUNDLE)

    def test_validate_reports_missing_and_unexpected(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            source = Path(directory)
            write_tree(source, BUNDLE["required_files"])
            DOCS.validate_documentation_bundle(manifest(), source)
            (source / "extra.md").write_text("x", encoding="utf-8")
            with self.assertRaises(SystemExit) as raised:
                DOCS.validate_documentation_bundle(manifest(), source)
            self.assertIn("unexpected=['extra.md']", str(raised.exception))
            (source / "extra.md").unlink()
            (source / "README.md").unlink()
            with self.assertRaises(SystemExit) as raised:
                DOCS.validate_documentation_bundle(manifest(), source)
            self.assertIn("missing=['README.md']", str(raised.exception))

    def test_stage_copies_bundle_under_package_directory(self) -> None:
        import os

        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            write_tree(root / "docs/bundle", BUNDLE["required_files"])
            stage = root / "stage"
            stage.mkdir()
            previous = Path.cwd()
            os.chdir(root)
            try:
                DOCS.stage_documentation_bundle(manifest(), stage)
            finally:
                os.chdir(previous)
            staged = {
                p.relative_to(stage / "share/docs").as_posix()
                for p in (stage / "share/docs").rglob("*")
                if p.is_file()
            }
        self.assertEqual(staged, set(BUNDLE["required_files"]))

    def test_archive_validation_tar_and_zip(self) -> None:
        bundle = [f"pkg/share/docs/{name}" for name in BUNDLE["required_files"]]
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            tar = root / "pkg.tar.gz"
            make_tar(tar, ["pkg/bin/example", *bundle])
            DOCS.validate_release_archive(manifest(), BINARIES, tar)
            zipped = root / "pkg.zip"
            make_zip(zipped, ["pkg/bin/example.exe", *bundle])
            DOCS.validate_release_archive(manifest(), BINARIES, zipped, windows=True)
            with self.assertRaises(SystemExit):
                DOCS.validate_release_archive(manifest(), BINARIES, zipped)

            missing_docs = root / "missing.tar.gz"
            make_tar(missing_docs, ["pkg/bin/example", bundle[0]])
            with self.assertRaises(SystemExit) as raised:
                DOCS.validate_release_archive(manifest(), BINARIES, missing_docs)
            self.assertIn("missing=['guides/start.md']", str(raised.exception))

            extra = root / "extra.zip"
            make_zip(extra, ["pkg/bin/example", *bundle, "pkg/share/docs/other.md"])
            with self.assertRaises(SystemExit) as raised:
                DOCS.validate_release_archive(manifest(), BINARIES, extra)
            self.assertIn("unexpected=['other.md']", str(raised.exception))

            no_binary = root / "nobin.tar.gz"
            make_tar(no_binary, bundle)
            with self.assertRaises(SystemExit):
                DOCS.validate_release_archive(manifest(), BINARIES, no_binary)


class InstallRenderTests(unittest.TestCase):
    def _render(self, values: dict) -> dict:
        try:
            import sc_compose  # noqa: F401
        except ModuleNotFoundError:
            self.skipTest("sc-compose bindings are not provisioned in this environment")
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "input.json"
            path.write_text(json.dumps(values), encoding="utf-8")
            loaded = INSTALL.load_install_values(path)
            output = Path(directory) / "publish-artifacts.toml"
            INSTALL.render_template(Path("release/publish-artifacts.toml.j2"), loaded, output)
            return tomllib.loads(output.read_text(encoding="utf-8"))

    def test_render_omits_table_when_undeclared(self) -> None:
        rendered = self._render(InstallValuesTests.valid_values())
        self.assertNotIn("documentation_bundle", rendered)

    def test_render_round_trips_declared_bundle(self) -> None:
        values = InstallValuesTests.valid_values()
        values["documentation_bundle"] = dict(BUNDLE)
        rendered = self._render(values)
        self.assertEqual(rendered["documentation_bundle"], BUNDLE)

    def test_load_rejects_incomplete_bundle(self) -> None:
        values = InstallValuesTests.valid_values()
        values["documentation_bundle"] = {**BUNDLE, "required_files": []}
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "input.json"
            path.write_text(json.dumps(values), encoding="utf-8")
            with self.assertRaises(Exception):
                INSTALL.load_install_values(path)


if __name__ == "__main__":
    unittest.main()
