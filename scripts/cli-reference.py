#!/usr/bin/env python3
"""Generate or verify the pinned docli references from the real Clap trees."""

from __future__ import annotations

import argparse
import filecmp
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parent.parent
TARGETS = (
    ("sc-lint", "sc-lint", "sc-lint", "cli_reference_tests::cli_reference_matches_real_clap_definition"),
    ("sc-lint-boundary", "sc-lint-boundary", "sc-lint-boundary", "tests::cli_reference_matches_real_clap_definition"),
    ("sc-lint-portability", "sc-lint-portability", "sc-lint-portability", "tests::cli_reference_matches_real_clap_definition"),
    ("sc-lint-runtime", "sc-lint-runtime", "sc-lint-runtime", "tests::cli_reference_matches_real_clap_definition"),
)


def run(command: list[str], env: dict[str, str]) -> None:
    subprocess.run(command, cwd=ROOT, env=env, check=True)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=("check", "write"), nargs="?", default="check")
    args = parser.parse_args()
    version = subprocess.run(
        ["docli", "--version"], cwd=ROOT, check=True, capture_output=True, text=True
    ).stdout.strip()
    if version not in {"docli 0.2.0", "docli v0.2.0"}:
        raise SystemExit(f"expected pinned docli 0.2.0, found {version!r}")

    with tempfile.TemporaryDirectory(prefix="sc-lint-cli-reference-") as temp_name:
        temp = Path(temp_name)
        for package, binary, stem, test_name in TARGETS:
            model = temp / f"{stem}.json"
            env = os.environ.copy()
            env["SC_DOCLI_MODEL_OUT"] = str(model)
            run(
                ["cargo", "test", "-p", package, "--bin", binary,
                 test_name, "--", "--exact"],
                env,
            )
            markdown = temp / f"{stem}.md"
            html_dir = temp / stem
            run(
                ["docli", "generate", "--input", str(model), "--markdown", str(markdown), "--html", str(html_dir)],
                os.environ.copy(),
            )
            outputs = (
                (markdown, ROOT / "docs-bundle" / "cli-reference" / f"{stem}.md"),
                (html_dir / "index.html", ROOT / "docs-bundle" / "cli-reference" / stem / "index.html"),
            )
            for generated, checked_in in outputs:
                if args.mode == "write":
                    checked_in.parent.mkdir(parents=True, exist_ok=True)
                    shutil.copyfile(generated, checked_in)
                elif not checked_in.is_file() or not filecmp.cmp(generated, checked_in, shallow=False):
                    raise SystemExit(f"stale CLI reference: {checked_in.relative_to(ROOT)} (run python3 scripts/cli-reference.py write)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
