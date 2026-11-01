# 2026-11-01-client-settings-ia · 验收与证据

- 验收人：待填
- 日期：待填
- PR/commit：待填

> 状态：`draft`（未开工）。实现完成后按任务表逐项填证据路径。

## 1. 验收命令与结果

| 命令 | 结果 | 证据路径 |
| --- | --- | --- |
| `python3 scripts/spec-check.py` | 待跑 | — |
| `python3 scripts/docs-check.py` | 待跑 | — |
| `zk validate` | 待跑 | — |
| `zk compose --edition personal --profile default --channel official --check` | 待跑 | — |
| `zk capability --check` | 待跑 | — |
| `cargo test -p zk-contract-ipc -p zk-desktop` | 待跑 | — |
| `pnpm -r typecheck && pnpm -r lint && pnpm -r test:unit && pnpm -r build` | 待跑 | — |
| `pnpm e2e`（设置/axe 用例） | 待跑 | — |

## 2. 手工检查

- [ ] 设置六态/键盘/a11y/错误提示
- [ ] 保存冲突（两窗口同时改）
- [ ] 窄窗 375/900/1280 布局与焦点
- [ ] personal/enterprise 分区差异

## 3. 文档与决策

- [ ] SSOT 文档已更新：`docs/16`、`docs/28`、`docs/19`
- [ ] ADR/D 已记录：待定（pane slot 机制）
- [ ] `docs/README.md` 已登记新主题：不适用
- [ ] 变更 spec 已归档到 `specs/archive/`

## 4. 回滚

- 回滚步骤：回退 commit；设置文档兼容（新增字段可选）
- 数据影响：无数据库迁移
- 已验证回滚：待填

## 5. 剩余风险 / 后续

| 项 | 影响 | 后续 spec |
| --- | --- | --- |
| MCP/审批策略/子代理/终端/jobs 配置语义 | 高 | 见 parity 报告 §7 Phase 1/2 |
| 设置搜索/快捷键 | 低 | 后续增强 |

## 6. 结论

- [ ] `done`：证据齐全，可合并
- [ ] `abandoned`：原因：待填

## 6. 复用证据（C17）

- 搜索记录：`rg -n "settings.provider|slot_binding" tools/zk packages/ui`
- 复用清单：`ui-kit.useDialogFocus`、`ui-kit.SlotErrorBoundary`、`ui-settings-provider`
- 新增能力登记：`@capability ui.settings.pane`（`zk capability` 生成）
