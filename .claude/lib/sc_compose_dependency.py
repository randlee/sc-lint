"""Single source of truth for the sc-compose dependency contract."""

from __future__ import annotations

import re


MIN_SC_COMPOSE = (1, 7, 0)
MAX_SC_COMPOSE = (2, 0, 0)
SC_COMPOSE_VERSION_RANGE = ">=1.7.0,<2"
MIN_SC_COMPOSE_TEXT = SC_COMPOSE_VERSION_RANGE
MIN_SC_COMPOSE_BINDING = MIN_SC_COMPOSE
MIN_SC_COMPOSE_BINDING_TEXT = SC_COMPOSE_VERSION_RANGE
SC_COMPOSE_INSTALL = (
    "install or upgrade sc-compose to a compatible released version "
    f"({SC_COMPOSE_VERSION_RANGE}) without downgrading a compatible installation"
)
SC_COMPOSE_BINDING_INSTALL = (
    "python3 -m pip install --user --break-system-packages "
    f"'sc-compose{SC_COMPOSE_VERSION_RANGE}'"
)
_VERSION_RE = re.compile(
    r"(?<!\d)(\d+)\.(\d+)\.(\d+)(?:[-+][0-9A-Za-z.-]+)?"
)


def parse_version(text: str | None) -> tuple[int, int, int] | None:
    """Extract a comparable semantic version from tool output."""

    if not text:
        return None
    match = _VERSION_RE.search(text)
    return tuple(int(part) for part in match.groups()) if match else None


def is_supported_version(version: tuple[int, int, int] | None) -> bool:
    """Return whether a parsed sc-compose version is in the consumer range."""

    return version is not None and MIN_SC_COMPOSE <= version < MAX_SC_COMPOSE
