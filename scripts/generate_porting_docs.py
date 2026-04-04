#!/usr/bin/env python3
"""Generate the Python-to-Rust porting matrix from pinned reference repos."""

from __future__ import annotations

import ast
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parent.parent
PYTHON_ROOT = REPO_ROOT / "reference" / "openai-agents-python"
JS_ROOT = REPO_ROOT / "reference" / "openai-agents-js"
DOCS_ROOT = REPO_ROOT / "docs"
PYTHON_SRC = PYTHON_ROOT / "src" / "agents"
PYTHON_TESTS = PYTHON_ROOT / "tests"

MATRIX_PATH = DOCS_ROOT / "PORTING_MATRIX.md"


RENAME_MAP: dict[str, tuple[str, str, str]] = {
    "__init__.py": (
        "openai-agents",
        "crates/openai-agents/src/lib.rs",
        "Facade re-exports the stable surface from internal crates.",
    ),
    "py.typed": (
        "openai-agents",
        "crates/openai-agents/src/lib.rs",
        "Rust has no runtime typing marker equivalent.",
    ),
    "tool.py": (
        "agents-core + agents-openai",
        "crates/agents-core/src/tool.rs + crates/agents-openai/src/tools.rs",
        "Generic tool abstractions and OpenAI hosted tools split across crates.",
    ),
    "responses_websocket_session.py": (
        "agents-openai",
        "crates/agents-openai/src/websocket.rs",
        "OpenAI transport-specific session lives in the OpenAI crate.",
    ),
    "models/openai_provider.py": (
        "agents-openai",
        "crates/agents-openai/src/provider.rs",
        "Provider entry point becomes a dedicated OpenAI provider module.",
    ),
    "models/openai_responses.py": (
        "agents-openai",
        "crates/agents-openai/src/models.rs",
        "Responses model is grouped with the other OpenAI model implementations.",
    ),
    "models/openai_chatcompletions.py": (
        "agents-openai",
        "crates/agents-openai/src/models.rs",
        "Chat Completions model is grouped with the other OpenAI model implementations.",
    ),
    "models/default_models.py": (
        "agents-openai",
        "crates/agents-openai/src/defaults.rs",
        "Default OpenAI API/model resolution belongs in the OpenAI crate.",
    ),
    "memory/openai_conversations_session.py": (
        "agents-openai",
        "crates/agents-openai/src/memory.rs",
        "OpenAI conversation storage is OpenAI-specific state management.",
    ),
    "memory/openai_responses_compaction_session.py": (
        "agents-openai",
        "crates/agents-openai/src/memory.rs",
        "Responses compaction is OpenAI-specific state management.",
    ),
    "extensions/memory/sqlalchemy_session.py": (
        "agents-extensions",
        "crates/agents-extensions/src/memory/database_session.rs",
        "Capability preserved as a Rust-native database-backed session.",
    ),
    "extensions/memory/encrypt_session.py": (
        "agents-extensions",
        "crates/agents-extensions/src/memory/encrypted_session.rs",
        "Capability preserved as a Rust-native encrypted session wrapper.",
    ),
    "handoffs/__init__.py": (
        "agents-core",
        "crates/agents-core/src/handoff/mod.rs",
        "Plural Python package becomes singular Rust module tree.",
    ),
    "handoffs/history.py": (
        "agents-core",
        "crates/agents-core/src/handoff/history.rs",
        "Plural Python package becomes singular Rust module tree.",
    ),
    "run_internal/__init__.py": (
        "agents-core",
        "crates/agents-core/src/internal/mod.rs",
        "Runner internals stay crate-private under an internal module tree.",
    ),
    "realtime/README.md": (
        "agents-realtime",
        "crates/agents-realtime/README.md",
        "Runtime notes move from an in-package README to crate docs.",
    ),
    "voice/imports.py": (
        "agents-voice",
        "crates/agents-voice/src/lib.rs",
        "Python optional-import helper has no direct Rust analogue.",
    ),
}


OPENAI_MODEL_FILES = {
    "models/_openai_retry.py",
    "models/_openai_shared.py",
    "models/_retry_runtime.py",
    "models/chatcmpl_converter.py",
    "models/chatcmpl_helpers.py",
    "models/chatcmpl_stream_handler.py",
    "models/default_models.py",
    "models/fake_id.py",
    "models/openai_chatcompletions.py",
    "models/openai_provider.py",
    "models/openai_responses.py",
    "models/reasoning_content_replay.py",
    "responses_websocket_session.py",
}


def python_files() -> list[Path]:
    return sorted(path for path in PYTHON_SRC.rglob("*") if path.is_file())


def relative_to_src(path: Path) -> str:
    return path.relative_to(PYTHON_SRC).as_posix()


def extract_symbols(path: Path) -> list[str]:
    if path.suffix == ".py":
        source = path.read_text(encoding="utf-8")
        tree = ast.parse(source)
        symbols: list[str] = []
        for node in tree.body:
            if isinstance(node, (ast.ClassDef, ast.FunctionDef, ast.AsyncFunctionDef)):
                if not node.name.startswith("_"):
                    symbols.append(node.name)
            elif isinstance(node, ast.ImportFrom) and path.name == "__init__.py":
                for alias in node.names:
                    if alias.name == "*":
                        continue
                    name = alias.asname or alias.name
                    if not name.startswith("_"):
                        symbols.append(name)
        seen: list[str] = []
        for symbol in symbols:
            if symbol not in seen:
                seen.append(symbol)
        return seen
    if path.name == "py.typed":
        return ["typing marker"]
    if path.suffix == ".md":
        return ["documentation"]
    return []


def format_symbols(symbols: list[str]) -> str:
    if not symbols:
        return "-"
    if len(symbols) <= 8:
        return ", ".join(symbols)
    return ", ".join(symbols[:8]) + f", … (+{len(symbols) - 8} more)"


def find_tests(relative_path: str) -> str:
    stem = Path(relative_path).stem
    parent = Path(relative_path).parent.name
    matches: list[str] = []
    for test_path in PYTHON_TESTS.rglob("test_*.py"):
        if test_path.name == f"test_{stem}.py":
            matches.append(test_path.relative_to(PYTHON_ROOT).as_posix())
        elif parent and test_path.parent.name == parent and test_path.name == f"test_{stem}.py":
            matches.append(test_path.relative_to(PYTHON_ROOT).as_posix())
    deduped: list[str] = []
    for match in matches:
        if match not in deduped:
            deduped.append(match)
    return ", ".join(deduped[:3]) if deduped else "-"


def map_target(relative_path: str) -> tuple[str, str, str, str]:
    if relative_path in RENAME_MAP:
        crate, target, note = RENAME_MAP[relative_path]
        status = "deferred-with-rationale" if relative_path in {"py.typed", "voice/imports.py"} else "renamed"
        if relative_path == "__init__.py":
            status = "folded"
        return crate, target, status, note

    parts = Path(relative_path).parts
    stem = Path(relative_path).stem

    if parts[0] == "extensions":
        crate = "agents-extensions"
        if parts[1] == "experimental":
            target = f"crates/agents-extensions/src/{Path(relative_path).with_suffix('.rs').as_posix()}"
            return crate, target, "direct", "Experimental APIs stay in the extensions crate."
        if parts[1] == "models":
            target = f"crates/agents-extensions/src/providers/{stem}.rs"
            return crate, target, "renamed", "Non-core provider adapters live under providers/."
        if parts[1] == "memory":
            target = f"crates/agents-extensions/src/memory/{stem}.rs"
            return crate, target, "direct", "Optional session backends stay in the extensions crate."
        target = f"crates/agents-extensions/src/{Path(relative_path).with_suffix('.rs').as_posix()}"
        return crate, target, "direct", "Optional extension preserved as a dedicated module."

    if parts[0] == "realtime":
        crate = "agents-realtime"
        if parts[-1] == "__init__.py":
            return crate, "crates/agents-realtime/src/lib.rs", "folded", "Package exports fold into crate root."
        return crate, f"crates/agents-realtime/src/{Path(*parts[1:]).with_suffix('.rs').as_posix()}", "direct", "Realtime subsystem maps to the realtime crate."

    if parts[0] == "voice":
        crate = "agents-voice"
        if parts[-1] == "__init__.py":
            return crate, "crates/agents-voice/src/lib.rs", "folded", "Package exports fold into crate root."
        return crate, f"crates/agents-voice/src/{Path(*parts[1:]).with_suffix('.rs').as_posix()}", "direct", "Voice subsystem maps to the voice crate."

    if parts[0] == "memory":
        if relative_path.startswith("memory/openai_"):
            return (
                "agents-openai",
                f"crates/agents-openai/src/{Path(relative_path).with_suffix('.rs').as_posix()}",
                "direct",
                "OpenAI-specific memory remains in the OpenAI crate.",
            )
        if parts[-1] == "__init__.py":
            return "agents-core", "crates/agents-core/src/memory/mod.rs", "folded", "Package exports fold into a memory module tree."
        return "agents-core", f"crates/agents-core/src/{Path(relative_path).with_suffix('.rs').as_posix()}", "direct", "Core session abstractions and backends remain in the core crate."

    if relative_path in OPENAI_MODEL_FILES or parts[0] == "models" and relative_path.startswith("models/openai_"):
        return (
            "agents-openai",
            f"crates/agents-openai/src/{Path(relative_path).with_suffix('.rs').as_posix()}",
            "direct",
            "OpenAI transport and provider logic belongs in the OpenAI crate.",
        )

    if parts[0] == "models":
        if parts[-1] == "__init__.py":
            return "agents-core", "crates/agents-core/src/model/mod.rs", "folded", "Package exports fold into the core model module."
        return "agents-core", f"crates/agents-core/src/{Path(relative_path).with_suffix('.rs').as_posix()}", "direct", "Provider-agnostic model abstractions remain in the core crate."

    if parts[0] == "mcp":
        if parts[-1] == "__init__.py":
            return "agents-core", "crates/agents-core/src/mcp/mod.rs", "folded", "Package exports fold into the MCP module tree."
        return "agents-core", f"crates/agents-core/src/{Path(relative_path).with_suffix('.rs').as_posix()}", "direct", "MCP support remains in the core crate."

    if parts[0] == "tracing":
        if parts[-1] == "__init__.py":
            return "agents-core", "crates/agents-core/src/tracing/mod.rs", "folded", "Package exports fold into the tracing module tree."
        return "agents-core", f"crates/agents-core/src/{Path(relative_path).with_suffix('.rs').as_posix()}", "direct", "Tracing stays in the core crate."

    if parts[0] == "run_internal":
        return (
            "agents-core",
            f"crates/agents-core/src/internal/{Path(*parts[1:]).with_suffix('.rs').as_posix()}",
            "renamed",
            "Runner internals stay crate-private under internal/.",
        )

    if parts[0] == "handoffs":
        if parts[-1] == "__init__.py":
            return "agents-core", "crates/agents-core/src/handoff/mod.rs", "folded", "Plural package becomes a singular module tree."
        return "agents-core", f"crates/agents-core/src/handoff/{stem}.rs", "renamed", "Plural package becomes a singular module tree."

    if parts[-1] == "__init__.py":
        return "agents-core", "crates/agents-core/src/lib.rs", "folded", "Package exports fold into crate-level re-exports."

    return (
        "agents-core",
        f"crates/agents-core/src/{Path(relative_path).with_suffix('.rs').as_posix()}",
        "direct",
        "Core module maps directly into the core crate.",
    )


def python_head() -> str:
    return (PYTHON_ROOT / ".git" / "refs" / "heads" / "main").read_text(encoding="utf-8").strip()


def js_head() -> str:
    return (JS_ROOT / ".git" / "refs" / "heads" / "main").read_text(encoding="utf-8").strip()


def write_matrix() -> None:
    rows = []
    for path in python_files():
        relative_path = relative_to_src(path)
        crate, target, status, notes = map_target(relative_path)
        rows.append(
            {
                "python_path": f"src/agents/{relative_path}",
                "target_module": target,
                "target_crate": crate,
                "symbols": format_symbols(extract_symbols(path)),
                "tests": find_tests(relative_path),
                "notes": notes,
                "status": status,
            }
        )

    lines = [
        "# Porting Matrix",
        "",
        f"Generated from `reference/openai-agents-python` at `{python_head()}` and `reference/openai-agents-js` at `{js_head()}`.",
        "",
        f"Python source files mapped: **{len(rows)}**.",
        "",
        "| Python path | Target Rust module | Target crate | Exported symbols | Translated tests | Parity notes | Status |",
        "| --- | --- | --- | --- | --- | --- | --- |",
    ]

    for row in rows:
        lines.append(
            "| {python_path} | {target_module} | {target_crate} | {symbols} | {tests} | {notes} | {status} |".format(
                **{key: str(value).replace("|", "\\|") for key, value in row.items()}
            )
        )

    MATRIX_PATH.write_text("\n".join(lines) + "\n", encoding="utf-8")


if __name__ == "__main__":
    if not PYTHON_SRC.exists():
        raise SystemExit("Python reference repo is missing. Clone reference/openai-agents-python first.")
    if not JS_ROOT.exists():
        raise SystemExit("JS reference repo is missing. Clone reference/openai-agents-js first.")
    DOCS_ROOT.mkdir(parents=True, exist_ok=True)
    write_matrix()
