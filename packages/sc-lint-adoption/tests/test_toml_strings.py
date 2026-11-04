"""Consumer TOML retains Unicode scalar values and escaped control characters."""
from pathlib import Path
import runpy
import tomllib

INSTALL = runpy.run_path(str(Path(__file__).resolve().parents[1] / "install.py"))


def test_unicode_and_control_characters_roundtrip_in_values_and_keys():
    value = "🚀 start " + "".join(chr(code) for code in range(32)) + "\x7f"
    encoded = INSTALL["toml_value"]({value: [value]})
    assert tomllib.loads("entry = " + encoded)["entry"] == {value: [value]}
    key = INSTALL["toml_key"](value)
    assert tomllib.loads(key + " = " + INSTALL["toml_value"](value)) == {value: value}
