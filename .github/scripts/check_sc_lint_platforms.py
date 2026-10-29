"""Check sc-lint's complete release platform contract at preflight."""

from pathlib import Path
import sys
import tomllib
from release_platforms import PLATFORMS


targets = {target: metadata[0] for target, metadata in PLATFORMS.items()}
with Path(sys.argv[1]).open("rb") as source:
    manifest = tomllib.load(source)
if manifest["project"]["name"] == "sc-lint":
    archives = [entry["target"] for entry in manifest["release_targets"]]
    distributions = [entry for entry in manifest["python_distributions"] if entry["name"] == "sc-lint"]
    if len(archives) != len(targets) or set(archives) != targets.keys() or len(distributions) != 1:
        raise SystemExit("sc-lint must declare exactly six release targets and one Python distribution")
    wheels = distributions[0]["wheels"]
    if len(wheels) != len(targets) or {entry["target"]: entry["platform"] for entry in wheels} != targets:
        raise SystemExit("sc-lint must declare the six matching Python wheel platforms")
    for entry in manifest["release_targets"] + wheels:
        target = entry["target"]
        expected_runner = PLATFORMS[target][1]
        if entry["os"] != expected_runner:
            raise SystemExit(f"{target} must use {expected_runner}")
    channels = manifest["channels"]
    homebrew = {entry["key"]: entry["target"] for entry in channels["homebrew"]["assets"]}
    if homebrew.get("linux_arm") != "aarch64-unknown-linux-gnu":
        raise SystemExit("Homebrew must declare the Linux ARM archive")
    for name in ("winget", "scoop"):
        if channels[name].get("installer_arm64_target") != "aarch64-pc-windows-msvc":
            raise SystemExit(f"{name} must declare the Windows ARM archive")
    with Path(distributions[0]["source"], "pyproject.toml").open("rb") as source:
        python_config = tomllib.load(source)
    with Path(distributions[0]["cargo_manifest"]).open("rb") as source:
        cargo_config = tomllib.load(source)
    if python_config["project"]["requires-python"] != ">=3.9" or "abi3-py39" not in cargo_config["dependencies"]["pyo3"]["features"]:
        raise SystemExit("sc-lint wheels must require Python >=3.9 and enable abi3-py39")
    print("sc-lint six-platform archive and wheel contract verified")
