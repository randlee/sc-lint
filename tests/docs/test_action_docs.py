from __future__ import annotations

import re
from pathlib import Path

import yaml


ROOT = Path(__file__).resolve().parents[2]


def test_ci_action_examples_match_declared_inputs() -> None:
    docs = (ROOT / "docs-bundle/ci.md").read_text()
    action = yaml.safe_load((ROOT / "action.yml").read_text())
    input_names = set(action["inputs"])

    examples = []
    for block in re.findall(r"```yaml\s*\n(.*?)\n```", docs, flags=re.DOTALL):
        parsed = yaml.safe_load(block)
        if not isinstance(parsed, list):
            continue
        examples.extend(
            step
            for step in parsed
            if isinstance(step, dict)
            and isinstance(step.get("uses"), str)
            and step["uses"].startswith("randlee/sc-lint@")
        )

    assert examples, "ci.md must contain a reusable sc-lint Action example"
    for step in examples:
        with_inputs = step.get("with")
        assert isinstance(with_inputs, dict), f"Action example has no with: mapping: {step}"
        unknown = set(with_inputs) - input_names
        assert not unknown, f"Action example uses undeclared inputs: {sorted(unknown)}"
        assert "version" in with_inputs, f"Action example must supply version: {step}"
