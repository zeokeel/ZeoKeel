---
name: zeokeel-change
description: Use when starting any ZeoKeel change that touches code, contracts, data, or behavior. Creates a specs/changes/<id>/ five-file contract, gates implementation on user approval, and enforces small verified steps plus evidence.
---

# ZeoKeel 变更流程

## 何时使用

用户要求新增/修改/删除任何代码、契约、数据、路由、页面、发布行为时，**先用本 Skill**。
纯错别字/格式修正可跳过 spec，但仍要跑 `docs-check`。

## 开工前（只读，不写代码）

1. 读 `AGENTS.md` → `docs/README.md`（SSOT/任务路径）。
2. 打开本次主题的**唯一权威文档**；不要从 `research/` 或历史文档取规范。
3. 检查 `specs/changes/` 是否已有同目标变更；有则续用，不另建。
4. 检查 `docs/15`（D）与 `docs/13`（ADR）是否已有结论；冲突时停下来问用户，不自行选。

## 建 spec

```bash
bash scripts/new-change.sh <kebab-slug>
# => specs/changes/<YYYY-MM-DD>-<slug>/{proposal,spec,design,tasks,acceptance}.md
```

填写顺序：

1. `proposal.md`：背景、目标、**非目标**、成功指标、风险、需用户拍板项、变更预算。
2. 状态保持 `draft`，把"需要用户拍板"发给用户；**用户确认后**改为 `approved`。
3. `spec.md`：WHEN/THEN 需求、错误/权限/数据/兼容、可执行验收断言。
4. `design.md`：影响的包/契约/路由/迁移/生成物，方案与备选，安全，发布/回滚，测试计划。
5. `tasks.md`：每任务必须有写作用域 + `verify` 命令；默认 ≤10 文件/≤400 行。

## 实现

- 按 `tasks.md` 顺序，一次一个可独立提交的步骤。
- 改契约先跑 conformance / public-api diff；改迁移只追加。
- 不手改 `@generated`；不跨层依赖具体实现；不新增运行期插件加载。
- 每个任务完成后立刻跑它的 `verify`，失败不进入下一步。
- 发现 spec 错了：先改 spec（必要时重新 `approved`），再改代码。

## 收尾

```bash
python3 scripts/spec-check.py
python3 scripts/docs-check.py
# 再跑本次涉及的代码/契约/UI 门禁（见 AGENTS.md §6）
```

- 填 `acceptance.md`：命令、结果、证据路径、文档/ADR、回滚、剩余风险。
- 耐久结论回写 `docs/`；新决策写 D/ADR，不留在 spec 里。
- `git mv specs/changes/<id> specs/archive/<id>`，状态改 `done`。

## 禁止

- 没有 spec 开工；`draft` 状态改产品代码。
- 一个 spec 塞多个无关目标。
- 用"我测过了"代替命令与证据路径。
- 在 spec 里复制 `docs/` 的规范正文（只链接 SSOT）。
