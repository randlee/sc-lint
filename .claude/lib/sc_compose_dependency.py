"""Single source of truth for the sc-compose dependency contract."""

from __future__ import annotations

import re


MIN_SC_COMPOSE = (1, 7, 0)
MIN_SC_COMPOSE_TEXT = ">= 1.7.0"
SC_COMPOSE_WHEEL_PIN = "1.7.0"
MIN_SC_COMPOSE_BINDING_TEXT = f"== {SC_COMPOSE_WHEEL_PIN}"
SC_COMPOSE_INSTALL = "install a released sc-compose CLI at or above 1.7.0"
SC_COMPOSE_BINDING_INSTALL = (
    "python3 -m pip install --user --break-system-packages "
    f"'sc-compose=={SC_COMPOSE_WHEEL_PIN}'"
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
