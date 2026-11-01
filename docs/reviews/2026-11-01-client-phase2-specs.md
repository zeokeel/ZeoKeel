# 2026-11-01-client-phase2-specs · 独立评审（C19）

- 状态：本轮评审完成，**CHANGES-REQUESTED**（P0=1 / P1=5 / P2=15，均 OPEN；见 §7）。
- 评审对象：HEAD `41249878f465edd4ae2af5958f5ebb10a6241dc9`（Phase 1 四份 spec 已合并）+ 三份 Phase 2 spec 共 17 文件 SHA256（15 五件套 + composer/side 的 `interaction.md` 2 个；activity 仍缺；§0.1；`draft`，用户已批准 D1–D6/D1–D6/D1–D4）。
- 关联 spec：`2026-11-01-client-composer-surface`、`2026-11-01-client-side-panels`、`2026-11-01-client-activity-panels`。
- 评审者：`spec-reviewer`（C19 独立只读评审；Phase 1 四份 spec 复评记录见 `docs/reviews/2026-11-01-client-ui-parity-specs.md`）。
- 与实现者关系/独立性：未参与三份 Phase 2 spec 的写作、设计或实现；本评审只读取 HEAD 代码/spec/docs/research 与可执行门禁；唯一写入为本记录（C19）。
- 评审日期：2026-11-01。
- 复评日期/结论：2026-11-01 / CHANGES-REQUESTED（首轮）；修复后由同一评审员复评。
- 结论：**CHANGES-REQUESTED**（P0=1、P1=5、P2=15；详见 §7/§8）。
- 写入边界：只写本文件；未修改任何 spec/代码/生成物/配置；`git status --porcelain` 仅显示三份新 spec 目录（未跟踪）与本记录。

## 0. 方法与事实基线

### 0.1 评审对象 SHA256（17 文件）

```text
222a6a8a1b36adb18f0b422bfe71e07c26c0a5cdce1026e747f2c4120dc8dc0e  specs/changes/2026-11-01-client-side-panels/design.md
365f4fdc1a91b84d57bbd580938a0521b665b2dc5dfc93f3a0ca697cb2cc426d  specs/changes/2026-11-01-client-composer-surface/tasks.md
41d6df1fb61c4dec988acc5cd33b6df92019f77ea01906480d7cf5f38e437330  specs/changes/2026-11-01-client-composer-surface/interaction.md
42699bb493ac304d27f029feac0dd8c6c6736714b6404a3362284e8721fd50c4  specs/changes/2026-11-01-client-activity-panels/design.md
620667eb33d108ace9aa505dcdffcb5655e016ddf5a5295743170a1c54bc2d65  specs/changes/2026-11-01-client-side-panels/proposal.md
78ffb3103db894a706db4fec7f09cd46c26b688f702a4aad5d2009ac96cbf65d  specs/changes/2026-11-01-client-composer-surface/proposal.md
886199c37969d00a25c88558aa0d2167ac0472c9a7e0ddeb1f47b726bb076c41  specs/changes/2026-11-01-client-side-panels/interaction.md
9cba732618f04dceefda9b1c01196444585177f9cb06077cbb21c7fcba71aa5b  specs/changes/2026-11-01-client-activity-panels/acceptance.md
9da8172fa52b35db6a6f0f2c3b0df1476177b50664e694cf8dec891fccca0215  specs/changes/2026-11-01-client-composer-surface/spec.md
ae383461676dd00f3a97655431b05b6c27ea48e901d97c057608429c408c17a5  specs/changes/2026-11-01-client-side-panels/acceptance.md
bd4a8bc2afb06edccebf320baebbf518f7b0bb5c7d1123a90754ef8dc0b3b584  specs/changes/2026-11-01-client-activity-panels/spec.md
c0cc7bc57e6b11923bafea50622977ffd4a59d3cea197c29294e4c015c6577fc  specs/changes/2026-11-01-client-activity-panels/tasks.md
c2119a5c01d58d22ce6e0945b7b0d95a9fdecec9a911bb3cc8aed9390aa665c5  specs/changes/2026-11-01-client-side-panels/tasks.md
c7c78c1fd2201a0a953694a093b41eb03493cceb5946b099603fbdf4715e8636  specs/changes/2026-11-01-client-side-panels/spec.md
d2898e08ce3e192e0ff56de0fbab08ffbf72c4564e08a8d356e8d45373355252  specs/changes/2026-11-01-client-activity-panels/proposal.md
d7190900c208b511ae6f4ef8844b29a2e7f77d09b5f1c874ec2b72039299a9f5  specs/changes/2026-11-01-client-composer-surface/design.md
e1d42a813d3465f13d3675acdc43e687e2b75677e1e63d4e3d533db26289a0f8  specs/changes/2026-11-01-client-composer-surface/acceptance.md
```

> 注：评审期间（2026-11-01）工作树并发新增了 `client-composer-surface/interaction.md`、`client-side-panels/interaction.md` 并更新其 tasks/acceptance（这 4 个文件 hash 变化）；其余 spec 文件 hash 不变；本记录以**当前 17 文件**为对象；`client-activity-panels/` 仍无 `interaction.md`（P1-4）。并发改动还包括 AGENTS/docs/agent-rig 等（非本评审对象），代码事实仍以 HEAD `4124987` 为准。

### 0.2 方法

- 只读：三份 spec 全文（17 文件，1898 行 + 2 个 interaction 契约）、Phase 1 四份已实现 spec 的最终状态（依赖的顺序/基线）、HEAD 代码与生成物、`docs/16/19/24/26/28`、`research/client-dsh-parity-2026-11-01.md`、`target/audits/*`、`KNOWN-LIMITATIONS.md`。
- 事实核验：对 spec 引用的命令/API/数据形状逐条 `grep`/`sed` 查代码（§1，≥8 条；实际 18 条）。
- 合规：按 AGENTS.md C1–C19 检查（§2）。
- 可验收性：逐任务核对写作用域、依赖、运行器与 profile（§5）。
- 安全：附件、终端、C18 fork/archive、检索脱敏（§6）。
- 本机未安装/未执行：`cargo public-api`、真实 Tauri/三平台、真实 provider 等属实现阶段；本评审不冒充。

## 1. 事实核验（HEAD `4124987`，path:line）

| # | 事实点 | 代码证据（HEAD） | 与 spec 的关系 |
| --- | --- | --- | --- |
| F1 | ⌘K 命令目录现状 | `apps/desktop/web/src/App.tsx:591-607`：当前 `commands` 为 personal **4 条**（new-conversation/toggle-sidebar/open-settings/toggle-inspector），enterprise +3（skills/connectors/apps）=7；`runCommand` 在 `:608-616` | composer proposal:87/spec:120 称「现有 personal 6 条/enterprise 9 条」**不准确**；实际是 4/7，cancel/retry 由本 spec task 5 新增后才成为 6/9（P2-1） |
| F2 | composer 现状 | `packages/ui/ui-chat/web/src/AssistantComposer.tsx:13-53` 使用 `ComposerPrimitive.Root/Input/Send/Cancel` + `useAuiState`，仅计数/离线提示 | composer proposal:13 的「9-51 纯文本 textarea」描述基本成立（行号轻微漂移），替换为受控 textarea 的范围合理 |
| F3 | `workspace.files` 现状 | `apps/desktop/src/ipc.rs:702-716`、`:914-956`：`list_workspace_files(root)`，depth≤2、≤300，只跳过 `.git`；**`.zeokeel` 当前会被列出** | composer e2e 经 `/ipc workspace.files` 断言 `.zeokeel/attachments` 存在**可行**（F3）；side R1 要 UI 隐藏 `.zeokeel`，host 兼容需保留旧行为（side spec:13/97 成立） |
| F4 | 文件 DTO | `packages/contracts/session/src/lib.rs:84-124`：`FileKind`/`FileEntry`/`WorkspaceFilesResult` | composer proposal:122、side design:32 引用正确；side 扩展 `path/depth/limit` 需 additive |
| F5 | SQLite 版本 | `packages/client/store-session-sqlite/src/lib.rs:105-109`（`user_version`/`has_column`）、`:210` v5、`:308` `messages.turn_id` 迁移；当前 **user_version=5** | side spec:94「v6 加 pinned/archived」合理（Phase 1 已到 v5）；`has_column` 存在，迁移路径可行 |
| F6 | 会话契约现状 | `packages/contracts/session/src/lib.rs:37-48` `Conversation{id,workspace_id,title,created_at_ms,updated_at_ms}`；`:218-240` `SessionStore` 仅 create/list/append/messages | side spec:93-95 新增 pinned/archived + 默认方法可行；现有 Rust `Conversation` 字面量仅 `session/tests/conformance.rs:9` 与 `store-session-sqlite/src/lib.rs:371`（均在 side 写作用域内，compile surface 已覆盖） |
| F7 | 右栏现状 | `packages/ui/ui-inspector/web/src/index.tsx:23` `TABS=["files","changes","terminal"]`；`:31` 固定 `w-[288px]`；`:77` terminal 占位 `inspectorTerminalSkeleton` | side proposal:14-16 的行号/事实成立；Phase 2b 扩展合理 |
| F8 | 持久终端 API | `crates/zk-kit-tools/src/persistent.rs:293-404` 公开 `open_session`/`run_in_session`/`close_session`/`list_sessions`；`:19` `MAX_SESSIONS_PER_CONVERSATION=8`；`:99-111` session id 规则 | side D1/design:34/192 的复用描述成立；side R3 的 ≤8/会话上限有代码依据 |
| F9 | schedule 公开 API/形状 | `packages/client/tool-schedule/src/lib.rs:76-160`：`Tool` 支持 create/list/update/delete；list 返回含 `next_run_ms`；update 支持 enabled/interval/prompt/cron；**没有 prompt 长度上限校验**；delete 未知 id 返回 `"deleted 0"`（成功） | activity design:27/131 的复用路径成立；但 activity spec:57/86 的 prompt>32 KiB 需 **host 校验**（P2-13）；delete 未知 id 与 spec:86/106 的「拒绝」冲突（P1-3） |
| F10 | 工具行落库形状 | `apps/desktop/src/events.rs:88-102` 写 `role="tool"` 行，content=`{name,args,status,result}` + `turn_id`；`packages/contracts/agent/src/lib.rs:111-119` `ToolCallFinished.args: String`（红acted JSON **字符串**，非对象） | activity design:130 的「工具行→goal/todo/present 派生」可行；但 activity design 的 `PersistedToolRow.args: Option<Value>` 与工具行 `args` 为字符串不一致，需显式二次 `serde_json::from_str`（P2-10） |
| F11 | goal 结果形状 | `packages/client/tool-goal/src/driver.rs:263-300`：set 返回含 `objective/status/rounds/max_rounds/reason` 的 JSON；`:130-140` run 报告只含 `executed/reason/round/status/rounds/max_rounds/result`（无 objective） | activity design 仅接受含 objective 的结果（set/status/update/cancel）并跳过 run 报告，与代码一致（activity spec R4 成立） |
| F12 | todo/present/team 形状 | `packages/client/tool-todo/src/lib.rs:49-84`：list 返回数组、add 返回 `"added <id>"`；`tool-present/src/lib.rs:41` 返回 `{presented,description}`；`tool-agent-team/src/lib.rs:366-379` 返回 `{objective,status,workers[id,task,status],completed,failed,cancelled,total,...}` | activity spec R3/R5/R7 的派生规则与代码一致 |
| F13 | jobs/subagent API | `crates/zk-kit-jobs/src/lib.rs:119-175`：`list/output/kill`，snapshot 含 id/command/cwd/status/stdout/stderr（截断）；`crates/zk-kit-subagent/src/lib.rs:123-134` `AgentInfo{id,parent,depth,status,prompt}`、`:208` list、runner status/result/interrupt | activity design:21-22 的复用与过滤依据成立 |
| F14 | session_search 现状 | `packages/client/tool-session-query/src/lib.rs:28-60`：遍历 `list_conversations_in` + `messages_in`，`contains`，limit 50，返回**完整 `content`**（无 snippet） | side spec R6/design:189 要抽取 `zk_kit_session::search` 并返回 snippet ±60；task 5 却说 `tool-session-query` 行为不变（P2-6） |
| F15 | CSP | `apps/desktop/tauri.conf.json:20`：`img-src 'self' data:`、`frame-src 'none'`，**没有 `blob:`**；`serve.rs` 未设置 CSP 响应头 | side design:147 的 data URL 预览符合 CSP；composer design:107 的 `URL.createObjectURL` 缩略图（blob URL）在 Tauri 下**被 CSP 拦截**（P1-1） |
| F16 | 生成器模板归属 | `tools/zk/templates/app_desktop.Cargo.toml` 被 git 跟踪，但首行 `# @generated by zk compose -- do not edit (template: ...)`；`tools/zk/src/main.rs:1778-1782` **读取该模板**写 `apps/desktop/Cargo.toml`；当前模板无 `zk-kit-tools`/`zk-kit-jobs`/`zk-kit-subagent`/`zk-tool-schedule` | side task6/activity task3 要改「@generated 且作为生成器输入」的模板，C11 归属不清（P2-11） |
| F17 | capability 索引行号 | `shared/contracts/capability-index.json:42-44` contract.ipc、`:62-64` contract.session、`:106-108` kit.jobs、`:122-124` kit.session、`:130-132` kit.tools | 三份 spec 引用的 `:43/:63/:107/:123/:131` 与 gzip 行号一致（事实核验通过） |
| F18 | e2e 运行器 | `scripts/m1-e2e.sh:12-19` 第二段显式跑 `e2e/m1-serve.spec.ts` 等；`:10` mock 段 `--grep-invert "serve carrier"`；`e2e/m1-serve.spec.ts:10` suite 名为 `M1 serve carrier...`；`scripts/agent-e2e.sh:124` 支持 `ZEOKEEL_AGENT_GREP`，`:56-68` 已含 todo/goal/schedule/present 假上游分支 | composer/side 的 `pnpm e2e`（m1-serve）与 activity 的 `bash scripts/agent-e2e.sh` 运行器/分段正确（§5）；新增用例应放进对应 describe/文件以保证 mock 段跳过、serve 段执行 |
| F19 | 交互契约现状 | `docs/11-testing-and-evidence.md:169-175`：UI 变更必须有 `specs/changes/<id>/interaction.md`（`active/planned`）+ P0/P1 的 `@interaction:<ID>` 测试与证据；评审期间 composer/side 已新增 `planned` 契约（composer 17 个 ID、side 18 个 ID；acceptance 已列 ID/测试落点）；**activity 目录仍无 `interaction.md`**；`docs/11:175` 只列 composer/side，未列 activity | composer/side 的 planned 契约满足在途 spec 的 WARN 口径（可保留）；activity 缺契约（P1-4） |
| F20 | 交互门禁工具 | `docs/11-testing-and-evidence.md:155-156/171` 要求 `pnpm e2e:ui` 与 `python3 scripts/ui-interaction-gate.py --self-test && python3 scripts/ui-interaction-gate.py`；实际 `scripts/ui-interaction-gate.py` 不存在（exit 2）、根 `package.json` 无 `e2e:ui`、`e2e/interaction/` 不存在 | composer/side 新增的 tasks/acceptance verify 当前**不可执行**（P1-5）；不能以「门禁绿」冒充 |

## 2. C1–C19 合规检查

| 约束 | 结论 | 依据/问题 |
| --- | --- | --- |
| C1 个人版优先 | 通过 | 三份 spec 均 personal/enterprise 同面，无企业专属前置；composer/activity 不新增 provider 调用 |
| C2 构建期组合（无运行期加载） | 基本通过 | 三份均不新增 slot/包/运行期加载；activity 固定 tab、composer 组件内菜单；**activity 对 `zk-tool-schedule` 的 app 直接依赖需说明 C4/C2 边界（P2-12）** |
| C3 一切皆包 | 通过 | 无新包；复用 ui-inspector/ui-chat/ui-kit/ui-sessions 与 kit/tool |
| C4 契约是唯一稳定缝 | 有风险 | desktop 组合根已有 `zk-connector-mcp` 直接依赖先例；activity 再加 `zk-tool-schedule` 工具包直接依赖（side/activity template），需 Lead 接受并文档化（P2-12）；composer 自行实现路径安全而非复用 kit（C17，见 P0-1） |
| C5 requires 角色 | 通过 | 无 requires 变更 |
| C6 企业弱管控 | 通过 | 终端/activity 动作为本地显式用户操作，无企业审批/白名单；KNOWN 记录无审批/审计 |
| C7 企业 AI 只走网关 | 通过 | composer 模型标签只读；enterprise 不出现本地 key 控件 |
| C8 无部门/用户组 | 通过 | 无相关变更 |
| C9 TS/Rust 严格 | 通过 | 设计未引入 `any`/`as`/`unwrap`；实现阶段按 docs/18 |
| C9b 零 unsafe | 通过 | 无 unsafe 设计 |
| C10 外部输入校验 | 基本通过 | 新 IPC（attachment/activity/workspace.file_read/terminal/session.*）设计均要求 host 校验 + 生成 schema；**activity `activity.action` 的 `updates` 为自由 Value，需 host 明确校验（P2-10 相关）** |
| C11 生成物唯一所有权 | **有冲突** | 三份 spec 都有生成物任务；但 side/activity 要改的 `tools/zk/templates/app_desktop.Cargo.toml` 自身标 `@generated`（F16），需澄清模板是源还是生成物（P2-11） |
| C12 先文档后代码 | **不通过（P1-2）** | composer 新增 `workspace.attachment_save`、activity 新增 `activity.snapshot/action`，但两份的 docs 任务只有 docs/19/28/KNOWN（composer tasks:10/31）或 docs/16/KNOWN（activity tasks:11/33），**未更新 `docs/26` §5 IPC 命令/错误表**；Phase 1 mcp 已在该表登记 `mcp.test`（F9/docs/26:173-178），side 也把 docs/26 放进任务 0a |
| C13 更新通道 | 通过 | 无更新通道变更 |
| C14 渠道/登录 | 通过 | 无登录/渠道变更 |
| C15 参考仓不作依赖 | 通过 | 仅 research/audit 事实引用 |
| C16 无 spec 不开工 | **不通过（P1-4/P2-8）** | 五件套齐全、状态 draft、决策记录在 proposal §7；composer/side 已于评审期间补 `interaction.md`（planned，满足在途 WARN 口径），但 activity 仍缺（P1-4）；activity spec §6 仍把 D1–D4 标为「待定」与 proposal §7 已定矛盾（P2-8）；评审通过后再置 approved |
| C17 复用/单一实现 | **有风险** | jobs/subagent/goal/todo/present/schedule 复用清晰；**composer 附件路径安全明确「不复制 zk-kit-tools::safe_path 语义」，自行实现**（composer design:126/39、proposal §9），既触发 C17，也导致 P0-1 的父目录符号链接逃逸；side 反而把 zk-kit-tools 加进 desktop |
| C18 workspace 归属 | 基本通过 | 附件固定 `.zeokeel/<conv>` 且校验会话归属；side fork/archive 保持 workspace_id；search 限当前 workspace；**但附件父目录符号链接可越界（P0-1）**，activity jobs cwd 过滤需 component-aware（P2-14） |
| C19 发现即文档/评审后施工 | 通过（本记录） | 三份 spec 在评审前不改代码；本评审只写记录 |

## 3. 跨 spec 与 Phase 1 写作用域/顺序

### 3.1 依赖顺序（三份 spec 自身声明，核对一致）

`Phase 1 全部（已合并 @4124987）` → `composer-surface`（2a）→ `side-panels`（2b）→ `activity-panels`（2b 子 spec）。
- composer tasks §5、side tasks §0b/§5、activity tasks §5 的顺序声明一致；activity 明确依赖 side 的 `previewRequest`/file_read/预览。
- Phase 1 四份 spec 已实现合并；Phase 2 不与其争写同一未合并文件。

### 3.2 共享文件矩阵（写者为串行，建议 owner）

| 共享文件 | composer | side | activity | 顺序/owner 建议 |
| --- | --- | --- | --- | --- |
| `packages/contracts/ipc/src/lib.rs` | attachment_save | 5 命令 | 2 命令 | composer→side→activity；每段独立提交，后一 spec 在前者合并后改 |
| `packages/contracts/session/src/lib.rs` | 不写 | pinned/archived + 默认方法 | 不写 | side 独占 |
| `apps/desktop/src/ipc.rs` | attachment_save | files/file_read/terminal/session.* | activity.* | composer→side→activity；side/activity 均需在前者合并后改 |
| `apps/desktop/web/src/{App.tsx,controller.ts,protocol.ts,i18n.tsx}` | 是 | 是 | 是（activity 无 controller） | 同一顺序；side/activity 均声明独占各自阶段 |
| `packages/ui/ui-kit/web/src/{types.ts,context.tsx}` | 是 | 是 | 是 | 同一顺序 |
| `packages/ui/ui-inspector/web/src/index.tsx` | 不写 | 树/预览/终端 | activity tab | side→activity |
| `packages/ui/ui-chat/web/src/AssistantThread.tsx` | strip 引用块 | 不写 | turnTail | composer→activity |
| `e2e/m1-serve.spec.ts` | 追加 | 追加 | 不写（agent-rig） | **composer 与 side 都写，但两者 §5 的「独占共享文件」清单未列该文件（P2-4）** |
| `tools/zk/templates/app_desktop.Cargo.toml` | 不写 | 加 zk-kit-tools | 加 jobs/subagent/tool-schedule | side→activity；且文件标 @generated（P2-11） |
| `docs/19`、`docs/28`、`KNOWN-LIMITATIONS.md` | 是 | 是 | 部分 | 顺序写；owner 清单在 composer/side §5 不完整（P2-4） |

### 3.3 结论

**无硬性写冲突**（顺序声明清楚、后两者明确「前者合并后排队」）；但 (a) `e2e/m1-serve.spec.ts` 与 docs 的 owner 未在两份 tasks §5 完整登记，(b) activity 需要 side 的 `previewRequest`（side design 已提供），(c) 建议 Lead 在合并时按上表逐段验收：contract→desktop→ui-kit→component→app→e2e，避免跨 spec 的同一文件并行修改。

## 4. 预算核对

| spec | 声明预算 | 实际文件清单计数 | 结论 |
| --- | --- | --- | --- |
| composer | ≤15 | 契约 1 + desktop 1 + ui-kit 2 + ui-chat 3（AssistantComposer/AssistantThread/composer.test）+ desktop-web 4（App/controller/protocol/i18n）+ e2e 1 + docs 3 = **15** | 合规；但补充 docs/26 §5 会变 16，超 Lead 预算，需同步 §8/预算例外（P1-2） |
| side | ≤28 | 契约 3（session lib/tests + ipc lib）+ store/kit 5（store lib/tests、kit lib/search test、tool-session-query）+ desktop 2（ipc.rs、template）+ ui-kit 2 + ui-inspector 2 + ui-sessions 1 + desktop-web 4 + app tests 2 + e2e 1 + docs 6 = **28** | 合规（Lead 已批预算，proposal §8 明示） |
| activity | ≤14 | 契约 1 + 模板 1 + desktop 1 + ui-kit 2 + ui-inspector 1 + ui-chat 1 + desktop-web 3 + e2e 2 + docs 2 = **14** | 合规；补充 docs/26 §5 会变 15，需同步 §8/预算例外（P1-2） |

- 三份均超过模板「>10 文件」阈值，但均在用户批准的 Phase 2a/2b/子 spec 预算内（composer §8、side §8、activity §8 均有记录）；无“未批即超”问题。
- 预算未计入生成物/本 spec 目录（符合模板）。

## 5. 可验收性与运行器

| spec | verify/运行器核对 | 结论 |
| --- | --- | --- |
| composer | `pnpm --dir packages/ui/ui-kit/web typecheck/test:unit`、`ui-chat typecheck/lint/test:unit`、`apps/desktop/web typecheck/test:unit`、`cargo test -p zk-contract-ipc -p zk-desktop`、`pnpm e2e`（m1-serve 段，F18 正确）、`schema-check`/`compose --check` 均为现有可执行命令；`file chooser` e2e 可经 serve 跑 | 可执行；补 docs/26 后再验收 |
| side | `cargo test -p zk-contract-session -p zk-contract-ipc -p zk-store-session-sqlite -p zk-kit-session -p zk-tool-session-query -p zk-desktop`、`pnpm e2e`（m1-serve）、`pnpm --dir ...` 均存在；v5→v6 迁移单测路径明确 | 可执行；`tool-session-query` 行为不变与 snippet 的冲突见 P2-6 |
| activity | `bash scripts/agent-e2e.sh` + `ZEOKEEL_AGENT_GREP`（F18 支持）；`cargo test -p zk-contract-ipc -p zk-desktop`；`pnpm e2e` 不用于真实 rig；e2e 文件 `agent-rig.spec.ts` 与任务一致 | 可执行；snapshot/action schema 与 e2e 剧本需覆盖 P1-3/P2-10 的边界 |

- e2e 证据路径已在三份 acceptance 给出（`target/evidence/<id>/`），成功指标可测（命令集一致、附件 IPC 断言、树展开/刷新、搜索命中、present 卡、axe 0 serious）。
- **交互契约（AGENTS §8/docs/11 §8.6）**：composer/side 评审期间已补 `interaction.md`（planned）并在 tasks/acceptance 纳入 `pnpm e2e:ui`/`ui-interaction-gate.py`；但 activity 仍缺（P1-4），且这些 verify 命令在 HEAD 工作树中**尚不存在**（P1-5：无 `scripts/ui-interaction-gate.py`、无 `e2e:ui` 脚本、无 `e2e/interaction/`），当前不可执行。修复后需按 docs/11 §8.6 补 `active` 契约与 P0/P1 标签测试/证据。
- 未发现将 `pnpm e2e` 误用于真实 agent-rig 的情况（activity 正确用 agent-e2e）；m1-serve 的 suite 名含 "serve carrier" 会被 mock 段 grep-invert 跳过（F18），新增用例放在该 suite 内不会污染 mock 段。

## 6. 安全/隐私专项

| 面 | 核对 | 结论/问题 |
| --- | --- | --- |
| 附件路径/大小/类型 | spec R3：1 MiB、≤5、allowlist、basename sanitize、固定 `.zeokeel/attachments/<conv>`、会话归属校验；host 解码后大小/magic/空文件（composer spec:30-35、design:96-104/167-168） | 大小/类型/名称校验方向正确；**父目录符号链接可越界（P0-1）**；未复用 `zk-kit-tools` 路径安全（C17/P0-1） |
| 附件内容泄露 | 不写日志、不进 i18n、不跨 workspace；工具可读（本地） | 通过（设计有声明） |
| 终端边界 | side design:192-195：用户显式、原样交给持久 shell、token/CORS、无审批/审计（KNOWN）；命令输入 ≤16 KiB/输出 ≤64 KiB/超时 ≤10s；≤8 会话 | 边界清晰；无新增 PTY/shell 实现；弱管控已文档化（P2 无需新增） |
| C18 fork/archive | side spec:47-49、design:186-190：fork 同 workspace、archive 隐藏不删、复制消息不复制 pin/archive；store v6 仅加列 | 通过；`list_conversations_in` 返回全部（含 archived）与现有调用方需同步过滤（已列入 side tasks 10/11） |
| 检索脱敏/隐私 | side spec R6/design:164/189：返回 snippet（±60）与内容；**未写脱敏策略**；工具行 args 已被 Phase 1 脱敏，但 user/assistant 文本可能含密钥；搜索结果与会话正文同源 | **P2-7**：明确“检索不额外泄露（与打开会话同权限）”或对已知密钥形态做掩码；至少不要写日志 |
| activity 数据边界 | jobs 按 cwd 过滤（design:113/171）、subagent 按 parent、goal/todo/present 当前会话、schedule 当前 workspace；错误不泄露工作区外路径 | 方向正确；cwd 前缀需 component-aware `Path::starts_with`（P2-14） |
| activity 动作 | job kill/subagent interrupt/schedule 写均为显式用户动作，非 agent 工具；无新增审批/网络 | 通过 |

## 7. 问题清单（P0/P1/P2，文件:行 + 问题 + 修法 + 状态）

> 全部 OPEN。P0=1、P1=5、P2=15。修复后需同一评审员复评；P0/P1 修复前不得置 `approved`（C16/C19）。

### P0（阻断）

**P0-1 附件父目录符号链接可逃逸 workspace（安全/C17/C18）**
- 位置：`composer/design.md:103-104`（固定路径 `.zeokeel/attachments/<conversation_id>/<uuid>-<sanitized>`）、`:167-168`（「拒绝符号链接目标（写入新文件不受影响）」）、`:39`/`proposal §9:126`（不复用 `zk-kit-tools::safe_path`）、`spec.md:33-34`、`tasks.md:18`。
- 问题：路径拼接只保证文件名（uuid+sanitized basename）不逃逸，但 **`.zeokeel`/`attachments` 本身可以是攻击者仓库里的符号链接**；`create_dir_all` + 写入会跟随父目录符号链接，把附件写到 workspace 外（任意用户可写路径），违反 C18 与「workspace 外写入」威胁模型；design 只写「拒绝符号链接目标」不足以覆盖父目录。另：自行实现路径安全且明确不复用 `zk-kit-tools`，违反 C17（side 反而已在同一 desktop 依赖中加入 `zk-kit-tools`，说明可复用）。
- 修法：把附件路径安全实现为 **单一 kit helper**（扩展 `zk-kit-tools` 的 `safe_path` 或新增 `attachment_path`，C17）；创建目录后对父目录/各组件做 `symlink_metadata` 检查，canonicalize 后以 `Path::starts_with` 校验 workspace root；写入使用 no-follow/临时文件+rename；补 host 单测：workspace 内 `.zeokeel` 为指向外部的 symlink、attachments 为 symlink、文件名为 symlink、`..`/绝对路径均拒绝且不写盘；同步 `docs/26` 与 `KNOWN-LIMITATIONS`。
- 状态：OPEN。

### P1（需本轮修复后复评）

**P1-1 缩略图 `URL.createObjectURL`（blob URL）被 Tauri CSP 拦截**
- 位置：`composer/design.md:107`；`apps/desktop/tauri.conf.json:20`（`img-src 'self' data:`，无 `blob:`）；`composer/spec.md:30-31`、`acceptance.md:29`。
- 问题：附件 chip 的图片缩略图用 `URL.createObjectURL` 生成 `blob:` URL；Tauri WebView CSP 只允许 `self data:`，缩略图在桌面端会加载失败，R3/验收「图片缩略图」不可达（serve/e2e 浏览器无此 CSP 时可能误绿）。
- 修法：改用 `FileReader` 生成 `data:` URL（CSP 已允许），或显式把 `blob:` 加入 `img-src`（需安全评估/两处 CSP 同步）；补 Tauri 手测/截图证据；更新 acceptance。
- 状态：OPEN。

**P1-2 新 IPC 命令/错误码未登记到 `docs/26` §5（C12/C16 SSOT）**
- 位置：`composer/tasks.md:10/31`（docs 仅 19/28/KNOWN）、`composer/spec.md:95`；`activity/tasks.md:11/33`（docs 仅 16/KNOWN）、`activity/spec.md:102`；对照 `docs/26-workspace-and-conversations.md:156-181`（M1 IPC 表，Phase 1 已登记 `mcp.test`）与 side `tasks.md:11`（0a 明确改 docs/26 §5）。
- 问题：`workspace.attachment_save`、`activity.snapshot`、`activity.action` 及 `attachment/*`、`activity/*` 错误码没有进入 IPC 命令/错误 SSOT；违背「先文档后代码」与 C16/C12，验收时 docs-check 绿但语义缺口不会被发现。
- 修法：composer task 0 增加 `docs/26` §5 两行（命令/错误码）；activity task 0 增加 `docs/26` §5 两行与错误码（activity 设计:122 现在写「docs/16/KNOWN 记录」应改为 docs/26 §5）；同步更新 proposals §8 预算（composer 16/activity 15）并取得 Lead 的预算例外记录；验收增加 `rg` 对照 docs/26 与 contract 命令常量。
- 状态：OPEN。

**P1-3 activity `schedule_delete` 未知 id 的契约冲突**
- 位置：`activity/spec.md:57/86`（未知 id 拒绝，`activity/schedule-invalid`）、`:106`（「不存在返回错误或成功？design 定：不存在=可读错误」）；`activity/design.md:120`（action 复用 tool、id/updates host 校验）；`packages/client/tool-schedule/src/lib.rs:167-172`（delete 未知 id 返回 `"deleted 0"` 成功，不报错）。
- 问题：spec 冻结了「不存在=可读错误」，但复用 `Tool::call_json` 的 delete 会成功返回；design 没有定义 host 如何检测（预 list 或解析 `deleted 0`），按当前设计实现会与 spec/验收边界表冲突。
- 修法：host 在 `activity.action` 前 `list` 校验 id 存在（或解析 delete 输出 `deleted 0` 并映射 `activity/schedule-invalid`），补 host 单测；design §2.3/§2.4 写明唯一映射。
- 状态：OPEN。

**P1-4 activity-panels 缺 `interaction.md`（AGENTS §8 / docs/11 §8.6）**
- 位置：`specs/changes/2026-11-01-client-activity-panels/` 只有五件套；`docs/11-testing-and-evidence.md:169-175`（UI 变更必须有 `interaction.md`；`:175` 只列 composer/side）；activity `tasks.md`/`acceptance.md` 无 `pnpm e2e:ui`/`@interaction`/`interaction/` 证据。
- 问题：activity 是纯客户端 UI（activity tab、job 停止、subagent 中断、schedule 编辑/删除、turnTail present/plan），却未提供交互契约与 P0/P1 的 ID→测试→证据映射；在 composer/side 已补 planned 契约的情况下，activity 仍缺失会阻塞其三份 spec 的同一批评审/实现验收。
- 修法：新增 `activity-panels/interaction.md`（planned；开工改 active），覆盖 ACT2-SNAPSHOT/POLL、ACT2-JOB-STOP、ACT2-SUBAGENT-INTERRUPT、ACT2-SCHEDULE-EDIT/DELETE、ACT2-TURNTAIL-PRESENT/PLAN、ACT2-STALE/ERROR；P0/P1 映射 `e2e/interaction/activity-*.spec.ts` 的 `@interaction:<ID>` 与 `target/evidence/2026-11-01-client-activity-panels/interaction/<ID>.json`；tasks/acceptance 增加 `pnpm e2e:ui --grep activity` 与 `ui-interaction-gate.py`；docs/11:175 补 activity 基线。
- 状态：OPEN。

**P1-5 tasks 中的交互门禁 verify 当前不可执行（可验收性）**
- 位置：`composer/tasks.md:51-52`、`composer/acceptance.md:24-25`、`side/tasks.md:59-60`、`side/acceptance.md:29-30` 新列 `pnpm e2e:ui --grep ...` 与 `python3 scripts/ui-interaction-gate.py`；实际根 `package.json` 无 `e2e:ui`、`scripts/ui-interaction-gate.py` 不存在（exit 2）、`e2e/interaction/` 不存在（F20）。
- 问题：这些 verify 命令按当前仓库状态无法运行，task 9/13 的收尾门禁会直接失败；与 docs/11 §8.6 声称的命令不一致。
- 修法：随 `ui-interaction-verification` step 落地 `scripts/ui-interaction-gate.py`、`pnpm e2e:ui` 脚本与 `e2e/interaction/` 目录；在此之前把这两行标为「待工具落地」或从 verify 暂移；验收记录实际 exit code（不得写「已绿」）。
- 状态：OPEN。

### P2（非阻断；建议本轮/实现阶段顺手修）

| # | 位置 | 问题 | 修法 | 状态 |
| --- | --- | --- | --- | --- |
| P2-1 | `composer/proposal.md:87`、`spec.md:120`；`apps/desktop/web/src/App.tsx:591-607` | 称现有 personal 6/enterprise 9 命令，实际 4/7（cancel/retry 由 task 5 新增） | 改为「现有 4/7，本 spec 新增 cancel/retry 后 6/9」；验收以最终 catalog 断言 | OPEN |
| P2-2 | `composer/spec.md:42` vs `design.md:117` | R4 说点击打开「设置 → 模型」入口，design 明确 `openModelSettings()=openSettings()` 不做 pane 深链 | 二选一：spec 改为「打开设置」或实现模型 pane 深链（需 settings 路由/预算）；验收文案同步 | OPEN |
| P2-3 | `composer/design.md:65-70`、`spec.md:65-69` | 引用块按行 `@path` 编码，未定义路径含换行/`--- referenced files ---`/以 `@` 开头等转义；可能破坏 strip/注入伪条目 | 规范路径编码（拒绝控制字符或 percent-encode）、仅尾部块、补解析边界单测 | OPEN |
| P2-4 | `composer/tasks.md:23/31`、`side/tasks.md:29/38` | 两份都追加 `e2e/m1-serve.spec.ts`，但各自 §5「独占共享文件」清单未列该文件；docs/19、docs/28、KNOWN 的 owner 也不完整 | 在两份 tasks §5 登记 e2e 文件与 docs owner；保持 composer→side 串行（至少一方明确「后合并者改」） | OPEN |
| P2-5 | `side/spec.md:12` | R1 写「符号链接不跟随/不展示（或标记为文件但禁止预览，design 定一种）」，design §2.3 未给出选择 | design 明确一种（建议不展示 symlink + 预览拒绝），补 UI 单测/e2e | OPEN |
| P2-6 | `side/tasks.md:22`、`side/spec.md:55-59`、`side/design.md:189`、`tool-session-query/src/lib.rs:28-60` | task 5 称 tool-session-query「行为不变」，但 kit 返回 snippet±60，而现工具返回完整 content；二者不一致 | 明确兼容策略：工具输出保持完整 content（kit 另给 snippet）或接受截断并更新 docs/acceptance；补一致性测试 | OPEN |
| P2-7 | `side/design.md:164/189`、`side/tasks.md:5` | 检索结果含用户/助手原文 snippet，未写脱敏/隐私边界（任务要求「检索脱敏」） | 明确与打开会话同权限、不额外记录日志；或对已知密钥形态掩码；acceptance 增加负向检查 | OPEN |
| P2-8 | `activity/spec.md:122-125`、`activity/proposal.md:95-107` | spec §6 仍列 D1–D4 为「待定/待拍板」，proposal 已按用户批准定稿 | spec §6 改为已定（勾选）并引用 proposal §7；验收引用同一决策 | OPEN |
| P2-9 | `activity/design.md:144` | 计划标题正则 `^#{1,3}\s*(plan|计划)\b` 对中文「计划」后接空格/标点时 `\b` 不成立（JS 中文字符非 `\w`），中文计划标题可能不匹配 | 改为 `(?=\s|$)` 或显式 lookahead；补「`## 计划`」单测 | OPEN |
| P2-10 | `activity/design.md:130`、`events.rs:88-102`、`agent/src/lib.rs:111-119` | 工具行 `args` 是 JSON 字符串，design 的 `PersistedToolRow.args: Option<Value>` / `args.text` 未说明二次解析；直接使用会拿不到字段 | 明确 `args` 先 `serde_json::from_str` 再取字段；损坏/legacy 行测试；`updates` 仍须 host 校验 | OPEN |
| P2-11 | `tools/zk/templates/app_desktop.Cargo.toml:1`、`side/tasks.md:23`、`activity/tasks.md:19` | 模板文件被 git 跟踪但标 `@generated`，side/activity 却计划手改它；C11 归属不清 | 明确模板是**源**（去掉/改用 source 头）或改为生成器输入参数；`zk compose --check` 覆盖；评审记录 | OPEN |
| P2-12 | `activity/design.md:13/217`、`activity/tasks.md:19` | 为 activity 在 desktop 直接加 `zk-tool-schedule`（业务 tool 包）依赖，C4/C2 边界仅靠「先例」；若某 profile 排除该包，组合语义不清 | Lead 明确接受并在 docs/24/KNOWN 记录，或把 schedule 读面移到 kit/contract；确保 personal/enterprise 均包含该包 | OPEN |
| P2-13 | `activity/spec.md:57/86`、`proposal §1`（prompt≤32KiB 称复用工具） | `tool-schedule` 实际无 prompt 长度校验；spec 说「复用工具校验」不准确 | 由 host 在 schedule_update 前校验 prompt≤32 KiB（design 已说 host 校验长度），补 host 单测；修正 proposal 措辞 | OPEN |
| P2-14 | `activity/design.md:113/171` | jobs cwd「前缀」过滤若用字符串 starts_with 会放过 `/workspace-evil`；需 canonical + component-aware | 用 `Path::starts_with`/canonicalize 后比较；补 host 负向测试 | OPEN |
| P2-15 | `docs/11-testing-and-evidence.md:175` | docs/11 基线只列 composer/side，未列 activity（而 activity 是同一批 UI spec）；另 docs/11 的交互门禁命令与当前工作树不一致 | 补 activity 基线；工具落地后同步 docs/11 与 package scripts；不得把 planned 契约当 active 门禁 | OPEN |

## 8. 结论与后续

- **结论：CHANGES-REQUESTED**（P0=1 / P1=5 / P2=15，全部 OPEN）。
- **P0-1** 必须本轮修复（附件路径安全）；**P1-1…P1-5** 修复前不得置 `approved`（C16/C19），不得开始产品代码实现；P2 可在实现阶段修复，但 P2-1/P2-4/P2-8/P2-11/P2-15 建议本轮一并处理以免复评反复。activity 的 `interaction.md`（P1-4）与交互门禁工具（P1-5）是同一批 UI 前置，建议与 composer/side 的 planned 契约一起落地。
- 修复后请 Lead 安排 **同一评审员** 复评（本记录 hash 将成为复评对象；spec 文件变动即失效）。
- 本轮未验证项（不冒充）：真实 Tauri WebView 缩略图（P1-1 需手测）、真实 `pnpm e2e`/`agent-e2e`、SQLite v6 实迁移、public-api diff；`scripts/ui-interaction-gate.py` 当前不存在（P1-5），交互门禁无法执行；均为实现阶段证据。
- 本记录字段齐全：评审对象（HEAD+17 文件 hash）、评审者、独立性、结论、P0/P1/P2、复评日期/结论、关联 spec（三个 change id 均出现）。
- 复评日期/结论：**2026-11-01 / CHANGES-REQUESTED**（首轮评审完成；P0=1、P1=5、P2=15）。

## 15. 复评（第二轮，2026-11-01）

- 复评对象：HEAD `551468924a487534d537a4a90b818265e5113ed8` + 三份 Phase 2 spec 共 **18 文件 SHA256**（15 五件套 + 3 个 `interaction.md`；首评 17 文件后 activity 新增 `interaction.md`，其余文件均被作者修复更新）。
- 评审者/独立性：`spec-reviewer`（同一评审员；未参与三份 spec 写作/实现；唯一写入仍为本记录）。
- 复评日期/结论：2026-11-01 / **APPROVED**（P0=0 / P1=0；新增 P2=4，均非阻断；见下）。
- 环境基线说明：工作树存在并发会话未提交改动（AGENTS/docs/11/19/27/39/README、`scripts/new-change.sh`、`playwright.interaction.config.ts`、`e2e/interaction/`、`scripts/ui-interaction-gate.py`、`specs/templates/interaction.md`、`2026-11-01-ui-interaction-verification`、agent-rig 等），按 Lead 指示视为环境基线，不计入三份 spec 缺陷；本复评只核对三份 spec 的依赖写法。

### 15.1 本轮对象 SHA256（18 文件）

```text
0d9f3e83e7ed70500f56124d01522331e238c0ab04a8cd057279ebafafac4c22  specs/changes/2026-11-01-client-activity-panels/interaction.md
1a2cd2d1006edfa21b419613d6f419ebea7de4efd25b4024c6df9c8662d7a73c  specs/changes/2026-11-01-client-composer-surface/proposal.md
2f25166e431a12362f44b479a6f392ba1e6d4b5d82ce7cd1d02262ec85e82455  specs/changes/2026-11-01-client-activity-panels/spec.md
3b2d17988b6d6666443168fe405b81512cd56c01d76328b7e46233d95896ef50  specs/changes/2026-11-01-client-composer-surface/acceptance.md
590d42be434179941f461431922a5cf852ed87b6670ea8aecc483f0da53c6fcd  specs/changes/2026-11-01-client-side-panels/interaction.md
6157e3d1e5c3b645e55efd319d9d16080ac051a5629a91dbde280dfd8fb3f24e  specs/changes/2026-11-01-client-composer-surface/spec.md
63be9be9ad47c4f6f38371da4ed153f4052577aafd6b5335343f9bdd3eaa0c3d  specs/changes/2026-11-01-client-side-panels/acceptance.md
645f2177d371459cd815e2071eb2797886da5ae1491523631f7b08af301cb425  specs/changes/2026-11-01-client-activity-panels/acceptance.md
76cef548b3d171b605003b9e6b9d835d9cadd28ff11f04067705aee30f57ec22  specs/changes/2026-11-01-client-side-panels/tasks.md
7a9e286d4ae2d9056573e2dcb6a3dfa0d75904cde3baf93a03c1f83e5173f07e  specs/changes/2026-11-01-client-composer-surface/design.md
83121c676458632510d54ec3b4bafc00f6df613b87e1bc2c64bdee413fdb1819  specs/changes/2026-11-01-client-side-panels/proposal.md
83877358dc01c63e5269cb1c77ba688ff987000901f8418a342f4bae0ab8012b  specs/changes/2026-11-01-client-side-panels/design.md
8ac4cc6cf1b2a6815e93c7d04474990b9a2fe2703572c8684911aa871c8b7c9a  specs/changes/2026-11-01-client-activity-panels/tasks.md
9aa024ff23d2a87faca5aecc9a086a3fe93582afd21630caa319d724b1e1bf34  specs/changes/2026-11-01-client-composer-surface/interaction.md
a06ff3481c5fcc87db803323b9d9e04a44d8fb59871db5a746149babbf69e470  specs/changes/2026-11-01-client-side-panels/spec.md
a14cf608532a4e1346ca777ce33c92ef46f3a6ab4b23cf40f39fc98b133536c0  specs/changes/2026-11-01-client-activity-panels/proposal.md
c52297d7c658e91639356322e8473d99631b7712d1280953fb0568e9585fd703  specs/changes/2026-11-01-client-composer-surface/tasks.md
e78d1e3b8e0e8c4fddd7096401f8f16a0225b9e08e6a6251e6d17f2a75d8a1a7  specs/changes/2026-11-01-client-activity-panels/design.md
```

### 15.2 首评 P0/P1/P2 逐条状态

| 条目 | 状态 | 复评依据 |
| --- | --- | --- |
| **P0-1** 附件父目录 symlink 逃逸 | **RESOLVED** | `safe_join_under` 定为 `zk-kit-tools` 单一 helper（逐组件 no-follow、canonicalize 父目录、`starts_with(root)`、拒绝绝对/`..`；design:97-101）；host 经生成器的 `zk-composed` facade 调用（design:111-115；tasks task 3/3b），app 不直接依赖 kit/tool（C4/C2）；kit/host 单测覆盖父目录/文件名 symlink（acceptance:14/93；design:203）。C17 单一实现已落实。 |
| **P1-1** 缩略图 blob vs CSP | **RESOLVED** | spec:31/design:125 改为 `FileReader.readAsDataURL` 的 `data:` URL，不使用 `blob:`、不改 CSP；acceptance:56/94 有对应断言。 |
| **P1-2** 新 IPC 未登记 docs/26 §5 | **RESOLVED** | composer task 0 增 `docs/26` §5（`workspace.attachment_save` + `attachment/*`）；activity task 0/acceptance:67 增 `activity.snapshot/action` + `activity/*`；composer 预算 §8 为 16、activity §8 为 15（Lead 例外已记录）。首评的预算+1 冲突已由 Lead 例外解决。 |
| **P1-3** schedule delete 未知 id 冲突 | **RESOLVED** | activity spec:57/88 冻结「host 先 list 预检 / `deleted 0` → `activity/schedule-invalid`」；design:128-130；tasks task 4；acceptance:93。 |
| **P1-4** activity 缺 interaction.md | **RESOLVED** | `activity-panels/interaction.md` 已新增（planned，ACT2-* P0/P1 ID）；acceptance:30-42 映射 `e2e/interaction/activity.spec.ts` + 证据；tasks task 9/10 已列。 |
| **P1-5** 交互门禁 verify 不可执行 | **RESOLVED（UNVERIFIED 依赖）** | 三份 tasks/acceptance 已明确「依赖 `2026-11-01-ui-interaction-verification`；当前 UNVERIFIED，实现前确认脚本/`e2e:ui`」（composer tasks:51-52、side:59-60、activity:20/26）；复评时并发工作树已落地 `scripts/ui-interaction-gate.py`/`playwright.interaction.config.ts`/`e2e:ui`，门禁实跑见 §15.4；spec 不再声称已绿，依赖写法正确。 |
| **P2-1** 命令数 6/9 | RESOLVED | composer spec:126 改为「现 personal 4/enterprise 7；本 spec 新增 cancel/retry 后 6/9」 |
| **P2-2** 模型点击语义 | RESOLVED（残留 scope 文案见 P2-R2） | spec:44/design:11/129 统一 `openSettings(paneId)` + `SettingsDialog.initialPane`；task 5 写 ui-settings |
| **P2-3** 引用块编码 | RESOLVED | design:105/122-123：拒绝控制字符/换行、`@@` 转义、仅尾部块；spec 验收:111 |
| **P2-4** owner 表 | RESOLVED | composer tasks §5:64、side tasks §5:69 增 `e2e/m1-serve.spec.ts`/`tools/zk/src/main.rs` 串行 owner；docs owner 说明 |
| **P2-5** side symlink 显示决策 | RESOLVED | side spec:12/design:144：不展示 symlink + 预览/父目录 symlink 拒绝；acceptance:60 |
| **P2-6** 检索 snippet vs 工具行为 | RESOLVED | side design:196/202、tasks:22：kit 返回 snippet，`session_search` 工具仍返回完整 content；acceptance:64 |
| **P2-7** 检索隐私/脱敏 | RESOLVED | side spec:61/design:202-203：同权限、不写日志/上报、不跨 workspace；snippet 显示；acceptance:65 |
| **P2-8** activity spec §6 决策矛盾 | RESOLVED | activity spec:122-127 改为 `[x]` 已定 D1–D4，与 proposal §7 一致 |
| **P2-9** 中文 plan 正则 | RESOLVED（新正则仍有 P2-R4） | activity design:154-155 改中文感知 lookahead，不使用 `\b`；interaction ACT2-TURNTAIL-PLAN-01 |
| **P2-10** 工具行 args 二次解析 | RESOLVED | activity design:110-112：`args` 为 JSON 字符串，`PersistedToolRow.args` 二次 `serde_json::from_str`；tasks:20 |
| **P2-11** @generated 模板 | RESOLVED | composer/side/activity design:103-115/159-162 明确 `tools/zk/templates/...` 为输入源、只改生成器源 `tools/zk/src/main.rs`、不手改模板/产物；`zk compose --check` 验收 |
| **P2-12** app→tool 直接依赖 | RESOLVED（残留风险表见 P2-R3） | activity design:104-106/tasks:19：生成 `zk-composed` facade，app 不直接依赖 jobs/subagent/tool-schedule |
| **P2-13** prompt 32 KiB 校验 | RESOLVED | activity spec:58/design:129：host 预校验（tool-schedule 无长度校验），tasks:20 |
| **P2-14** jobs cwd 前缀 | RESOLVED | activity spec:21/design:121：canonicalize + 组件级 `Path::starts_with` |
| **P2-15** docs/11 基线 | RESOLVED（环境依赖） | docs/11:175 已含 activity planned；门禁命令由并发 `ui-interaction-verification` 落地；三份 spec 标注 UNVERIFIED，不谎报 |

### 15.3 本轮新发现 P2（非阻断，建议置 approved 前顺手修）

1. **P2-R1 composer 预算口径不一致**：`composer/proposal.md:8` 仍写「15 个文件」，但 §8:109（及 task 0/acceptance:95）为 **16**（Lead 例外）；同文件自相矛盾，应改头部为 16 并在 §8 保留例外说明。
2. **P2-R2 composer 写作用域自相矛盾**：`composer/tasks.md:21`（task 5）把 `packages/ui/ui-settings/web/src/index.tsx` 列入写作用域，`:33` 又写「明确不写 `packages/ui/ui-settings/**`」；需保留 task 5 的 `SettingsDialog.initialPane` 写入并从「不写」中排除（或改由 ui-kit 提供初始 pane）；否则实现者会误判。
3. **P2-R3 activity 风险表残留旧依赖描述**：`activity/design.md:230` 仍写「`zk-tool-schedule` 直接依赖包的 C4 边界质疑 / app 为组合根…」，与同文件 104-106（改为生成 facade、app 不直接依赖）及 P2-12 冲突；删除/改写该行。
4. **P2-R4 activity 计划正则可匹配非标题行**：`activity/design.md:154-155` 为 `^(#{1,6}\s*)?(计划|Plan)(?=\s|$|[：:])`，标题前缀可选 ⇒ 普通段落开头「计划…」也会被当成计划卡，违背 spec R7「Markdown 标题行」与 interaction 的「仅标题段落」；要求 `#{1,6}` 必选（或明确放宽 spec/验收并加负向测试）。

### 15.4 复评门禁（2026-11-01，独立执行）

| 命令 | 结果 |
| --- | --- |
| `python3 scripts/spec-check.py` | exit 0；0 errors / 4 warnings（三份 + `ui-interaction-verification` 均 draft，预期） |
| `python3 scripts/docs-check.py` | exit 0；0 errors / 0 warnings / 41 files |
| `python3 scripts/review-check.py` | exit 0；ok（30 records；本记录在置 approved 后满足三份 change 的字段/change id/结论要求） |
| `python3 scripts/ui-interaction-gate.py --self-test` | exit 0；9 checks OK（并发环境已落地） |
| `python3 scripts/ui-interaction-gate.py` | exit 0；mode=proposal-status，4 契约 / 19 tags / 54 ids；planned 契约（composer/side/activity）按设计 WARN skip；active P0/P1 全绿（环境基线，不计入三份 spec 缺陷） |
| `git status --porcelain` 抽查 | 三份 spec 目录为未跟踪；本记录外无本评审写入；并发改动不纳入对象 |

### 15.5 新矛盾复核

- **safe-path 与 C17**：单一 `zk_kit_tools::safe_join_under` + 生成 facade，C17 通过；无第二套路径安全实现。
- **activity 依赖 side-panels**：activity 明确复用 side 的 `previewRequest`/file_read/预览；side design:135-141 提供 `previewRequest` prop；顺序 side→activity；无新冲突。
- **interaction 命令依赖**：三份 spec 均写「依赖 `2026-11-01-ui-interaction-verification`，当前 UNVERIFIED」，并发基线中的 gate 脚本/e2e:ui/config 已存在且自测通过；实现前仍需把各 `interaction.md` 从 planned 改 active 并补 P0/P1 测试/证据，spec 未声称已绿。

### 15.6 结论

- 首评 **P0=1 / P1=5 / P2=15** 全部 **RESOLVED**（P1-5 按「依赖并发 spec + UNVERIFIED」关闭；P2 全部按作者修复关闭）。
- 本轮新发现 **P2=4**（P2-R1…R4，均非阻断，建议置 approved 前顺手修；其中 R1/R2 是同一 spec 内的文档/写作用域矛盾，修起来很小）。
- **复评结论：APPROVED**（P0=0 / P1=0；新增 P2=4，非阻断）。三份 spec 可置 `approved`：需把 proposal 状态/评审链接回填（composer 预算头改为 16）、acceptance 引用本复评；`review-check` 在 status=approved 后将校验本记录（含三个 change id 与最新 APPROVED 结论）。
- 失效条件：本记录 §15.1 的 18 文件任一 hash 变化（除批准后的行政性回填/本记录自身）→ 复评失效；P2-R1…R4 修复亦会改 hash，建议随批准前一次改完并重算。
- 未验证（不冒充）：真实 Tauri WebView 缩略图（P1-1 修法需 Tauri 手测）、真实 `pnpm e2e`/`cargo test`/SQLite v6、public-api diff、交互契约 P0/P1 实测证据；均为实现阶段。
