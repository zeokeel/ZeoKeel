# 2026-11-01-client-workspace-new-session · C19/C20 独立评审

- 状态：本轮评审完成，**CHANGES-REQUESTED**（P0=1 / P1=1 / P2=4；见 §6）。
- 评审对象：HEAD `324b3244f70b85c86d1150df7a25c4a124335567` + 8 文件 SHA256（§0.1）。
- 关联 spec：`2026-11-01-client-workspace-new-session`（唯一 change id）。
- 评审者：`spec-reviewer`（C19/C20 fresh-context 非实现者）。
- 独立性：未参与本 spec 的写作/设计/实现；只读 `ui-sessions`/controller/reducer、`docs/19/26/27`、DSH navigation、模板与 8 文件；唯一写入为本记录。
- 评审日期：2026-11-01。
- 结论：**CHANGES-REQUESTED（P0=1 / P1=1 / P2=4）**；不可置 approved，不进入实现。
- 复评安排：P0/P1 修复后由同一评审员复评；`interaction.md`/`intuition.md` 仍 `planned`，实现开始前改 `active`；用户测试仍 UNVERIFIED。

## 0. 对象与方法

### 0.1 对象 SHA256（8 文件）

```text
ab4da149d1cce5bb0e9d0168bbb0611bf63fdcbef390fc51ee1257bb59dff308  specs/changes/2026-11-01-client-workspace-new-session/acceptance.md
16eed5b819e3338f23aabbdbdd23e2375bb9fdceda2750e519526635dc9d64e1  specs/changes/2026-11-01-client-workspace-new-session/design.md
b215f159dab9d8b74ae43e1fea0a2768f424c924a9d19d1839fe82bcc65b5d43  specs/changes/2026-11-01-client-workspace-new-session/interaction.md
eb1a9b79db4ffe96f8caf3ddc7178db9b0cf61754d171f825f2c775ec3f271ad  specs/changes/2026-11-01-client-workspace-new-session/intuition.md
aec25e7ad7cde4602875f938f312f9b09e853a1c051b3e4e6f8526e727924ac8  specs/changes/2026-11-01-client-workspace-new-session/intuition-walkthrough.md
a9a3c887ef79ba6a643ce82340dc7454b7222f7ed5d33309b00bbf8b2aa2924e  specs/changes/2026-11-01-client-workspace-new-session/proposal.md
ab0779c7022d9d002a468ff593980aa424cfbe9f440f39f1cb28574eea031f52  specs/changes/2026-11-01-client-workspace-new-session/spec.md
293165d335d7dd34051acaaa0f72fbc425639b3a8070c1ff877e610d7fc3b524  specs/changes/2026-11-01-client-workspace-new-session/tasks.md
```

### 0.2 依据

`AGENTS.md` C1–C20；`docs/27` §2.5–§2.8、`docs/19` §2/§11/§11.1、`docs/11` §8.6、`docs/26`（C18）；`research/dsh-ui-modules/navigation.md` §1.1/§2.1；`specs/templates/*`；当前代码（`packages/ui/ui-sessions/web/src/index.tsx`、`apps/desktop/web/src/state/{controller,reducer}.ts`、`apps/desktop/web/src/i18n.tsx`）。

## 1. 事实核验（HEAD `324b324`，path:line）

| # | 事实 | 证据 | 与 spec 的关系 |
| --- | --- | --- | --- |
| F1 | 现有 Workspace 行结构 | `packages/ui/ui-sessions/web/src/index.tsx:580-680`：折叠按钮 + 标题按钮（`selectWorkspace`）+ `⋯`（hover/focus 出现）；行 `.group`；当前 `⋯` 已有 `opacity-0 group-hover:opacity-100 focus-visible:opacity-100` 模式 | design §2.1 在标题与 `⋯` 间加 scoped `+`、复用同类样式可行；按钮与行 title 是 sibling，不会嵌套 button（事实支持） |
| F2 | 全局新建会话 | `apps/desktop/web/src/state/controller.ts:342-365` `createConversation()` 固定 `activeWorkspaceId`、`session.create{workspace_id,title}`、`conversationNewTitle`（i18n `apps/desktop/web/src/i18n.tsx:69/566`） | spec R2 全局行为不变；design `createConversationIn` 复用同 IPC 可行 |
| F3 | active 行 | `ui-sessions/index.tsx:596-645` `workspaceActive = workspace.id === activeWorkspaceId`；workspace 行有 `bg-muted`/字体；行 title 无 `aria-current` | R2 active 行常显/高亮由现有 `workspaceActive` 支持；design 只需条件 class；无需新数据 |
| F4 | `conversation_created` reducer 的流式行为 | `apps/desktop/web/src/state/reducer.ts:522-545`：设置 `activeConversationId=新会话`、`messages=[]`、`messagesState=idle`，**未清/未保留 `streaming`/`streamId`**；`agent_event` 路径（`:682-706`）作用于当前 `state.messages` | **R3 同 workspace 流式「旧会话保留流状态、可切回继续」与现有 reducer 不符**：新会话把 messages 清空，streaming 仍为 true，后续事件会落到新会话上下文（P0-1） |
| F5 | 切换 Workspace 的既有语义 | `docs/26-workspace-and-conversations.md:36`「切换 Workspace = 取消当前活动 turn」、`:182` 同义；`controller.ts:397-413` `selectWorkspace` 在 streaming 时 `chat.cancel` 后 `workspace_selected` | 跨 workspace 确认后取消/新建方向与 C18 一致；同 workspace 不切换、不取消，因此 F4 的问题不能由现有 switch 语义兜底 |
| F6 | 现有 Modal | `ui-sessions/index.tsx:35` 私有 `Modal`（focus trap/Esc/返回焦点）；proposal §9 明确复用、不新增 Dialog | 跨 workspace 确认复用现有 Modal 符合 C17；但「默认焦点取消」需显式 `data-autofocus`/测试（P2-1） |
| F7 | 现有 streaming 状态 | `reducer.ts:42-43/127-128` 全局 `streaming/streamId`；`agent_event` 只按当前 active 会话更新 | 没有 per-conversation 流状态；同 workspace 在流式中切换 activeConversationId 会破坏背景流（P0-1） |
| F8 | 预算模板 | `specs/templates/proposal.md:8`：超过 10 文件或 400 行需拆分；本目录 **8 文件、643 行** | 文件数 8 合规，但 643>400，无 Lead 例外/拆分记录（P1-1） |
| F9 | 交互门禁工具 | `scripts/ui-interaction-gate.py` 与 `pnpm e2e:ui` 已在当前工作树存在（前序 `ui-interaction-verification` 落地） | tasks:5/acceptance 仍写「当前 UNVERIFIED/脚本不确定」，是旧环境文案；实现前应更新（P2-4），不构成行为阻断 |
| F10 | C20 产物 | `interaction.md` 2 个 P0（WSNEW-01/02）+1 P1（03）；`intuition.md` 12 INT，P0 映射脚本 2/2；walkthrough 3×3 矩阵满格（W1–W9）；P0 摩擦 W1/W4/W7 均 `closed`；用户测试 **UNVERIFIED** | C20 机械要求通过（§5）；P0-1 不属于 C20 文件缺陷，而是 D3/现有状态机冲突 |

## 2. C1–C20 合规

| 约束 | 结论 |
| --- | --- |
| C1 个人版优先 | 通过（个人/企业同面，仅本机 workspace） |
| C2 构建期组合 | 通过（无新 slot/包/运行期加载） |
| C3 一切皆包 | 通过（只改 ui-sessions/ui-kit/app，无新包） |
| C4 契约稳定缝 | 通过（不改 `session.create`；仅 UiActions additive） |
| C5 requires | 不适用 |
| C6 弱管控 | 通过（无企业策略） |
| C7 企业 AI | 不适用（无模型调用） |
| C8 角色 | 不适用 |
| C9/C9b | 通过（设计无 unsafe/裸 as） |
| C10 外部输入 | 通过（无新 IPC/DTO；`session.create` 既有校验） |
| C11 生成物 | 通过（无生成物变化） |
| C12 先文档 | 通过（task 0 docs/19；无契约/数据变化） |
| C13/C14/C15 | 不适用 |
| C16 无 spec 不开工 | **基本通过**：D1–D4 已在 proposal §7 `[x]`/spec §6/design/acceptance 记录 Lead 批准；状态 draft 待评审 |
| C17 单一实现 | 通过（复用现有 Modal/样式/`session.create`；不建第二套建会话路径/Dialog） |
| C18 workspace 归属 | **有冲突**：无 workspace 不建、workspace_id 不可变、跨 workspace 取消后新建均正确；但同 workspace 流式新建会清空当前 messages 并保留全局 streaming，违反「旧会话流状态保留」（P0-1） |
| C19 | 本记录（P0/P1 未清不得 approved） |
| C20 | 通过机械要求：planned 文件存在、P0 映射 2/2、3×3 矩阵、P0 摩擦 closed、用户测试 UNVERIFIED、planned→active 在 tasks #6 |

## 3. 交互正确性（重点核对）

| 项 | 核对 | 结论 |
| --- | --- | --- |
| hover + focus 可见、非 hover-only | design §2.1 `group-hover` + `group-focus-within` + `focus-visible:opacity-100`；active 行常显；interaction `SES2-WSNEW-02` | 设计满足；需在实现中保证 `group` 类覆盖新按钮（现有行已有） |
| active 行常显 | design `workspaceActive ? opacity-100`；现有 `workspaceActive` 变量 | 满足 |
| aria-label 含 workspace 名 | design `workspaceNewConversationAria({workspace})`；interaction 元素表 | 满足 |
| 点击不触发行选中/折叠 | design `event.stopPropagation()`；按钮是 title 的 sibling | 设计满足；实现需避免把按钮放进 title button 内 |
| busy 防重复/失败保持选择 | design `busyWorkspaceId` + UI disabled；controller 失败 toast 不 dispatch | 满足 |
| 键盘 Enter/Space | 原生 `<button>`；Tab 顺序 design/tasks | 满足 |
| 无 workspace 不渲染 | 现有 `visibleWorkspaces` 渲染；spec R1/R2；空态提示 | 满足 |
| 跨 workspace 流式确认 | design 复用现有 Modal；spec R3 默认焦点取消；`docs/26:36/182` 切换取消 turn | 语义方向正确；需确保 Modal 默认焦点「取消」（P2-1） |
| 同 workspace 流式 | design 直接 dispatch `conversation_created`，不取消当前流 | **与 reducer 现状冲突（P0-1），见 §6** |

## 4. 预算与可验收性

- **P1-1 行数超阈值**：8 文件（真实：实现 5 + 测试 2 + docs 1）符合 proposal §8 的 8；但目录 **643 行 > 400**，`specs/templates/proposal.md:8` 要求拆分或 Lead 例外；proposal §8 只写「否（单一步骤；≤8）」且按文件数判断，未记录行数例外。建议把「Workspace 行按钮/action」与「流式确认/跨 workspace」拆为两个小 spec，或 Lead 书面例外。
- tasks verify：`pnpm --dir ui-sessions/web test:unit`、`apps/desktop/web test:unit`、`pnpm e2e`、`pnpm -r typecheck/lint/test:unit/build`、`i18n-check` 命令均存在；`pnpm e2e:ui`/`ui-interaction-gate.py` 已落地（F9），但 tasks:5/acceptance 未更新「当前不存在」文案（P2-4）。
- acceptance：P0 `SES2-WSNEW-01/02` 证据路径与 interaction 证据根一致；`e2e/interaction/sessions-workspace-new.spec.ts` 计划明确；用户测试 `usability/` UNVERIFIED。
- 文档：design §7 提到 `KNOWN-LIMITATIONS` 记录 brand/Mod+N 差异，但 tasks 0/预算只有 docs/19，无 KNOWN 写作用域（P2-3）；不影响行为，但 C12 文档回写不完整。

## 5. C20 直觉/交互完整性

- `interaction.md`：2 个 P0（`SES2-WSNEW-01` 多 workspace 建到目标、`SES2-WSNEW-02` 键盘/焦点/aria）+ P1 `03` 流式/全局/无 workspace；`@interaction` 测试落点与证据路径有。
- `intuition.md`：12 INT；脚本核对 P0 映射 **2/2**（无缺失）；每行有可观察判据/反例/验证方式/证据；用户测试 **UNVERIFIED** 明写。
- `intuition-walkthrough.md`：3 persona × 3 top task 矩阵满格（P1/P2/P3 × T1/T2/T3，W1–W9）；每行 4 问；摩擦表 W1/W4/W7 为 P0 且均 `closed`，无 OPEN；修法与 spec/design/tasks 引用一致。
- 结论：C20 机械/走查要求通过；但 P0 `SES2-WSNEW-03` 的流式同 workspace 分支在现有 reducer 下不成立（P0-1 是 spec/实现冲突，不是 C20 文件缺失）。

## 6. 问题清单（P0/P1/P2，文件:行 + 修法 + 状态）

### P0（阻断）

**P0-1 同 workspace 流式中新建会清空当前消息并保留全局 streaming，D3/R3 的“旧流保留/可切回继续”无法按现有 reducer 实现**
- 位置：`spec.md:25-30`（R3 同 workspace 允许、旧会话流状态保留）、`design.md:87-91`（`conversation_created` 只切 activeConversationId、旧 turn 状态保留）、`design.md:59`；代码 `apps/desktop/web/src/state/reducer.ts:522-545`（`conversation_created` 设置 `messages:[]`/`idle`，不清 `streaming`/`streamId`）、`:42-43/127-128`（全局 streaming）、`:682-706`（`agent_event` 只作用于当前 messages）。
- 问题：同 workspace 流式中选择 scoped 新建后，`conversation_created` 清空 messages 并把 activeConversationId 切到新会话，但全局 `streaming=true`/`streamId` 仍是旧流；旧流的 `agent_event` 没有 per-conversation 缓冲/路由，后续事件会按当前 activeConversationId 应用到新会话或 UI 的 `messages` 上，旧会话既无“保留流状态”也无“切回继续”；与 R3/D3 的明确要求冲突。这不是“既有 reducer 行为”，而是现有 reducer 没有该能力。
- 修法（择一，必须在 spec/design/tasks 冻结）：
  (a) **最小可行**：同 workspace 流式中不直接切换 activeConversationId（创建后提示/排队，或流结束再选中），并明确当前会话继续显示；但需改 R3/D3 文案（否则违背 Lead 决策）；
  (b) **改为先取消旧流再新建**（与跨 workspace 一致），但违背 D3「同 workspace 不取消」；
  (c) **实现 per-conversation 流状态**（`streamingByConversation`/消息 buffer + 事件按 conversation_id 路由），这是正确但超出当前 8 文件预算，需拆/加预算并补 reducer/controller 测试；
  (d) 若产品接受“切换后旧流继续但 UI 不显示”，必须显式记录 KNOWN 并禁止“切回继续”的承诺。
  推荐 (c) 或重写 D3 为“同 workspace 也先确认/取消”，并在 `spec.md`/`design.md`/`interaction`/`acceptance` 体现；补「流式中在同 workspace 建会话 → 旧流完成/取消后新会话消息不串」的 app 单测 + e2e。
- 状态：OPEN。

### P1（需批准前修）

**P1-1 目录 643 行 > 400，未拆分/无 Lead 例外**
- 位置：`specs/templates/proposal.md:8`（>10 文件或 400 行需拆）；`proposal.md:69-81`（§8 仅按 8 文件判定「不拆」）；目录 8 文件共 643 行（`wc -l`）。
- 问题：文件数 8 合规，但行数超过模板 400 阈值；proposal §8 的「否（单一步骤；≤8）」只回答文件数，没有记录行数例外/拆分理由。C16/spec 模板要求拆分或 Lead 书面例外。
- 修法：拆为「Workspace 行按钮/action」+「流式同/跨 workspace 确认/状态」两个小 spec；或 Lead 在 §8 记录书面例外（说明 8 文件/643 行仍单一可审），并同步 tasks/acceptance。推荐拆分（流式部分正是 P0-1 风险所在）。
- 状态：OPEN。

### P2（非阻断；建议本轮顺手修）

1. **P2-1 跨 workspace 确认「默认焦点取消」需在现有 Modal 上落实**：design:58/111 说默认「取消」，但 `ui-sessions/index.tsx:35` Modal 的初始焦点是 `[data-autofocus] ?? focusables()[0]`；现有 Modal 无 confirm 场景的 `data-autofocus` 约定。需在确认内容上给「取消」`data-autofocus` 并补 `SES2-WSNEW-03` 键盘断言；否则可能默认聚焦到确认/关闭按钮。
2. **P2-2 `workspaceUnavailable` 逐行禁用语义未落地**：spec R4:36 要求目标不可用按钮禁用并提示；design §2.3:97-99 说「若现状无逐行状态，则以 host 错误提示为准」——即可能不满足 R4。需明确是否新增逐行可用性（预算/数据源）或把 R4 改为「host 错误提示 + 不建到默认」，并补测试。
3. **P2-3 KNOWN-LIMITATIONS 未进 tasks/预算**：design §7:163 与 proposal D2/D4 要求记录 brand 点击/`Mod+N` 后置差异，但 tasks #0/预算只有 docs/19；需加 KNOWN 写作用域或明确由统一回写任务处理（C12）。
4. **P2-4 交互门禁文案过期**：tasks:5/acceptance 仍写「当前 `ui-interaction-verification` 未落地/脚本不确定」，但工作树已有 `scripts/ui-interaction-gate.py` 与 `e2e:ui`（F9）；实现前更新为实际命令/UNVERIFIED 口径，避免误报不可执行。

## 7. 结论与后续

- **结论：CHANGES-REQUESTED（P0=1 / P1=1 / P2=4）**；不可置 `approved`，不得进入实现。
- P0-1 是 D3/R3 与现有 reducer 的硬冲突：同 workspace 流式新建无法“保留旧流并可切回”；需在 spec/design 冻结状态机方案（推荐实现 per-conversation 流状态或改 D3），否则 R3 不可验收。
- P1-1 行数超模板阈值，需拆分或 Lead 例外；建议与 P0-1 的流式状态改造一起拆。
- C20 机械要求（P0 映射 2/2、3×3、P0 摩擦 closed、用户测试 UNVERIFIED）通过；P0-1 修复后需同一评审员复评，且 `interaction.md`/`intuition.md` 仍为 planned，实现开始前改 active 并补证据。
- 字段齐全：评审对象（HEAD+8 hash）、评审者、独立性、结论、P0/P1/P2、复评安排、关联 spec（`2026-11-01-client-workspace-new-session`）。
- 复评日期/结论：**2026-11-01 / CHANGES-REQUESTED**（首轮；P0=1、P1=1、P2=4）。

## 7. 复评（第二轮，2026-11-01）

- 对象：HEAD `324b3244f70b85c86d1150df7a25c4a124335567` + 8 文件 SHA256（§7.1）。
- 评审者/独立性：同一评审员；未参与实现；仅写本记录。
- 结论：**CHANGES-REQUESTED（P0=0 / P1=1 / P2=0）**；scoped streaming 修复通过，但全局/header/⌘K 新建入口在 streaming 时仍可触发同一 reducer 问题。

### 7.1 对象 SHA256（8 文件）

```text
0488e6e12fe5fc8cebc29aafc5abf09cc0c05f4a152d8ad8eb1fd540a5ca57c3  specs/changes/2026-11-01-client-workspace-new-session/tasks.md
05166c602230c0e3fa5e268703e40a775eb6391770098c1d6cb4be6049ff3fba  specs/changes/2026-11-01-client-workspace-new-session/acceptance.md
1113cd3d242229775b5c6e072c0fa5c4b3f34bb96b9168d75e0ad0a7eafe2231  specs/changes/2026-11-01-client-workspace-new-session/spec.md
1a795fe4843a7d59f37ef5322e0cc3ae6e0e71d045fb5b1848f6059ce0d5f13c  specs/changes/2026-11-01-client-workspace-new-session/proposal.md
1f06ef0789fae746dd0bc1063e223d58adb58dbab9df319b627278bd921fe428  specs/changes/2026-11-01-client-workspace-new-session/design.md
34447fad0a63ae774fe13fe37436113f158e6ee18b16002cdae8b1e71394c4df  specs/changes/2026-11-01-client-workspace-new-session/intuition-walkthrough.md
79399565765e0bc8f1f1bde2c0390878b6ade3c875ebcce0de70f21cee812709  specs/changes/2026-11-01-client-workspace-new-session/intuition.md
e6452b41f94fecf85cde66e3a70cbedd37c5f7e611006c86c2d4fe2b5d6f8f6e  specs/changes/2026-11-01-client-workspace-new-session/interaction.md
```

### 7.2 首评条目关闭

| 条目 | 状态 | 复评依据 |
| --- | --- | --- |
| P0-1 同/跨 workspace 流式路径 | **RESOLVED（仅 scoped 入口）** | proposal D3 修订、spec R3/design §2.1/§2.2/tasks #2/#3 删除同/跨 workspace 流式确认路径；streaming 时所有 Workspace 行 `+` 禁用（aria-disabled/tooltip），不调用 `createConversationIn`，因此 scoped 路径不再触发 `conversation_created` 清空/串消息；walkthrough W7/W8 与 acceptance P0-1 已同步。 |
| P1-1 8 文件/643 行 | **RESOLVED（Lead 例外）** | proposal §8 记录 Lead 书面批准 8 文件/约 643 行；P2-3 追加 KNOWN 后 9 文件（≤10），预算/tasks 一致。 |
| P2-1 无确认 Modal | **RESOLVED** | 流式确认路径删除，无 data-autofocus 需求；spec/design/interaction 无确认弹窗。 |
| P2-2 R4 unavailable | **RESOLVED** | spec R4:37/design §2.3 统一为「host `session.create` 错误为准；本地仅明显无效（空 path/id）禁用」，不新增逐行探测。 |
| P2-3 KNOWN | **RESOLVED** | tasks #0 写作用域/预算加入 `KNOWN-LIMITATIONS`（brand/Mod+N、单流模型差异）。 |
| P2-4 门禁文案 | **RESOLVED** | tasks/acceptance/interaction 改为已落地 `pnpm e2e:ui --grep "workspace-new"` / `pnpm e2e:ui:gate`。 |

### 7.3 复评新发现

**P1-1 全局/header/⌘K 新建入口在 streaming 时仍可用，仍会走 `conversation_created`**
- 位置：`spec.md:17-25`（R2 全局/header/⌘K 保持现状）、`design.md:90-95`（旧 `createConversation()` 不变；全局/header/⌘K 仍走全局）；代码 `apps/desktop/web/src/state/controller.ts:342-365`（`createConversation` 无 streaming guard）、`reducer.ts:522-545`（`conversation_created` 清空 messages、切 activeConversationId、保留全局 streaming/streamId）。
- 问题：本次只在 Workspace 行 scoped `+` 上禁用；侧栏顶部/header `+` 与 ⌘K `new-conversation` 在 streaming 时仍可点击并调用同一 `createConversation`，仍会清空当前消息/切换 activeConversationId，出现与 P0-1 相同的串消息路径。R3 的「流式期间不切换/不串消息」只在 scoped 按钮成立，用户仍可从其它入口触发。
- 修法（择一，必须冻结）：(a) streaming 时同时禁用/拦截侧栏顶部、header `+`、⌘K `new-conversation`（在 `createConversation` 入口统一 guard，toast「当前正在生成，停止后可新建」），与 scoped 行为一致；(b) 若坚持「全局行为不变」，必须在 spec/KNOWN 明确这是已知 pre-existing 风险并另 spec 修复，不能以本 spec 的「不串消息」验收通过。推荐 (a)，同时补全局三入口的 streaming 单测/e2e。
- 状态：OPEN。

### 7.4 C20 与门禁

- C20：12 INT、2 个 P0、P0 映射 2/2、3×3 walkthrough（W1–W9）、P0 摩擦 W1/W4/W7 closed、用户测试 **UNVERIFIED**；本轮改动未破坏。
- `spec-check` exit 0（0 errors / 4 warnings，均为其他 draft/in-progress spec）；`docs-check` exit 0（41 files）；`review-check` exit 0（35 records）。
- 未验证：真实 streaming/global 入口实测、≥5 人用户测试；不冒充。

### 7.5 结论

- **复评结论：CHANGES-REQUESTED（P0=0 / P1=1 / P2=0）**；scoped P0-1 已修，但全局/header/⌘K 流式入口仍是同一 reducer 风险，不可置 approved。
- 修复 P1-1 后同一评审员复评；关联 spec `2026-11-01-client-workspace-new-session`；用户测试 UNVERIFIED，不得声称符合直觉。
- 复评日期/结论：**2026-11-01 / CHANGES-REQUESTED**（P0=0、P1=1、P2=0）。

## 8. 第三轮复评（2026-11-01）

- 对象：HEAD `324b3244f70b85c86d1150df7a25c4a124335567` + 8 文件 SHA256（§8.1）。
- 评审者/独立性：同一评审员；未参与实现；仅写本记录。
- 结论：**APPROVED（P0=0 / P1=0 / P2=0）**；上轮 P1-1（全局/header/⌘K 未受 streaming guard）已关闭。

### 8.1 对象 SHA256（8 文件）

```text
0539ae2c61fd6f09ef7a54fcac3b16e61cc67436ce0d41c83809ccc37fadc344  specs/changes/2026-11-01-client-workspace-new-session/tasks.md
18e4c441d7bafec320ffb16b491fe2b4bfc974b6d7e32243f4d2847cbf48d6d5  specs/changes/2026-11-01-client-workspace-new-session/acceptance.md
30eacb52911cb56246190432b65b4b08d444810c92dc1e60bfe842a9de7dcdf3  specs/changes/2026-11-01-client-workspace-new-session/proposal.md
331be714769451f12eb023fda729584459b255757a951b487fd14108be8d3a4d  specs/changes/2026-11-01-client-workspace-new-session/intuition.md
8bff9b9b1df471c0d83892d6c7844523d7b600e8037b813f28fb965b9e2586f8  specs/changes/2026-11-01-client-workspace-new-session/interaction.md
ba542661e302729e35d3acb6eeaf84c72a775354cf572fdfe4a9464459cf8450  specs/changes/2026-11-01-client-workspace-new-session/spec.md
e012523ea78dbd6f09d9e3050f6f5e9d9b5f26ef290965a4a0f92c2ef5f17475  specs/changes/2026-11-01-client-workspace-new-session/design.md
e8344206faa0d1c8b6ca2a517efd3d25ff42abf2ccd67aff7d525de54d0e1dc8  specs/changes/2026-11-01-client-workspace-new-session/intuition-walkthrough.md
```

### 8.2 关闭核对

| 条目 | 状态 | 依据 |
| --- | --- | --- |
| P1-1（上轮新）全局/header/⌘K 未受 guard | **RESOLVED** | spec R2:17-24 / R3:30-33：三入口统一 streaming guard；design:64/91-97：`createConversation`/`createConversationIn` 共用 guard helper，streaming 时 toast+return，不调用 IPC/不切 active；UI scoped/header/⌘K 均禁用；tasks #3/#4、acceptance:44/54、interaction `SES2-WSNEW-03` 同步；补三入口/恢复/不串消息单测+e2e。`conversation_created`（reducer:522-545）在 streaming 时不再被任何新建入口触发，scoped/global/⌘K 一致。 |
| 首评 P0-1（scoped） | RESOLVED | streaming 时 scoped 行 `+` 禁用；删除同/跨 workspace 确认 Modal；不切 active/不串消息；D3 修订与 DSH 差异记录。 |
| P1-1 预算 | RESOLVED（Lead 例外） | Lead 2026-11-01 书面批准 8 文件/约 643 行；P2-3 增 KNOWN 后 9≤10；proposal §8/tasks:32/acceptance:65 一致。 |
| P2-1…P2-4 | RESOLVED | 无确认 Modal；R4/design 统一 host 错误为准；KNOWN 进 tasks/预算；门禁命令改为已落地 `pnpm e2e:ui`/`e2e:ui:gate`。 |
| C20 | 通过 | 2 P0、12 INT、P0 映射 2/2、3×3 walkthrough W1–W9、P0 摩擦 closed、用户测试 UNVERIFIED；planned→active 在 tasks #6。 |

### 8.3 门禁

- `python3 scripts/spec-check.py` exit 0（0 errors / 4 warnings，均为其他 draft/in-progress spec）。
- `python3 scripts/docs-check.py` exit 0（0 errors / 0 warnings / 41 files）。
- `python3 scripts/review-check.py` exit 0（35 records；本记录追加后最新 APPROVED）。
- 未验证：真实 streaming/三入口实测、≥5 人用户测试；不冒充。

### 8.4 结论

- **上轮 P1-1 已关闭；本轮未发现新 P0/P1/P2**。
- **复评结论：APPROVED（P0=0 / P1=0 / P2=0）**。可置 `approved` 并进入实现（side-panels 之后、activity 之前的 Workspace 行 scoped 新建；三入口 streaming guard）。
- 关联 spec：`2026-11-01-client-workspace-new-session`；对象 hash §8.1；用户测试仍 UNVERIFIED。
- 复评日期/结论：**2026-11-01 / APPROVED**。
