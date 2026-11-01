# 2026-11-01-client-command-surface · 验收

- 状态：`draft`（proposal §7 D1–D6 **已定（Lead 2026-11-01，推荐 A）**；待同一评审员复评，未批准前不实现）
- 验收人：待独立评审 / 实现者回填
- 评审：`docs/reviews/2026-11-01-client-command-surface.md`（首轮 CHANGES-REQUESTED；P1 修复后同一评审员复评）
- 证据根：`target/evidence/2026-11-01-client-command-surface/`
- 交互/直觉：`interaction.md`、`intuition.md`、`intuition-walkthrough.md`（均 `planned`；实现开始时一起改 `active`）

## 1. 门禁与证据

| 命令 | 结果 | 证据 |
| --- | --- | --- |
| `python3 scripts/spec-check.py` | 待跑 | `.../logs/spec-check.txt` |
| `python3 scripts/docs-check.py` | 待跑 | `.../logs/docs-check.txt` |
| `cargo test -p zk-contract-ipc` | 待跑 | `.../logs/contract-tests.txt` |
| `cargo test --workspace export_schema && git diff --exit-code` | 待跑 | `.../logs/schema-idempotent.txt` |
| `bash scripts/schema-check.sh` | 待跑 | `.../logs/schema-check.txt` |
| `cargo test -p zk-desktop` | 待跑 | `.../logs/host-tests.txt` |
| `zk compose --edition personal --profile default --channel official --check` | 待跑 | `.../logs/compose-check.txt` |
| `zk capability --check` | 待跑 | `.../logs/capability-check.txt` |
| `pnpm -r typecheck && pnpm -r lint && pnpm -r test:unit && pnpm -r build` | 待跑 | `.../logs/pnpm-gates.txt` |
| `bash scripts/i18n-check.sh` | 待跑 | `.../logs/i18n-check.txt` |
| `pnpm e2e`（m1-serve 命令场景） | 待跑 | `.../logs/e2e.log`、`.../screenshots/`、`.../axe.json` |
| `pnpm e2e:ui --grep command`（task-33 基线已落地；实现时运行） | 待跑 | `.../interaction/<ID>.json` |
| `python3 scripts/ui-interaction-gate.py`（基线已落地；实现时运行） | 待跑 | `.../logs/interaction-gate.txt` |

## 1.5 交互验收（ID → 测试 → 证据）

> 状态：`planned`；实现时把 `interaction.md`/`intuition.md` 改 `active` 并逐项填实际测试/证据。
> P0 无确认/无测试不得上线。

| ID | 优先级 | 测试 | 证据 |
| --- | --- | --- | --- |
| CMD2-MENU-01 | P0 | `e2e/interaction/command-surface.spec.ts` | `.../CMD2-MENU-01.json` |
| CMD2-MENU-02 | P0 | 同上 | `.../CMD2-MENU-02.json` |
| CMD2-FILE-01 | P0 | 同上 | `.../CMD2-FILE-01.json` |
| CMD2-MODEL-01 | P0 | 同上 | `.../CMD2-MODEL-01.json` |
| CMD2-PERM-01 | P0 | 同上 | `.../CMD2-PERM-01.json` |
| CMD2-GOAL-01 | P0 | 同上 | `.../CMD2-GOAL-01.json` |
| CMD2-EXPORT-01 | P0 | 同上 | `.../CMD2-EXPORT-01.json` |
| CMD2-FEEDBACK-01 | P0 | 同上 | `.../CMD2-FEEDBACK-01.json` |
| CMD2-KEY-01 | P1 | 同上 | `.../CMD2-KEY-01.json` |
| CMD2-A11Y-01 | P1 | 同上 | `.../CMD2-A11Y-01.json` |
| CMD2-ERR-01 | P1 | 同上 | `.../CMD2-ERR-01.json` |
| CMD2-VIS-01 | P2（不进门禁） | 同上 | — |

## 1.6 直觉验收（C20）

- 文件：`intuition.md`（planned，≥3 INT，P0 全覆盖）+ `intuition-walkthrough.md`（3 persona × 3 top task + 摩擦表，P0 无 OPEN）。
- P0 摩擦：见 walkthrough §3；均已在 design/tasks 落最小修法并关闭。
- 用户测试：**UNVERIFIED**（未执行；发布前 ≥5 人 think-aloud，证据 `target/evidence/2026-11-01-client-command-surface/usability/`）；**未完成前不得声称“符合直觉/直观”**。
- 评审：待 fresh-context 评审员按走查复核（P0 摩擦未关闭 → 不通过）。

## 2. 手工检查

- [ ] `/` 空查询：添加(file,goal,plan,feedback) / 指令(compact,permission,model,export) / 技能(skill)；6 可用、3 占位；无 UI 动作
- [ ] ⌘K 仍有 UI 动作（new-conversation/toggle-sidebar/open-settings/toggle-inspector/cancel/retry；enterprise +3）且行为不变
- [ ] `/` 过滤/↑↓/Enter/Tab/Esc/Shift+Tab/点击；未知命令不发送；Batch2 不执行、不发送
- [ ] `/file` 选择器/取消/失败/附件上限；非 /file 带附件拒绝并保留
- [ ] `/model` personal 模型+effort / enterprise 网关列表；当前值/默认/会话覆盖；切会话/重启恢复；失败回滚；不写全局
- [ ] `/permission` 三档映射/当前值/恢复/失败；文案「仅审批、无沙箱、不限制员工」
- [ ] `/goal` set/view/clear/pause/resume + GoalBar；同一 `.zeokeel/goal.json`；无工具行；非法/超限拒绝
- [ ] `/export` Markdown 内容/文件名/≤8 MiB/空会话/失败；serve 下载；Tauri 展示/复制路径（不承诺自动打开）；工作区内
- [ ] `/feedback` 分类/描述≤4 KiB/本地保存/取消/失败/无网络；企业版标 UNVERIFIED 不假成功
- [ ] 六态/键盘/焦点/axe/i18n；override/export/feedback 文件不跨 workspace；无日志含内容
- [ ] C17：ui-sessions 私有 Modal 已迁移到 ui-kit 共享 Modal，无两套 Dialog；既有确认行为/Esc/焦点不回归

## 3. 文档与决策

- [ ] `docs/19` §2/§11 已回写命令面/键位/两 surface/GoalBar/错误与空态
- [ ] `docs/26` §5 已登记命令/错误码/文件路径/限制（override/export/feedback）
- [ ] `KNOWN-LIMITATIONS` 已记录 D3–D5 差异；`docs/19`/`docs/26` 已回写（docs/16 由统一回写任务处理）
- [x] proposal §7 D1–D6 已由 Lead 2026-11-01 按推荐 A 定稿（评审：`docs/reviews/2026-11-01-client-command-surface.md`）；状态保持 `draft`

## 4. 数据/兼容/回滚

- [ ] 旧会话无 override → 全局默认；旧客户端不调用新命令；IPC additive。
- [ ] `.zeokeel/conversation-overrides.json` 损坏降级为默认并提示；不覆盖用户文件。
- [ ] export/feedback 为 workspace 内新文件，回滚保留（孤儿；记录 KNOWN）。
- [ ] 无 DB migration；会话/消息/usage/workspace 归属不变（C18）。

## 4.5 评审条目关闭（2026-11-01 command-surface 评审）

| 条目 | 处理 |
| --- | --- |
| P1-1 D1–D6 决策 | proposal §7 全部 `[x]` 并写 Lead 2026-11-01 推荐 A；review 链接；tasks/acceptance 改为已定 |
| P1-2 override 并发 | 全局 config-mutex + turn 前应用/turn 后恢复；其它 conversation turn 排队；`settings.set` apply/publish 同锁（turn 中 save 不改变当前 override，结束后按 revision）；permission 用 `set_approval_policy` 单点、不改 gate owner；补两会话并发/恢复/失败/settings revision/turn 中 settings.set 测试 |
| P2-1 排队取消 | `chat.cancel` 只作用当前运行 turn；排队 turn 获锁后检查 cancelled 跳过（不执行/不报错）；补测试 |
| P1-3 personal `/model` | 无 provider 发现 → 当前模型+手动输入+recentModels(localStorage≤8)+effort；enterprise host 缓存 gatewayModels；proposal「DSH 差异」诚实记录 |
| P1-4 预算例外 | proposal §8 记录 Lead 书面批准 24 文件/约 728 行；超 28 拆 `/export` |
| P1-5 Modal 重复 | ui-kit `index.tsx` 共享 Modal；tasks #3b 迁移 ui-sessions 私有 Modal，禁止两套 Dialog |
| P2-1/2/3 | 预算统一 24；enterprise 列表来自 `auth.configure` 可选 `models[]` host 缓存/校验；serve 下载、Tauri 展示/复制路径（`system.open_url` 仅 http(s)），不承诺自动打开 |

## 5. 结论

- 当前：`draft`，未实现；P0/P1 交互与直觉文件为 `planned`，待同一评审员复评通过后实现。
- 完成时必须填实际命令结果/证据路径；未执行项标 UNVERIFIED，不得声称符合直觉。
