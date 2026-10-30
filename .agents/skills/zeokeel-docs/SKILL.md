---
name: zeokeel-docs
description: Use when changing ZeoKeel documentation, decisions, or SSOT wording. Enforces one authoritative document per topic, summary-plus-link instead of duplicated specs, decision records in D/ADR, and docs-check clean.
---

# 修改 ZeoKeel 文档

## 何时使用

改 `docs/`、`AGENTS.md`、根 `README.md`、`specs/` 中的规范/决策/说明时。
写代码时同步改文档也走本 Skill。

## 先定位唯一权威

1. 读 `docs/README.md` 的 SSOT 表：本主题的**唯一权威文档**是哪份。
2. 只改那份的规范正文；其它文档只允许：
   - ≤5 行摘要 + `详见 X`，或
   - 一行链接。
3. 不要在 `research/` 里写规范；`14`/`17` 是历史，不得作为依据。
4. 新主题先登记到 `docs/README.md` §2；新术语先登记 §4。
5. 决策变更：先改 `docs/15`（结论）或 `docs/13`（ADR 理由），再改实现/其它文档。

## 改文档的步骤

1. 如果改变行为/决策：先建/复用 change spec（`zeokeel-change`）。
2. 改权威文档正文。
3. 全局搜索旧说法并删除/改链接：

```bash
rg -n "<旧术语|旧结论>" docs AGENTS.md README.md
```

4. 检查数字/清单：包数、端点数、契约数、限额**不要手写**，由生成器产出或引用 `docs/generated/`。
5. 跑门禁：

```bash
python3 scripts/docs-check.py
python3 scripts/spec-check.py
```

6. `docs-check` 必须 **0 errors / 0 warnings**；新增禁用词先加到 `scripts/docs-check.py` 的 `BANNED`。

## 写作规则

- 一个规范只写一次；摘要必须标注"详见 X"。
- 用文件:行/链接，不写"上面/前面"。
- 不复制表结构/字段/流程到第二份文档。
- 不把 `research/` 的审计意见直接写成规范；先映射到 SSOT 文档。
- 删除内容时保留历史说明（"旧口径已废弃，见 X"）只在 ADR/历史文档，不在规范正文。

## 禁止

- 在多个文档各写一份"可执行细节"。
- 修改 `docs/14`/`docs/17` 当作现行规范。
- 手写会漂移的计数/清单。
- 不跑 `docs-check` 就声称文档完成。
