#!/usr/bin/env python3
"""Validation helpers for the four F.1-owned configure contracts.

This module intentionally contains no configure field definitions.  The JSON
Schema files remain the sole public authority for context, request, plan, and
result payloads.
"""

from __future__ import annotations

from dataclasses import dataclass
import json
from pathlib import Path
from typing import Any

from jsonschema import Draft202012Validator


REPOSITORY_ROOT = Path(__file__).resolve().parent.parent
SCHEMA_DIRECTORY = REPOSITORY_ROOT / "schemas"


@dataclass(frozen=True)
class SchemaProblem:
    """One schema-validation problem rendered with an RFC 6901-style pointer."""

    pointer: str
    message: str


def schema_path(contract: str) -> Path:
    """Return the F.1-owned schema path for a named configure contract."""
    return SCHEMA_DIRECTORY / f"sc-lint-configure-{contract}.schema.json"


def load_schema(contract: str) -> dict[str, Any]:
    """Load one public configure schema without copying its shape into Python."""
    return json.loads(schema_path(contract).read_text(encoding="utf-8"))


def validate(contract: str, instance: object) -> list[SchemaProblem]:
    """Return deterministic schema errors for an F.1 contract instance."""
    validator = Draft202012Validator(load_schema(contract))
    errors = sorted(validator.iter_errors(instance), key=_error_sort_key)
    return [
        SchemaProblem(pointer=_pointer(error.absolute_path), message=error.message)
        for error in errors
    ]


def _error_sort_key(error: Any) -> tuple[str, str]:
    return (_pointer(error.absolute_path), error.message)


def _pointer(parts: object) -> str:
    escaped = [str(part).replace("~", "~0").replace("/", "~1") for part in parts]
    return "/" + "/".join(escaped) if escaped else "/"
