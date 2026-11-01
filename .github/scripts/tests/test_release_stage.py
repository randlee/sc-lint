"""Release upload sets come from the manifest, never checked-in release files."""
import hashlib
import re
from pathlib import Path
import shlex
import subprocess
import sys
import tomllib

import pytest
import yaml

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import release_stage

ROOT = Path(__file__).resolve().parents[3]
VERSION = "1.2.3"


def manifest_assets(manifest, prerelease):
    names = {f"{manifest['project']['archive_prefix']}_{VERSION}_{item['target']}.{item['archive']}" for item in manifest["release_targets"]}
    for package in manifest["python_distributions"]:
        distribution = package["name"].replace("-", "_")
        names.update(f"{distribution}-{VERSION}-cp39-abi3-{wheel['platform']}.whl" for wheel in package["wheels"])
        if package["sdist"] and not prerelease:
            names.add(f"{distribution}-{VERSION}.tar.gz")
    return names


@pytest.mark.parametrize("prerelease", [False, True])
def test_workflow_uploads_exact_manifest_assets_and_checksums(tmp_path, prerelease):
    manifest_path = ROOT / "release/publish-artifacts.toml"
    manifest = tomllib.loads(manifest_path.read_text())
    expected = manifest_assets(manifest, prerelease)
    filename = "prerelease-archive.yml" if prerelease else "release.yml"
    workflow = yaml.load((ROOT / ".github/workflows" / filename).read_text(), Loader=yaml.BaseLoader)
    job = workflow["jobs"]["release"]
    command = next(step["run"] for step in job["steps"] if "release_stage.py" in step.get("run", ""))
    argv = shlex.split(command)
    assert argv[:2] == ["python3", ".github/scripts/release_stage.py"]
    assert ("--prerelease" in argv) == prerelease
    source = tmp_path / ("assets" if prerelease else "artifacts")
    source.mkdir()
    for index, name in enumerate(sorted(expected)):
        directory = source / f"job-{index}"
        directory.mkdir()
        (directory / name).write_bytes(name.encode())
    # Neither downloaded junk nor the checked-in release tree belongs in an upload.
    (source / "unexpected.zip").write_bytes(b"stray archive")
    (source / "sc_lint-9.9.9-cp39-abi3-win_amd64.whl").write_bytes(b"wrong version")
    (source / "checksums.txt").write_text("untrusted downloaded checksums")
    release = tmp_path / "release"
    release.mkdir()
    for path in (ROOT / "release").iterdir():
        if path.is_file():
            (release / path.name).write_text("checked-in metadata")
    destination = tmp_path / job["env"]["RELEASE_ASSET_DIR"]
    assert destination != release and release not in destination.parents
    substitutions = {"$RELEASE_ARTIFACT_MANIFEST": str(manifest_path), "$RELEASE_VERSION": VERSION, "$RELEASE_ASSET_DIR": str(destination)}
    args = [substitutions.get(arg, arg) for arg in argv[2:]]
    result = subprocess.run([sys.executable, str(ROOT / argv[1]), *args], cwd=tmp_path, capture_output=True, text=True)
    assert result.returncode == 0, result.stderr
    if prerelease:
        upload = next(step for step in job["steps"] if 'gh release create "$tag"' in step.get("run", ""))
        assert upload["working-directory"] == "${{ env.RELEASE_ASSET_DIR }}"
        assert "mapfile" not in upload["run"] and "find ." not in upload["run"]
        selection = next(line.strip() for line in upload["run"].splitlines() if " upload-list " in line)
        assert selection == 'upload_list="$(python3 "$GITHUB_WORKSPACE/.github/scripts/release_stage.py" upload-list .)"'
        create = [line.strip() for line in upload["run"].splitlines() if line.strip().startswith("if gh release create")]
        assert create == ['if gh release create "$tag" --prerelease --title "$tag" --generate-notes "${assets[@]}"; then']
        selected = subprocess.run([sys.executable, str(ROOT / ".github/scripts/release_stage.py"), "upload-list", "."], cwd=destination, capture_output=True, text=True)
        assert selected.returncode == 0, selected.stderr
        uploaded = set(selected.stdout.splitlines())
    else:
        upload = next(step for step in job["steps"] if step.get("uses", "").startswith("softprops/action-gh-release@"))
        assert upload["with"]["files"].strip() == "${{ env.RELEASE_ASSET_DIR }}/*"
        uploaded = {path.name for path in destination.glob("*")}
        verify = next(step for step in job["steps"] if step.get("name") == "Verify collected Python distributions")
        assert '--asset-dir "$RELEASE_ASSET_DIR"' in verify["run"]
    patterns = subprocess.check_output([sys.executable, str(ROOT / ".github/scripts/release_artifacts.py"), "release-asset-patterns", "--manifest", str(manifest_path)], text=True).splitlines()
    assert all(any(re.fullmatch(pattern, name) for pattern in patterns) for name in uploaded - {"checksums.txt"})
    assert uploaded == expected | {"checksums.txt"}
    assert {path.name for path in destination.iterdir()} == uploaded
    checksums = dict(line.split("  ", 1)[::-1] for line in (destination / "checksums.txt").read_text().splitlines())
    assert set(checksums) == expected
    assert all(checksums[name] == hashlib.sha256((destination / name).read_bytes()).hexdigest() for name in expected)


def small_manifest():
    return {"release_binaries": [{"name": "tool"}], "project": {"archive_prefix": "tool"}, "release_targets": [{"target": "x86_64-pc-windows-msvc", "archive": "zip"}]}


@pytest.mark.parametrize("problem", ["missing", "duplicate", "stale_destination"])
def test_staging_fails_before_upload_on_incomplete_or_ambiguous_inputs(tmp_path, problem):
    source = tmp_path / "artifacts"
    source.mkdir()
    name = "tool_1.2.3_x86_64-pc-windows-msvc.zip"
    if problem != "missing":
        (source / name).write_bytes(b"payload")
    destination = tmp_path / "stage"
    if problem == "duplicate":
        (source / "another-job").mkdir()
        (source / "another-job" / name).write_bytes(b"duplicate")
    if problem == "stale_destination":
        destination.mkdir()
        (destination / "stray.md").write_text("stale")
    with pytest.raises((ValueError, FileExistsError)):
        release_stage.stage(small_manifest(), VERSION, source, destination)


@pytest.mark.parametrize("legacy", [False, True])
def test_wheels_only_manifest_uses_shared_release_contract(tmp_path, legacy, monkeypatch, capsys):
    import release_artifacts
    from types import SimpleNamespace
    manifest = tomllib.loads((ROOT / "release/publish-artifacts.toml").read_text())
    manifest["release_binaries"] = []
    package = manifest["python_distributions"][0]
    package["sdist"] = False
    if legacy:
        package["wheels"] = ["ubuntu-latest", "windows-latest"]
        names = {"sc_lint-1.2.3-cp39-abi3-manylinux_2_28_x86_64.whl", "sc_lint-1.2.3-cp39-abi3-win_amd64.whl"}
    else:
        names = {name for name in manifest_assets(manifest, False) if name.endswith(".whl")}
    source = tmp_path / "source"
    source.mkdir()
    for name in names:
        (source / name).write_bytes(name.encode())
    destination = tmp_path / "stage"
    release_stage.stage(manifest, VERSION, source, destination)
    assert {path.name for path in destination.iterdir()} == names | {"checksums.txt"}
    monkeypatch.setattr(release_artifacts, "load_manifest", lambda path: manifest)
    assert release_artifacts.cmd_release_asset_patterns(SimpleNamespace(manifest="controlled.toml")) == 0
    patterns = capsys.readouterr().out.splitlines()
    assert all(any(re.fullmatch(pattern, name) for pattern in patterns) for name in names)


@pytest.mark.parametrize("names", [[], ["checksums.txt"], ["payload.zip"]])
def test_upload_list_rejects_incomplete_stage(tmp_path, names):
    for name in names:
        (tmp_path / name).write_bytes(b"fixture")
    with pytest.raises(ValueError, match="requires payloads and checksums"):
        release_stage.upload_list(tmp_path)
