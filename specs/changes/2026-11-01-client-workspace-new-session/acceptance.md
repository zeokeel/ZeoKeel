# 2026-11-01-client-workspace-new-session · 验收

> 本 spec 已通过 C19/C20 独立评审：docs/reviews/2026-11-01-client-workspace-new-session.md（最终 APPROVED）。

- 状态：`draft`（proposal §7 D1–D4 **已定（Lead 2026-11-01）**，D3 修订为流式禁用；待独立评审，未批准前不实现）
- 验收人：待独立评审 / 实现者回填
- 评审：n/a（新增 spec；实现前需 C19/C20 独立评审）
- 证据根：`target/evidence/2026-11-01-client-workspace-new-session/`
- 交互/直觉：`interaction.md`、`intuition.md`、`intuition-walkthrough.md`（均 `planned`；实现开始时一起改 `active`）

## 1. 门禁与证据

| 命令 | 结果 | 证据 |
| --- | --- | --- |
| `python3 scripts/spec-check.py` | 待跑 | `.../logs/spec-check.txt` |
| `python3 scripts/docs-check.py` | 待跑 | `.../logs/docs-check.txt` |
| `pnpm -r typecheck && pnpm -r lint && pnpm -r test:unit && pnpm -r build` | 待跑 | `.../logs/pnpm-gates.txt` |
| `bash scripts/i18n-check.sh` | 待跑 | `.../logs/i18n-check.txt` |
| `pnpm e2e`（既有 m1 不回归） | 待跑 | `.../logs/e2e.log` |
| `pnpm e2e:ui --grep "workspace-new"`（工具已落地） | 待跑 | `.../interaction/<ID>.json` |
| `pnpm e2e:ui:gate`（工具已落地） | 待跑 | `.../logs/interaction-gate.txt` |

## 1.5 交互验收（ID → 测试 → 证据）

> 状态：`planned`；实现时把 `interaction.md`/`intuition.md` 改 `active` 并逐项填实际测试/证据。

| ID | 优先级 | 测试 | 证据 |
| --- | --- | --- | --- |
| SES2-WSNEW-01 | P0 | `e2e/interaction/sessions-workspace-new.spec.ts` | `.../SES2-WSNEW-01.json` |
| SES2-WSNEW-02 | P0 | 同上 | `.../SES2-WSNEW-02.json` |
| SES2-WSNEW-03 | P1 | 同上 | `.../SES2-WSNEW-03.json` |
| SES2-WSNEW-VIS-01 | P2（不进门禁） | 同上 | — |

## 1.6 直觉验收（C20）

- 文件：`intuition.md`（planned，≥3 INT，P0 全覆盖）+ `intuition-walkthrough.md`（3 persona × 3 top task + 摩擦表，P0 无 OPEN）。
- P0 摩擦：见 walkthrough §3；均已用最小修法落到 design/tasks 并关闭。
- 用户测试：**UNVERIFIED**（未执行；发布前 ≥5 人 think-aloud，证据 `target/evidence/2026-11-01-client-workspace-new-session/usability/`）；**未完成前不得声称“符合直觉/直观”**。
- 评审：待 fresh-context 评审员按走查复核（P0 摩擦未关闭 → 不通过）。

## 2. 手工检查

- [ ] 每个 Workspace 行有 scoped `+`；hover 显示、keyboard focus 显示、active 行常显；`aria-label` 含 workspace 名
- [ ] 点击/Enter/Space 在**该** workspace 建会话并选中；不经过 picker；active workspace 正确切换；其它 workspace 会话不变
- [ ] 行内按钮不触发整行选中/折叠；Tab 顺序：折叠→标题→新建→`⋯`；按钮独立 focus ring
- [ ] 全局「新建会话」/header `+`/⌘K `new-conversation` 与 scoped 一样受 streaming guard（同一禁用/提示）；非流式按 active workspace 规则；无 workspace 不建、按钮不可见/禁用提示
- [ ] 流式中所有 Workspace 行 `+` 禁用 + tooltip/aria；停止后恢复；不建/不切/不串消息；无确认 Modal
- [ ] 失败（workspace 不可用/session.create 失败）以 host 错误为准；可读提示、选择不变、按钮恢复；快速重复只建 1 个
- [ ] 折叠 workspace 新建后自动展开并滚动到新会话；reduced-motion 无额外动画
- [ ] i18n zh/en；axe 0 serious；无新增契约/迁移/slot/第二套建会话路径

## 2.5 评审条目关闭（2026-11-01 workspace 评审）

| 条目 | 处理 |
| --- | --- |
| P0-1 流式新建清空/串消息 | 删除「同 workspace 允许/跨 workspace 确认 Modal」；修订 D3：**streaming 时 scoped/global/⌘K 三入口统一禁用** + 同一 tooltip/aria「当前正在生成，停止后可新建」，停止后恢复；不切 activeConversationId；补三入口禁用/恢复/不串消息单测+e2e；proposal 记录 DSH per-session 流差异 |
| P1-1 行数超阈值 | proposal §8 记录 Lead 书面批准 8 文件/约 643 行；P2-3 追加 KNOWN 后总 9（≤10） |
| P2-1 默认焦点 | 确认 Modal 路径已删除（流式禁用替代）；无 data-autofocus 需求 |
| P2-2 R4 unavailable | 统一为「以 host `session.create` 错误为准；本地仅明显无效（空 path/id）禁用」，不新增逐行可用性探测 |
| P2-3 KNOWN | tasks #0 写作用域与预算加入 `KNOWN-LIMITATIONS`（brand/Mod+N 后置、单流模型差异） |
| P2-4 门禁文案 | tasks/acceptance 改为已落地的 `pnpm e2e:ui --grep "workspace-new"` / `pnpm e2e:ui:gate` |

## 3. 文档与决策

- [ ] `docs/19` §2 已回写 scoped 新建/可见性/流式禁用/无 workspace 边界；`KNOWN-LIMITATIONS` 已记录 brand/`Mod+N` 后置与单流模型差异
- [x] proposal §7 D1–D4 已由 Lead 2026-11-01 定稿（D3 修订为流式禁用）；状态保持 `draft`，待独立评审
- [x] **P1-1 预算例外**：Lead 书面批准 8 文件/约 643 行；P2-3 追加 KNOWN 后总 9（≤10，不拆分）
- [ ] 无 ADR（无契约/数据/安全边界变化）

## 4. 数据/兼容/回滚

- [ ] 无 DB/文件/契约变更；旧客户端/旧会话兼容；`session.create` 不变
- [ ] 回滚移除按钮/action 即可；已建会话归属不变；C18 不变量保持

## 5. 结论

- 当前：`draft`；交互/直觉 `planned`；待用户拍板 + 独立评审后实现。
- 完成时必须填实际命令/证据；用户测试未执行标 UNVERIFIED，不得声称符合直觉。
