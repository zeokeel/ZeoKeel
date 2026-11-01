# 2026-11-01-client-workspace-new-session · 验收与证据

> 独立评审通过（`docs/reviews/2026-11-01-client-workspace-new-session.md` §8）；C20 `intuition.md` + `intuition-walkthrough.md` 已交付（专家走查；真人用户测试 UNVERIFIED）。
> 实现者：impl-settings；证据根 `target/evidence/2026-11-01-client-workspace-new-session/`。

- 验收人：impl-settings（实现者回填）
- 日期：2026-11-01
- PR/commit：未提交（共享工作树；由 Lead 汇总提交）
- 状态：`done`（本地实现与证据齐全；UNVERIFIED 见 §1 备注）

## 1. 验收命令与结果

| 命令 | 结果 | 证据 |
| --- | --- | --- |
| `pnpm -r typecheck` | **exit 0** | `logs/pnpm-final.log` |
| `pnpm -r lint` | **exit 0** | `logs/pnpm-final.log` |
| `pnpm -r test:unit` | **exit 0**（desktop-web 131，含 createConversation guard/catalog 断言） | `logs/pnpm-final.log` |
| `pnpm -r build` | **exit 0** | `logs/pnpm-final.log` |
| `pnpm e2e`（mock 8 + m1-serve 10，含既有回归） | **mock 8/8、m1-serve 10/10（exit 0）**；等价序列执行（工作树未提交） | `logs/e2e-main.log`、`logs/gates-run.log` |
| `pnpm e2e:ui --grep "workspace-new"`（本 spec 3 个 P0/P1） | **3/3 passed（exit 0）**，3 份证据 + 截图 | `logs/e2e-interaction.log` + `interaction/<ID>.json` |
| `pnpm e2e:ui`（完整交互套件） | **64/64 passed（exit 0）**（61 既有 + 3 本 spec；修复了 scoped `+` 引起的 2 处既有选择器歧义） | `logs/e2e-ui-full.log` |
| `python3 scripts/ui-interaction-gate.py` / `--strict` | **self-test 0 / gate 0 / strict 0**（enforced P0/P1=62；intuition 5 contracts/60 rows） | `logs/gates-summary.txt` |
| `python3 scripts/spec-check.py` / `docs-check.py` | **0 / 0** | `logs/gates-summary.txt` |
| `bash scripts/i18n-check.sh` | **0** | `logs/gates-summary.txt` |
| 视觉/ARIA 基线 `SES2-WSNEW-VIS-01`（P2，不进门禁） | **UNVERIFIED**：像素/aria snapshot 需人工确认后 `pnpm e2e:ui:update`（agent 不自动接受），当前仅截图证据 | `interaction/SES2-WSNEW-VIS-01.png` |
| 真人用户测试（≥5 人 think-aloud） | **UNVERIFIED**（发布前执行；C20） | `intuition-walkthrough.md` §4 |

## 2. 手工检查（对应 spec R1–R5）

- [x] R1：每 Workspace 行 scoped `+`；hover/focus-visible 可见、active 行常显；`aria-label` 含 workspace 名；点击/Enter/Space 在目标 workspace 创建并选中（active workspace + 新会话切换），不经 picker、不误建到原 active；不触发行选中/折叠（事件 `stopPropagation` + 独立按钮），按钮 busy 防重复；失败 toast、选择不变；无 workspace 不渲染。
- [x] R2：active 行按钮常显；scoped/global/header `+`/⌘K `new-conversation` 都走 controller `createConversation` 同一入口（唯一 streaming/busy/workspace guard）；非流式 scoped 用行 workspace、全局/⌘K 用 active workspace；无 workspace 禁用/`workspaceRequired`。
- [x] R3：`streaming=true` 三入口全部禁用 + 同一文案「当前正在生成，停止后可新建」（scoped/global `title`+`aria-disabled`、⌘K descriptor `disabledReason`）；不建/不切/不串消息；停止后恢复；删除旧「同 workspace 允许/跨 workspace 确认 Modal」路径。
- [x] R4：失败可读 toast、当前选择不变、按钮恢复；目标 workspace 不可用以 host `session.create` 错误为准；快速重复点击仅 1 个（busy）；标题沿用 `conversationNewTitle`；不改 `session.create` 契约。
- [x] R5：按钮可读名称含 workspace、focus ring、Tab 顺序在折叠/标题/+/⋯ 合理、Esc 不误关侧栏（既有）；axe 0 serious；无新增 slot/包/DB/契约；i18n zh/en；与 DSH 差异（无 brand 点击/`Mod+N`、单流）已记录 KNOWN。

## 3. 交互 ID 覆盖

| ID | 测试（`e2e/interaction/sessions-workspace-new.spec.ts`） | 证据 |
| --- | --- | --- |
| SES2-WSNEW-01 | 双 workspace：行 + 在目标 workspace 建会话并选中、另一 workspace 不变、无 picker | `SES2-WSNEW-01.json` |
| SES2-WSNEW-02 | 键盘/focus-visible、active 常显、aria-label 含名、点击不折叠、Enter 创建、axe 0 | `SES2-WSNEW-02.json` |
| SES2-WSNEW-03 | streaming 三入口统一禁用（scoped/global 同步快照 + ⌘K descriptor 单测）、不建不切、完成后恢复 | `SES2-WSNEW-03.json` |
| SES2-WSNEW-VIS-01（P2） | Workspace 行截图（hover/active 态） | `SES2-WSNEW-VIS-01.png`（像素基线 UNVERIFIED） |

## 4. 文档与决策

- [x] `docs/19` §2.0.1 会话侧栏 Workspace 行 scoped 新建 + 三入口 streaming guard + DSH 差异。
- [x] `KNOWN-LIMITATIONS`：单流假设（无 per-session 后台流）、无 brand 点击/`Mod+N`、无 picker/逐行探测、busy 客户端抑制、视觉基线 UNVERIFIED。
- [x] 决策沿用 proposal §7 D1–D4（Lead 2026-11-01）；无新增 ADR/D 编号/capability/slot/契约。

## 5. 回滚

- 回退 commit；无 DB/schema/IPC 变更（仍 `session.create{workspace_id,title?}`）；已建会话归属正确 workspace，移除按钮无数据影响。

## 6. 剩余风险 / 后续

| 项 | 影响 | 后续 |
| --- | --- | --- |
| 单流：流式中禁止新建 | 中 | 另 spec 评估 per-session 后台流（DSH parity） |
| 无 `Mod+N`/brand 点击入口 | 低 | proposal D2/D4 后置 |
| 像素/aria 基线未确认；用户测试未执行 | 中 | reviewer 确认 + ≥5 人 think-aloud |

## 7. 结论

- [x] `done`：本地实现与证据齐全（UNVERIFIED 项见 §1）
- [ ] `abandoned`：——

## 8. 复用证据（C17）

- 复用：ui-sessions 既有行样式/图标按钮/Icons、`UiActions.createConversation`（扩展可选 workspaceId 参数，非第二套路径）、controller 唯一 `createConversation` guard、i18n 字典、既有 server `session.create`；无新建包/slot/契约/Modal。
