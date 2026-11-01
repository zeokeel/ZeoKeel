# 2026-11-01-client-approval-tool-rendering · 验收与证据

- 验收人：dsh-ref（实现者；待非实现者复评）
- 日期：2026-11-01
- PR/commit：未提交（工作区改动；提交后回填 commit/tag）
- 评审：`docs/reviews/2026-11-01-client-ui-parity-specs.md`（本 spec 为复评对象；通过后回填复评结论）
- 证据根：`target/evidence/2026-11-01-client-approval-tool-rendering/`（截图 `screenshots/`、axe `axe.json`、golden `golden/`、日志 `logs/`）

> 状态：**实现完成，证据已填；待非实现者复评通过后转 `done` 并归档**（不预写「已通过」）。证据根 `target/evidence/2026-11-01-client-approval-tool-rendering/`。

## 1. 验收命令与结果

| 命令 | 结果 | 证据路径 |
| --- | --- | --- |
| `cargo test -p zk-contract-agent -p zk-contract-tool -p zk-contract-session` | PASS（10 tests） | `target/evidence/2026-11-01-client-approval-tool-rendering/logs/contract-tests.txt` |
| `cargo test -p zk-kit-tools` | PASS（12 tests） | `.../logs/kit-redaction-tests.txt` |
| `cargo check -p zk-agent-loop -p zk-tool-session-query`（P1-R2 编译面） | PASS | `.../logs/cross-package-check.txt` |
| `cargo test -p zk-tool-fs-write -p zk-tool-fs-edit -p zk-tool-bash -p zk-tool-pwsh -p zk-tool-worktree -p zk-connector-mcp` | PASS（28 tests） | `.../logs/tool-metadata-tests.txt` |
| `cargo test -p zk-agent-rig`（事件链 S1–S5 golden） | PASS（37 lib + 13 integration = 50 tests） | `.../logs/agent-rig-tests.txt` |
| `cargo test -p zk-store-session-sqlite`（v5 迁移/写入顺序 golden） | PASS（11 tests） | `.../logs/store-tests.txt` |
| `cargo test -p zk-desktop`（broker typed/超时、approval.decide 4 值+alias+非法、BridgeSink 落库 args 已脱敏/turn_id 正确） | PASS（62 tests；新增 BridgeSink 2 项、broker typed 1 项） | `.../logs/desktop-tests.txt` |
| `pnpm --dir packages/ui/ui-kit test:unit` / `pnpm --dir packages/ui/ui-chat test:unit` / `pnpm --dir apps/desktop/web test:unit` | PASS（ui-kit 3 / ui-chat 8 / desktop-web 125） | `.../logs/web-unit-ui-kit.txt`、`.../logs/web-unit-ui-chat.txt`、`.../logs/web-unit-desktop.txt` |
| `pnpm -r typecheck && pnpm -r lint && pnpm -r test:unit` | PASS（含 build；见 logs/pnpm-gates.txt） | `.../logs/pnpm-gates.txt` |
| `bash scripts/i18n-check.sh` | PASS（exit 0） | `.../logs/i18n-check.txt` |
| `bash scripts/agent-e2e.sh`（真实 personal rig + `e2e/agent-rig.spec.ts`） | PASS（15/15 passed，42.3s） | `.../logs/agent-e2e.log`、`.../screenshots/`、`.../axe.json` |
| 提交后复跑 `bash scripts/agent-e2e.sh`（HEAD `ee932b7` 基线） | **exit 0：15 passed（43.0s）** | `.../logs/agent-e2e-final.log` |
| 提交后复跑 `pnpm e2e`（mock 回归） | **exit 0：16 passed（56.8s）** | `.../logs/pnpm-e2e-final.log` |
| 提交后 `cargo test --workspace`（Lead 独立复跑） | **exit 0** | 本次会话输出；见 §6 备注 |
| `cargo test --workspace export_schema && git diff --exit-code` | PASS（等价幂等校验：连续两次 export 后 schema/ts 全量 sha256 一致；`git diff` 非空仅因本 spec 的生成物待提交，见 golden/schema-hashes.txt） | `.../logs/export-schema.txt` |
| `bash scripts/schema-check.sh` | PASS（schema-check ok） | `.../logs/schema-check.txt` |
| `cargo check --workspace --all-targets`（P1-R2 跨包字面量编译面） | PASS（exit 0） | `.../logs/workspace-check.txt` |
| `cargo tree -p zk-contract-tool`（断言输出无 `zk-contract-agent`，P0-13.1 无环） | PASS（输出 0 行 zk-contract-agent） | `.../logs/contract-deps.txt` |
| `cargo public-api diff`（`docs/04` §10；**本机未安装 → UNVERIFIED，不冒充已跑**） + 两侧反序列化对拍样本 | **UNVERIFIED** | `.../logs/public-api.txt`（或记录工具不可用） |
| `./target/debug/zk capability --check` | PASS（45 entries，无新增） | `.../logs/capability-check.txt` |
| `./target/debug/zk compose --edition personal --profile default --channel official --check` | PASS（17 files up to date） | `.../logs/compose-check.txt` |
| `python3 scripts/spec-check.py && python3 scripts/docs-check.py && python3 scripts/review-check.py` | PASS（另有 docs-command-check/agent-tool-matrix） | `.../logs/spec-gates.txt` |
| 额外回归：`cargo test --workspace`（全 workspace，含 UI 无关 crate） | PASS（exit 0；431 个 test result 行，无 FAILED） | `.../logs/workspace-tests.txt` |

> e2e 运行器说明（P1-14）：审批/工具竞态用例**必须**用 `bash scripts/agent-e2e.sh`（真实 personal 组合 + 假上游 + `playwright.agent.config.ts`）；
> `pnpm e2e`（`scripts/m1-e2e.sh` 的 mock profile）**不覆盖**审批代码，不作为本 spec 证据；已作为回归实跑 PASS（`logs/pnpm-e2e.log`，m1-a11y/m1-serve 等 16+8+16 tests，exit 0）。

## 2. 手工检查

- [x] 审批卡展示：动作摘要/完整命令（脱敏）/工作目录（越界标记）/读写路径 + 网络目标/diff 展开/风险/倒计时
- [x] 默认焦点在「拒绝」；`Esc` 拒绝；快捷键不默认确认
- [x] 超时拒绝：倒计时归零后显示「已超时拒绝」，迟到点击无效
- [x] 「始终允许（本会话）」：同工具不再弹决策；`approval_requested{auto:true} → approval_resolved{allow_session}` 成对；新会话恢复询问
- [x] 拒绝/取消：工具卡终态「已拒绝/已取消」；刷新后仍存在
- [x] **工具执行中取消**（审批卡 cancel 与 `chat.cancel` 两条入口各验一次）：刷新后工具卡为 `cancelled`、assistant 行 `status=cancelled`、不白屏；**result 非空时显示「已取消（副作用可能已发生）」提示且保留原结果文本（不伪造失败）**
- [x] 刷新后工具卡完整恢复（名称/脱敏参数/结果/状态/归属 turn）；旧会话（无 `turn_id`）按 legacy 规则归并不跨轮
- [x] 损坏的 `role="tool"` 行降级为可折叠原文，不白屏
- [x] usage 实时递增且归属当前 turn，终态一致
- [x] 含 `sk-…`/`Bearer …`/`KEY=…` 的 bash 调用：审批卡与会话 DB 均掩码
- [x] 未声明元数据的工具卡显示「该工具未声明影响面」，不阻塞、不伪造
- [x] axe 无 serious（证据 `.../axe.json`，由 e2e `AxeBuilder` 断言）
- 说明：默认焦点/超时/always-allow/刷新/执行中取消（`chat.cancel` 入口）/掩码/axe 已由 `e2e/agent-rig.spec.ts` 实跑；Esc 拒绝由 `ApprovalPart` 的 window `keydown` 实现并经代码评审，**未单独 e2e**（不冒充手测）；**审批卡 cancel 入口未单独 e2e**，其路径为 `approval.decide{cancel}` → `ApprovalBroker::resolve_scoped(Cancel)` + `runtime.cancel_agent()`（`apps/desktop/src/ipc.rs` 代码审查），broker typed 通信由 `zk-desktop` 单测覆盖 AllowSession/Reject/release_all；「未声明元数据」降级卡与「越出工作区」标记为代码审查项，未单独 e2e；DB 掩码见 `golden/db-tool-rows.txt`（`secret_leak_rows=0`、`args` 掩码、`status=cancelled,result=hi`）。

## 3. 文档与决策

- [x] `docs/19` §4.1/§4.2 已记录覆盖 7/9+1/9 与三项非目标（记住范围仅路径/超时延长一次/二次确认）、通知 Phase 3
- [x] `docs/38` §2/§3 已记录 `ApprovalDecision` 4 值 + 旧值 alias、`ToolCallStatus`、S1–S5 事件序列表（S3 拆 S3a–S3c）
- [x] `docs/24` 契约依赖方向：agent→tool 单向；`ApprovalMetadata`/`ApprovalRisk`/`ApprovalDecision` 定义在 tool、agent re-export；禁止 tool→agent（P0-13.1）
- [x] `docs/28` §4.6 **分开写**：「工具终态枚举」= success|failed|denied|cancelled（持久化）+ pending|running（仅内存）；「组件六态」= AGENTS §7（空/加载/错误/离线/流式中/已取消）；审批决策枚举 4 值——不得混写（P2-N3）
- [x] `KNOWN-LIMITATIONS.md` 已写脱敏边界/legacy 归并/shell 网络目标边界
- [x] parity 报告缺口状态已回写
- [x] `docs/README.md`：不适用
- [x] Lead 2026-11-01 决策（网络目标/元数据缺失/strict-always-allow/legacy 归并/不新增 `approval_skipped`/config 对齐）已记录于 proposal §7 与 spec §6
- [x] Lead 2026-11-01 第三轮决策（P1-R2 选项① `Tool::sensitive_arg_keys` 默认方法 + 跨包编译面 + workspace check；P1-R3 终态先落库/取消单一路径；P2-N2/N3/N10）已记录于 proposal §7 与 spec §6
- [x] Lead 2026-11-01 第四轮决策（P0-13.1 类型移到 `zk-contract-tool` + agent re-export + docs/24 依赖方向；P1-13.2 执行中取消终态；P2-N16 public-api UNVERIFIED）已记录于 proposal §7 与 spec §6

## 4. 回滚

- 回滚步骤：回退 commit；事件/会话字段 `serde(default)`；SQLite v5 仅加列（旧二进制忽略新列）→ 数据无需回滚
- 数据影响：无破坏性迁移；新库 v5 对旧版本可读（固定列 SELECT）
- 已验证回滚：回退 commit 后 `cargo check --workspace --all-targets` 仍绿（字段全部 `serde(default)`；SQLite 旧二进制忽略 `turn_id` 列，仅加列迁移）

## 5. 剩余风险 / 后续（含评审 T5/P1-12 口径）

| 项 | 影响 | 归属/后续 |
| --- | --- | --- |
| `docs/19` §4.1「记住范围：仅本会话/仅该工具/**仅该路径**」未覆盖（只做「始终允许（本会话）× 按工具名」） | 中 | Phase 2/3（proposal §3） |
| `docs/19` §4.1「超时……可延长一次」未覆盖（只做剩余时间 + 超时默认拒绝） | 低 | Phase 2/3 |
| `docs/19` §4.2「危险操作二次确认」未覆盖 | 中 | Phase 2/3 |
| `docs/19` §4.2 系统通知/托盘红点/失焦通知 | 中 | Phase 3 诊断/通知 |
| 脱敏无法保证自由文本/自造格式密钥（base64、任意 random） | 中 | `KNOWN-LIMITATIONS` 边界；后续可扩展 redaction 规则 |
| legacy 工具行（无 `turn_id`）归并依赖「后一条 assistant」启发式 | 低 | 旧数据一次性；Phase 2 评估结构化 `tool_calls` |
| 未实现 `approval_metadata` 的第三方工具卡缺影响面 | 低 | 显示 degraded；生态工具按需补齐 |
| shell 网络目标无法静态判定（`network_undetermined`） | 中 | 诚实展示；不做流量拦截（C6） |
| 审批跨重启/全局永久允许 | 低 | 不提供（`docs/19` §4.1） |
| 工具卡 keyed view（54 种） | 低 | Phase 2 capability surfaces |
| 预算例外（~42 文件 / 900–1300 行，超模板 10 文件/400 行阈值） | 低 | **Lead 批准例外（2026-11-01；第四轮 ~42 文件，含 docs/24）**，见 proposal §8；范围扩大则按 Phase 1a/1b/1c 拆分 |
| config spec R2 与本 spec 事件序列表的合并引用 | 低 | 已与 config 作者对齐（Lead 2026-11-01）；R2 引用由 config 作者在复评前落实 |
| 跨包字段编译面（agent-loop / tool-session-query tests 字面量） | 中 | P1-R2 选项①：`ToolSpec` 不动；agent-loop(src+tests)/tool-session-query tests 进写作用域 + `cargo check --workspace --all-targets`（tasks #1b/#14） |
| 取消路径终态行丢失（BridgeSink cancelled 短路） | 中 | P1-R3：终态先落库再抑制 bus；agent-rig 单一 `TurnCancelled`；取消刷新 e2e（tasks #5/#7/#12） |
| 契约类型归属错误导致 agent↔tool 依赖环 | 低 | P0-13.1：类型移 tool、agent re-export；`cargo tree -p zk-contract-tool` 无 agent；workspace check（tasks #0/#1/#14、§1） |
| 执行中取消语义歧义（副作用被伪装成取消/成功） | 中 | P1-13.2：冻结 cancelled + result 原样 + 副作用提示；result 空/非空区分执行前/后；慢工具单测+e2e（tasks #5/#9/#12） |
| `cargo public-api` 工具未安装 → 契约 API 证据缺位 | 低 | P2-N16：acceptance 记 UNVERIFIED（不冒充）；`docs/04` §10 对拍样本 + cargo tree/workspace check 补证据 |

## 6. 结论

- [x] `done`：实现与证据齐全；**待非实现者复评通过**后转 done/归档（当前不称「已通过」）
- [ ] `abandoned`：原因：不适用

## 7. 复用证据（C17）

- 搜索记录：`rg "ApprovalGate|approval.decide|role.*tool|parseAgentEvent|approval_metadata" packages apps`
- 能力索引复用：`contract.agent`（`shared/contracts/capability-index.json:11`）、`contract.tool`（`:71`）、`kit.approval`（`:91`）；**本 spec 不新增 capability/INDEX 条目**
- 复用清单：`ApprovalBroker`、`BridgeSink`、`ToolCallPart`/`ApprovalPart`、`zk-kit-tools`、`translate`、`scripts/agent-e2e.sh` + `e2e/agent-rig.spec.ts`
- 新增包内实现（不登记 INDEX）：工具行解析/归并 `groupToolRows` 实现于 **`packages/ui/ui-kit/web/src/toolRows.ts`**（单一实现），`ui-chat/web/src/toolRows.ts` 仅 `export ... from "@zeokeel/ui-kit"` 以保留 design 的入口名；原因：app 只允许 import `@zeokeel/ui-kit`（`eslint.config.js` docs/27 §3 rule 2），`@zeokeel/ui-chat` 被 lint 禁止 → 归并逻辑放 kit、ui-chat re-export，无第二份实现。`zk-kit-tools::redact_tool_args`（单一脱敏）。
- 契约类型归属（P0-13.1）：`zk-contract-tool` 定义 `ApprovalMetadata`/`ApprovalRisk`/`ApprovalDecision`；`zk-contract-agent` 仅 `pub use` 兼容旧路径（无新增依赖，agent→tool 已存在）

## 8. 评审条目关闭对照（复评用）

| 评审条目 | 关闭依据（实现/证据落点） | 复评核对 |
| --- | --- | --- |
| P0-2 审批链双实现/互斥事件链 | 单 owner=agent-rig（design §2.0 求值顺序、tasks #5）；broker 无允许集合（tasks #7）；S1–S5 事件表（S3 拆 S3a–S3c） | `cargo test -p zk-agent-rig` golden + 代码审查 |
| P0-3 工具行方向/状态不可达 | `Message.turn_id` + SQLite v5（tasks #1/#6）；denied/cancelled 终态事件（#5）；BridgeSink turn 归属（#7）；toolRows 归并（#9）；store golden + 刷新 e2e（#6/#12） | store 测试 + `scripts/agent-e2e.sh` |
| P1-4 capability 登记错误 | 删除 `contract.approval`；复用既有 id（proposal §9、design §1.5）；`zk capability --check` 验收 | acceptance §1 |
| P1-5 生成物门禁 | export_schema + schema-check 进 tasks #2/#14 与 acceptance §1 | 命令 exit 0 + `git diff` 空 |
| P1-11 决策 wire 漂移 | typed 4 值 + alias（tasks #1/#7）；前端/协议（#8/#10）；单测 | `cargo test -p zk-desktop` + 前端单测 |
| P1-12 §4.1 100% 不实 | 7/9 完整 + 1/9 部分 + 非目标（proposal §3/§5、spec R2） | 文档对照 `docs/19` §4.1 |
| P1-13 无脱敏 | `sensitive_arg_keys` + 单一 redaction（tasks #3/#5/#7）；负向 e2e（#12）；边界（#13） | 卡片/DB 断言 + `KNOWN-LIMITATIONS` |
| P1-14 e2e 错误 profile | `bash scripts/agent-e2e.sh` + `e2e/agent-rig.spec.ts`（tasks #12、acceptance §1 说明） | e2e 日志 |
| P1-16 元数据由 agent-rig 猜/读文件 | `Tool::approval_metadata` 工具侧（tasks #4）；agent-rig 只聚合（#5） | 工具包单测 + 代码审查 |
| P1-1/P1-2 跨 spec/文档先行 | tasks §0/§5；先文档后代码 | tasks 勾选 + docs 链接 |
| P2-1 行号漂移 | 本 spec 引用改为实测行号（proposal/design） | diff 检查 |
| P2-2 两个 `## 6` | 本文件章节：结论 §6、复用 §7 | 结构检查 |
| P2-9 证据路径未定义 | 预填 `target/evidence/2026-11-01-client-approval-tool-rendering/` | acceptance §1/§2 |
| **P1-R2** 跨包字段编译面 | `Tool::sensitive_arg_keys()` 默认方法（不动 `ToolSpec`）；agent-loop(src+tests)/tool-session-query tests 进写作用域（tasks #1b）；`cargo check --workspace --all-targets` 门禁（#14、§1、§4） | workspace check 日志 + 代码审查 |
| **P1-R3** 取消路径被 BridgeSink 抑制 | BridgeSink 终态先落库再判断 cancelled（bus 仅抑制非终态、TurnCancelled 仍发）；assistant status 由 `turn_cancelled` 标志决定；approval-cancel 单一路径（resolve(Cancel)+cancel_agent；agent-rig 统一 TurnCancelled）（design §2.5.2–§2.5.3、tasks #5/#7） | `zk-desktop` 单测 + 取消刷新 e2e（#12） |
| **P2-N2** cancel/reject 措辞不一 | 唯一映射 design §2.0 S3a–S3c（超时 reject+timed_out；显式 cancel；断线 reject）；spec R2/R8/§2 只引用本表 | 文档对照 + 事件链 golden |
| **P2-N3** docs/28 概念混用 | 拆分「工具终态枚举」与「组件六态」（tasks #13、§3） | docs 检查 |
| **P2-N10** 行号残余漂移 | `agent/src/lib.rs:165-174`（proposal §1） | diff 检查 |
| **P0-13.1** 跨契约依赖环 | 类型定义移 `zk-contract-tool`、agent `pub use` 保持旧路径（tasks #1）；docs/24 依赖方向（#0/#13）；`cargo tree -p zk-contract-tool` 无 agent + workspace check（#14、§1） | cargo tree 日志 + 代码审查 |
| **P1-13.2** 执行中取消终态 | outcome 后检查 cancelled → 发 `ToolCallFinished{cancelled}`、result 原样保留；UI cancelled+副作用提示（tasks #5/#9/#11）；慢工具单测+e2e（#5/#12） | `zk-agent-rig` 单测 + agent-e2e + §2 手工检查 |
| **额外（实现中发现）** typed 决策被 `GlobalGate` 默认实现折叠 | `zk-kit-approval::GlobalGate::decide_scoped` 必须转发到已安装 gate（否则 `allow_session`→`allow_once`、`cancel`→`reject`，always-allow/取消决策失效）；新增 `zk-kit-approval` 单测 | `cargo test -p zk-kit-approval` + always-allow/cancel e2e |
| **额外（实现中发现）** 非流式取消丢弃执行中工具终态 | 非流式 provider 的 50ms 取消轮询新增 `in_tool` 计数：工具执行中不立即返回 `AGENT_CANCELLED`，等 outcome 返回并由工具闭包发 `ToolCallFinished{cancelled}`（P1-13.2 的实际落地） | `cargo test -p zk-agent-rig`（execution_cancel_keeps_produced_result）+ 取消刷新 e2e |
| **P2-N16** public-api 证据缺位 | acceptance 增 `cargo public-api diff`（未装 → UNVERIFIED）+ `docs/04` §10 反序列化对拍样本（tasks #14、§1/§5） | 工具不可用记录 + 对拍样本 |
