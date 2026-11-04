"""Keep independently shipped platform and helper-policy registries aligned."""
import ast
import json
from pathlib import Path
import re
import sys
import tomllib

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from release_platforms import PLATFORMS

ROOT = Path(__file__).resolve().parents[3]


def test_action_and_fixture_targets_match_release_manifest():
    manifest = tomllib.loads((ROOT / "release/publish-artifacts.toml").read_text())
    expected = {row["target"]: row["archive"] for row in manifest["release_targets"]}
    assert set(expected) == set(PLATFORMS)
    copies = []
    for filename in ("action/index.js", "action/test/action.test.cjs"):
        source = (ROOT / filename).read_text()
        rows = re.findall(r'"((?:linux|darwin|win32)/(?:x64|arm64))": \["([^"\n]+)", "([^"\n]+)"\]', source)
        assert len(rows) == len(expected), filename
        assert {triple: extension for _, triple, extension in rows} == expected, filename
        copies.append({host: (triple, extension) for host, triple, extension in rows})
    assert copies[0] == copies[1]


def test_kit_policy_names_match_schema_and_classify_adapter_scopes():
    tree = ast.parse((ROOT / "packages/sc-lint-adoption/install.py").read_text())
    policies = next(set(ast.literal_eval(node.value)) for node in tree.body
                    if isinstance(node, ast.Assign)
                    and any(isinstance(target, ast.Name) and target.id == "HELPER_POLICY_TABLES" for target in node.targets))
    schema = json.loads((ROOT / "packages/sc-lint-adoption/install.schema.json").read_text())
    schema_policies = {name for name, value in schema["properties"].items()
                       if value.get("type") == "object"} - {"profiles", "ci", "analyzers", "test", "justfile"}
    assert policies == schema_policies
    adapter = (ROOT / "crates/sc-lint/src/python_adapter.rs").read_text()
    scope_function = adapter.split("pub const fn config_scope", 1)[1].split("\n    }", 1)[0]
    scopes = set(re.findall(r'=> "([^"\n]+)"', scope_function))
    # These adapter scopes are outside the current kit input contract.
    # Keep that exclusion explicit so new scopes require a policy decision.
    non_kit_scopes = {"function_length", "view.findings"}
    assert scopes - policies == non_kit_scopes
    assert scopes & policies == {"line_counts", "identities"}
    # The remaining policies belong to Python helpers rather than Rust adapters.
    assert policies - scopes == {"version_sync", "cargo_shear", "logging", "artifacts"}
