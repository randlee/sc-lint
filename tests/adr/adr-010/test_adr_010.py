"""ADR-010 acceptance: PORT-006 prefix detection is default-on and additive-only.

Each test runs the real `sc-lint-portability analyze` binary against a copy of
`fixture/`, a workspace whose production code holds three Unix path literals.
"""
from __future__ import annotations

import json
import shutil
import subprocess
import tempfile
from functools import cache
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
FIXTURE = Path(__file__).resolve().parent / "fixture"
DEFAULT_FINDINGS = {
    ("PORT-006", "crates/example/src/lib.rs", "/usr/lib/foo"),
    ("PORT-006", "crates/example/src/lib.rs", "/etc/app.conf"),
    ("PORT-006", "crates/example/src/scratch.rs", "/tmp/x"),
}


@cache
def analyzer_binary() -> Path:
    build = subprocess.run(
        ["cargo", "build", "--quiet", "-p", "sc-lint-portability", "--message-format=json"],
        cwd=ROOT,
        text=True,
        capture_output=True,
        check=True,
    )
    for line in build.stdout.splitlines():
        message = json.loads(line)
        if message.get("reason") == "compiler-artifact" and message["target"]["name"] == "sc-lint-portability" and message.get("executable"):
            return Path(message["executable"])
    raise AssertionError("cargo build did not report the sc-lint-portability executable")


def findings(config: str | None) -> set[tuple[str, str, str]]:
    with tempfile.TemporaryDirectory() as temporary:
        workspace = Path(temporary) / "workspace"
        shutil.copytree(FIXTURE, workspace)
        command = [str(analyzer_binary()), "analyze", "--root", str(workspace), "--format", "json"]
        if config is not None:
            (workspace / "sc-lint.toml").write_text(config)
            command += ["--config", str(workspace / "sc-lint.toml")]
        result = subprocess.run(command, text=True, capture_output=True, check=False)
        assert result.returncode == 0, result.stderr
        report = json.loads(result.stdout)
        rows = set()
        for finding in report["findings"]:
            location, _, _ = finding["message"].partition(": ")
            source = Path(location.rsplit(":", 1)[0]).resolve().relative_to(workspace.resolve()).as_posix()
            rows.add((finding["rule_id"], source, finding["message"].split("`")[1]))
        return rows


def test_default_on_detects_builtin_prefixes_without_configuration() -> None:
    assert findings(None) == DEFAULT_FINDINGS


def test_configured_prefixes_are_additive_only() -> None:
    configured = findings('[tool.sc-lint.analyzers.portability]\nunix_path_prefixes = ["/opt/"]\n')
    assert configured >= DEFAULT_FINDINGS, DEFAULT_FINDINGS - configured


def test_exceptions_are_explicit_and_file_scoped() -> None:
    excepted = findings('[tool.sc-lint.analyzers.portability]\nexclude_paths = ["crates/example/src/scratch.rs"]\n')
    assert excepted == DEFAULT_FINDINGS - {("PORT-006", "crates/example/src/scratch.rs", "/tmp/x")}
