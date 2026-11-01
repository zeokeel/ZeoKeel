# 2026-11-01-client-command-surface · C19/C20 独立评审

- 状态：本轮评审完成，**CHANGES-REQUESTED**（P0=0 / P1=5 / P2=3，均 OPEN；见 §6）。
- 评审对象：HEAD `596b1672fc19ac18a7e78d7e343201143be95561` + 8 文件 SHA256（`proposal/spec/design/tasks/acceptance/interaction/intuition/intuition-walkthrough`；§0.1）。
- 关联 spec：`2026-11-01-client-command-surface`（唯一 change id；单 spec 不计子 spec）。
- 评审者：`spec-reviewer`（C19/C20 fresh-context 非实现者）。
- 独立性：未参与本 spec 的写作、设计或实现；只读 DSH 审计、现有代码/契约、docs/11/19/27 与 8 文件；唯一写入为本记录。
- 评审日期：2026-11-01。
- 结论：**CHANGES-REQUESTED（P0=0 / P1=5 / P2=3）**；不进入实现。
- 复评安排：P1 修复后由同一评审员复评；C20 走查/交互文件仍 `planned`，实现开始前需改 `active`；用户测试仍 UNVERIFIED。

## 0. 对象与方法

### 0.1 对象 SHA256（8 文件）

```text
15bf965806f71d50da45886dfb1298e47f3dcc0ad9451c599b2797b828942f2c  specs/changes/2026-11-01-client-command-surface/acceptance.md
06e9c39e8eba51eab76a9481767c7811679c9b80326417bb9f318e1f83de0d99  specs/changes/2026-11-01-client-command-surface/design.md
c61473de4ad217ad529775514814c0d031ad31f84df56ea057597b264e8909da  specs/changes/2026-11-01-client-command-surface/interaction.md
037651733333f9e54959e1c3e74d127ed44954ee359d806fd61be1b6eb2e27aa  specs/changes/2026-11-01-client-command-surface/intuition.md
15ecb3b8e1d2c0e7805ce1afd3b3e7a62142dc0f84356578833a995d787dd0fe  specs/changes/2026-11-01-client-command-surface/intuition-walkthrough.md
5d234a049e0c9edd85e268bdb06925e14bda1e04ed5a95899da3db3c152f9024  specs/changes/2026-11-01-client-command-surface/proposal.md
b1ba62fd9500d9b9c9c0d5534f33b641e9e43f29016ba4589e30c52c82b5fd44  specs/changes/2026-11-01-client-command-surface/spec.md
8f60bbe3a430fd8e078d1767edcb0bcc4266c8602b7b9ae1985b2babb67cddc9  specs/changes/2026-11-01-client-command-surface/tasks.md
```

### 0.2 依据

`AGENTS.md` C1–C20；`docs/27` §2.5–§2.8、`docs/19` §2/§11/§11.1、`docs/11` §8.6；`target/audits/dsh-reference-inventory.md` §5；`docs/26` §5；当前代码：`apps/desktop/src/ipc.rs`、`apps/desktop/web/src/App.tsx`、`packages/ui/ui-kit/web/src/types.ts`、`packages/ui/ui-sessions/web/src/index.tsx`、`packages/contracts/{agent,config,ipc}/src/lib.rs`、`packages/client/tool-goal/src/driver.rs`、`packages/client/store-session-sqlite/src/lib.rs`。

## 1. 事实核验（HEAD `596b167`，path:line）

| # | 事实 | 证据 | 与 spec 的关系 |
| --- | --- | --- | --- |
| F1 | DSH `/` 命令面 | `target/audits/dsh-reference-inventory.md:319-340`：6 Host（compact/export/feedback/goal/permission/plan）+ 2 client（model/file）+ skill；`/export` 为 **ZIP**、`/feedback` 为打开反馈弹窗、`/permission` 为沙箱+审批预设；`341-342`：未知命令拒绝、附件仅声明命令可接收 | spec R1/R2 的 1:1 映射基本准确；`/export` MD、`/feedback` 本地、`/permission` 仅审批的差异**已在 proposal/design 诚实记录**（D3/D4/D6） |
| F2 | 现有 composer catalog | `apps/desktop/web/src/App.tsx:92-117` `buildCommandCatalog`：personal 6（含 cancel/retry）、enterprise +3；`packages/ui/ui-kit/web/src/types.ts:641-656` `CommandDescriptor` 当前仅 id/title/group/hint/disabled | 扩展 `surface/kind/disabledReason` 是 additive；唯一 catalog 事实成立（非第二目录） |
| F3 | personal 模型来源 | `packages/contracts/config/src/lib.rs:23-34` `ProviderConfig{kind,base_url,model,api_key}`：只有**单个 `model` 字符串**，无模型数组/列表 | **spec R4「个人版显示 provider 模型列表」没有数据来源**（P1-3） |
| F4 | enterprise 模型列表 | `apps/desktop/web/src/App.tsx:134` `gatewayModels` 状态；`auth.ts:79-92` 从 gateway 响应取 `models[]` | enterprise 列表来源存在；host 侧 revalidation 无列表来源（P2） |
| F5 | agent 配置是全局热切换 | `packages/contracts/agent/src/lib.rs:327-340` `configure_agent/configure_model` 为 agent 实例的全局变更；`packages/client/agent-rig/src/lib.rs:905/925` 实现 | per-conversation override 若直接调它们，会跨会话串（P1-2） |
| F6 | turn 并发粒度 | `apps/desktop/src/ipc.rs:43` `turns: HashMap<conversation_id, Sender<TurnJob>>`；`:2538-2549` 每个 conversation 独立 FIFO worker | **不同 conversation 可并发 turn**；design §2.3 的「单 turn 串行（现有 active-turn gate）」不成立（P1-2） |
| F7 | 审批策略 | `packages/contracts/config/src/lib.rs:136-170` `ApprovalPolicy::{Ask,AutoReadOnly,Strict}` + `should_ask`；`Ask` 仅 `requires_approval`，`Strict` 全问 | D6 三档映射与代码一致；`/permission` 不能沙箱化的事实已诚实写明 |
| F8 | tool-goal 公开 driver | `packages/client/tool-goal/src/driver.rs:237/263/303/334` `goal/set_goal/update_goal/cancel_goal`；同一 `.zeokeel/goal.json` | `/goal` host 单 owner 方案可行（D2） |
| F9 | 现有 session/store | `packages/client/store-session-sqlite/src/lib.rs:335` 已到 v6；`apps/desktop/src/ipc.rs:944-1041` `workspace.files` 懒加载 | 本 spec 无 DB migration、复用 store 读取成立；override 文件走 workspace 元数据 |
| F10 | ui-sessions 私有 Modal | `packages/ui/ui-sessions/web/src/index.tsx:35` `function Modal`（局部）；`ui-kit` 当前无共享 Modal | proposal 说要提升该 Modal，但 tasks 只新增 `ui-kit/Modal.tsx`、未列出 ui-sessions 迁移 ⇒ 两套 Modal 并存（P1-5，C17） |
| F11 | docs/26 §5 现状 | `docs/26-workspace-and-conversations.md:156-175` 只有既有 IPC（含 attachment_save），**无 goal.*/session.override_*/session.export/feedback.submit** | task 0 计划登记；当前 spec 尚未落地，属实现前文档任务（非本轮 P1，但需按 C12 执行） |
| F12 | 预算模板阈值 | `specs/templates/proposal.md:8`：超过 **10 文件或 400 行**需拆分；本目录 24 文件、8 文件共 **728 行** | 未拆且无 Lead 例外（P1-4） |
| F13 | C20 产物 | `interaction.md` 8 个 P0；`intuition.md` 12 INT，P0 映射 8/8（脚本核对）；walkthrough 有 3×3 矩阵；用户测试 **UNVERIFIED** | C20 基本完整（P0 映射/矩阵通过） |
| F14 | 下载/打开能力 | `apps/desktop/src/ipc.rs:1699` 只有 `system.open_url`（URL），无通用“打开文件所在位置/下载” IPC；serve 侧无 Content-Disposition | spec R7「Tauri 与 serve 都提供下载/打开所在位置」与 design:103「若没有 opener 则仅显示路径」不一致（P2-3） |

## 2. 1:1 差距诚实性核对

| DSH 事实 | 本 spec | 诚实性 |
| --- | --- | --- |
| `/export` 下载 ZIP（含日志） | Batch1 Markdown 到 `.zeokeel/exports/` | **诚实**：proposal D3/design:119/KNOWN 任务均写明 ZIP 留 Batch2 |
| `/feedback` 平台提交/反馈弹窗 | 本地 JSON + 「本地保存」；企业服务端 UNVERIFIED | **诚实**：D4/design:110/spec R8 明确不假发送 |
| `/permission` 含 sandbox（只读/工作区写/完全权限） | 仅审批映射 Strict/AutoReadOnly/Ask；UI 标注无沙箱/不限制员工 | **诚实**：D6/spec R5 明确差异，不宣称沙箱 |
| `/model` provider/模型/推理等级列表 | personal 模型 + effort；enterprise gateway 列表 | **不诚实/不可实现**：personal 只有单 `model` 字段，无列表来源（P1-3）；enterprise host revalidation 无列表来源（P2-2） |
| Batch2 `/compact /plan /skill` | disabled + 「未实现（Batch 2）」；不可执行 | **通过**：D5/spec R2+interaction `CMD2-MENU-02` 均冻结 |
| 未知命令不静默降级 | 拒绝、不发送、草稿保留 | **通过**：spec R2/interaction `CMD2-MENU-02` |

## 3. 会话 override / goal / permission 专项

- **D1 存储**：workspace 文件 `.zeokeel/conversation-overrides.json`，按 conversation 键；无 DB migration；原子写；不含 key。设计可行，C18 正确（workspace 归属）。
- **P1-2（阻断）全局 agent 配置与并发 turn 的兼容**：通过 `configure_model/configure_agent` 应用 override 是**全局**变更（F5），而 desktop 的 turn worker 是**每 conversation** 独立 FIFO（F6），不同会话可并发 turn；design §2.3 只说「单 turn 串行（现有 active-turn gate）」，但仓库没有该全局 gate。结果可能：会话 A 的 override 被会话 B 的 turn 使用/被切回恢复时覆盖，违背 R4/R5「只影响当前会话」。design 还写「跨会话并发 turn 不支持（记录 KNOWN）」，但 spec/R4/R5 未写该限制，acceptance 也未验证；不能以 KNOWN 替代行为规格。
- **P1-2 附带的 permission seam 未冻结**：design:84 仍写「按 override 提供 ApprovalGate 包装/**或**扩展 turn context；实现任务明确最小 seam」，没有选定 seam；task 6 也只写「turn 前应用 ... 并切回恢复」。审批 override 是安全敏感行为，必须在 design/tasks 冻结唯一实现（推荐 turn context 的 policy 字段，避免包装器改全局 gate），并补 host 单测。
- **settings revision/agent 热切换**：override 不进全局 `settings`，本身不触发 revision；但 turn 结束「重新应用 global settings」没有 revision/generation 栅栏，若用户在同 turn 保存 settings/hot-reload，可能恢复成错误代际。design §2.3 未描述；建议以 settings revision 读取并校验（或 turn 级快照），补测试。另：enterprise gateway 列表在 host 的 revalidation 来源未定（P2-2）。
- **goal 单 owner**：host 走 `tool_goal::driver` + 同一 `.zeokeel/goal.json`，工具行不产生；GoalBar 只读同一状态；方向正确（C17/C18）。`pause=blocked+reason`/`resume=active` 需补 driver 的合法性测试（P2 建议）。
- **permission**：`Strict/AutoReadOnly/Ask` 映射与代码一致，但 `Ask`（完全权限）仍不能绕过 requires_approval；spec/UI 必须保持这一弱不变量（R5 已写），实现时不得改 `ApprovalPolicy::should_ask`。

## 4. C1–C20 合规

| 约束 | 结论 |
| --- | --- |
| C1 个人版优先 | 通过（personal/enterprise 分支明确） |
| C2 构建期组合 | 通过（无运行期加载/新 slot） |
| C3 一切皆包 | 通过（仅 ui-chat/ui-kit，无新包） |
| C4 契约稳定缝 | 通过（host 经 tool-goal 公开 driver；新 IPC DTO；不跨包具体类型） |
| C5 requires | 不适用 |
| C6 弱管控 | 通过（`/permission` 明确无沙箱/不限制员工；不新增企业策略） |
| C7 企业 AI 网关 | 通过（enterprise `/model` 只 gateway 列表，不持上游 key） |
| C8 角色 | 不适用 |
| C9/C9b 严格/unsafe | 通过（设计无 unsafe/裸 as；实现按 docs/18） |
| C10 外部输入 | 通过（新 IPC Rust DTO + generated schema；override/enum/长度校验；但 enterprise host 列表来源 P2） |
| C11 生成物 | 通过（task 2 生成器写） |
| C12 先文档 | 基本通过（task 0 有 docs/19/26/16/KNOWN；docs/26 §5 尚未落地，属实现前） |
| C13/C14 更新/渠道 | 不适用 |
| C15 参考仓 | 不适用于依赖（仅 audit 事实） |
| C16 无 spec 不开工 | **不通过**：Lead 已批准 D1–D6，但 proposal §7 仍全部未勾选、tasks:4/acceptance:3 仍写「待拍板」；决策记录未落（P1-1） |
| C17 单一实现 | **不通过**：新增 ui-kit Modal 但 ui-sessions 私有 Modal 未迁移（F10；P1-5）；其余复用方向正确 |
| C18 workspace 归属 | 通过（override/goal/export/feedback 均 workspace 内；归属校验） |
| C19 发现即文档/评审 | 本记录（P1 未清前不得 approved） |
| C20 直觉/交互契约 | 基本通过：8 P0 ID、12 INT、P0 映射 8/8（脚本）、3×3 walkthrough、P0 摩擦 closed（W2/W3/W4/W5/W6/W8）、用户测试 UNVERIFIED 写明；planned→active 在 tasks #11 有步骤；不把 e2e 当直觉证明 |

## 5. 预算与可验收性

- **P1-4 超阈值未拆分**：`specs/templates/proposal.md:8` 明确 >10 文件或 >400 行需拆分；本 spec **24 文件**（即使按 §8 计）、目录 **728 行**，proposal §8 仅写「不拆」但无 Lead 书面例外。task 明确要求核对「>28 或行数 >400 是否应拆 `/export` `/feedback` 子 spec」；按模板应拆（至少 `/export` 与 `/feedback` 各自子 spec，或 Lead 记录例外并说明审计成本）。
- **P2-1 预算数字不一致**：proposal:8 写「25 个文件」，§8:81 写「24」、tasks:39/acceptance 按 24；需统一（建议 24）。
- **可执行性**：`spec-check`/`docs-check`/`cargo`/`pnpm e2e` 命令路径存在；`pnpm e2e:ui`/`scripts/ui-interaction-gate.py` 在并发基线已落地（task 提示 d0b7741），但 tasks:54/acceptance:24 仍标「待工具落地/UNVERIFIED」；实现前应把实际脚本存在性/命令结果回填，不能沿用旧 UNVERIFIED 文案。
- **evidence**：acceptance 1.5 的 8 个 P0 证据路径与 interaction 的 `target/evidence/<id>/interaction/<ID>.json` 对齐；`m1-serve.spec.ts` 追加场景明确；用户测试路径 `usability/` 明确且 UNVERIFIED。
- **文档**：docs/26 §5 新命令/错误码/文件路径尚未登记（task 0），实现前必须落；否则 C12/C16 不完整。

## 6. 问题清单（P0/P1/P2，文件:行 + 修法 + 状态）

### P1（阻断；修复后复评）

**P1-1 D1–D6 已由 Lead 批准，但 spec 决策记录仍为「未拍板」**
- 位置：`proposal.md:70-76`（D1–D6 全部 `- [ ]`）、`:77`「未拍板前保持 draft」；`tasks.md:4`「D1–D6 待用户拍板」；`acceptance.md:3`「待拍板」；而任务背景明确 Lead 已按推荐 A 批准 D1–D6。
- 问题：C16 要求 spec 经用户确认并记录决策；当前 8 文件相互矛盾，读者/实现者会误以为未批准。
- 修法：proposal §7 六项勾选 `[x]` 并记录「Lead 2026-11-01 批准推荐 A」；tasks/acceptance 改为「已拍板」；`proposal.评审` 填本记录（批准后）；重算 hash 复评。
- 状态：OPEN。

**P1-2 会话级 model/permission override 依赖全局 agent 配置，存在跨会话串/并发 turn 未定义**
- 位置：`design.md:62-85`（§2.3 override 应用；:72-73 `configure_model/configure_agent`；:73「单 turn 串行/跨会话并发 turn 不支持」；:84 permission seam 未定）、`spec.md:34-51`（R4/R5 声称只影响当前会话）、`tasks.md:22`（task 6）；代码事实 `apps/desktop/src/ipc.rs:43/2538-2549`、`packages/contracts/agent/src/lib.rs:327-340`。
- 问题：`configure_model/configure_agent` 是 agent 全局变更；desktop 每 conversation 独立 turn worker，可并发。design 的「单 turn 串行」无对应全局 gate；permission seam（包装 gate 或扩展 turn context）也未冻结。并发时一个会话的 model/permission override 可能作用于另一会话的 turn，违反 R4/R5 的会话隔离，且 `/permission` 是安全敏感项。
- 修法（择一并冻结）：(a) 增加全局 turn 互斥（所有 conversation 同时只允许一个 turn）并在 spec/acceptance/KNOWN 写明；或 (b) 为每个 turn 创建/切换独立 agent 配置（turn-level context），不调用全局 `configure_model/configure_agent`；或 (c) 把 override 限制为单 workspace 单 active turn 并显式拒绝并发。优先 (b)；同时固定 permission seam（推荐 turn context 的 policy 字段，不改全局 gate）并补「两会话并发、一个 override、切回恢复」host/e2e 测试。
- 状态：OPEN。

**P1-3 `/model` personal「provider 模型列表」无数据来源**
- 位置：`spec.md:34-41`（R4：个人版显示 provider 模型列表+effort）、`design.md:62-74`（只定义 override 存储/应用，未定义模型列表来源）、`:69`（enterprise gateway 列表来源也带 UNVERIFIED）；代码 `packages/contracts/config/src/lib.rs:23-34` 只有单个 `provider.model`，`ModelsPane` 无模型数组。
- 问题：DSH `/model` 是 provider/模型/推理等级列表；我们的设置契约没有模型列表，`/model` 无法按 R4 渲染可选模型（除非硬编码，违反 1:1/可维护）。这是未记录的 1:1 差距，不是 D3/D4 已记录的差异。
- 修法：明确模型列表来源并落契约/DTO：例如新增 `session.model_options`（personal 返回 provider 已知模型 + current；enterprise 返回 gatewayModels），或把 `/model` 明确为「当前模型只读 + 手动模型名输入」（并写 KNOWN/偏离 DSH），二者择一；在 spec R4/design/KNOWN 记录，补 host 校验与 UI 空态测试。
- 状态：OPEN。

**P1-4 24 文件 / 728 行超模板拆分阈值，无 Lead 例外**
- 位置：`specs/templates/proposal.md:8`（>10 文件或 400 行需拆分）；`proposal.md:80-92`（§8 24 文件、明确「不拆」）；本目录 8 文件共 728 行；task 要求核对是否拆 `/export` `/feedback`。
- 问题：同时超过文件数与行数阈值，proposal 未记录 Lead 例外/拆分理由；C16/spec 模板要求拆分或书面例外。当前单 spec 范围过大（8 命令+3 占位+override/goal/export/feedback），评审/实现风险高。
- 修法：拆 `/export` 与 `/feedback` 为子 spec（或至少拆 `/export`），保留本 spec 为命令框架+`/file /model /permission /goal`；或由 Lead 在 proposal §8 记录书面例外（含预算/审计理由），并更新 tasks/acceptance。推荐拆分（与 task 的「>400 行」要求一致）。
- 状态：OPEN。

**P1-5 C17：新增 ui-kit Modal 但 ui-sessions 私有 Modal 未迁移，形成两套 Dialog**
- 位置：`proposal.md:105`（称把 ui-sessions 私有 Modal 提升为 ui-kit 共享原语）、`tasks.md:19`（只新增 `ui-kit/Modal.tsx`，写作用域无 ui-sessions）、`packages/ui/ui-sessions/web/src/index.tsx:35`（现有私有 Modal 仍在）；interaction:23 要求 FeedbackDialog 复用 ui-kit Modal。
- 问题：实现后 ui-sessions 的 `Modal` 与新 `ui-kit/Modal.tsx` 并存，违反 C17「同一 Dialog 只有一套实现」；proposal 的提升声明未落到任务/写作用域/预算。
- 修法：把 `ui-sessions` Modal 迁移到 `ui-kit/Modal.tsx` 并让 ui-sessions import（增加 ui-sessions 写作用域/预算或从 ui-kit 现有原语提升）；或明确本 spec 不新增 Modal、复用现有组件（但无法满足共享要求）；更新 proposal/tasks/budget 与验收。
- 状态：OPEN。

### P2（非阻断；建议本轮顺手修）

1. **P2-1 预算数字不一致**：`proposal.md:8` 写 25，§8:81/tasks:39/acceptance 为 24；统一为 24（若拆分则重算）。
2. **P2-2 enterprise `/model` host 列表校验来源未定**：`design.md:69` 写「host 重新校验列表可由 gateway_credentials/bootstrap 提供；若 host 没有列表则前端+host 双重长度/枚举，标 UNVERIFIED」；需在实现前确定 host 的 gatewayModels 来源（`apps/desktop/web/src/auth.ts:79-92` 只在前端），否则 enterprise 可提交不在列表的模型。
3. **P2-3 `/export` 下载/打开能力口径矛盾**：spec R7:65 要求「Tauri 与 serve 都提供下载/打开所在位置」，design:103 说「Tauri 若没有 opener 则仅显示路径」；仓库只有 `system.open_url`（`apps/desktop/src/ipc.rs:1699`），无通用文件打开/下载 IPC。二选一：spec 降级为「显示路径/复制路径，Tauri 不提供打开」（写 KNOWN），或新增最小文件打开 IPC 并进预算/安全评审。

## 7. 门禁与结论

- 门禁（评审时执行）：待写记录后复跑 `spec-check`/`docs-check`/`review-check`；本轮命令结果与本记录一起提交（见重跑记录）。
- 未验证（不冒充）：真实 Tauri/serve 下载、企业网关模型列表、并发 turn 实测、≥5 人用户测试；用户测试按 C20 仍 **UNVERIFIED**，不得声称「符合直觉」。
- **结论：CHANGES-REQUESTED（P0=0 / P1=5 / P2=3）**。
- P1-1 补决策记录；P1-2 冻结 override 应用 seam/并发模型；P1-3 定义 `/model` 模型来源；P1-4 拆分或 Lead 例外；P1-5 消除 Modal 重复。修复后同一评审员复评；C20 的 `interaction.md`/`intuition.md` 仍 planned，实现开始前必须与 `active` 同步并补 P0/P1 标签测试/证据。
- 关联 spec 与字段齐全：评审对象（HEAD+8 hash）、评审者、独立性、结论、P0/P1/P2、复评安排、关联 spec（`2026-11-01-client-command-surface`）。
- 复评日期/结论：**2026-11-01 / CHANGES-REQUESTED**（首轮；P0=0、P1=5、P2=3）。

## 8. 复评（第二轮，2026-11-01）

- 对象：HEAD `324b3244f70b85c86d1150df7a25c4a124335567` + 8 文件 SHA256（§8.1）。
- 评审者/独立性：同一评审员；未参与实现；仅写本记录。
- 结论：**CHANGES-REQUESTED（P0=0 / P1=1 / P2=1）**；首评 P1/P2 多数关闭，但 config-mutex 未覆盖 settings.set 的全局热切换，P1-2 仍不完整。

### 8.1 对象 SHA256（8 文件）

```text
2689811c37f7ace638da81f21bd8a5dabf680a7c02a88c6abe647d6e82ead52f  specs/changes/2026-11-01-client-command-surface/acceptance.md
3398b62e0a7f6fe7100eb77d72b275d0c3bfddccd035a4e35244e09662b5ba0f  specs/changes/2026-11-01-client-command-surface/intuition.md
5fd95828e18dd0f2ad0f09a4b4a3b2b23c7e1a3bf472262e8cf2885954d2127a  specs/changes/2026-11-01-client-command-surface/proposal.md
8767a8ef64d0a18868053de2fb2dcf4f37254882707a7a0bbe5a07bd28df22d0  specs/changes/2026-11-01-client-command-surface/intuition-walkthrough.md
d5194c09b9304b61aa91ba4bf6f0bb0441f3456334689d87176697d539080dca  specs/changes/2026-11-01-client-command-surface/interaction.md
da286456860f8c99f5026ae6e124b8cc23f6912589c986fa607ba65675328ab6  specs/changes/2026-11-01-client-command-surface/spec.md
efe05fb1362b44e3f2256d8babb96a31e44e557a8e0bd9fce6e9d789c8809edc  specs/changes/2026-11-01-client-command-surface/design.md
f18b9311a45c95e7fe2e55aba292e597ebc22322b0e34d1a2212a55f74b53591  specs/changes/2026-11-01-client-command-surface/tasks.md
```

### 8.2 首评条目关闭

| 条目 | 状态 | 复评依据 |
| --- | --- | --- |
| P1-1 D1–D6 | **RESOLVED** | proposal §7 全部 `[x]`，Lead 2026-11-01 推荐 A；评审链接/tasks/acceptance 已回填 |
| P1-2 config-mutex/overrides | **部分 RESOLVED，仍有 P1** | design §2.3:74-80 已定义 turn 级 global mutex、snapshot/revision 恢复、并发排队；agent-rig `set_approval_policy` 存在（lib.rs:270），permission 单点方案可落。**但 `settings.set` 路径在 mutex 外直接 apply_agent_settings/configure_model**（`apps/desktop/src/ipc.rs:609-670`；`apply_agent_settings:182-204`），可在 turn 运行中改写全局 agent/provider，覆盖当前会话 override；P1-2 未完全消除（P1-1）。 |
| P1-3 `/model` 数据源 | **RESOLVED** | personal 明确降级「当前模型+手动输入+recentModels≤8+effort」并记录 DSH 差异（proposal:70/spec:37/design:67）；enterprise 列表由 `auth.configure` 可选 models[] host 缓存校验；诚实 |
| P1-4 预算 | **RESOLVED（Lead 例外）** | proposal:8/89 记录 24 文件/约 728 行 Lead 书面例外；§8 清单合计 24 |
| P1-5 Modal 重复 | **RESOLVED** | ui-kit Modal 放 `index.tsx`；tasks #3b 迁移 ui-sessions 私有 Modal、禁止两套；预算含 ui-sessions 1 |
| P2-1/2/3 | **RESOLVED** | 预算统一 24；enterprise 列表来源 `auth.configure`，UI 仅展示；export Tauri 仅展示/复制路径、不承诺自动打开；proposal/spec/design/tasks 一致 |

### 8.3 复评新发现

**P1-1 settings.set 的全局 agent/provider 热切换不在 config-mutex 内，仍可跨会话串/覆盖 override**
- 位置：`spec.md:39`（turn 前应用/turn 后 snapshot/revision 恢复）、`design.md:74-80`（mutex 只覆盖 turn；恢复逻辑）；代码 `apps/desktop/src/ipc.rs:609-670`（`SETTINGS_SET` 直接 `apply_agent_settings`/provider `configure_model`）、`apply_agent_settings:182-204`。
- 问题：mutex 只包裹 turn 开始/结束；settings UI 保存仍在独立 IPC 线程直接改全局 agent/provider。turn 持有 override 期间用户保存设置，会把全局配置改掉，当前 turn 使用错误 model/permission；随后 turn 结束按 revision 重新读取最新全局，可能掩盖该次错误。与「override 只影响当前会话 + 全局 config-mutex」目标冲突。
- 修法：`SETTINGS_SET` 的 apply/publish 阶段获取同一 global config mutex（或排队到当前 turn 结束；窗口期显示 busy/稍后重试），保证 turn 的 apply→run→restore 原子；补「turn 运行中 settings.set」测试（override 不被覆盖、结束后按 revision 恢复）；必要时把 settings 热切换统一到 command_surface mutex。
- 状态：OPEN。

**P2-1 排队 turn 的取消/停止语义未定义**
- design 的排队保证「不丢请求」，但 `chat.cancel` 对尚未取得 mutex 的 turn 只在 streams map 中置 flag；worker 何时/是否跳过、UI 是否显示排队中，未在 spec/design/tasks 冻结。建议定义 queued turn 可取消/取消后不执行，并补并发/取消测试（P2）。

### 8.4 C20 与门禁

- C20 文件仍 12 INT、8 个 P0、P0 映射 8/8、3×3 walkthrough、P0 摩擦 closed、用户测试 **UNVERIFIED**；本轮修改未破坏映射（计划仍为 planned，实现时 active）。
- `spec-check` exit 0（0 errors / 4 warnings，均为其他 draft/in-progress spec）；`docs-check` exit 0（41 files）；`review-check` exit 0（35 records，追加后仍）。
- 未验证：真实并发/settings 实测、Tauri 导出、用户测试；不冒充。

### 8.5 结论

- **首评 P1-1/P1-3/P1-4/P1-5 与 P2 全部 RESOLVED；P1-2 部分修复，新增 P1-1 + P2-1**。
- **复评结论：CHANGES-REQUESTED（P0=0 / P1=1 / P2=1）**，不可置 approved；修复 settings.set 与 turn 的互斥/排队后同一评审员复评。
- 关联 spec：`2026-11-01-client-command-surface`；对象 hash 见 §8.1；用户测试 UNVERIFIED 不得声称符合直觉。
- 复评日期/结论：**2026-11-01 / CHANGES-REQUESTED**（P0=0、P1=1、P2=1）。

## 9. 第三轮复评（2026-11-01）

- 对象：HEAD `324b3244f70b85c86d1150df7a25c4a124335567` + 8 文件 SHA256（§9.1）。
- 评审者/独立性：同一评审员；未参与实现；仅写本记录。
- 结论：**APPROVED（P0=0 / P1=0 / P2=0）**；上轮 P1-1（settings.set 未纳入 mutex）与 P2-1（排队取消）已关闭。

### 9.1 对象 SHA256（8 文件）

```text
0dd97cdb1bc5dd4bbcf98e8e55705ecb16895ba0bee2657815e13701f7dd34ad  specs/changes/2026-11-01-client-command-surface/acceptance.md
31f13a5ff1a1836c6b7ef526664d59e2a3825c0b17f2f8a403032f93f6ab2300  specs/changes/2026-11-01-client-command-surface/spec.md
61a98aab77e23b85b8dbe9996f6d77772aad75b16426d345f171c4c71d9fd6bd  specs/changes/2026-11-01-client-command-surface/intuition.md
6a116e9a5891a3da5f6522a61299c1a9db5b241367f0079c1c6d7791981dbcdd  specs/changes/2026-11-01-client-command-surface/tasks.md
7826bb0aca4eabdc8232df629edad5fb645f424e6319101adaab9f6b576c7a96  specs/changes/2026-11-01-client-command-surface/proposal.md
7ffb616a3e0157d5ff1a2ad54233ca5c3dfd410745321a7711637356bcc9c95b  specs/changes/2026-11-01-client-command-surface/interaction.md
a714551230bb72f4f20de7509676eef71cdfb6de43afdebab0a005e1fbae3979  specs/changes/2026-11-01-client-command-surface/intuition-walkthrough.md
bb5ef9e36a9aedf734d863fe5c0c9f02d6d6c4c58e021b073cbef454f1de6578  specs/changes/2026-11-01-client-command-surface/design.md
```

### 9.2 关闭核对

| 条目 | 状态 | 依据 |
| --- | --- | --- |
| P1-1（上轮新）`settings.set` 未纳入 config-mutex | **RESOLVED** | spec:40 明确 `SETTINGS_SET` apply/publish 必须获取同一 config-mutex（或排队到 turn 结束）；turn 中保存不改当前 override，结束按 snapshot/revision 恢复；design:81/tasks #6/acceptance:87 同步；补「turn 运行中 settings.set」测试。当前代码 `SETTINGS_SET`（ipc.rs:609-670）路径已被纳入设计写作用域，不再与 turn 的 apply/restore 并发。 |
| P2-1 排队 turn 取消语义 | **RESOLVED** | spec:41 冻结：cancel 只作用当前运行 turn；排队 turn 获锁后检查 cancelled，已取消跳过、不产生事件/消息/错误；design:151/tests、acceptance:88 同步。 |
| 首评 P1-1…P1-5/P2 | RESOLVED | D1–D6 勾选；`/model` 手动+recent+effort 诚实降级；Lead 24 文件/728 行例外；Modal 共享放 ui-kit index + 迁移 ui-sessions；预算/enterprise/Tauri 口径统一。 |
| C20 | 通过 | 8 P0、12 INT、P0 映射 8/8、3×3 walkthrough、P0 摩擦 closed、用户测试 UNVERIFIED；plan planned→active 在 tasks #11。 |

### 9.3 门禁

- `python3 scripts/spec-check.py` exit 0（0 errors / 4 warnings，均为其他 draft/in-progress spec）。
- `python3 scripts/docs-check.py` exit 0（0 errors / 0 warnings / 41 files）。
- `python3 scripts/review-check.py` exit 0（35 records；本记录追加后最新 APPROVED）。
- 未验证：真实并发/settings 实测、Tauri 导出、用户测试；不冒充；C20 发布前仍需 ≥5 人 think-aloud。

### 9.4 结论

- **上轮 P1-1/P2-1 已关闭；本轮未发现新 P0/P1/P2**。
- **复评结论：APPROVED（P0=0 / P1=0 / P2=0）**。可置 `approved` 并进入实现（command-surface 的 `/` 命令面、model/permission override、goal/export/feedback；顺序按现有 spec）。
- 关联 spec：`2026-11-01-client-command-surface`；对象 hash §9.1；用户测试仍 UNVERIFIED。
- 复评日期/结论：**2026-11-01 / APPROVED**。
