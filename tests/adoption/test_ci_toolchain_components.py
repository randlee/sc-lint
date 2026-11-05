from __future__ import annotations

import json
import os
from pathlib import Path
import shutil
import stat
import subprocess
import sys
import tomllib

from install_helpers import FIXTURES
from install_helpers import run
from install_helpers import write_input


TOOLCHAIN = "1.83.0"
PROFILES = {
    "fmt": ["cargo", "fmt", "--all", "--check"],
    "clippy": ["cargo", "clippy", "--workspace", "--all-targets", "--", "-D", "warnings"],
}
COMPONENT_STEP = "rustup component add rustfmt clippy"

FAKE_TOOL = r'''from __future__ import annotations

import json
import os
import re
import subprocess
import sys
from pathlib import Path


def main() -> int:
    tool, *args = sys.argv[1:]
    state_path = Path(os.environ["SC_LINT_FAKE_STATE"])
    state = json.loads(state_path.read_text(encoding="utf-8"))
    toolchain_text = (Path.cwd() / "rust-toolchain.toml").read_text(encoding="utf-8")
    toolchain = re.search(r'(?m)^channel\s*=\s*"([^"]+)"', toolchain_text).group(1)
    state["events"].append({"tool": tool, "args": args, "toolchain": toolchain})

    if tool == "rustup":
        if args[:2] != ["component", "add"]:
            return 2
        installed = state["components"].setdefault(toolchain, [])
        for component in args[2:]:
            if component not in installed:
                installed.append(component)
        state_path.write_text(json.dumps(state), encoding="utf-8")
        return 0

    if tool == "cargo":
        command = args[0]
        component = {"fmt": "rustfmt", "clippy": "clippy"}[command]
        ok = component in state["components"].get(toolchain, [])
        state["cargo_calls"].append({"command": command, "toolchain": toolchain, "ok": ok})
        state_path.write_text(json.dumps(state), encoding="utf-8")
        if not ok:
            print(f"missing {component} for {toolchain}", file=sys.stderr)
            return 23
        return 0

    if tool == "just":
        state_path.write_text(json.dumps(state), encoding="utf-8")
        if args == ["setup"]:
            return 0
        if args == ["lint"]:
            for command in state["commands"]:
                result = subprocess.run(
                    " ".join(command), shell=True, cwd=os.environ["SC_LINT_CONSUMER"],
                    env=os.environ.copy(), check=False,
                )
                if result.returncode:
                    return result.returncode
            return 0
    return 2


raise SystemExit(main())
'''


def _install_consumer(parent: Path) -> tuple[Path, str, dict]:
    parent.mkdir(parents=True)
    consumer = parent / "consumer"
    shutil.copytree(FIXTURES / "empty-workspace", consumer)
    (consumer / "rust-toolchain.toml").write_text(
        f'[toolchain]\nchannel = "{TOOLCHAIN}"\nprofile = "minimal"\n', encoding="utf-8"
    )
    input_path = write_input(consumer, profiles=PROFILES)
    result = run("--input", str(input_path), str(consumer))
    assert result.returncode == 0, result.stderr
    config = tomllib.loads((consumer / "sc-lint.toml").read_text(encoding="utf-8"))["tool"]["sc-lint"]
    workflow = (consumer / ".github/workflows/sc-lint.yml").read_text(encoding="utf-8")
    return consumer, workflow, config


def _fake_tool_directory(path: Path) -> Path:
    path.mkdir()
    dispatcher = path / "fake_tool.py"
    dispatcher.write_text(FAKE_TOOL, encoding="utf-8")
    if os.name == "nt":
        for name in ("rustup", "cargo", "just"):
            command = subprocess.list2cmdline([sys.executable, str(dispatcher), name])
            (path / f"{name}.cmd").write_text(f"@{command} %*\n", encoding="utf-8")
    else:
        for name in ("rustup", "cargo", "just"):
            wrapper = path / name
            wrapper.write_text(
                f"#!{sys.executable}\nimport runpy, sys\nsys.argv.insert(1, {name!r})\n"
                f"runpy.run_path({str(dispatcher)!r}, run_name='__main__')\n",
                encoding="utf-8",
            )
            wrapper.chmod(wrapper.stat().st_mode | stat.S_IXUSR)
    return path


def _fake_environment(consumer: Path, tools: Path, state_path: Path, config: dict) -> dict[str, str]:
    commands = [step["command"] for step in config["lint"]]
    state_path.write_text(json.dumps({"components": {}, "commands": commands, "events": [], "cargo_calls": []}), encoding="utf-8")
    environment = os.environ.copy()
    environment.pop("RUSTUP_TOOLCHAIN", None)
    environment.update({
        "PATH": str(tools) + os.pathsep + environment.get("PATH", ""),
        "SC_LINT_FAKE_STATE": str(state_path),
        "SC_LINT_CONSUMER": str(consumer),
    })
    return environment


def _run_workflow_steps(steps: list[str], consumer: Path, environment: dict[str, str]) -> list[subprocess.CompletedProcess[str]]:
    results = []
    for step in steps:
        result = subprocess.run(
            step, shell=True, cwd=consumer, env=environment, text=True,
            capture_output=True, timeout=30, check=False,
        )
        results.append(result)
        if result.returncode:
            break
    return results


def test_pinned_minimal_toolchain_gets_fmt_and_clippy_before_lint(tmp_path: Path) -> None:
    consumer, workflow, config = _install_consumer(tmp_path / "with-components")
    assert 'profile = "minimal"' in (consumer / "rust-toolchain.toml").read_text(encoding="utf-8")
    assert [(step["name"], step["command"]) for step in config["lint"]] == list(PROFILES.items())

    lines = workflow.splitlines()
    action = lines.index("      - uses: dtolnay/rust-toolchain@stable")
    component = lines.index(f"      - run: {COMPONENT_STEP}")
    setup = lines.index("      - run: just setup")
    lint = lines.index("      - run: just lint")
    assert action < component < setup < lint
    steps = [line.removeprefix("      - run: ") for line in lines if line.startswith("      - run: ")]
    relevant = [step for step in steps if step in {COMPONENT_STEP, "just setup", "just lint"}]
    assert relevant == [COMPONENT_STEP, "just setup", "just lint"]

    tools = _fake_tool_directory(tmp_path / "fake-bin")
    state_path = tmp_path / "positive-state.json"
    environment = _fake_environment(consumer, tools, state_path, config)
    results = _run_workflow_steps(relevant, consumer, environment)
    assert len(results) == len(relevant)
    assert all(result.returncode == 0 for result in results), results
    state = json.loads(state_path.read_text(encoding="utf-8"))
    assert state["components"] == {TOOLCHAIN: ["rustfmt", "clippy"]}
    assert [event["tool"] for event in state["events"]] == ["rustup", "just", "just", "cargo", "cargo"]
    assert state["cargo_calls"] == [
        {"command": "fmt", "toolchain": TOOLCHAIN, "ok": True},
        {"command": "clippy", "toolchain": TOOLCHAIN, "ok": True},
    ]

    negative_consumer, _, negative_config = _install_consumer(tmp_path / "without-components")
    negative_state = tmp_path / "negative-state.json"
    negative_env = _fake_environment(negative_consumer, tools, negative_state, negative_config)
    negative = _run_workflow_steps(relevant[1:], negative_consumer, negative_env)
    assert len(negative) == 2
    assert negative[0].returncode == 0
    assert negative[1].returncode != 0
    assert "missing rustfmt" in negative[1].stderr
    negative_result_state = json.loads(negative_state.read_text(encoding="utf-8"))
    assert negative_result_state["cargo_calls"] == [{"command": "fmt", "toolchain": TOOLCHAIN, "ok": False}]
