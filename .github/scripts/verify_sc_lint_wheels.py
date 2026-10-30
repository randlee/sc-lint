"""Verify the sc-lint wheels selected for publication."""

from pathlib import Path
import sys
import tomllib

from verify_release_arch import verify

with Path(sys.argv[1]).open("rb") as source:
    manifest = tomllib.load(source)
if manifest["project"]["name"] == "sc-lint":
    distribution = next(item for item in manifest["python_distributions"] if item["name"] == "sc-lint")
    expected = {item["platform"]: item["target"] for item in distribution["wheels"]}
    found = set()
    for wheel in Path(sys.argv[2]).glob("sc_lint-*.whl"):
        parts = wheel.stem.rsplit("-", 3)
        if len(parts) != 4 or parts[1:3] != ["cp39", "abi3"]:
            raise SystemExit(f"expected cp39-abi3 wheel: {wheel.name}")
        platform = parts[3]
        if platform not in expected or platform in found:
            raise SystemExit(f"unexpected or duplicate wheel platform: {wheel.name}")
        verify(wheel, expected[platform])
        found.add(platform)
    if found != expected.keys():
        raise SystemExit("sc-lint publication is missing a declared wheel platform")
