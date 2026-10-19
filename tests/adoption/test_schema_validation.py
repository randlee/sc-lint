"""Required-field validation for nested installer schema objects."""
from __future__ import annotations

import runpy

import pytest
from install_helpers import INSTALL


@pytest.mark.parametrize("container", ["property", "array", "additional_property"])
@pytest.mark.parametrize("required", [True, False])
@pytest.mark.parametrize("record", [{}, {"reason": "Reviewed adapter."}])
def test_nested_schema_required_fields(container: str, required: bool, record: dict) -> None:
    schema_value = runpy.run_path(str(INSTALL))["_schema_value"]
    record_schema = {"type": "object", "properties": {"reason": {"type": "string"}}}
    if required:
        record_schema["required"] = ["reason"]
    if container == "array":
        value = [record]
        schema = {"type": "array", "items": record_schema}
        label = "settings[0]"
    elif container == "additional_property":
        value = {"adapter": record}
        schema = {"type": "object", "additionalProperties": record_schema}
        label = "settings.adapter"
    else:
        value = {"adapter": record}
        schema = {"type": "object", "properties": {"adapter": record_schema}}
        label = "settings.adapter"

    if required and not record:
        with pytest.raises(ValueError) as error:
            schema_value(value, schema, "settings")
        assert str(error.value) == f"{label} has missing required fields: ['reason']"
    else:
        schema_value(value, schema, "settings")
