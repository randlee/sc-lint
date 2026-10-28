from __future__ import annotations

import re
from pathlib import Path

import yaml


ROOT = Path(__file__).resolve().parents[2]


def _documented_keys(docs: str, section: str) -> set[str]:
    start = docs.index(f"### Action {section}")
    end = docs.find("\n### ", start + 1)
    if end == -1:
        end = len(docs)
    table = docs[start:end]
    return set(re.findall(r"^\|\s*`([^`]+)`\s*\|", table, flags=re.MULTILINE))


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


def test_ci_docs_match_every_declared_action_input_and_output() -> None:
    docs = (ROOT / "docs-bundle/ci.md").read_text(encoding="utf-8")
    action = yaml.safe_load((ROOT / "action.yml").read_text(encoding="utf-8"))

    for section in ("inputs", "outputs"):
        source_keys = set(action[section])
        documented_keys = _documented_keys(docs, section)
        assert source_keys <= documented_keys, (
            f"action.yml {section} missing from ci.md: "
            f"{sorted(source_keys - documented_keys)}"
        )
        assert documented_keys <= source_keys, (
            f"ci.md documents undeclared Action {section}: "
            f"{sorted(documented_keys - source_keys)}"
        )

    normalized_docs = " ".join(docs.split())
    assert "`artifact-url` and `checksums-url` are resolved independently" in normalized_docs
    assert "{release-base-url}/v{version}/sc-lint_{version}_{target}.{extension}" in normalized_docs
    assert "{release-base-url}/v{version}/checksums.txt" in normalized_docs


def test_runtime_rule_ids_in_readme_have_bundle_entries() -> None:
    runtime_readme = (ROOT / "crates/sc-lint-runtime/README.md").read_text(encoding="utf-8")
    bundle_guide = (ROOT / "docs-bundle/packages/sc-lint-runtime.md").read_text(
        encoding="utf-8"
    )

    documented_rule_ids = set(re.findall(r"SCB-RUNTIME-\d{3}", runtime_readme))
    bundle_rule_ids = set(
        re.findall(r"^- `(SCB-RUNTIME-\d{3})`:", bundle_guide, flags=re.MULTILINE)
    )

    assert documented_rule_ids, "runtime README must document at least one rule id"
    assert documented_rule_ids <= bundle_rule_ids, (
        "runtime docs bundle is missing documented rule ids: "
        f"{sorted(documented_rule_ids - bundle_rule_ids)}"
    )
