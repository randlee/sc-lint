"""TOML parsing for the sc_lint helpers.

The helpers parse TOML through ``sc_lint._native.toml_loads``, the same workspace
``toml`` crate the sc-lint CLI uses, so they run on Python 3.9 and 3.10 and
share the CLI's parser. A source checkout that has not built the native
extension (before ``just setup``) falls back to ``tomllib`` on Python 3.11 and
newer. With neither available, parsing raises ``ImportError`` with a recovery
message.

Both paths return the same value mapping:

- Datetime values become text. The native path returns the RFC 3339 text from
  the TOML source; the ``tomllib`` fallback returns ``isoformat()``, which can
  differ in offset spelling (``+00:00`` for ``Z``).
- Non-finite floats (``inf``, ``-inf``, ``nan``) become ``None``.
"""

import datetime
import json
import math

try:
    from sc_lint._native import toml_loads as _native_toml_loads
except ImportError:  # the extension is not built, or predates toml_loads
    _native_toml_loads = None
    try:
        import tomllib as _tomllib
    except ImportError:  # Python 3.9 and 3.10 have no tomllib
        _tomllib = None
else:
    _tomllib = None


class TOMLDecodeError(ValueError):
    """Raised when TOML text cannot be parsed."""


def loads(text: str) -> dict:
    """Parse TOML text into a dictionary."""
    if _native_toml_loads is not None:
        try:
            payload = _native_toml_loads(text)
        except ValueError as error:
            raise TOMLDecodeError(str(error)) from error
        return json.loads(payload)
    if _tomllib is None:
        raise ImportError(
            "TOML parsing needs the sc_lint native extension (sc_lint._native.toml_loads) "
            "or Python 3.11+ tomllib; run `just setup` or `maturin develop` to build the extension"
        )
    try:
        return _normalize(_tomllib.loads(text))
    except _tomllib.TOMLDecodeError as error:
        raise TOMLDecodeError(str(error)) from error


def load(source) -> dict:
    """Parse TOML from a binary file object opened in ``"rb"`` mode."""
    return loads(source.read().decode("utf-8"))


def _normalize(value):
    """Map ``tomllib`` values onto the native mapping: datetimes to text, non-finite floats to ``None``."""
    if isinstance(value, dict):
        return {key: _normalize(item) for key, item in value.items()}
    if isinstance(value, list):
        return [_normalize(item) for item in value]
    if isinstance(value, float) and not math.isfinite(value):
        return None
    if isinstance(value, (datetime.date, datetime.time)):
        return value.isoformat()
    return value
