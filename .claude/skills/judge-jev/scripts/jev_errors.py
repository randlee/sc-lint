"""jev_errors.py: the one ``JudgeError`` every script returns in ``Err``, and the redaction its envelope goes through.

``jev_judge`` and ``jev_graph`` import this module, so an error built in the graph layer reaches the CLI envelope as it is:
same class, same code, message, action and data. Standard library only.
"""

from __future__ import annotations

import os
import re


BEARER_RE = re.compile(r"(?i)(bearer\s+)[^\s\"']+")


def _redact_text(value):
    if not isinstance(value, str):
        return value
    text = value
    api_key = os.environ.get("TYPESAFE_API_KEY", "")
    if api_key:
        text = text.replace(api_key, "<redacted>")
    return BEARER_RE.sub(r"\1<redacted>", text)


def redact_sensitive(value):
    """Remove the live TypeSafe credential and bearer tokens from every emitted value. Iterative (an explicit stack, no
    recursion): a container nested a thousand deep is walked like any other and never raises RecursionError."""
    if not isinstance(value, (dict, list, tuple)):
        return _redact_text(value)
    # a frame is [source container, iterator over its items, output container, key under which it joins its parent]
    stack = [[value, iter(value.items() if isinstance(value, dict) else value), {} if isinstance(value, dict) else [], None]]
    while True:
        frame = stack[-1]
        source, items, out, key = frame
        try:
            item = next(items)
        except StopIteration:
            stack.pop()
            done = tuple(out) if isinstance(source, tuple) else out
            if not stack:
                return done
            parent = stack[-1]
            if isinstance(parent[2], dict):
                parent[2][key] = done
            else:
                parent[2].append(done)
            continue
        child_key, child = item if isinstance(source, dict) else (None, item)
        if isinstance(child, (dict, list, tuple)):
            stack.append([child, iter(child.items() if isinstance(child, dict) else child), {} if isinstance(child, dict) else [], child_key])
        elif isinstance(source, dict):
            out[child_key] = _redact_text(child)
        else:
            out.append(_redact_text(child))


class JudgeError:
    """An expected failure, carried in Err. Never raised: main() turns it into the error envelope and exit 2."""

    def __init__(self, code: str, message: str, recoverable: bool = False, action: str = "",
                 status=None, retry_after=None):
        self.code = code
        self.message = message
        self.recoverable = recoverable
        self.action = action
        self.status = status            # HTTP status when the error came from Jev
        self.retry_after = retry_after  # parsed Retry-After header, seconds, or None
        self.meta = {}                  # attempt / shortening bookkeeping, filled by judge_with_policy
        self.data = None                # optional facts that explain the error (PLAN.TOO_LARGE carries the bead's shape)

    def __str__(self) -> str:
        return self.message

    def __repr__(self) -> str:
        return f"JudgeError({self.code!r}, {self.message!r})"

    def to_json(self) -> dict:
        return redact_sensitive({
            "success": False,
            "data": self.data,
            "error": {
                "code": self.code,
                "message": str(self),
                "recoverable": self.recoverable,
                "suggested_action": self.action,
            },
        })
