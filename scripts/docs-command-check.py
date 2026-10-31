#!/usr/bin/env python3
"""Fail when declarative docs claim a `zk` command/subcommand that does not exist.

`docs-check.py` validates structure/links only. This gate knows the real CLI
surface and fails on unknown commands (including unknown subcommands such as
`zk db reset`) unless the mention is explicitly marked as backlog/planned near
it. Historical reviews/research/archive are skipped.
"""
from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
# Implemented top-level commands and their allowed subcommands.
IMPLEMENTED: dict[str, set[str] | None] = {
    "validate": None,
    "compose": None,
    "plan": None,
    "docs": None,
    "capability": None,
    "find": None,
    "lint": None,
    "channel": {"validate", "verify"},
    "db": {"lock"},
    "sbom": None,
}
BACKLOG_MARKERS = ("未实现", "backlog", "计划", "待实现", "尚未实现", "planned")
SKIP_DIRS = {"reviews", "research", "archive", "generated"}
PATTERN = re.compile(r"\bzk\s+([a-zA-Z][a-zA-Z0-9_-]*)(?:\s+([a-zA-Z][a-zA-Z0-9_-]*))?")
WINDOW = 500


def annotate_error(text: str, start: int, end: int) -> bool:
    window = text[max(0, start - WINDOW) : min(len(text), end + WINDOW)]
    return any(marker in window for marker in BACKLOG_MARKERS)


def scan_text(text: str) -> list[str]:
    errors: list[str] = []
    for match in PATTERN.finditer(text):
        command = match.group(1)
        subcommand = match.group(2)
        if command not in IMPLEMENTED:
            if not annotate_error(text, match.start(), match.end()):
                errors.append(f"`zk {command}` is not implemented and not marked as backlog")
            continue
        allowed = IMPLEMENTED[command]
        if allowed is not None:
            if subcommand is None or subcommand not in allowed:
                if not annotate_error(text, match.start(), match.end()):
                    shown = command if subcommand is None else f"{command} {subcommand}"
                    errors.append(f"`zk {shown}` is not an implemented subcommand and not marked as backlog")
    return errors


def scan(path: Path) -> list[str]:
    try:
        text = path.read_text(encoding="utf-8")
    except OSError as error:
        return [f"{path}: unreadable: {error}"]
    return [f"{path}: {error}" for error in scan_text(text)]


def self_test() -> int:
    cases = [
        ("zk db lock --check", 0),
        ("zk validate", 0),
        ("zk sbom --edition personal", 0),
        ("zk db reset", 1),
        ("zk init", 1),
        ("zk api --check", 1),
        ("zk api --check  # 未实现（backlog）", 0),
        ("`zk db reset`（未实现）", 0),
    ]
    failures = 0
    for text, expected in cases:
        actual = 1 if scan_text(text) else 0
        if actual != expected:
            print(f"self-test failed: {text!r} expected {expected} got {actual}", file=sys.stderr)
            failures += 1
    if failures:
        return 1
    print("docs-command-check self-test: ok")
    return 0


def main() -> int:
    if "--self-test" in sys.argv:
        return self_test()
    errors: list[str] = []
    targets = [path for path in sorted(ROOT.glob("docs/*.md")) if path.parent.name not in SKIP_DIRS and not path.name.startswith("14-")]
    for extra in ("AGENTS.md", "README.md"):
        candidate = ROOT / extra
        if candidate.is_file():
            targets.append(candidate)
    for path in targets:
        errors.extend(scan(path))
    if errors:
        for error in errors:
            print(f"docs-command-check: {error}", file=sys.stderr)
        print(f"docs-command-check: {len(errors)} error(s)", file=sys.stderr)
        return 1
    print("docs-command-check: ok")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
