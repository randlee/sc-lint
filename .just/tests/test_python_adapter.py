from __future__ import annotations

import json
from pathlib import Path
import sys
import unittest


JUST_DIR = Path(__file__).resolve().parents[1]
if str(JUST_DIR) not in sys.path:
    sys.path.insert(0, str(JUST_DIR))

from python_adapter import ADAPTER_SCHEMA
from python_adapter import AdapterError
from python_adapter import error_payload
from python_adapter import success_payload


class PythonAdapterTests(unittest.TestCase):
    def test_adapter_error_retains_message(self) -> None:
        error = AdapterError("config", "bad adapter config")

        self.assertEqual(error.kind, "config")
        self.assertEqual(error.message, "bad adapter config")

    def test_success_payload_shape_is_stable(self) -> None:
        payload = success_payload(summary="ok", data={"status": "pass"})

        self.assertEqual(payload["adapter_schema"], ADAPTER_SCHEMA)
        self.assertTrue(payload["ok"])
        self.assertEqual(payload["summary"], "ok")
        self.assertEqual(payload["data"]["status"], "pass")
        self.assertEqual(payload["diagnostics"], [])

    def test_error_payload_shape_is_stable(self) -> None:
        payload = error_payload(
            AdapterError(
                "backend_protocol",
                "adapter failure",
                details={"tool": "sample"},
                suggested_action="fix it",
            )
        )

        self.assertEqual(payload["adapter_schema"], ADAPTER_SCHEMA)
        self.assertFalse(payload["ok"])
        self.assertEqual(payload["error"]["kind"], "backend_protocol")
        self.assertEqual(payload["error"]["message"], "adapter failure")
        self.assertEqual(payload["error"]["details"]["tool"], "sample")
        self.assertEqual(payload["error"]["suggested_action"], "fix it")

    def test_adapter_schema_constant_is_stable(self) -> None:
        self.assertEqual(ADAPTER_SCHEMA, "sc-lint-python-v1")

    def test_payload_round_trip_is_json_stable(self) -> None:
        payload = success_payload(summary="round trip", data={"artifact": "ok"})
        parsed = json.loads(json.dumps(payload))

        self.assertEqual(parsed["adapter_schema"], ADAPTER_SCHEMA)
        self.assertTrue(parsed["ok"])
        self.assertEqual(parsed["data"]["artifact"], "ok")


if __name__ == "__main__":
    unittest.main()
