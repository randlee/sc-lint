#!/usr/bin/env python3
from __future__ import annotations

SECTIONS = (
    (
        "General",
        (
            ("help", "Show this help."),
            ("build", "Build the full workspace."),
            ("test", "Run the full source-maintainer test suite."),
            ("clean", "Remove workspace build artifacts."),
            ("setup", "Preview consumer setup against sc-lint.toml (dry run)."),
            ("upgrade", "Check consumer upgrade state (dry run)."),
            ("version [current|latest]", "Show workspace version state, or latest recommended dependency upgrades."),
            ("test-adoption", "Run the consumer-adoption kit tests (needs pytest on the host Python)."),
            ("test-adoption-integration", "Run the adoption tests against a product built into target/adoption-product."),
        ),
    ),
    (
        "Formatting",
        (
            ("fmt [check|write]", "Check Rust formatting (default), or format the workspace in place."),
        ),
    ),
    (
        "Lint",
        (
            ("lint", "Run the default source lint profile through the consumer contract."),
            ("lint <selector>", "Run one named lint step or profile from sc-lint.toml (the source repo defines source-full)."),
            ("lint <selector> <scope>", "Also select the platform scope: host, all, linux, macos, or windows."),
        ),
    ),
)


def render_help() -> str:
    lines = [
        "sc-lint repository task runner",
        "",
        "Usage:",
        "  just <recipe>",
        "",
    ]
    width = max(len(name) for _, recipes in SECTIONS for name, _ in recipes)
    for section_name, recipes in SECTIONS:
        lines.append(f"{section_name}:")
        for name, description in recipes:
            lines.append(f"  {name.ljust(width)}  {description}")
        lines.append("")
    return "\n".join(lines).rstrip() + "\n"


def main() -> int:
    print(render_help(), end="")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
