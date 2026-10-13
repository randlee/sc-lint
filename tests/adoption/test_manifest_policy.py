"""Installer schema and catalog coverage for optional boundary manifest policy."""
from __future__ import annotations

import json
import runpy

from install_helpers import KIT


def test_issue136_installer_roundtrip() -> None:
    fragment = json.loads((KIT / "schema.d" / "manifest-policy.json").read_text())
    schema = runpy.run_path(str(KIT / "install.py"), run_name="sc_lint_adoption_install")["load_schema"]()
    boundary = schema["properties"]["analyzers"]["properties"]["boundary"]
    keys = {
        "manifest_dependency_allowlists",
        "global_dependency_ownership",
        "manifest_section_rules",
    }
    assert keys <= set(fragment["properties"]["analyzers"]["properties"]["boundary"]["properties"])
    assert keys <= set(boundary["properties"])
    catalog = json.loads((KIT / "analyzer-catalog" / "boundary.json").read_text())
    assert keys == set(catalog["own_keys"])
    assert {"SCB-MANIFEST-003", "SCB-MANIFEST-004", "SCB-MANIFEST-005", "SCB-MANIFEST-006"} <= set(catalog["rules"])
