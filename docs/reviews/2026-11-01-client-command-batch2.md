# 2026-11-01-client-command-batch2 · C19/C20 独立评审

- 状态：本轮评审完成，**CHANGES-REQUESTED**（P0=1 / P1=3 / P2=3，均 OPEN；见 §7）。
- 评审对象：HEAD `516f68553dd45616cd06eebfefe0496751d67a81` + 三份 Batch2 spec 共 24 文件 SHA256（§0.1；实际目录 `client-compact-command`、`client-plan-mode`、`client-skill-commands`）。
- 关联 spec（三个 change id）：`2026-11-01-client-compact-command`、`2026-11-01-client-plan-mode`、`2026-11-01-client-skill-commands`。
- 评审者：`spec-reviewer`（C19/C20 fresh-context 非实现者）。
- 独立性：未参与三份 spec 的拆分/写作/实现；只读代码、docs/11/16/19/27/38、DSH 审计与 24 文件；唯一写入本记录。
- 评审日期：2026-11-01。
- 结论：**CHANGES-REQUESTED（P0=1 / P1=3 / P2=3）**；不可置 approved，不进入实现。
- 复评安排：P0/P1 修复后由同一评审员复评；三份 `interaction.md`/`intuition.md` 仍 `planned`，实现开始前改 `active`；用户测试均 UNVERIFIED。

## 0. 对象与方法

### 0.1 对象 SHA256（24 文件，三目录各 8）

```text
030fec83627b825ab9eaf995cb5fa6bbf4b579ab580a292457fb49305d33d1b5  specs/changes/2026-11-01-client-plan-mode/tasks.md
12eb008a03f0321c6558c4ba0e401130eaf91a2a2175e2578b0c50d253edb3e5  specs/changes/2026-11-01-client-compact-command/spec.md
1424ea6f3d31592f75ef6bf4ce3f9eab9d9b5dac0c97ddaa4a4726bc6be149be  specs/changes/2026-11-01-client-skill-commands/design.md
1fef8d8b9414b1c0dfc637818edc7c9683645953dc760d6be35ed4e6be8534d5  specs/changes/2026-11-01-client-skill-commands/interaction.md
29ac4487338fc027b76dd46c7b059375a64d49fe523178134910278f7361166c  specs/changes/2026-11-01-client-compact-command/interaction.md
327f77666034cefb8f942af3dcb27e9fdc49e87f42139de2b53d9b911833fa08  specs/changes/2026-11-01-client-skill-commands/intuition.md
387ee402e86ec292f02328988b3275eb1ca0da1ff3faa03be9bbbb5bf0742981  specs/changes/2026-11-01-client-plan-mode/intuition-walkthrough.md
45db4c1d0c10594f87a31e6f0d1c57052b5673fbac35e60c2c95c3c78fbcb275  specs/changes/2026-11-01-client-skill-commands/acceptance.md
60cdeaa9ce78f4bab77e193dc71912e7e6cae333eda66663f9a8bd2b2d7aa694  specs/changes/2026-11-01-client-skill-commands/proposal.md
60e776e24d1c58bd7da74b8a57127e60e255ba1bb35018d017b95dc5d60a383c  specs/changes/2026-11-01-client-compact-command/proposal.md
766a9df755ac20397621709d6bdbf53ea849ed71a7a7e65779f1143960a5db93  specs/changes/2026-11-01-client-plan-mode/design.md
8015bd86680307a94e13e2bef54d20afe5f401ad05ef5c3131bc0280c9982023  specs/changes/2026-11-01-client-compact-command/design.md
9b8761e606c5ebf0a510b6355af7784bf0f7a96f312697bfb29c7b32d2d51609  specs/changes/2026-11-01-client-compact-command/tasks.md
abdf0ea653b48840fa94bdf416c0ac935828a8e328bedaba914402872b8aae7e  specs/changes/2026-11-01-client-plan-mode/acceptance.md
afc17479b12cf699e85b42cfa54d8f14196e8e72bf38fb2ea0271c61ac26bffc  specs/changes/2026-11-01-client-skill-commands/intuition-walkthrough.md
c24fdd89fd52d0b6420be0920267c25ce32777ea80c5571160abb312ca35e863  specs/changes/2026-11-01-client-plan-mode/proposal.md
c34aeb3377bec6aca6622b14843d9c99ad612f522409cde5e84093dddd66076d  specs/changes/2026-11-01-client-compact-command/intuition-walkthrough.md
dbab602bdaa31d1820685f6360a9a96c1a665dfe76b0d3e4fa1a712c3cd84003  specs/changes/2026-11-01-client-plan-mode/spec.md
de772361fbcdd548c0fdba51c84e3599b45103ca74626db68fc117cea583a72b  specs/changes/2026-11-01-client-compact-command/acceptance.md
e16a889cf98d7169b10bcc9f42296d0ca3898b50898837b7df87855f8ce49ebc  specs/changes/2026-11-01-client-plan-mode/intuition.md
eb7cce42737e1a3e788a0e8cb69e0f198ced8aa4aed921dc6de4db5c61593ec6  specs/changes/2026-11-01-client-skill-commands/tasks.md
f06a9edb1e6577f630c8cf4495951abc714f8cbf86981fd69095b04bb25303e3  specs/changes/2026-11-01-client-compact-command/intuition.md
fbe807321a1f09d72ebe3bc45f8a7746276272ca999a633a95ca76ca26686985  specs/changes/2026-11-01-client-skill-commands/spec.md
fdfac24e1ac6b09c568acadbbbe8e3d467cfa06f573cc1e04fadf38a90e332fd  specs/changes/2026-11-01-client-plan-mode/interaction.md
```

（实际目录名：`2026-11-01-client-compact-command`、`2026-11-01-client-plan-mode`、`2026-11-01-client-skill-commands`；原 30 文件 `client-command-backends` 已删除，`git status` 无该目录。）

### 0.2 依据

`AGENTS.md` C1–C20；`docs/27` §2.5–§2.8、`docs/19` §2.1b/§11、`docs/11` §8.6、`docs/26`、`docs/38` B8；`target/audits/dsh-reference-inventory.md` §5；代码：`packages/contracts/{agent,session,ipc}`、`packages/client/agent-rig`、`crates/zk-kit-tools`、`packages/client/tool-*`、`packages/client/connector-mcp`、`apps/desktop/src/ipc.rs`、`apps/desktop/web/src`、`packages/ui/ui-chat`、`ui-inspector`。

## 1. 事实核验（HEAD `516f685`，path:line）

| # | 事实 | 证据 | 与 spec 的关系 |
| --- | --- | --- | --- |
| F1 | history 回放权威 | `packages/contracts/agent/src/lib.rs:63/304` 明确 `SessionStore::messages_in` 是 replay 权威；agent-rig `run_stream_in_shared_with_history`；`HistoryRole`/`HistoryMessage` 在 `contracts/agent:54-70` | compact 的「marker 视图 + host replay 注入摘要」方向可行；但改 `messages_in` 会影响所有调用方（F2） |
| F2 | `messages_in` 调用方 | `apps/desktop/src/ipc.rs:568`（session.messages UI）、`:2757`（turn replay）、`:5040`（fork 校验）、`packages/client/tool-session-query/src/lib.rs:48/65`（搜索/read）、`apps/desktop/src/command_surface.rs:84`；store 实现在 `store-session-sqlite:704` | compact design 把 `messages_in` 改成压缩视图，**会同时影响 UI/搜索/fork/read**；没有 raw/compacted 双视图（P1-1） |
| F3 | 现有 agent-rig 工具裁剪 | `agent-rig/src/lib.rs:300-342` `dynamic_tools` 按 `ToolSpec` 构造；计划用 `mutating` 过滤 | plan mode 依赖 `ToolSpec.mutating`；但 delegation tools 的 spec 是 false（F4） |
| F4 | delegation 工具 spec | `tool-subagent/src/lib.rs:37/72`（subagent/subagent_start）`mutating:false`；`tool-agent-team:264` false；`tool-workflow:123` false；`tool-ralph:16` false；`tool-subagent-control` 全 false | plan mode 的「只保留只读 + exit」会把 subagent/agent_team/workflow/ralph **当成只读而保留**；其子 agent 可执行 mutating（P0-1） |
| F5 | mutating 的真工具 | `tool-schedule:79` true；`connector-mcp:914` true；`tool-bash:18`/`fs-write:17`/`fs-edit:18`/`job-kill:15` 等 true | MCP/schedule/命令类会被 plan filter 挡住（通过）；真正缺口是 delegation（P0-1） |
| F6 | subagent 子 agent | `crates/zk-kit-subagent:14` runner "same model/tools"；agent-rig `RigSubagentRunner::new(model.clone(), tools.clone())`（`:1597`） | 子 agent 继承 tools，未继承 plan filter；plan mode 的 mutating 限制不传递（P0-1） |
| F7 | 现有 tool-skill | `tool-skill/src/lib.rs:34-59`：只扫 workspace `.agents/skills`；list 返回换行 name、read 无 frontmatter 解析；`SkillTool` spec 只读 | skill spec 说要单实现扩展 frontmatter/user-invocable，方向可；但 list 输出从 name 变为 JSON 是 agent 工具行为变化（P2-3） |
| F8 | enterprise 已安装技能路径 | `apps/desktop/src/ipc.rs:2388-2420` `installed_skills` 扫 `skill_root()`；`skill_root()` 在 `:1900-1912` 为 `$ZEOKEEL_HOME/skills/<name>/<version>`（安装器写这里） | **skill spec 说 enterprise 已安装技能与 personal 共用 `.agents/skills`，与代码不符**；按现有 tool-skill 无法列出/读取企业已安装技能（P1-2） |
| F9 | 命令占位现状 | `apps/desktop/web/src/App.tsx:118-124` `slash-plan/slash-compact/slash-skill` 当前 `disabled`；Batch1 catalog | 三 spec 把占位改真实的方向一致；plan/compact/skill 菜单分组已存在 |
| F10 | 右栏 tab order | `docs/16-ui-packages.md:27` 当前 files 10/changes 20/terminal 30/activity 40 预留；plan spec 要 order 50 | docs/16 尚未登记 plan 50；plan tasks #0 只列 docs/19/26/KNOWN，未含 docs/16（P2-2） |
| F11 | 新 tool-plan 包 | `ls packages/client/tool-plan` 不存在；spec 计划新建四件套 | C3/C2 方向可；需 role/edition/capability 登记，tasks 已写 |
| F12 | agent-rig policy 单点 | `agent-rig/src/lib.rs:280` `set_approval_policy` 存在；无 `set_plan_mode` | plan spec 新增 `set_plan_mode` 与 Batch1 的 facade 模式一致；需生成器源，不能手改生成物 |
| F13 | 三份预算/实际文件 | 每目录 8 文件（五件套+interaction+intuition+walkthrough），实现清单 compact 12/plan 16/skill 9；模板 `specs/templates/proposal.md:8` >10 或 400 行需拆 | 文件数/行数：compact 348、plan 320、skill 291；compact/plan 实现文件数 >10 且 proposal 未写 Lead 书面例外（P2-1） |
| F14 | C20 | compact 1 P0、plan 2 P0、skill 1 P0；三份 intuition 分别 6/12/6 INT；脚本核对 P0 映射 1/1、2/2、1/1；三份 walkthrough 均 3×3；用户测试 UNVERIFIED | C20 机械要求通过；plan 的 P0 mutating 过滤缺口不来自 C20 文件，而是实现语义（P0-1） |
| F15 | 串行/共享 owner | 三份 design §0 写 compact→plan→skill；compact 首写 contracts/ipc/ui-chat/App/controller/i18n/e2e，后两份 rebase | 顺序清楚；但 compact 预算未计 docs，plan/skill 追加 docs；共享文件 owner 需在 tasks/PR 严格执行（P2-1/P2-2） |
| F16 | 错误码/契约 | 三 spec 各自新增 `compact/*`、`plan/*`、`skill/*`；`docs/26` 任务；当前 contract 无这些命令（grep 无） | additive 方向正确；需生成物/schema 与 docs/26 同批 |

## 2. 三份 spec 专项核对

### 2.1 compact（`2026-11-01-client-compact-command`）

- **设计可行性**：host `session.compact` + `zk-kit-compaction` summarizer；marker `role=system` 复用 `append_message`；`messages_in` 压缩视图；host replay 注入摘要；无 migration；失败不写；方向可。
- **P1-1（见 §6）**：把 `messages_in` 全局改为压缩视图会同时影响 UI `session.messages`、`session_search`/read、fork、command_surface 等；**原始消息只留在 DB，没有公开 raw 视图**，与 proposal「原始会话仍可审计」不符；搜索旧消息和 fork 原始历史会丢。应保留 `messages_in` 原始语义，新增 `messages_in_compacted`/`messages_view` 仅供 replay/UI 压缩视图，或加 view 参数；补 UI/search/fork 回归。
- **其他**：marker 以 `role=system` 存储，`CompactDivider` 渲染；需 UI parser 不把 marker 当普通消息；P2 提示。预算 12（P2-1 超模板阈值，无 Lead 例外）。
- **C20**：1 P0 映射 1/1；3×3；P0 摩擦 W1/W3 closed；用户 UNVERIFIED；通过机械要求。

### 2.2 plan-mode（`2026-11-01-client-plan-mode`）

- **设计可行性**：`tool-plan` 四件套 + agent-rig `set_plan_mode` + dynamic_tools 过滤 + plan tab order50 + plans.json；方向可。
- **P0-1（见 §6）**：plan mode 的只读保证只按 `ToolSpec.mutating` 过滤；`subagent`/`agent_team`/`workflow`/`ralph` 都是 `mutating:false`，会被保留；其子 agent 用 `tools.clone()`（F6）可执行 mutating 工具，plan mode 的「mutating 0 次」被绕过。MCP/schedule 是 true，能挡住；delegation 是缺口。必须在 plan filter 中把 delegation 工具按 mutating 处理/阻断，或把 plan mode 传播到 child agent，并为每个 delegation 工具补「mutating 0」测试。
- **`exit_plan_mode` 审批边界**：工具 `mutating:false/requires_approval:false`，本地 plan review（D3）；**不绕过 approval gate**（不写 gate owner），但实现必须证明它只写 plans.json、不执行计划；P2 建议在 spec 明确「不走 approval gate，由本地审核」并测试。
- plan tab order 50 未进 docs/16 tasks（P2-2）；`tool-plan` role/capability 登记 tasks 已写；预算 16（P2-1）。
- **C20**：2 P0 映射 2/2；3×3；P0 摩擦 W1/W3 closed；用户 UNVERIFIED；通过机械要求。

### 2.3 skill-commands（`2026-11-01-client-skill-commands`）

- **P1-2（见 §6）**：enterprise 已安装商店技能实际在 `$ZEOKEEL_HOME/skills/<name>/<version>`（F8），而 `tool-skill` 只扫 workspace `.agents/skills`；spec「安装后同一目录」不成立，按设计无法列出/读取企业已安装技能。必须让 host `skill.list/read` 对 enterprise 读 `skill_root`（含版本选择、frontmatter/user-invocable、safe path），或安装时 materialize 到受控目录并明确 C18/permission；不能把「已安装」当成 workspace `.agents/skills`。
- **P1-3（见 §6）**：design §2.2 host 直接 `tool_skill::tool().call_json`；tasks 写作用域只有 `apps/desktop/src/ipc.rs`，没有 `tools/zk/src/main.rs`/`zk-composed` facade；按 Batch1/Batch2 的 C4 规则，app 不应直接依赖工具实现。应由生成 facade 暴露 tool-skill（或把 list/read 提升到 kit/IPC wrapper），补 budget/任务。
- **P2-3**：`tool-skill list` 当前返回换行 name（F7），spec 改 JSON array；这是 agent 工具输出格式变化，需保持 `skill` tool/调用方兼容或明确迁移；补行为测试。
- **C20**：1 P0 映射 1/1；3×3；P0 摩擦 W1/W3 closed；用户 UNVERIFIED；通过机械要求。

## 3. C1–C20 合规

| 约束 | 结论 |
| --- | --- |
| C1 个人版优先 | 通过（三 spec personal/enterprise 分支明确） |
| C2 构建期组合 | 基本通过（tool-plan 新包、无运行期加载；facade 需生成器） |
| C3 一切皆包 | 通过（tool-plan 四件套；skill/compact 不新建包） |
| C4 契约缝 | **skill 有风险**：host 直接 import `tool_skill`（P1-3）；plan 经 agent-rig/生成 facade 方向正确；compact kit 复用 |
| C5 requires | 不适用 |
| C6 弱管控 | 通过（plan 明确无 sandbox/企业策略；skill 不越权） |
| C7 企业网关 | compact enterprise 用 gateway；skill enterprise 已安装本地；plan 本地；通过 |
| C8 角色 | 不适用 |
| C9/C9b | 通过（设计无 unsafe/裸 as） |
| C10 外部输入 | 通过（新 IPC DTO/schema；plan/compact/skill 入参校验；需实现） |
| C11 生成物 | 通过（各任务有 export_schema/schema-check） |
| C12 先文档 | 基本通过（compact/plan/skill task0 docs/19/26/KNOWN；plan 缺 docs/16 order50，P2-2） |
| C13/C14/C15 | 不适用 |
| C16 无 spec 开工 | 通过（D1–D6/D3–D6/D5–D6 已勾选；draft 待评审） |
| C17 单一实现 | compact: new kit 单摘要；plan: tool-plan/no second gate；skill: 单 tool-skill 解析；**skill 的 host 直接依赖是 C4/C17 违规点（P1-3）**；compact `messages_in` 视图破坏单一 read 语义（P1-1） |
| C18 workspace | 基本通过（compact/plan/skill 均 workspace 归属）；skill enterprise `$ZEOKEEL_HOME` 全局路径需明确权限/归属（P1-2） |
| C19 | 本记录（P0/P1 未清不得 approved） |
| C20 | 通过机械要求：三份 planned、P0 映射 1/1、2/2、1/1、3×3、P0 摩擦 closed、用户 UNVERIFIED；但 plan P0 语义缺口见 P0-1 |

## 4. 预算与串行

| spec | 实现预算 | 实际 spec 目录 | 判断 |
| --- | --- | --- | --- |
| compact | 12 | 8 文件 348 行 | 实现清单 12（F13）；>10 文件阈值，proposal 未写 Lead 书面例外（P2-1） |
| plan-mode | 16 | 8 文件 320 行 | 实现清单 16；>10 文件阈值，未写例外（P2-1） |
| skill-commands | 9 | 8 文件 291 行 | ≤10，合规 |

- 串行/owner：三份 design §0 均写 compact→plan→skill；compact 首写 `contracts/ipc`、`ui-chat/AssistantComposer`、`App/controller/i18n`、`e2e/m1-serve`；后两份 rebase 追加。方向正确；需在 tasks/PR 严格执行（P2-1/2）。
- 原 30 文件 `client-command-backends` 已删除（F13）；无残留目录。

## 5. 可验收性

- tasks verify：`cargo test -p zk-contract-ipc/...`、`zk validate/compose --check/capability`、`pnpm -r typecheck/lint/test:unit/build`、`pnpm e2e`、`pnpm e2e:ui --grep ...`、`pnpm e2e:ui:gate` 均可执行（工具已落地）；acceptance 证据根/P0 路径清楚。
- **C20**：三份 interaction/intuition 均 planned，实现开始时 active；用户测试未执行均标 UNVERIFIED；不得声称符合直觉。
- 未验证（不冒充）：真实 provider/gateway 摘要、真实 e2e、用户测试、并发/子 agent plan 实测；实现后补证据。

## 6. 问题清单（P0/P1/P2，file:line + 修法 + OPEN）

### P0

**P0-1 plan mode 的 mutating 过滤可被 delegation 工具绕过（含 subagent/agent_team/workflow/ralph）**
- 位置：`plan-mode/spec.md:13`（所有 mutating 拒绝）、`plan-mode/design.md:23`（active 只保留只读 + exit；mutating 直接拒绝）、`plan-mode/interaction.md` `CMD3P-PLAN-01`；代码 `agent-rig/src/lib.rs:300-342`（dynamic_tools 按 spec）、`tool-subagent/src/lib.rs:37/72`（mutating false）、`tool-agent-team:264`、`tool-workflow:123`、`tool-ralph:16`、`crates/zk-kit-subagent:14`、`agent-rig:1597`（child tools.clone）。
- 问题：plan filter 只按 `ToolSpec.mutating`，delegation 工具声明只读但会启动子 agent；子 agent 继承全部 tools，未继承 plan filter，故 plan mode 下可经 subagent/agent_team/workflow/ralph 执行 mutating 工具，违背 R2「mutating 0 次」。MCP(`connector-mcp:914`)与 schedule(`tool-schedule:79`)声明 mutating true，会被 filter 挡住（通过）。
- 修法：plan filter 将 delegation 工具视为 mutating/阻断（推荐），或把 `PlanModeSource` 传播到 child agent 并强制其 dynamic_tools 过滤；对 `subagent`/`agent_team`/`workflow`/`ralph` 分别补「plan mode 下 child mutating 0」测试；更新 spec/design/interaction/acceptance。
- 状态：OPEN。

### P1

**P1-1 compact 把 `messages_in` 改成压缩视图，破坏原始历史/搜索/fork 的读取语义**
- 位置：`compact-command/spec.md:15`（marker + `messages_in` 视图）、`design.md:39-41`、proposal §4「原始仍可审计」；调用方 `ipc.rs:568`（UI）、`:2757`（replay）、`:5040`（fork）、`tool-session-query:48/65`（搜索/read）、`command_surface.rs:84`。
- 问题：`messages_in` 是所有会话读取的唯一入口（store:704/contract）；改成 marker+最近 N 后，UI 看不到旧消息、`session_search` 搜不到旧内容、fork 只复制压缩视图；原始消息仍在 DB 但没有公开 raw API，proposal「原始会话仍可审计」无法通过产品接口成立。也改变了 store 契约语义（C4/C17）。
- 修法：保留 `messages_in` 原始语义；新增 `messages_in_compacted`/`messages_view`（或 view 参数）仅供 host replay 与 CompactDivider UI；搜索/fork/审计读 raw；补 UI/search/fork/conformance 回归；若决定全压缩，必须在 spec 明示「原始仅 DB 审计」并改 proposal/acceptance。
- 状态：OPEN。

**P1-2 enterprise 已安装技能不在 workspace `.agents/skills`，skill 来源设计不可实现**
- 位置：`skill-commands/spec.md:8`（enterprise 已安装商店技能同目录）、`design.md:19-21`（host 调 tool-skill，tool-skill 扫 `.agents/skills`）；代码 `apps/desktop/src/ipc.rs:2388-2420`（installed_skills）与 `skill_root`（`$ZEOKEEL_HOME/skills/<name>/<version>`）；`tool-skill/src/lib.rs:34-59`（workspace `.agents/skills`）。
- 问题：企业安装器写 `$ZEOKEEL_HOME/skills/<name>/<version>`，tool-skill 只读 workspace `.agents/skills`；按现设计 enterprise 技能组为空/不可用，且无法读已安装内容；spec 的「同一目录」是事实错误。
- 修法：host `skill.list/read` 对 enterprise 读 `skill_root`（name/version 选择、frontmatter/user-invocable、safe path、已安装校验），personal 仍读 workspace `.agents/skills`；或安装时 materialize 到受控 workspace 目录并明确 C18/权限；更新 spec/design/tasks/acceptance，补 enterprise 已安装/未安装测试。
- 状态：OPEN。

**P1-3 skill host 直接 import `tool_skill`，破坏 C4 组合根边界**
- 位置：`skill-commands/design.md:18-20`（host 调 `tool_skill::tool().call_json`）；`tasks.md:13-17` 写作用域只有 `contracts/ipc`、`tool-skill`、`apps/desktop/src/ipc.rs`、UI；无 `tools/zk/src/main.rs`/generated facade。
- 问题：按 Batch1/Batch2 C4 规则，desktop 不应直接依赖工具实现（activity 用 zk-composed facade；plan 用 agent-rig facade）；host 直接 import `tool_skill` 会新增 app→tool 具体实现依赖，且 `tool-skill` 不是桌面基线的直接依赖。
- 修法：在 `zk-composed` 生成 facade 暴露 tool-skill（或把 list/read 的权限/路径逻辑提升到 kit），host 仅经 facade；加生成器任务/预算；或重新论证并记录 C4 例外（不推荐）。
- 状态：OPEN。

### P2（非阻断；建议本轮/实现阶段顺手修）

1. **P2-1 compact/plan 预算超 >10 文件阈值且无 Lead 书面例外**：`specs/templates/proposal.md:8` 要求 >10 文件或 400 行拆分；compact 12/plan 16 实现文件，proposal §8 未写 Lead 例外；建议记录例外（与 command-surface 同样）或拆 plan review UI/工具包。
2. **P2-2 plan tab order 50 未进 docs/16 任务**：`docs/16-ui-packages.md:27` 当前仅 activity 40；plan spec 要 order 50，但 plan tasks #0 只列 docs/19/26/KNOWN；需加 docs/16 回写。
3. **P2-3 `tool-skill list` 输出格式迁移**：现有 list 返回换行 name（F7），skill spec 改为 JSON array；需保持 `skill` agent tool 调用方兼容（或明确版本迁移）并补行为测试，避免 agent 工具输出回归。

## 7. 结论

- **结论：CHANGES-REQUESTED（P0=1 / P1=3 / P2=3）**，不可置 approved，不进入实现。
- P0-1：plan mode 的只读承诺可被 delegation 工具绕过，必须阻断/传播 plan filter；P1-1：compact 改变 `messages_in` 语义影响 UI/search/fork/审计；P1-2：enterprise 安装路径与 `.agents/skills` 不符；P1-3：skill host 直连 tool-skill 违反 C4。修复后同一评审员复评。
- C20 三份机械要求通过（P0 映射 1/1、2/2、1/1；3×3；P0 摩擦 closed；用户 UNVERIFIED）；`interaction.md`/`intuition.md` 仍 planned，实现开始前改 active。
- 未验证：真实 provider/gateway 摘要、真实 e2e、用户测试、plan 子 agent 实测；不冒充。
- 字段齐全：评审对象（HEAD+24 hash）、评审者、独立性、结论、P0/P1/P2、复评安排、关联 spec（三个 change id 均出现）。
- 复评日期/结论：**2026-11-01 / CHANGES-REQUESTED**（首轮；P0=1、P1=3、P2=3）。

## 8. 复评（第二轮，2026-11-01）

- 对象：HEAD `516f68553dd45616cd06eebfefe0496751d67a81` + 24 文件 SHA256（§8.1）。
- 评审者/独立性：同一评审员；未参与实现；仅写本记录。
- 结论：**CHANGES-REQUESTED（P0=1 / P1=0 / P2=1）**；P0-1 的「全部 delegation」仍漏 `subagent_fork`。

### 8.1 对象 SHA256（24 文件）

```text
053a88cbd1d65efac1ba6ce507e5abb46f87e36c0d6d5860a0650d8d67d49e50  specs/changes/2026-11-01-client-skill-commands/intuition-walkthrough.md
0c3ef306dcdc8dad5255812ade20701f0866aefe4925d77a7a75d352e176c194  specs/changes/2026-11-01-client-compact-command/intuition-walkthrough.md
0cc89e82ce84ef45a475ab145145e39f65c065f3c8ed78673ade1dec283b16da  specs/changes/2026-11-01-client-skill-commands/design.md
29aebcf2ff167b8b02a9bdc316876478e1834e769f997f38189d90a346b921ce  specs/changes/2026-11-01-client-skill-commands/tasks.md
33c2fdd676cc4aedb07a8762610201823100a9e6020a92e27690d95cc6bc467d  specs/changes/2026-11-01-client-plan-mode/intuition.md
5670b26e62224b855bfa9f309f08ea082f9254d742de9731212eacd44e13ed08  specs/changes/2026-11-01-client-skill-commands/intuition.md
58290ba29b02642a7f49be8762351fd8644b25a43ae7d326e08e50f3cf1eac77  specs/changes/2026-11-01-client-plan-mode/design.md
592df361be9becffbb1d520047a7087e7d2fce048aa5fca2d1e75d10523b3049  specs/changes/2026-11-01-client-plan-mode/proposal.md
5e21619c84dc2c726d41abb49055bb2376d42bf3b4687c824aca4af5929915b5  specs/changes/2026-11-01-client-compact-command/intuition.md
5eb8c5d8003b00c20d2fc41aad177dcd0903507602b7fc14a2abf61fafe5a97b  specs/changes/2026-11-01-client-compact-command/interaction.md
6aaf84912975a3eccedb626ed2d88aa10b15e386c52067d665d14fe64c39bf8b  specs/changes/2026-11-01-client-plan-mode/intuition-walkthrough.md
8b6710f3708f26583d3f4cc05e198a17c37e51db15aa1afe43c344d6b6e9a7a3  specs/changes/2026-11-01-client-skill-commands/interaction.md
92f25b9d1ddbcc1a8e270ebcb7fcc9f685fba538ef7437db5904e3f2e5a45162  specs/changes/2026-11-01-client-compact-command/spec.md
af192cb2c77b73cf827fc044caf4da081f77fbb2deb2c71ecf17406c94f8ca88  specs/changes/2026-11-01-client-plan-mode/interaction.md
bc12e88af47c91a9f8437be31926e3c4e0088f4a56de3cb1dc160ae0203816a7  specs/changes/2026-11-01-client-plan-mode/spec.md
c4eed939c90e4eced80ccd9a5f538914add966c4481139cbc00753474712761e  specs/changes/2026-11-01-client-compact-command/acceptance.md
c84da9f0bbe72d865daa8f22d4d8114c2fc05ccb146001a5a8e9f8932755bae1  specs/changes/2026-11-01-client-plan-mode/tasks.md
c86bb8679358d284bb494faef39829a957bcb822840302d8604ef317996c3fe4  specs/changes/2026-11-01-client-compact-command/proposal.md
c99d9929962dbabff9880ebdccffa742e458c71842c0d4fd004e6f80c1b17753  specs/changes/2026-11-01-client-skill-commands/acceptance.md
db88fea3d67ea9f51768bd7dcad404896aee6147c79b165aaf77bc9cbbb87bfd  specs/changes/2026-11-01-client-plan-mode/acceptance.md
dd73af08a80c87b6d8a1b3113f7fc646a48d9274ad228f1d07656c4283aa2900  specs/changes/2026-11-01-client-skill-commands/spec.md
e820603008f0a7ebf6df9dfe280c6d2daaf66bed0908cac7dff066c1bc482034  specs/changes/2026-11-01-client-skill-commands/proposal.md
eb9d553fefc3981bfe1dd1dee9ae48ed993c4984f800cd9bf577b6d9f9537a7e  specs/changes/2026-11-01-client-compact-command/design.md
f3af3df7d593789d957158e3dcdc7634f6ef0f2483e3feae4cec3721f170c256  specs/changes/2026-11-01-client-compact-command/tasks.md
```

### 8.2 首评条目关闭

| 条目 | 状态 | 依据 |
| --- | --- | --- |
| P0-1 plan delegation 绕过 | **部分 RESOLVED（仍 P0）** | 白名单/黑名单已加入 `subagent`/`subagent_start`/`agent_team`/`workflow`/`ralph` + schedule/MCP；但实际组合中还有 **`subagent_fork`**（`packages/client/tool-subagent-fork/src/lib.rs:17` `mutating:false`，role `tool.subagent_fork`，personal/enterprise editions 均启用；`zk.toml:349`），可 fork 子 agent 继承全部 tools，仍绕过 plan 只读边界。design/spec/tasks/acceptance 的 delegation 枚举与逐项测试均漏它。 |
| P1-1 compact messages_in | **RESOLVED** | spec:15-16/design:40-41/tasks #4/acceptance:30 恢复 `messages_in` raw；新增 `session.compacted_view` 仅 replay/分隔 UI/摘要；search/fork/审计 raw 且补测试。 |
| P1-2 skill enterprise 来源 | **RESOLVED** | spec:8/16、design:16/20 改为 personal `.agents/skills` + enterprise `$ZEOKEEL_HOME/skills/<name>/<version>`（版本/frontmatter/user-invocable/safe path/未安装禁用）；tasks #3/#4/acceptance 同步。 |
| P1-3 skill hosted facade | **RESOLVED** | design:9/19、tasks #3b 增加 `tools/zk/src/main.rs` facade；host 经 `zk-composed`（不 direct import tool_skill）；预算 10。 |
| P2-1/2/3 | **RESOLVED** | compact/plan Lead 书面预算例外已记录；plan docs/16 order50 进 tasks #0；`tool-skill list` 默认 text 兼容旧换行 + `format:"json"` 可选。 |

### 8.3 复评新发现

**P0-1（剩余）`subagent_fork` 未列入 plan delegation 黑名单**
- 位置：`plan-mode/spec.md:13`、`design.md:25`（枚举 `subagent/subagent_start/agent_team/workflow/ralph`）、`design.md:29`（逐项测试列表）、`tasks.md:16/33`、`acceptance.md:28`、`interaction.md:23`；代码 `packages/client/tool-subagent-fork/src/lib.rs:17`（`name:"subagent_fork"`、`mutating:false`）、`zk.toml:349`/editions（角色启用）。
- 问题：设计写「所有 delegation」但枚举/测试漏 `subagent_fork`；它通过 `zk_kit_subagent::runner().fork()` 启动子 agent，子 agent 继承全部 tools（`agent-rig:1597`），plan mode 父层未阻断 ⇒ 仍可执行 mutating，P0-1 未完全关闭。
- 修法：把 `subagent_fork` 加入白名单外的 delegation 黑名单/能力集合，并在 agent-rig 逐项阻断测试（child mutating 0）中覆盖；以后从 role/capability（如 `tool.subagent*`）派生而非硬编码名单（P2-1）。
- 状态：OPEN。

**P2-1（建议）delegation 名单应从 role/capability 派生**
- 位置：`plan-mode/design.md:25/29`、`tasks.md:16`；当前手写枚举已漏 `subagent_fork`，未来新增 delegation 仍会漏。
- 修法：在 agent-rig 以 `tool.subagent*`/delegation capability 或统一 `is_delegation` 分类判断，不维护手写名字列表；补覆盖测试。

### 8.4 C20 与门禁

- C20：三份 planned、P0 映射 1/1、2/2、1/1、3×3、P0 摩擦 closed（plan W1/W3 以当前 design 的 whitelist/blacklist 为修法）、用户测试 UNVERIFIED；本轮修改未破坏映射。
- `spec-check` exit 0（0 errors / 5 warnings，均为其他 draft/in-progress spec）；`docs-check` exit 0（41 files）；`review-check` exit 0（36 records；追加本记录后最新 CHANGES-REQUESTED）。
- 未验证：真实 agent-rig plan/child 实测、真实 provider/gateway 摘要、用户测试；不冒充。

### 8.5 结论

- 首评 P1-1/P1-2/P1-3 与 P2 全部 RESOLVED；P0-1 仅剩 `subagent_fork` 漏项（P0）。
- **复评结论：CHANGES-REQUESTED（P0=1 / P1=0 / P2=1）**，不可置 approved；补 `subagent_fork` 黑名单/测试后同一评审员复评即可通过。
- 关联 spec：`2026-11-01-client-compact-command`、`2026-11-01-client-plan-mode`、`2026-11-01-client-skill-commands`；对象 hash §8.1；用户测试 UNVERIFIED。
- 复评日期/结论：**2026-11-01 / CHANGES-REQUESTED**（P0=1、P1=0、P2=1）。

## 9. 最终复评（subagent_fork 修复后，2026-11-01）

- 对象：HEAD `516f68553dd45616cd06eebfefe0496751d67a81` + 24 文件 SHA256（§9.1）。
- 评审者/独立性：同一评审员；未参与实现；仅写本记录。
- 结论：**APPROVED（P0=0 / P1=0 / P2=1，P2 非阻塞）**；三份可一起置 approved 并进入实现（串行 compact → plan-mode → skill-commands）。

### 9.1 对象 SHA256（24 文件）

```text
053a88cbd1d65efac1ba6ce507e5abb46f87e36c0d6d5860a0650d8d67d49e50  specs/changes/2026-11-01-client-skill-commands/intuition-walkthrough.md
0c3ef306dcdc8dad5255812ade20701f0866aefe4925d77a7a75d352e176c194  specs/changes/2026-11-01-client-compact-command/intuition-walkthrough.md
0cc89e82ce84ef45a475ab145145e39f65c065f3c8ed78673ade1dec283b16da  specs/changes/2026-11-01-client-skill-commands/design.md
29aebcf2ff167b8b02a9bdc316876478e1834e769f997f38189d90a346b921ce  specs/changes/2026-11-01-client-skill-commands/tasks.md
5670b26e62224b855bfa9f309f08ea082f9254d742de9731212eacd44e13ed08  specs/changes/2026-11-01-client-skill-commands/intuition.md
5c95fc4344b0f3d05ef6b27481d54ee95b434438f860689992b8e3041151d0cf  specs/changes/2026-11-01-client-plan-mode/acceptance.md
5e21619c84dc2c726d41abb49055bb2376d42bf3b4687c824aca4af5929915b5  specs/changes/2026-11-01-client-compact-command/intuition.md
5eb8c5d8003b00c20d2fc41aad177dcd0903507602b7fc14a2abf61fafe5a97b  specs/changes/2026-11-01-client-compact-command/interaction.md
7c321a563c82445257d94db63bc94894ac341cf479471081454548093e9cbf38  specs/changes/2026-11-01-client-plan-mode/intuition-walkthrough.md
8b6710f3708f26583d3f4cc05e198a17c37e51db15aa1afe43c344d6b6e9a7a3  specs/changes/2026-11-01-client-skill-commands/interaction.md
92f25b9d1ddbcc1a8e270ebcb7fcc9f685fba538ef7437db5904e3f2e5a45162  specs/changes/2026-11-01-client-compact-command/spec.md
bc12e88af47c91a9f8437be31926e3c4e0088f4a56de3cb1dc160ae0203816a7  specs/changes/2026-11-01-client-plan-mode/spec.md
bdc4ea6a15654d547131e55068d629610386f7e355d6e096b3e032694d6e189e  specs/changes/2026-11-01-client-plan-mode/intuition.md
c4eed939c90e4eced80ccd9a5f538914add966c4481139cbc00753474712761e  specs/changes/2026-11-01-client-compact-command/acceptance.md
c86bb8679358d284bb494faef39829a957bcb822840302d8604ef317996c3fe4  specs/changes/2026-11-01-client-compact-command/proposal.md
c99d9929962dbabff9880ebdccffa742e458c71842c0d4fd004e6f80c1b17753  specs/changes/2026-11-01-client-skill-commands/acceptance.md
d734113d849216312db9e883d1080d601c4f57fe5793c2fe183e0b38fe2371f8  specs/changes/2026-11-01-client-plan-mode/tasks.md
dd73af08a80c87b6d8a1b3113f7fc646a48d9274ad228f1d07656c4283aa2900  specs/changes/2026-11-01-client-skill-commands/spec.md
e5037087f14e138087184fa3af2fc1afeb6f21dca0c92b85ed3bf6d222c4a567  specs/changes/2026-11-01-client-plan-mode/proposal.md
e820603008f0a7ebf6df9dfe280c6d2daaf66bed0908cac7dff066c1bc482034  specs/changes/2026-11-01-client-skill-commands/proposal.md
ea196f02edcbfa1b757f73b6fa59a5af547a799ef8afe9ebc6448c5bd0a381a7  specs/changes/2026-11-01-client-plan-mode/design.md
eb9d553fefc3981bfe1dd1dee9ae48ed993c4984f800cd9bf577b6d9f9537a7e  specs/changes/2026-11-01-client-compact-command/design.md
ec45f5e641ecf4cf99485bbb3a8b54e1cca3fd0232f310caae6d7a9a75b02cf9  specs/changes/2026-11-01-client-plan-mode/interaction.md
f3af3df7d593789d957158e3dcdc7634f6ef0f2483e3feae4cec3721f170c256  specs/changes/2026-11-01-client-compact-command/tasks.md
```

### 9.2 P0 关闭核对

| 条目 | 状态 | 依据 |
| --- | --- | --- |
| P0-1 `subagent_fork` 绕过 plan 只读 | **RESOLVED（行为）** | design:25 黑名单加入 `subagent_fork`；design:26 `PlanModePolicy` 派生规则（`name=="subagent"`/`starts_with("subagent_")`/`{agent_team,workflow,ralph}`，role/capability 优先、静态六项兜底）；design:30/tasks:16/33/acceptance:28/interaction:23/INT-05/walkthrough W1 均含 `subagent_fork` 专项 + child mutating 0；实际 `tool-subagent-fork` role/edition 启用（F）。 |
| P1-1 compact | RESOLVED | 上轮已修：`messages_in` raw、`session.compacted_view` 仅 replay/分隔；search/fork/审计 raw；测试。本轮 compact 文件 hash 未变（上轮状态保留）。 |
| P1-2 skill 来源 | RESOLVED | personal `.agents/skills` / enterprise `$ZEOKEEL_HOME/skills/<name>/<version>`；版本/frontmatter/user-invocable/safe path；skill 文件 hash 未变。 |
| P1-3 skill facade | RESOLVED | host 经 `zk-composed` generated facade（tasks #3b/预算 10），不 direct import tool_skill；skill 文件未变。 |
| P2-1/P2-2/P2-3（上轮） | RESOLVED | compact/plan Lead 预算例外、plan docs/16 order50 回写、tool-skill list 默认 text 兼容 + `format:"json"` 可选。 |

### 9.3 残留 P2（不阻塞）

1. **P2-1 `plan-mode/spec.md:13` 仍未显式列出 `subagent_fork`**（其枚举为 `subagent/subagent_start/agent_team/workflow/ralph 等`）；design/tasks/acceptance/interaction/INT/walkthrough 已显式含 fork。行为规范有「所有 delegation 工具…等」+ design 的 `subagent_` 派生，P0 已关闭；建议在置 approved 前把 spec R2 枚举补上 `subagent_fork`，保持六处一致。非阻塞（Lead 指示 P2 不再阻塞三份批准）。

### 9.4 C20 与门禁

- C20：三份 planned、P0 映射 1/1、2/2、1/1、3×3、P0 摩擦 closed、用户测试 UNVERIFIED；plan W1 已含 subagent_fork 的 delegation 全阻断（行为由 design PlanModePolicy 保证）。
- `spec-check` exit 0（0 errors / 5 warnings，均为其他 draft/in-progress spec）；`docs-check` exit 0（41 files）；`review-check` exit 0（36 records，追加后最新 APPROVED）。
- 未验证：真实 agent-rig child/plan 实测、真实 provider/gateway 摘要、用户测试；不冒充。

### 9.5 结论

- **P0=0 / P1=0；P2=1（spec 枚举文档性残留，不阻塞）**。
- **三份一起 APPROVED**：`2026-11-01-client-compact-command`、`2026-11-01-client-plan-mode`、`2026-11-01-client-skill-commands`；可置 approved 并进入实现，顺序 **compact → plan-mode → skill-commands**，共享文件按 compact 首写、后两份 rebase。
- 实现开始时三份 interaction/intuition 改 active 并补 P0/P1 标签测试/evidence；用户测试仍 UNVERIFIED，发布前 ≥5 人 think-aloud，此前不得声称「符合直觉」。
- 关联 spec 三个 change id 均出现；对象 hash §9.1；复评日期/结论：**2026-11-01 / APPROVED（P0=0、P1=0、P2=1）**。
