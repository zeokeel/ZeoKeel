#!/usr/bin/env python3
"""specs/ 变更规格门禁：模板完整、变更五件套、状态合法、任务有 verify。"""
from __future__ import annotations
import re, sys, pathlib

ROOT = pathlib.Path(__file__).resolve().parent.parent
SPECS = ROOT / "specs"
TEMPLATES = ["proposal", "spec", "design", "tasks", "acceptance"]
STATUSES = {"draft", "approved", "in-progress", "done", "abandoned"}
errors: list[str] = []
warns: list[str] = []

def err(m): errors.append(m)
def warn(m): warns.append(m)

def check_templates():
    for name in TEMPLATES:
        p = SPECS / "templates" / f"{name}.md"
        if not p.exists():
            err(f"缺失模板 specs/templates/{name}.md")
            continue
        if "<change-id>" not in p.read_text():
            warn(f"模板 {name}.md 未包含 <change-id> 占位符")

def check_change(d: pathlib.Path):
    name = d.name
    for f in TEMPLATES:
        p = d / f"{f}.md"
        if not p.exists():
            err(f"{name}: 缺少 {f}.md")
    proposal = d / "proposal.md"
    status = None
    if proposal.exists():
        m = re.search(r"状态[:：]\s*`?([a-z-]+)`?", proposal.read_text())
        if not m:
            err(f"{name}: proposal.md 缺少“状态：”")
        else:
            status = m.group(1)
            if status not in STATUSES:
                err(f"{name}: 非法状态 {status!r}")
    tasks = d / "tasks.md"
    if tasks.exists():
        text = tasks.read_text()
        if "| # | 任务 |" not in text or "验证" not in text:
            err(f"{name}: tasks.md 缺任务表/验证列")
    if status == "done":
        if not (d / "acceptance.md").exists():
            err(f"{name}: done 但缺 acceptance.md")
    elif status == "approved":
        pass
    elif status is None:
        pass
    else:
        warn(f"{name}: 状态 {status}（尚未 approved；approved 前不得改产品代码）")

def main():
    if not SPECS.exists():
        err("specs/ 不存在")
    check_templates()
    changes = SPECS / "changes"
    if changes.exists():
        for d in sorted(changes.iterdir()):
            if d.is_dir() and not d.name.startswith("."):
                check_change(d)
    for w in warns:
        print("WARN:", w)
    for e in errors:
        print("ERR :", e)
    print(f"\nspec-check: {len(errors)} errors, {len(warns)} warnings")
    return 0 if not errors else 1

if __name__ == "__main__":
    sys.exit(main())
