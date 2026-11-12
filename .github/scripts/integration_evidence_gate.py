"""Require the latest committed integration evidence for a release version."""
from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys


def git(*args: str) -> str:
    return subprocess.check_output(["git", *args], text=True, stderr=subprocess.PIPE)


def check(version: str, ref: str) -> str:
    if not re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+", version):
        raise ValueError("version must be X.Y.Z")
    paths = git("ls-tree", "-r", "--name-only", ref, "--", "site/reports/integration/").splitlines()
    pattern = re.compile(rf"^site/reports/integration/(\d{{8}}T\d{{6}}Z-v{re.escape(version)}-[0-9a-f]+)/results\.json$")
    candidates = sorted((match.group(1), path) for path in paths if (match := pattern.fullmatch(path)))
    if not candidates:
        raise ValueError(f"no integration evidence report for version {version} in {ref}")
    name, path = candidates[-1]
    directory = f"site/reports/integration/{name}"
    for artifact in ("report.html", "panel.xhtml", "pytest.xml"):
        if f"{directory}/{artifact}" not in paths:
            raise ValueError(f"integration evidence missing {artifact}: {directory}")
    data = json.loads(git("show", f"{ref}:{path}"))
    counts = data.get("counts", {})
    tests = data.get("tests", [])
    if (data.get("version") != version or data.get("status") != "PASS" or data.get("exit_code") != 0
            or not isinstance(tests, list) or not tests
            or not isinstance(counts, dict) or counts.get("passed", 0) < 1
            or counts.get("failed") != 0 or counts.get("error") != 0
            or any(test.get("outcome") not in ("passed", "skipped") for test in tests)):
        raise ValueError(f"integration evidence is not PASS for version {version}: {path}")
    for outcome, key in (("passed", "passed"), ("failed", "failed"), ("error", "error"), ("skipped", "skipped")):
        if counts.get(key) != sum(test.get("outcome") == outcome for test in tests):
            raise ValueError(f"integration evidence has inconsistent {key} count: {path}")
    return path


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--version", required=True)
    parser.add_argument("--ref", required=True)
    args = parser.parse_args()
    try:
        print(f"integration evidence PASS: {check(args.version, args.ref)}")
    except (ValueError, TypeError, AttributeError, json.JSONDecodeError, subprocess.CalledProcessError) as error:
        print(f"integration evidence gate: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
