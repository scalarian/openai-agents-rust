#!/usr/bin/env python3
"""Generate the exhaustive behavior parity ledger from pinned upstream test trees."""

from __future__ import annotations

import json
from collections import OrderedDict
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parent.parent
DOCS_ROOT = REPO_ROOT / "docs"
PYTHON_TESTS = REPO_ROOT / "reference" / "openai-agents-python" / "tests"
JS_PACKAGES = REPO_ROOT / "reference" / "openai-agents-js" / "packages"
OVERRIDES_PATH = DOCS_ROOT / "behavior_parity_overrides.json"
OUTPUT_PATH = DOCS_ROOT / "BEHAVIOR_PARITY.md"


DEFAULT_STATUS = "omitted-with-rationale"
DEFAULT_COVERAGE = ["n/a"]
DEFAULT_NOTE = (
    "Tracked upstream family; Rust parity is not yet closed for this family in the current "
    "runtime audit."
)


SECTION_ORDER = [
    "Core Runner",
    "Agent / Tool",
    "Sessions",
    "Model Settings / Providers",
    "OpenAI",
    "MCP",
    "Realtime",
    "Voice",
    "Tracing",
    "Extensions",
    "JS Package Families",
]


def python_families() -> list[str]:
    families: list[str] = []
    for path in sorted(PYTHON_TESTS.rglob("test_*.py")):
        families.append(path.relative_to(PYTHON_TESTS).with_suffix("").as_posix())
    return families


def js_families() -> list[str]:
    families: list[str] = []
    for path in sorted(JS_PACKAGES.glob("*/test/**/*.test.ts")):
        package = path.relative_to(JS_PACKAGES).parts[0]
        relative = path.relative_to(JS_PACKAGES / package / "test").as_posix()
        suffix = ".test.ts"
        if not relative.endswith(suffix):
            continue
        families.append(f"js/{package}/{relative[:-len(suffix)]}")
    return families


def section_for_family(family: str) -> str:
    if family.startswith("js/"):
        return "JS Package Families"
    if family.startswith("extensions/") or family in {"test_visualization", "test_extension_filters"}:
        return "Extensions"
    if family.startswith("voice/"):
        return "Voice"
    if family.startswith("realtime/"):
        return "Realtime"
    if family.startswith("mcp/"):
        return "MCP"
    if family.startswith("models/") or family.startswith("model_settings/"):
        return "Model Settings / Providers"
    if family.startswith("tracing/") or family.startswith("test_trace") or family.startswith("test_tracing"):
        return "Tracing"
    if family.startswith("memory/") or family.startswith("test_session") or family.startswith("fastapi/"):
        return "Sessions"
    if family.startswith("test_openai") or family.startswith("test_responses") or family.startswith("test_server_conversation_tracker"):
        return "OpenAI"
    if any(
        family.startswith(prefix)
        for prefix in (
            "test_agent",
            "test_function",
            "test_tool",
            "test_handoff",
            "test_apply",
            "test_shell",
            "test_computer",
            "test_output_tool",
            "test_local_shell_tool",
            "test_visualization",
        )
    ):
        return "Agent / Tool"
    return "Core Runner"


def load_overrides() -> dict[str, dict[str, object]]:
    return json.loads(OVERRIDES_PATH.read_text(encoding="utf-8"))


def build_rows() -> OrderedDict[str, list[tuple[str, dict[str, object]]]]:
    overrides = load_overrides()
    rows: OrderedDict[str, list[tuple[str, dict[str, object]]]] = OrderedDict(
        (section, []) for section in SECTION_ORDER
    )
    for family in python_families() + js_families():
        row = {
            "status": DEFAULT_STATUS,
            "coverage": DEFAULT_COVERAGE,
            "notes": DEFAULT_NOTE,
        }
        row.update(overrides.get(family, {}))
        rows[section_for_family(family)].append((family, row))
    return rows


def render_table(section: str, rows: list[tuple[str, dict[str, object]]]) -> str:
    lines = [f"### {section}", "", "| Family | Status | Rust coverage | Notes |", "| --- | --- | --- | --- |"]
    for family, row in rows:
        coverage = ", ".join(f"`{path}`" for path in row["coverage"])
        lines.append(
            f"| `{family}` | `{row['status']}` | {coverage} | {row['notes']} |"
        )
    lines.append("")
    return "\n".join(lines)


def main() -> None:
    rows = build_rows()
    tracked = sum(len(section_rows) for section_rows in rows.values())
    output = [
        "# Behavior Parity",
        "",
        "This document is generated from the pinned Python and JS test trees plus",
        "`docs/behavior_parity_overrides.json`.",
        "",
        "Allowed statuses:",
        "",
        "- `covered`: there is Rust coverage for the family and the runtime surface is materially present",
        "- `omitted-with-rationale`: intentionally not closed yet or environment-specific; the omission is explicit",
        "",
        f"Tracked upstream families: `{tracked}`",
        "",
    ]
    for section, section_rows in rows.items():
        output.append(render_table(section, section_rows))
    OUTPUT_PATH.write_text("\n".join(output).rstrip() + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
