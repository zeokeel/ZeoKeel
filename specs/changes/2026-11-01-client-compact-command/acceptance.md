# 2026-11-01-client-compact-command · 验收

- 状态：`draft`（D1/D2/D6 已定 Lead 2026-11-01；待独立评审）
- 评审：n/a（新增 spec；实现前 C19/C20）
- 证据根：`target/evidence/2026-11-01-client-compact-command/`
- 交互/直觉：interaction.md、intuition.md、intuition-walkthrough.md（planned→active）

## 1. 门禁
| 命令 | 结果 | 证据 |
| --- | --- | --- |
| spec-check/docs-check | 待跑 | logs/ |
| cargo test -p zk-contract-ipc -p zk-kit-compaction -p zk-desktop | 待跑 | logs/cargo.log |
| export_schema 幂等 + schema-check | 待跑 | logs/schema.log |
| pnpm -r typecheck/lint/test:unit/build + i18n | 待跑 | logs/pnpm.log |
| pnpm e2e；e2e:ui --grep command-compact；e2e:ui:gate | 待跑 | interaction/CMD3C-COMPACT-01.json |

## 1.5 P0
| ID | 优先级 | 测试 | 证据 |
| --- | --- | --- | --- |
| CMD3C-COMPACT-01 | P0 | `e2e/interaction/command-compact.spec.ts` | `.../CMD3C-COMPACT-01.json` |
| CMD3C-KEY-01 | P1 | 同上 | `.../CMD3C-KEY-01.json` |
| CMD3C-ERR-01 | P1 | 同上 | `.../CMD3C-ERR-01.json` |
| CMD3C-VIS-01 | P2 | 同上 | — |

## 1.6 C20
- intuition planned（≥3 INT，P0 覆盖）；walkthrough 3×3；P0 摩擦无 OPEN；用户测试 **UNVERIFIED**；未跑前不得声称符合直觉。

## 2. 手工检查
- [ ] `/compact` 真实命令、分组/键位；无 unimplemented；Batch1 不回归。
- [ ] 成功：marker+最近 6 轮、原始消息保留、UI 分隔/usage、下一轮摘要生效；`messages_in` 原始语义不变；compacted view 仅 replay/分隔；search/fork/审计仍见 raw。
- [ ] 流式/超限/失败/取消不改历史；重复 no-op；错误码可读。
- [ ] 六态/i18n/axe/键盘；workspace 归属；无 migration/自动压缩。

## 3. 数据/兼容
- [ ] 无 marker 旧行为；search/fork/audit raw；原始 DB 保留；回滚删 marker 回原历史。

## 4. 结论
- 当前 draft；实现后填证据；未执行标 UNVERIFIED。
