"""Deterministic native-release verification tests: synthetic assets and controlled commands."""
import hashlib
import io
import struct
import sys
import tarfile
import tomllib
import urllib.request
import zipfile
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import release_native_verify as native

ROOT = Path(__file__).resolve().parents[3]


def test_matrix_uses_manifest_targets_and_native_hosts():
    manifest = tomllib.loads((ROOT / native.MANIFEST_PATH).read_text())
    rows = native.native_matrix(manifest)["include"]
    assert [row["target"] for row in rows] == [entry["target"] for entry in manifest["release_targets"]]
    hosts = {row["target"]: row for row in rows}
    assert hosts["aarch64-unknown-linux-gnu"]["runner"] == "ubuntu-24.04-arm"
    assert hosts["aarch64-pc-windows-msvc"] == {
        "target": "aarch64-pc-windows-msvc", "runner": "windows-11-arm", "python_arch": "arm64",
    }
    assert hosts["x86_64-apple-darwin"]["runner"] == "macos-15-intel"
    manifest["release_targets"] = manifest["release_targets"][-1:]
    assert native.native_matrix(manifest)["include"] == [hosts["aarch64-pc-windows-msvc"]]


def test_missing_native_runner_is_a_failure():
    with pytest.raises(ValueError, match="no supported native GitHub-hosted runner"):
        native.native_matrix({"release_targets": [{"target": "riscv64gc-unknown-linux-gnu"}]})
    with pytest.raises(ValueError, match="no release targets"):
        native.native_matrix({})


def executable(target):
    kind, arch = native.EXPECTED[target]
    data = bytearray(128)
    if kind == "elf":
        data[:4] = b"\x7fELF"
        data[5] = 1
        struct.pack_into("<H", data, 18, arch)
    elif kind == "pe":
        data[:2] = b"MZ"
        struct.pack_into("<I", data, 0x3C, 64)
        data[64:68] = b"PE\0\0"
        struct.pack_into("<H", data, 68, arch)
    else:
        data[:4] = b"\xcf\xfa\xed\xfe"
        struct.pack_into("<I", data, 4, arch)
    return bytes(data)


class Assets:
    def __init__(self, target, *, payload_target=None):
        self.target = target
        self.version = "1.2.3"
        windows = native.native_host(target)[0] == "Windows"
        extension = "zip" if windows else "tar.gz"
        platform = {"x86_64-unknown-linux-gnu": "manylinux_2_28_x86_64", "aarch64-unknown-linux-gnu": "manylinux_2_28_aarch64",
                    "x86_64-apple-darwin": "macosx_11_0_x86_64", "aarch64-apple-darwin": "macosx_11_0_arm64",
                    "x86_64-pc-windows-msvc": "win_amd64", "aarch64-pc-windows-msvc": "win_arm64"}[target]
        self.manifest_data = {
            "project": {"archive_prefix": "tool", "name": "sc-lint"},
            "release_targets": [{"target": target, "archive": extension}],
            "release_binaries": [{"name": "one"}, {"name": "two"}],
            "python_distributions": [{"name": "sc-lint", "wheels": [{"target": target, "platform": platform}]}],
        }
        self.archive_name = f"tool_{self.version}_{target}.{extension}"
        self.wheel_name = f"sc_lint-{self.version}-cp39-abi3-{platform}.whl"
        payload = executable(payload_target or target)
        prefix = self.archive_name.removesuffix(".tar.gz").removesuffix(".zip")
        output = io.BytesIO()
        files = {f"{prefix}/bin/{name}{'.exe' if windows else ''}": payload for name in ("one", "two")}
        if windows:
            with zipfile.ZipFile(output, "w") as archive:
                for name, data in files.items():
                    archive.writestr(name, data)
        else:
            with tarfile.open(fileobj=output, mode="w:gz") as archive:
                for name, data in files.items():
                    entry = tarfile.TarInfo(name)
                    entry.size, entry.mode = len(data), 0o755
                    archive.addfile(entry, io.BytesIO(data))
        wheel = io.BytesIO()
        with zipfile.ZipFile(wheel, "w") as archive:
            archive.writestr("sc_lint/_native" + (".pyd" if windows else ".so"), payload)
        self.payloads = {self.archive_name: output.getvalue(), self.wheel_name: wheel.getvalue()}
        sums = "".join(f"{hashlib.sha256(data).hexdigest()}  {name}\n" for name, data in self.payloads.items())
        self.payloads["checksums.txt"] = sums.encode()
        self.assets = [{"name": name, "id": index} for index, name in enumerate(self.payloads)]
        self.downloads = []

    def manifest(self, tag):
        assert tag == "v1.2.3"
        return self.manifest_data

    def json(self, endpoint):
        assert endpoint == "releases/tags/v1.2.3"
        return {"assets": self.assets}

    def get(self, endpoint, *, binary):
        assert binary
        self.downloads.append(endpoint)
        asset_id = int(endpoint.rsplit("/", 1)[-1])
        name = next(entry["name"] for entry in self.assets if entry["id"] == asset_id)
        return self.payloads[name]


def host(monkeypatch, target):
    system, arch = native.native_host(target)
    monkeypatch.setattr(native.platform, "system", lambda: system)
    monkeypatch.setattr(native.platform, "machine", lambda: "ARM64" if arch == "aarch64" else "AMD64")


def commands(*, wrong_version=None, pip_error=False):
    seen = []

    def run(argv):
        seen.append(argv)
        if "pip" in argv and pip_error:
            raise ValueError("pip installation failed: controlled failure")
        if argv[1:3] == ["-m", "venv"]:
            for relative in ("bin/python", "Scripts/python.exe"):
                python = Path(argv[3]) / relative
                python.parent.mkdir(parents=True, exist_ok=True)
                python.write_text("controlled Python executable")
        if argv[-1] == "--version":
            assert Path(argv[0]).is_file(), "the wheel does not publish a console script"
            name = "sc-lint" if "-c" in argv else Path(argv[0]).name.removesuffix(".exe")
            return f"{name} {'9.9.9' if name == wrong_version else '1.2.3'}"
        return ""

    return seen, run


@pytest.mark.parametrize("target", list(native.EXPECTED))
def test_each_native_platform_checks_every_binary_and_installs_its_wheel(monkeypatch, tmp_path, capsys, target):
    assets = Assets(target)
    host(monkeypatch, target)
    seen, run = commands()
    native.verify_release(assets, "v1.2.3", target, tmp_path, run=run)
    versions = ["sc-lint" if "-c" in argv else Path(argv[0]).name.removesuffix(".exe") for argv in seen if argv[-1] == "--version"]
    assert versions == ["one", "two", "sc-lint"]
    installed_cli = next(argv for argv in seen if "-c" in argv)
    assert "-I" in installed_cli and "sc_lint.run" in installed_cli[installed_cli.index("-c") + 1]
    pip = next(argv for argv in seen if "pip" in argv)
    assert pip[-1] == str(tmp_path / assets.wheel_name)
    assert pip[pip.index("--retries") + 1] == "0"
    assert len(assets.downloads) == 3
    assert "Python:" in capsys.readouterr().out


def test_wrong_host_fails_before_downloading(monkeypatch, tmp_path):
    target = "aarch64-unknown-linux-gnu"
    host(monkeypatch, "x86_64-unknown-linux-gnu")
    assets = Assets(target)
    with pytest.raises(ValueError, match="native runner required"):
        native.verify_release(assets, "v1.2.3", target, tmp_path)
    assert assets.downloads == []


@pytest.mark.parametrize("missing", ["archive", "wheel", "checksums"])
def test_missing_asset_fails_clearly(monkeypatch, tmp_path, missing):
    target = "x86_64-unknown-linux-gnu"
    host(monkeypatch, target)
    assets = Assets(target)
    name = {"archive": assets.archive_name, "wheel": assets.wheel_name, "checksums": "checksums.txt"}[missing]
    assets.assets = [entry for entry in assets.assets if entry["name"] != name]
    with pytest.raises(ValueError, match="required release asset|expected one sc_lint wheel"):
        native.verify_release(assets, "v1.2.3", target, tmp_path)


@pytest.mark.parametrize("corrupt", ["archive", "wheel"])
def test_checksum_applies_to_both_archive_and_wheel(monkeypatch, tmp_path, corrupt):
    target = "x86_64-unknown-linux-gnu"
    host(monkeypatch, target)
    assets = Assets(target)
    assets.payloads[assets.archive_name if corrupt == "archive" else assets.wheel_name] += b"corrupted"
    seen, run = commands()
    with pytest.raises(ValueError, match="SHA256 mismatch"):
        native.verify_release(assets, "v1.2.3", target, tmp_path, run=run)
    assert seen == []


@pytest.mark.parametrize("wrong_version", ["two", "sc-lint"])
def test_archive_and_installed_wheel_versions_must_match_tag(monkeypatch, tmp_path, wrong_version):
    target = "x86_64-unknown-linux-gnu"
    host(monkeypatch, target)
    assets = Assets(target)
    _, run = commands(wrong_version=wrong_version)
    with pytest.raises(ValueError, match=wrong_version + ": version mismatch"):
        native.verify_release(assets, "v1.2.3", target, tmp_path, run=run)


def test_mislabeled_architecture_fails_even_on_emulation_capable_host(monkeypatch, tmp_path):
    target = "aarch64-pc-windows-msvc"
    host(monkeypatch, target)
    assets = Assets(target, payload_target="x86_64-pc-windows-msvc")
    _, run = commands()
    with pytest.raises(ValueError, match="payload architecture does not match"):
        native.verify_release(assets, "v1.2.3", target, tmp_path, run=run)


def test_pip_failure_is_not_skipped(monkeypatch, tmp_path):
    target = "x86_64-unknown-linux-gnu"
    host(monkeypatch, target)
    _, run = commands(pip_error=True)
    with pytest.raises(ValueError, match="pip installation failed"):
        native.verify_release(Assets(target), "v1.2.3", target, tmp_path, run=run)


class RunArtifact:
    def __init__(self, filename):
        self.filename = filename
        self.calls = []

    def json(self, endpoint):
        self.calls.append(endpoint)
        return {"total_count": 1, "artifacts": [{"name": "x86_64-unknown-linux-gnu", "id": 123, "expired": False}]}

    def get(self, endpoint, *, binary):
        assert endpoint == "actions/artifacts/123/zip" and binary
        output = io.BytesIO()
        with zipfile.ZipFile(output, "w") as archive:
            archive.writestr(self.filename, b"published archive")
        return output.getvalue()


def test_auto_trigger_identifies_tag_from_its_run_not_latest_release():
    manifest = Assets("x86_64-unknown-linux-gnu").manifest_data
    client = RunArtifact("tool_1.2.3_x86_64-unknown-linux-gnu.tar.gz")
    assert native.tag_from_run(client, manifest, "42") == "v1.2.3"
    assert client.calls == ["actions/runs/42/artifacts?per_page=100&page=1"]


def test_unavailable_or_misnamed_run_artifact_fails():
    manifest = Assets("x86_64-unknown-linux-gnu").manifest_data
    with pytest.raises(ValueError, match="exactly one release version"):
        native.tag_from_run(RunArtifact("unrelated.tar.gz"), manifest, "42")
    with pytest.raises(ValueError, match="triggering Release run id"):
        native.tag_from_run(RunArtifact("unused"), manifest, "")


def test_redirect_does_not_forward_github_token_to_asset_host():
    request = urllib.request.Request("https://api.github.com/repos/owner/repo/releases/assets/1", headers={"Authorization": "Bearer test-token"})
    redirected = native.StripRedirectCredentials().redirect_request(request, None, 302, "Found", {}, "https://release-assets.githubusercontent.com/asset")
    assert redirected.get_header("Authorization") is None


def test_zip_path_traversal_fails(tmp_path):
    path = tmp_path / "bad.zip"
    with zipfile.ZipFile(path, "w") as archive:
        archive.writestr("../outside", b"untrusted")
    with pytest.raises(ValueError, match="unsafe archive entry"):
        native.extract_archive(path, tmp_path / "output")


def test_workflow_only_triggers_for_release_or_explicit_dispatch():
    import yaml
    workflow = yaml.load((ROOT / ".github/workflows/release-native-verify.yml").read_text(), Loader=yaml.BaseLoader)
    assert set(workflow["on"]) == {"workflow_dispatch", "workflow_run"}
    assert workflow["on"]["workflow_run"]["workflows"] == ["Release"]
    assert workflow["on"]["workflow_run"]["types"] == ["completed"]
    assert workflow["permissions"] == {"contents": "read"}
    plan = workflow["jobs"]["plan"]
    assert plan["permissions"] == {"contents": "read", "actions": "read"}
    assert "needs.plan.outputs.verify" in workflow["jobs"]["native"]["if"]
    assert "permissions" not in workflow["jobs"]["native"]


def test_wheel_payload_must_match_native_target_before_pip(monkeypatch, tmp_path):
    target = "aarch64-pc-windows-msvc"
    host(monkeypatch, target)
    assets = Assets(target)
    wheel = io.BytesIO()
    with zipfile.ZipFile(wheel, "w") as archive:
        archive.writestr("sc_lint/_native.pyd", executable("x86_64-pc-windows-msvc"))
    assets.payloads[assets.wheel_name] = wheel.getvalue()
    assets.payloads["checksums.txt"] = "".join(
        f"{hashlib.sha256(assets.payloads[name]).hexdigest()}  {name}\n"
        for name in [assets.archive_name, assets.wheel_name]
    ).encode()
    seen, run = commands()
    with pytest.raises(ValueError, match="expected"):
        native.verify_release(assets, "v1.2.3", target, tmp_path, run=run)
    assert not any("pip" in argv for argv in seen)


def test_python_older_than_311_is_rejected_on_win_arm64(monkeypatch):
    host(monkeypatch, "aarch64-pc-windows-msvc")
    monkeypatch.setattr(native.sys, "version_info", (3, 10, 0))
    with pytest.raises(ValueError, match="Python 3.11 or newer"):
        native.assert_native("aarch64-pc-windows-msvc")


def test_missing_run_artifact_is_not_silently_skipped():
    manifest = Assets("x86_64-unknown-linux-gnu").manifest_data
    client = RunArtifact("unused")
    client.json = lambda endpoint: {"total_count": 0, "artifacts": []}
    with pytest.raises(ValueError, match="expected one available"):
        native.tag_from_run(client, manifest, "42")


def test_run_artifact_pagination_reads_each_page_once():
    manifest = Assets("x86_64-unknown-linux-gnu").manifest_data
    client = RunArtifact("tool_1.2.3_x86_64-unknown-linux-gnu.tar.gz")
    calls = []
    def page(endpoint):
        calls.append(endpoint)
        artifact = [] if endpoint.endswith("page=1") else [{"name": "x86_64-unknown-linux-gnu", "id": 123}]
        return {"total_count": 101, "artifacts": artifact}
    client.json = page
    assert native.tag_from_run(client, manifest, "42") == "v1.2.3"
    assert calls == ["actions/runs/42/artifacts?per_page=100&page=1", "actions/runs/42/artifacts?per_page=100&page=2"]


def test_actions_artifact_and_release_asset_downloads_use_their_required_media_types(monkeypatch):
    client = native.GitHub("owner/repo", "test-token")
    requests = []
    def open_request(request, timeout):
        requests.append((request.full_url, request.get_header("Accept")))
        return io.BytesIO(b"downloaded bytes")
    monkeypatch.setattr(client.opener, "open", open_request)
    client.get("actions/artifacts/123/zip", binary=True)
    client.get("releases/assets/456", binary=True)
    client.get("actions/runs/42/artifacts")
    assert [media_type for _, media_type in requests] == [
        "application/vnd.github+json", "application/octet-stream", "application/vnd.github+json",
    ]


def test_known_target_without_native_host_mapping_fails(monkeypatch):
    target = "x86_64-unknown-linux-gnu"
    monkeypatch.delitem(native.NATIVE_RUNNERS, native.native_host(target))
    with pytest.raises(ValueError, match="no native GitHub-hosted runner"):
        native.native_matrix(Assets(target).manifest_data)


def test_payload_commands_do_not_receive_github_tokens(monkeypatch):
    from types import SimpleNamespace
    monkeypatch.setenv("GH_TOKEN", "parent-gh-token")
    monkeypatch.setenv("GITHUB_TOKEN", "parent-github-token")
    monkeypatch.setenv("NATIVE_VERIFY_MARKER", "retained")
    child = {}
    def execute(argv, **kwargs):
        child.update(kwargs["env"])
        return SimpleNamespace(returncode=0, stdout="payload version", stderr="")
    monkeypatch.setattr(native.subprocess, "run", execute)
    assert native.run_command(["payload", "--version"]) == "payload version"
    assert "GH_TOKEN" not in child and "GITHUB_TOKEN" not in child
    assert child["NATIVE_VERIFY_MARKER"] == "retained"
    assert native.os.environ["GH_TOKEN"] == "parent-gh-token"
    assert native.os.environ["GITHUB_TOKEN"] == "parent-github-token"


class ReleaseRun(RunArtifact):
    def __init__(self, conclusion="success", *, published=True):
        super().__init__("tool_1.2.3_x86_64-unknown-linux-gnu.tar.gz")
        self.conclusion = conclusion
        self.published = published
        self.manifest_data = Assets("x86_64-unknown-linux-gnu").manifest_data

    def json(self, endpoint):
        if "/jobs?" in endpoint:
            self.calls.append(endpoint)
            return {"total_count": 1, "jobs": [{"name": "release", "conclusion": self.conclusion}]}
        if endpoint == "releases/tags/v1.2.3":
            self.calls.append(endpoint)
            if not self.published:
                raise ValueError("HTTP 404: production GitHub release is missing")
            return {"assets": []}
        return super().json(endpoint)

    def manifest(self, tag):
        assert tag == "v1.2.3"
        self.calls.append("manifest:" + tag)
        return self.manifest_data


def test_rehearsal_skips_before_artifact_tag_or_release_lookup(capsys):
    client = ReleaseRun("skipped", published=False)
    assert native.plan_release(client, {}, "", "42") is None
    assert client.calls == ["actions/runs/42/jobs?filter=latest&per_page=100&page=1"]
    assert "Skipping native verification" in capsys.readouterr().out


def test_production_release_is_planned_for_native_verification():
    client = ReleaseRun()
    tag, matrix = native.plan_release(client, client.manifest_data, "", "42")
    assert tag == "v1.2.3"
    assert matrix["include"][0]["target"] == "x86_64-unknown-linux-gnu"
    assert "releases/tags/v1.2.3" in client.calls


def test_production_missing_release_fails_instead_of_skipping():
    client = ReleaseRun(published=False)
    with pytest.raises(ValueError, match="production GitHub release is missing"):
        native.plan_release(client, client.manifest_data, "", "42")


def test_production_missing_assets_fails_instead_of_skipping(monkeypatch, tmp_path):
    client = ReleaseRun()
    tag, _ = native.plan_release(client, client.manifest_data, "", "42")
    target = "x86_64-unknown-linux-gnu"
    host(monkeypatch, target)
    assets = Assets(target)
    assets.assets = [entry for entry in assets.assets if entry["name"] != assets.wheel_name]
    with pytest.raises(ValueError, match="expected one sc_lint wheel"):
        native.verify_release(assets, tag, target, tmp_path)


def test_manual_dispatch_checks_the_release_even_if_no_run_id():
    client = ReleaseRun()
    tag, _ = native.plan_release(client, {}, "v1.2.3", "")
    assert tag == "v1.2.3"
    assert not any("/jobs?" in call for call in client.calls)


@pytest.mark.parametrize("jobs", [[], [{"name": "release", "conclusion": None}]])
def test_unknown_production_state_fails(jobs):
    client = ReleaseRun()
    client.json = lambda endpoint: {"total_count": len(jobs), "jobs": jobs}
    with pytest.raises(ValueError, match="cannot establish production status"):
        native.production_release_run(client, "42")


@pytest.mark.parametrize("conclusion", ["failure", "cancelled"])
def test_unsuccessful_release_job_skips_with_reason(conclusion, capsys):
    client = ReleaseRun(conclusion)
    assert native.plan_release(client, {}, "", "42") is None
    assert len(client.calls) == 1
    assert f"conclusion: {conclusion}" in capsys.readouterr().out


def test_publish_failure_does_not_skip_successful_release(monkeypatch, tmp_path):
    import yaml
    workflow = yaml.load((ROOT / ".github/workflows/release-native-verify.yml").read_text(), Loader=yaml.BaseLoader)
    assert "failure" in workflow["jobs"]["plan"]["if"]
    client = ReleaseRun()
    original_json = client.json
    def jobs_with_failed_publish(endpoint):
        if "/jobs?" in endpoint:
            return {"total_count": 2, "jobs": [
                {"name": "publish", "conclusion": "failure"},
                {"name": "release", "conclusion": "success"},
            ]}
        return original_json(endpoint)
    monkeypatch.setattr(client, "json", jobs_with_failed_publish)
    monkeypatch.setattr(native, "GitHub", lambda *args: client)
    manifest = tmp_path / "manifest.toml"
    manifest.write_text('[project]\narchive_prefix="tool"\n[[release_targets]]\ntarget="x86_64-unknown-linux-gnu"\narchive="tar.gz"\n')
    output = tmp_path / "outputs"
    assert native.main(["plan", "--run-id", "42", "--manifest", str(manifest), "--output", str(output)]) == 0
    assert output.read_text().startswith("verify=true\ntag=v1.2.3\n")


def test_plan_command_writes_disabled_outputs_for_skipped_release(monkeypatch, tmp_path):
    monkeypatch.setattr(native, "GitHub", lambda *args: ReleaseRun("skipped"))
    output = tmp_path / "outputs"
    assert native.main(["plan", "--run-id", "42", "--output", str(output)]) == 0
    assert output.read_text() == 'verify=false\ntag=\nmatrix={"include":[]}\n'


def test_release_job_api_name_matches_workflow():
    import yaml
    workflow = yaml.load((ROOT / ".github/workflows/release.yml").read_text(), Loader=yaml.BaseLoader)
    assert "release" in workflow["jobs"]
    assert workflow["jobs"]["release"].get("name", "release") == "release"
