#!/usr/bin/env python3
from __future__ import annotations

from dataclasses import dataclass
from typing import Any
import json


ADAPTER_SCHEMA = "sc-lint-python-v1"


@dataclass(frozen=True)
class AdapterError(Exception):
    kind: str
    message: str
    details: dict[str, Any] | None = None
    suggested_action: str | None = None


def success_payload(*, summary: str, data: dict[str, Any], diagnostics: list[str] | None = None) -> dict[str, Any]:
    return {
        "adapter_schema": ADAPTER_SCHEMA,
        "ok": True,
        "summary": summary,
        "data": data,
        "diagnostics": diagnostics or [],
    }


def error_payload(error: AdapterError, diagnostics: list[str] | None = None) -> dict[str, Any]:
    payload: dict[str, Any] = {
        "adapter_schema": ADAPTER_SCHEMA,
        "ok": False,
        "error": {
            "kind": error.kind,
            "message": error.message,
        },
        "diagnostics": diagnostics or [],
    }
    if error.details:
        payload["error"]["details"] = error.details
    if error.suggested_action:
        payload["error"]["suggested_action"] = error.suggested_action
    return payload


def write_json(payload: dict[str, Any]) -> None:
    print(json.dumps(payload, indent=2, sort_keys=True))
