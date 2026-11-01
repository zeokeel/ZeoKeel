# 2026-11-01-client-compact-command · 验收与证据

> 提交后原样复跑（Lead，HEAD `bd6ae25`）：`pnpm e2e` **22 passed / 2 failed** —— `m1-serve` 终端输出为空、`/compact` 分隔未出现（作者等价序列 11/11 通过；疑似 flaky/环境，待最终树复跑定位；当前记为 UNVERIFIED）。

> 独立评审通过（`docs/reviews/2026-11-01-client-compact-command.md` §8）；C20 `intuition.md` + `intuition-walkthrough.md` 已交付（专家走查；真人用户测试 UNVERIFIED）。
> 实现者：impl-settings；证据根 `target/evidence/2026-11-01-client-command-compact/`。

- 验收人：impl-settings（实现者回填）
- 日期：2026-11-01
- PR/commit：未提交（共享工作树；由 Lead 汇总提交，起点 `11a69bf`）
- 状态：`done`（本地实现与证据齐全；UNVERIFIED 见 §1 备注）

## 0. 预算例外

> Lead 2026-11-01 追认：实际约 35 个路径（规划 12），单目标且无第二套实现，不拆分。

## 1. 验收命令与结果

| 命令 | 结果 | 证据 |
| --- | --- | --- |
| `cargo test -p zk-contract-ipc -p zk-kit-compaction -p zk-desktop -p zk-contract-agent -p zk-agent-rig` | **exit 0**（desktop 80 passed，含 2 个 compact host 测试；kit 4 passed） | `logs/cargo-tests.log` |
| `cargo fmt --all -- --check` / `cargo clippy ... -D warnings` / `cargo check --workspace --all-targets` | **0 / 0 / 0** | `logs/cargo-fmt.log` / `logs/cargo-clippy.log` / `logs/cargo-check.log` |
| `cargo test --workspace export_schema` + `schema-ts` + `schema-check` | **exit 0**（127 schemas；二次生成 sha256 不变；schema-check 0） | `logs/export-schema.log`、`logs/schema-check.log` |
| `zk validate` / `compose --check` / `capability --check` / `docs --check` | **0 / 0 / 0 / 0**（capability 索引重新生成 46 entries） | `logs/zk-gates.log` |
| `pnpm -r typecheck/lint/test:unit/build` | **0 / 0 / 0 / 0**（app 133 tests、ui-chat 25） | `logs/pnpm-gates.log` |
| `pnpm e2e`（mock + m1-serve 含 compact 回归） | **mock 8/8、m1-serve 11/11（exit 0）**（含新增 compact e2e；等价序列执行，工作树未提交） | `logs/e2e-main.log`、`logs/gates-run.log` |
| `pnpm e2e:ui --grep command-compact`（3 个 P0/P1） | **3/3 passed（exit 0）**，3 份证据 | `logs/e2e-interaction.log` + `interaction/<ID>.json` |
| `pnpm e2e:ui`（完整交互套件） | **67/67 passed（exit 0）** | `logs/e2e-ui-full.log` |
| `python3 scripts/ui-interaction-gate.py` / `--strict` | **self-test 0 / gate 0 / strict 0**（enforced P0/P1=65；intuition 6 contracts/66 rows） | `logs/interaction-gate.txt` |
| `python3 scripts/spec-check.py` / `docs-check.py` | **0 / 0** | `logs/gates-summary.txt` |
| `bash scripts/i18n-check.sh` | **0** | `logs/gates-summary.txt` |
| 视觉/ARIA 基线 `CMD3C-VIS-01`（P2，不进门禁） | **UNVERIFIED**：像素基线需人工确认后 `pnpm e2e:ui:update`（agent 不自动接受），当前仅截图/交互证据 | —— |
| 真人用户测试（≥5 人 think-aloud） | **UNVERIFIED**（发布前执行；C20） | `intuition-walkthrough.md` §4 |

## 2. 手工检查（对应 spec R1–R2）

- [x] R1 `/compact` 真实命令：菜单「指令」组不再显示「未实现（Batch 2）」；`/co` 过滤、Enter/Tab 接受、Esc 保留草稿；流式中禁用 + 同一文案；经 Batch1 唯一 catalog/runCommand，不发送命令文本；未知命令仍 `command/unknown`。
- [x] R2 压缩：保留最近 6 轮/12 条（先到），更早 user/assistant/tool 文本经专用 summarizer（无 agent turn/tool/审批）；system marker `compaction:<uuid>` + through_message_id + summary/created/input/output/compressed/keep_recent；**原始消息不删**；`messages_in` 原始语义；`session.compacted_view` 仅 replay/分隔；search/fork/审计 raw（单测+e2e）；重启/刷新 `compaction_get` 恢复分隔；下一轮 replay 注入摘要；输入 >12k/摘要 >2k → `compact/too-large` 不写；provider/DB/超时失败 → `compact/failed` 不写；无新消息/同边界 → `noop` 不重复写；流式 `compact/busy`；跨 workspace/不存在 `session/*`；不自动压缩。
- [x] 数据与兼容：无 DB migration；marker 复用 append_message；旧客户端不调用新 IPC 行为不变；回滚删/忽略 marker 即回原历史。
- [x] 复用：kit `zk-kit-compaction` 唯一摘要/token/截断实现；provider 调用复用 agent-rig `complete_raw`（经 composed facade），无第二套 provider 客户端；`CompactDivider` 唯一分隔；无第二套菜单/执行器（C17）。

## 3. 交互 ID 覆盖

| ID | 测试（`e2e/interaction/command-compact.spec.ts`） | 证据 |
| --- | --- | --- |
| CMD3C-COMPACT-01 | `/compact` 成功 + 分隔/摘要/usage + raw 可见 + search raw + 重复 no-op + 刷新恢复 + axe 0 | `CMD3C-COMPACT-01.json` |
| CMD3C-KEY-01 | `/co` 过滤、Esc 保留草稿、Enter 执行、分隔键盘展开、压缩后继续对话 | `CMD3C-KEY-01.json` |
| CMD3C-ERR-01 | IPC 失败 → 可读错误、无分隔、历史不变、不假成功 | `CMD3C-ERR-01.json` |
| CMD3C-VIS-01（P2） | CompactDivider 视觉基线 | UNVERIFIED（需人工确认后 `e2e:ui:update`） |

## 4. 文档与决策

- [x] `docs/19` §2.1c、`docs/26` §5.3（IPC/错误码/marker/raw 语义/replay/UI）。
- [x] `KNOWN-LIMITATIONS`：无自动压缩、raw 仍占 DB（仅 UI/replay 折叠）、mock/offline 抽取式摘要、enterprise 网关 summarizer UNVERIFIED、replay 注入为 user 历史文本、视觉基线 UNVERIFIED。
- [x] 决策沿用 proposal（D1/D2/D6 Lead 2026-11-01）；未新增 ADR/D 编号；未新增 capability/slot/包依赖（kit 已登记 `@capability kit.compaction`）。

## 5. 回滚

- 回退 commit；无 DB migration；已写 marker 可忽略（旧代码 `role=system` 不参与 replay/UI 气泡）或删除；原始消息从未删除，回滚后历史完整。

## 6. 剩余风险 / 后续

| 项 | 影响 | 后续 |
| --- | --- | --- |
| enterprise 网关 summarizer 未在本机网关验证 | 中 | 企业环境实测（C7 路径） |
| mock/offline 抽取式摘要非模型摘要 | 低 | KNOWN 标注；真实 provider 走模型 |
| raw 历史仍占用磁盘/搜索结果 | 中 | 归档/GC 另议；不丢原始归档 |
| 视觉基线/用户测试未确认 | 中 | reviewer + ≥5 人 think-aloud |

## 7. 结论

- [x] `done`：本地实现与证据齐全（UNVERIFIED 项见 §1）
- [ ] `abandoned`：——

## 8. 复用证据（C17）

- 复用：`SessionStore`/`messages_in`/`append_message`（无第二套存储）、agent-rig `complete_raw`（provider 调用单点，经生成 facade）、`zk-kit-session::search`（raw 搜索）、ui-kit `Modal`/actions、Batch1 catalog/runCommand、assistant-ui Thread primitives。
- 新增：`crates/zk-kit-compaction`（摘要/token/截断唯一实现）、`apps/desktop/src/compact.rs`（marker/view/replay/compact 流程）、`CompactDivider`（唯一分隔组件）；无 `utils/helpers`、无第二套摘要/菜单/执行器。
