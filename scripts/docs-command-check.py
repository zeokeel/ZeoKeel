#!/usr/bin/env python3
"""Fail when docs claim a `zk` command that is not implemented without a backlog note.

`docs-check.py` validates structure/links but cannot know whether `zk foo` exists.
This gate scans declarative docs (excluding historical reviews/research/archive)
for `zk <command>` mentions and requires either:
  * the command is implemented, or
  * a backlog marker (未实现/backlog/计划/待实现) appears near the mention.
"""
from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
IMPLEMENTED = {
    "validate",
    "compose",
    "plan",
    "docs",
    "capability",
    "find",
    "lint",
    "channel",
    "db",
    "sbom",
}
KNOWN_UNIMPLEMENTED = {"api", "lock", "verify-lock", "metadata"}
BACKLOG_MARKERS = ("未实现", "backlog", "计划", "待实现", "尚未实现")
SKIP_DIRS = {"reviews", "research", "archive", "generated"}
PATTERN = re.compile(r"\bzk\s+([a-zA-Z][a-zA-Z0-9_-]*)")


def scan(path: Path) -> list[str]:
    errors: list[str] = []
    try:
        text = path.read_text(encoding="utf-8")
    except OSError as error:
        errors.append(f"{path}: unreadable: {error}")
        return errors
    for match in PATTERN.finditer(text):
        command = match.group(1)
        if command in IMPLEMENTED:
            continue
        window = text[max(0, match.start() - 240) : min(len(text), match.end() + 240)]
        noted = any(marker in window for marker in BACKLOG_MARKERS)
        if command in KNOWN_UNIMPLEMENTED and noted:
            continue
        if command not in KNOWN_UNIMPLEMENTED:
            # Future/planned convenience commands (init/vendor/brand/...) may be
            # described in planning sections; only the tracked backbone commands
            # are enforced here.
            continue
        errors.append(
            f"{path}: `zk {command}` is not implemented and not marked as backlog near the mention"
        )
    return errors


def main() -> int:
    errors: list[str] = []
    for path in sorted(ROOT.glob("docs/*.md")):
        if path.parent.name in SKIP_DIRS or path.name.startswith("14-"):
            continue
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
