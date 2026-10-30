#!/usr/bin/env python3
"""ZeoKeel 文档门禁：结构/链接/SSOT/已废弃口径/已知重复源。

退出码 0 = 全部通过；1 = 有违规。CI 直接跑本脚本。
"""
from __future__ import annotations
import re, sys, pathlib

ROOT = pathlib.Path(__file__).resolve().parent.parent
DOCS = sorted((ROOT / "docs").glob("*.md"))
ROOT_MD = [ROOT / "README.md", ROOT / "AGENTS.md"]
ALL = DOCS + ROOT_MD
errors: list[str] = []
warnings: list[str] = []

def err(msg: str): errors.append(msg)
def warn(msg: str): warnings.append(msg)

def slug(heading: str) -> str:
    s = heading.strip().lower()
    s = re.sub(r"^#+\s*", "", s)
    s = re.sub(r"[^\w\u4e00-\u9fff\- ]", "", s)
    return s.replace(" ", "-")

def check_h1():
    for f in DOCS:
        if f.name in ('README.md','PLAN.md'):
            continue
        text = _md_lines(f)
        h1 = next((l for l in text if l.startswith("# ")), None)
        if not h1:
            err(f"{f.relative_to(ROOT)}: 缺少 H1")
            continue
        m = re.match(r"#\s+(\d{2})\s*·", h1)
        expected = f.name[:2]
        if not m:
            err(f"{f.relative_to(ROOT)}: H1 不以 'NN ·' 开头: {h1[:60]}")
        elif m.group(1) != expected and f.name != "PLAN.md":
            err(f"{f.relative_to(ROOT)}: H1 编号 {m.group(1)} != 文件名 {expected}")
    if not (ROOT / "docs/README.md").exists():
        err("docs/README.md 不存在（唯一索引缺失）")

def _md_lines(f):
    """返回不在代码块中的行。"""
    out, in_code = [], False
    for l in f.read_text().splitlines():
        if l.strip().startswith("```"):
            in_code = not in_code
            continue
        if not in_code:
            out.append(l)
    return out

def check_headings():
    for f in ALL:
        lines = _md_lines(f)
        hs = [l.strip() for l in lines if re.match(r"^#{2,3}\s", l)]
        for h in set(hs):
            if hs.count(h) > 1:
                err(f"{f.relative_to(ROOT)}: 重复标题 {h[:60]}")
        levels = [len(re.match(r"^(#+)", l).group(1)) for l in lines if l.startswith("#")]
        prev = levels[0] if levels else 1
        for lv in levels[1:]:
            if lv > prev + 1:
                warn(f"{f.relative_to(ROOT)}: 标题跳级 {prev}->{lv}")
            prev = lv

def check_fences():
    for f in ALL:
        n = f.read_text().count("```")
        if n % 2:
            err(f"{f.relative_to(ROOT)}: 代码块未闭合（``` 数量 {n}）")

def check_tables():
    for f in ALL:
        lines = f.read_text().splitlines()
        for i, l in enumerate(lines):
            if re.match(r"^\|\s*:?-{2,}", l):
                prev = lines[i-1] if i else ""
                if not prev.strip().startswith("|"):
                    err(f"{f.relative_to(ROOT)}:{i+1}: 表格分隔行前不是表头")
                cols = prev.count("|")
                if l.count("|") != cols:
                    err(f"{f.relative_to(ROOT)}:{i+1}: 表格列数不一致")
                # 分隔行之前紧邻的行若已是数据行（同一表格块中），说明表头缺失
        # 检测“连续表格块中间空行导致断表”：表头后隔一行分隔行
        for i, l in enumerate(lines):
            if l.strip().startswith("|") and i+2 < len(lines) and lines[i+1].strip()=="":
                if re.match(r"^\|\s*:?-{2,}", lines[i+2]):
                    err(f"{f.relative_to(ROOT)}:{i+1}: 表头与分隔行之间有空行")

def check_links():
    link_re = re.compile(r"\[([^\]]+)\]\(([^)#]+)(#[^)]*)?\)")
    for f in ALL:
        for m in link_re.finditer(f.read_text()):
            target = m.group(2)
            if target.startswith(("http", "mailto")):
                continue
            p = (f.parent / target).resolve()
            if not p.exists():
                err(f"{f.relative_to(ROOT)}: 失效链接 -> {target}")
            elif m.group(3) and p.suffix == ".md":
                anchor = m.group(3)[1:]
                headings = [slug(l) for l in _md_lines(p) if l.startswith("#")]
                if anchor not in headings:
                    warn(f"{f.relative_to(ROOT)}: 锚点可能失效 {target}#{anchor}")

def check_ssot():
    idx = (ROOT / "docs/README.md").read_text()
    for f in DOCS:
        if f.name in ("README.md", "PLAN.md"):
            continue
        if f.name[:2] not in idx:
            err(f"docs/README.md: SSOT 未登记 {f.name}")
    # 根 README 不再复制完整地图
    root = (ROOT / "README.md").read_text()
    if "docs/README.md" not in root:
        err("README.md: 未指向 docs/README.md 唯一索引")

BANNED = [
    (r"store-session-jsonl", "会话存储必须 SQLite（24 §7）"),
    (r"app-<app_id>", "capability 不能按运行期 app_id 生成（19 §7.1）"),
    (r"可选哈希链", "审计哈希链必选（21 §7）"),
    (r"balance\s*=\s*amount", "overwrite 必须写 adjust（22 §3）"),
    (r"edition profile", "edition 与 profile 是两个概念"),
    (r"登录后服务端可覆盖", "品牌构建期注入、运行期只读"),
    (r"备份必含.*MASTER_KEY|MASTER_KEY.*备份必含", "master.key 不放进备份包（20 §3）"),
    (r"OEM 如需签名", "所有渠道不签名（ADR-033）"),
    (r"预付资金", "余额=企业配额（D60）"),
    (r"登录页显示地址输入", "空 server_url 用首配页，登录页不显示地址输入"),
]
def check_banned():
    for f in ALL:
        text = f.read_text()
        for pat, why in BANNED:
            for m in re.finditer(pat, text):
                # 允许否定/历史语境：同句含“不/禁止/不再/历史”
                line = text[:m.start()].splitlines()[-1] if text[:m.start()].splitlines() else ""
                if any(x in line for x in ("不", "禁止", "不再", "历史", "非")):
                    continue
                err(f"{f.relative_to(ROOT)}: 命中废弃口径 {m.group(0)!r}（{why}）")

def check_skills():
    base = ROOT / ".agents" / "skills"
    if not base.exists():
        err(".agents/skills/ 不存在")
        return
    for d in sorted(p for p in base.iterdir() if p.is_dir()):
        f = d / "SKILL.md"
        if not f.exists():
            err(f"{d.name}: 缺 SKILL.md")
            continue
        text = f.read_text()
        m = re.search(r"^---\n(.*?)\n---", text, re.S)
        if not m:
            err(f"{d.name}: 缺 YAML frontmatter")
            continue
        fm = m.group(1)
        nm = re.search(r"^name:\s*(.+)$", fm, re.M)
        ds = re.search(r"^description:\s*(.+)$", fm, re.M)
        if not nm or nm.group(1).strip() != d.name:
            err(f"{d.name}: frontmatter name 缺失或与目录不一致")
        if not ds or not ds.group(1).strip():
            err(f"{d.name}: description 为空")

def check_adr_d():
    p = ROOT / "docs/13-risks-and-adr.md"
    nums = [int(x) for x in re.findall(r"^### ADR-(\d{3})", p.read_text(), re.M)]
    if len(nums) != len(set(nums)):
        err("docs/13: ADR 编号重复")
    if nums != list(range(1, max(nums)+1)):
        err("docs/13: ADR 编号不连续")
    q = ROOT / "docs/15-open-questions.md"
    ds = [int(x) for x in re.findall(r"^\| D(\d+) \|", q.read_text(), re.M)]
    if len(ds) != len(set(ds)):
        err("docs/15: D 编号重复")
    if ds and ds != list(range(1, max(ds)+1)):
        err("docs/15: D 编号不连续")

def main():
    check_h1(); check_headings(); check_fences(); check_tables(); check_links()
    check_ssot(); check_banned(); check_adr_d(); check_skills()
    for w in warnings:
        print("WARN:", w)
    for e in errors:
        print("ERR :", e)
    print(f"\ndocs-check: {len(errors)} errors, {len(warnings)} warnings, {len(ALL)} files")
    return 0 if not errors else 1

if __name__ == "__main__":
    sys.exit(main())
