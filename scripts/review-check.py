#!/usr/bin/env python3
"""C19 review gate: approved specs must link a review record whose latest conclusion is APPROVED."""
from __future__ import annotations
import re, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
REVIEWS = ROOT / "docs" / "reviews"
CHANGES = ROOT / "specs" / "changes"
LEGACY = {"2026-10-30-m0-scaffold-minimal-demo", "2026-10-30-m1-personal-mvp"}
errors: list[str] = []

def read(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except OSError as exc:
        errors.append(f"{path}: cannot read ({exc})")
        return ""

def latest_conclusion(body: str) -> str | None:
    """Last line that states a conclusion token; avoids matching prose mentions."""
    found = None
    for line in body.splitlines():
        if "结论" in line:
            m = re.search(r"\b(APPROVED|CHANGES-REQUESTED)\b", line)
            if m:
                found = m.group(1)
    return found

for proposal in sorted(CHANGES.glob("*/proposal.md")):
    text = read(proposal)
    m = re.search(r"^-\s*状态[:：]\s*`?([a-z-]+)`?", text, re.M)
    status = m.group(1) if m else "unknown"
    if status not in {"approved", "in-progress", "done"}:
        continue
    rm = re.search(r"^-\s*评审[:：]\s*(.+)$", text, re.M)
    if not rm:
        errors.append(f"{proposal}: missing '- 评审：' (C19)")
        continue
    ref = rm.group(1).strip().strip("`")
    if ref.startswith("n/a"):
        if proposal.parent.name not in LEGACY or "C19 前遗留" not in ref:
            errors.append(f"{proposal}: 'n/a' review only allowed for pre-C19 legacy specs")
        continue
    path_text = ref.split("#", 1)[0].strip()
    record = ROOT / path_text
    if not record.is_file():
        errors.append(f"{proposal}: review record not found: {path_text}")
        continue
    body = read(record)
    if "关联 spec" not in body or proposal.parent.name not in body:
        errors.append(f"{record}: review record must name '关联 spec' and change id {proposal.parent.name}")
    conclusion = latest_conclusion(body)
    if conclusion != "APPROVED":
        errors.append(f"{proposal}: latest review conclusion is {conclusion or 'missing'} (need APPROVED)")
    for field in ("评审对象", "评审者", "结论"):
        if field not in body:
            errors.append(f"{record}: missing review field '{field}'")

for record in sorted(REVIEWS.glob("*.md")):
    if record.name == "README.md":
        continue
    body = read(record)
    for field in ("评审对象", "评审者", "结论"):
        if field not in body:
            errors.append(f"{record}: missing review field '{field}'")

if errors:
    for e in errors:
        print(f"review-check: {e}", file=sys.stderr)
    print(f"review-check: {len(errors)} error(s)", file=sys.stderr)
    sys.exit(1)
print(f"review-check: ok ({len(list(REVIEWS.glob('*.md')))} records)")
