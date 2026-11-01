# 2026-11-01-client-command-surface · 验收与证据（Batch 1）

> 独立评审通过（`docs/reviews/2026-11-01-client-command-surface.md` §9）；C20 `intuition.md` + `intuition-walkthrough.md` 已交付（专家走查；真人用户测试 UNVERIFIED）。
> 实现者：impl-settings；证据根 `target/evidence/2026-11-01-client-command-surface/`。

- 验收人：impl-settings（实现者回填）
- 日期：2026-11-01
- PR/commit：未提交（共享工作树；由 Lead 汇总提交）
- 状态：`done`（本地实现与证据齐全；UNVERIFIED 见 §1 备注）

## 1. 验收命令与结果

| 命令 | 结果 | 证据 |
| --- | --- | --- |
| `cargo test -p zk-contract-agent -p zk-agent-rig -p zk-contract-ipc -p zk-desktop` | **exit 0**（desktop 78 passed，含 override/goal/export/feedback/config-mutex 测试） | `logs/cargo-tests.log` |
| `cargo fmt --all -- --check`；`cargo clippy <5 crates> -D warnings`；`cargo check --workspace --all-targets` | **exit 0 / 0 / 0** | `logs/cargo-fmt.log`、`logs/cargo-clippy.log`、`logs/cargo-check.log` |
| `cargo test --workspace export_schema` + `schema-ts.py` + `schema-check.sh` | **exit 0**（121 schemas，二次生成 sha256 不变） | `logs/export-schema.log`、`logs/schema-check.log` |
| `zk validate/compose --check/capability --check/docs --check` | **exit 0 / 0 / 0 / 0** | `logs/zk-gates.log` |
| `pnpm -r typecheck/lint/test:unit/build` | **exit 0 / 0 / 0 / 0** | `logs/pnpm-typecheck.log`、`logs/pnpm-lint.log`、`logs/pnpm-unit.log`、`logs/pnpm-build.log` |
| `pnpm e2e`（m1-serve 含 `/` DSH 命令面 + 未知/占位；mock 阶段） | 等价序列 **m1-serve 10/10、mock 8/8（exit 0）** | `logs/e2e-main.log`、`logs/e2e-mock.log`、`logs/gates-run.log` |
| `pnpm e2e:ui --grep command`（本 spec 11 个 P0/P1） | **11/11 passed（exit 0）**，11 份证据 | `logs/e2e-interaction.log` + `interaction/<ID>.json` |
| `python3 scripts/ui-interaction-gate.py` / `--strict` | **self-test 0 / gate 0 / strict 0**（enforced P0/P1=59；intuition 4 contracts/48 rows） | `logs/interaction-gate.txt` |
| `spec-check/docs-check/review-check/i18n-check` | **0 / 0 / 0 / 0** | `logs/gates-summary.txt`、`logs/i18n.log` |
| 视觉/ARIA 基线 `CMD2-VIS-01`（P2，不进门禁） | **UNVERIFIED**：像素基线需人工确认后 `pnpm e2e:ui:update`（agent 不自动接受） | —— |
| 真人用户测试（≥5 人 think-aloud） | **UNVERIFIED**（发布前执行；C20） | `intuition-walkthrough.md` §4 |
| `cargo public-api diff` | **UNVERIFIED**（本机未安装；新增均为 additive） | —— |

## 2. 手工检查（对应 spec R1–R7）

- [x] R1 `/` 菜单：DSH 两节 + 技能分组（file/goal/plan/feedback、compact/permission/model/export、skill）；过滤大小写不敏感、↑↓/Enter/Tab/Esc/Shift+Tab、`aria-activedescendant`、焦点回输入框；触发词移除、正文保留；**无 UI 动作**（⌘K 保留 palette surface）。
- [x] R2 未知/占位/附件边界：未知命令 `command/unknown` 不发送；Batch2 `command/unimplemented` disabled 不可执行；仅 `/file` 打开选择器，其它命令带附件拒绝且 chip/草稿原位保留；流式中能力禁用（model/permission/file）。
- [x] R3 `/file`：复用附件流程（allowlist/≤1 MiB/≤5），不发送消息/不生成工具行。
- [x] R4 `/model`：personal=当前值+手动输入+recentModels(localStorage≤8)+effort（不宣称 provider 列表）；enterprise=host 缓存 gatewayModels（C7）；会话级 override 持久化 `.zeokeel/conversation-overrides.json`、切会话/重启恢复、不写全局 settings；全局 config-mutex turn 前应用/turn 后恢复；settings.set 共用同一 mutex；排队 turn 取消后跳过（`run_turn` 获锁后复检 cancelled）。
- [x] R5 `/permission`：三档 strict/auto_read_only/ask 语义映射 + 「仅审批、不限制工具/网络/文件、不是沙箱」说明；会话级持久化/回滚；`ApprovalPolicy::should_ask` 单点（经 facade `set_approval_preset`，不改 gate owner）。
- [x] R6 `/goal` + GoalBar：`/goal <objective>`/`/goal`/clear 内联确认；同一 `.zeokeel/goal.json`（tool.goal driver）；GoalBar 显示 objective/status/rounds、暂停/恢复/清除；不发送命令文本/不生成工具行。
- [x] R7 `/export`（≤8 MiB、文件名 sanitize、空会话/失败可读；serve `/download` 仅注册 workspace 的 exports；Tauri 展示/复制路径，不承诺自动打开）、`/feedback`（本地 JSON、分类/描述≤4 KiB、明示本地保存、不联网）；六态、i18n、axe、键盘。
- [x] 共享 Modal：`ui-kit` 新增唯一 `Modal`（`index.tsx` 导出），`ui-sessions` 私有 Modal 删除并迁移（workspace rename/remove 确认行为/Esc/焦点不回归），composer dialogs 复用；无第二套 Dialog（C17）。

## 3. 交互 ID 覆盖

| ID | 测试（`e2e/interaction/command-surface.spec.ts`） | 证据 |
| --- | --- | --- |
| CMD2-MENU-01 | 两节分组/过滤/接受关闭键/无 UI 动作 | `CMD2-MENU-01.json` |
| CMD2-MENU-02 | 未知命令/Batch2 占位不执行不发送 | `CMD2-MENU-02.json` |
| CMD2-FILE-01 | `/file` 打开选择器 + 其它命令附件拒绝 | `CMD2-FILE-01.json` |
| CMD2-MODEL-01 | personal 模型+effort、会话级持久化、全局 settings 不变 | `CMD2-MODEL-01.json` |
| CMD2-PERM-01 | 三档审批映射与持久化 | `CMD2-PERM-01.json` |
| CMD2-GOAL-01 | `/goal` 确认/GoalBar 暂停恢复清除/不发送 | `CMD2-GOAL-01.json` |
| CMD2-EXPORT-01 | 空会话错误、Markdown 落盘、`/download` 下载 | `CMD2-EXPORT-01.json` |
| CMD2-FEEDBACK-01 | 本地保存 JSON、超限校验 | `CMD2-FEEDBACK-01.json` |
| CMD2-KEY-01 | 键位/IME 安全 | `CMD2-KEY-01.json` |
| CMD2-ERR-01 | 命令错误/后端失败保留草稿、不假成功 | `CMD2-ERR-01.json` |
| CMD2-A11Y-01 | axe + 键盘遍历（菜单/dialog/GoalBar） | `CMD2-A11Y-01.json` |
| CMD2-VIS-01（P2） | 像素基线 | UNVERIFIED（人工确认后接入） |

## 4. 文档与决策

- [x] `docs/19` §2.1b（DSH `/` 命令面两 surface/键位/错误/GoalBar/模型/权限/导出/反馈）；`docs/26` §5.2（新 IPC/错误码/override/export/feedback 文件约定）。
- [x] `KNOWN-LIMITATIONS`：D3–D5 差异（无模型发现源、permission 非 sandbox、goal=driver 展示、export 不自动打开、feedback 本地、并发未长时压测、基线 UNVERIFIED）。
- [x] 决策沿用 proposal §7 D1–D6（Lead 2026-11-01），不新增 docs/15 D 编号；不新增 capability/slot/包。

## 5. 回滚

- 回退 commit；新增 IPC 与字段均 additive；`.zeokeel/{conversation-overrides.json,goal.json,exports,feedback}` 为工作区本地文件，旧客户端忽略；无 DB 迁移。config-mutex 恢复路径保证回滚后全局设置仍为最新 snapshot。

## 6. 剩余风险 / 后续

| 项 | 影响 | 后续 |
| --- | --- | --- |
| personal 无 provider 模型发现源 | 中 | 手动输入 + recents（诚实降级，DSH 1:1 语义） |
| permission 仅审批、无 sandbox | — | 已文档化，不宣称可限制员工（C6） |
| config-mutex 高并发仅双会话级测试 | 中 | 长时压测/真实多会话 UNVERIFIED |
| export 仅 Markdown、Tauri 不自动打开 | 低 | 后续可加更多格式/system.open_url 仅 http(s) |
| 视觉/aria 基线未确认；用户测试未执行 | 中 | reviewer + ≥5 人 think-aloud |

## 7. 结论

- [x] `done`：本地实现与证据齐全（UNVERIFIED 项见 §1）
- [ ] `abandoned`：——

## 8. 复用证据（C17）

- 复用：`tool.goal` driver 与 `.zeokeel/goal.json`（无第二套 goal 存储）、`ApprovalPolicy::should_ask`/agent-rig policy 单点（经 generated facade `set_approval_preset`）、`zk_kit_tools::safe_join_under`（override/export/feedback 路径）、附件流程（`/file`）、`session.export` 消息读取（SessionStore）、ui-kit 唯一 Modal、`ui.commands` 单一 catalog（palette/slash surface）。
- 新增包内实现：`apps/desktop/src/command_surface.rs`（override/goal/export/feedback + config-mutex）、`ui-chat/{CommandDialogs,GoalBar,FeedbackDialog}.tsx`、共享 `ui-kit/Modal.tsx`；无新建 `utils/helpers`、无第二套菜单/执行器/Dialog。
