# 2026-10-30 · Agent rig 实现对抗性验证

- **评审对象**：`packages/client/agent-rig`、`llm-rig`、`packages/client/tool-*`、`crates/zk-kit-{tools,jobs,approval,questions,events,session,subagent}`、desktop approval/question IPC、`e2e/agent-rig.spec.ts`、`scripts/agent-e2e.sh`、`docs/38`、`research/dsh-agent-inventory.md`、`specs/changes/2026-10-30-agent-dsh-parity`
- **评审者**：audit-arch（独立只读；仅写本记录）
- **结论**：CHANGES-REQUESTED
- **关联 spec**：`specs/changes/2026-10-30-agent-dsh-parity`
- 证据日期：2026-10-30；本记录为唯一新增文件，未改产品代码/spec。

## 1. 结论

**CHANGES REQUIRED（CHANGES-REQUESTED）。**

rig 循环/审批/ask-user/工具包**happy path 能跑**，但离 spec 的“DSH 全能力/全工具矩阵/真实 e2e”有硬缺口：

- spec 仍 `draft`；`docs/generated/agent-tool-matrix.md`、`docs/33-agent-prompts.md` 不存在；DSH 矩阵无 diff=0 证据。
- 工具矩阵缺 `call-timeout`、`agent-team`、`subagent-control`、`pwsh-persistent`、`web.search`、`tool-cordis`；`docs/38` 自相矛盾（同一工具既“已实现”又“待实现”）。
- `web.fetch` SSRF 只查首跳 host 前缀，未解析 DNS/redirect 后的私网；`172.17–31` 漏检；响应 `text()` 无 1MB 上限。
- `create_worktree` 把 path/branch 直接拼进 shell 命令，未转义/未限制工作区，命令注入/逃逸。
- 取消仅流式检查标志；非流式/工具执行中不可取消；审批无超时；max turns 无 `AGENT_MAX_STEPS`。
- rig hooks/extractor/vector/loader 未接，llm-rig 仅 OpenAI；与 spec 的 rig 能力矩阵不符。
- Playwright 只有 1 个测试（write approval + ask-user），无多步/拒绝/取消/最大步数/未知工具/provider 设置；工具测试多为 package_metadata。
- `docs/38` 把 terminal/schedule/jobs/subagent 等标“已实现”，实际是薄壳/部分实现；无矩阵/状态诚实。

## 2. 证据命令与结果

| 命令 | 结果 |
| --- | --- |
| `bash scripts/agent-e2e.sh` | **PASS**：1 passed（真实 personal serve + fake upstream；fs.write 审批 + ask-user） |
| `cargo test --workspace --offline` | **PASS** |
| `cargo clippy --workspace --all-targets --offline -- -D warnings` | **PASS** |
| `pnpm --dir apps/desktop/web test` | **PASS**：11 files / 45 tests |
| `pnpm --dir apps/desktop/web typecheck` | **PASS** |
| `zk validate` | **PASS**：100 packages scanned |
| `docs/generated/agent-tool-matrix.md` | **不存在** |
| `docs/33-agent-prompts.md` | **不存在** |
| `spec.md` 状态 | `draft`，非 approved；C16 未满足 |
| `e2e/agent-rig.spec.ts` | 仅 1 个测试；无拒绝/取消/多步/最大步数/未知工具/设置 provider |
| `tool-*` 测试 | 多数仅 `package_metadata` conformance；无每工具正/负矩阵证据 |
| `docs/38` | “已实现”列表与“待实现”列表重叠（terminal/jobs/schedule/goal/ralph/subagent 等） |

## 3. P0（阻塞）

### P0-1 · spec 未 approved + DSH 工具矩阵/提示词文档缺失

- `specs/changes/2026-10-30-agent-dsh-parity/spec.md` 状态 `draft`；C16 要求 approved spec 才实现。
- spec/acceptance/tasks 要求 `docs/generated/agent-tool-matrix.md` 与 `docs/33-agent-prompts.md`；两文件均不存在。
- 无矩阵 diff=0，无法证明“所有 DSH 工具都有对应包/角色/状态/测试”。

**修法**：先 approved spec；生成 `docs/generated/agent-tool-matrix.md`（DSH 包→ZeoKeel 包/角色/状态/测试）并要求 diff=0；补 `docs/33-agent-prompts.md` 来源/替代；acceptance 绑定文件与命令。

### P0-2 · 工具矩阵缺口与“已实现”虚假声明

- spec §3 要求：`fs.read/write/edit/search`、`bash`、`bash-persistent`、`pwsh`、`pwsh-persistent`、`terminal`、`web.fetch`、`web.search`、`todo`、`ask-user`、`present`、`skill`、`schedule`、`jobs`、`goal`、`ralph`、`subagent`、`subagent-control`、`workflow`、`worktree`、`session-query`、`call-timeout`、`working-directory`、`workspace-dependencies`、`tool-cordis`、`agent-team`。
- 仓库缺：`tool-call-timeout`、`tool-agent-team`、`tool-subagent-control`、`tool-pwsh-persistent`、`tool-web-search`、`tool-cordis`（无禁用/替代说明）。
- `docs/38` §4 同时把 terminal/jobs/schedule/goal/ralph/subagent 列“已实现”和“待实现”；`terminal` 的 open 只是 `pwd`，`jobs` 仅进程内，`schedule` 未接真实调度。
- 这使“全矩阵/每工具正负测试”不能成立。

**修法**：逐项补包或明确 `DISABLED/UNSUPPORTED` 与原因并进矩阵；删除 docs/38 重叠；每工具至少 1 正/1 负测试；`zk capability --check` 绑定。

### P0-3 · `web.fetch` SSRF 可绕过

- `packages/client/tool-web-fetch/src/lib.rs`：只校验初始 URL host 字符串前缀；未解析 DNS 后校验 IP；`reqwest` redirect 默认跟随（limited 3），重定向目标未复检；`172.16.0.0/12` 只查 `172.16.` 前缀，`172.17–31` 漏检；IPv6 `::1` 判断可能因 `host_str()` 无括号而失效；`response.text()` 无 1MB 上限。
- 攻击者可控 URL 可访问 `169.254.169.254`（云元数据）之外的私网/重定向私网，或大响应内存 DoS。

**修法**：解析并 pin IP（DNS rebinding 防护）、每次 redirect 复检 scheme/host/IP、完整私网/ULA/link-local 段、拒绝 userinfo、响应 `content_length`/分块 1MB 硬上限；补私网/重定向/DNS 负向测试。

### P0-4 · `create_worktree` 命令注入/工作区逃逸

- `packages/client/tool-worktree/src/lib.rs`：`format!("git worktree add -- {} {}", path, branch)` 直接拼 shell；path/branch 含空格/`;`/`$()` 可注入；未走 `safe_path`，可 `../../outside`/绝对路径创建仓库外 worktree。
- `run_bash` 允许任意命令是弱管控边界，但工具自己的参数必须安全；spec §3 要求工作区内/显式 allow_outside。

**修法**：path 用 `safe_path`，branch 校验 `[A-Za-z0-9._/-]`；用 `git worktree add` 的 argv 直接 spawn（不拼 shell）或严格 quote；补注入/逃逸负向。

## 4. P1（应修）

1. **取消语义不足**：`agent-rig` 仅流式循环检查 `cancelled`；非流式 `agent.prompt().await` 与工具执行中不可取消；spec §2 要求取消终止；补 CancellationToken/工具前后检查/取消 e2e。
2. **审批无超时**：`ApprovalGate.decide` 同步 bool，无 timeout；`kit.approval` 默认 DenyAll 但 UI 无通道会永久阻塞；补超时即拒绝、拒绝路径 e2e。
3. **max turns 错误码**：`default_max_turns(MAX_STEPS)` 触发 rig 错误，未映射 `AGENT_MAX_STEPS`；补 e2e。
4. **rig 能力未释放**：hooks/extractor/vector/loader 未接；`llm-rig` 仅 OpenAI；spec §0/§5 的 capability 包/`CAPABILITY_UNAVAILABLE` 未全落。
5. **工具实现薄/部分**：terminal open 只 pwd；jobs 仅进程内（重启丢失）；schedule 无真实调度；workflow 只线性无 DAG；ralph 未 fresh context；subagent 共享 context；与 DSH 语义/“fresh agent”不符。
6. **上下文隔离**：subagent/workflow/ralph 用同一 `TurnContext`/runner，未隔离 workspace/conversation；补子会话/预算/深度 e2e。
7. **Playwright 覆盖不足**：仅 `fs.write` 审批 + ask-user；无多步 fs→edit→bash、拒绝、取消、最大步数、未知工具、provider 设置；补矩阵。
8. **每工具测试不足**：大量 `tests/conformance.rs` 只 `package_metadata`；补正/负/workspace 逃逸/超时/输出上限。
9. **C17 重复实现**：`tool-fs` 自带 `read_confined` 与 `zk-kit-tools::safe_path` 重复；统一到 kit。
10. **docs/38 与实现不符**：“待实现”包含已存在工具；“已实现”包含薄壳；补状态列/矩阵。
11. **prompt parity 未证**：默认 preamble 有 DSH identity 片段，但 `docs/33-agent-prompts` 缺失、workspace instructions 无截断/去重细节证据；补来源/替代。
12. **事件实时性**：ToolCallStarted/Finished 在整轮结束后才发；工具执行中 UI 无进度；spec 事件语义未满足。

## 5. P2（建议）

- `docs/generated/agent-tool-matrix.md` 由 `zk capability` 生成并进 docs-check。
- subagent-control 补 send/interrupt/list，agent-team 实验标记。
- tool-cordis 明确禁用/替代（C2 运行期不加载）。
- `web.fetch` 用 reqwest async + 超时/大小限制。
- `jobs` 持久化到会话目录；`schedule` 接 cron/worker。
- `agent-e2e` 用 fake upstream 脚本覆盖多 turn；增加 cancel 场景。
- 工具名与 DSH 对齐（`web.fetch` vs `web_fetch`、`create_worktree` vs `worktree`）并进矩阵。

## 6. 对照 DSH 矩阵

| 能力 | 状态 |
| --- | --- |
| 多步循环/streaming/text delta | 基本有（e2e 只单工具） |
| 审批 fail-closed/拒绝 | 有 happy path；拒绝/超时无 e2e |
| 取消 | 仅流式检查；非流式/工具中不可 |
| max turns/context isolation | 未映射错误码/未隔离 |
| prompt parity | 片段有；来源文档缺失 |
| 全工具矩阵 | 缺 6 类；docs/38 矛盾 |
| ask-user | 有；e2e 1 条 |
| subagent/workflow/ralph | 薄实现，共享 context；无 control/fresh |
| jobs/persistent/terminal/goal/schedule/session-query | 部分；jobs 内存、terminal 薄、schedule 无调度 |
| rig 能力（hooks/extractor/vector/loader） | 未释放 |
| UI e2e 真实度 | 真实 serve + fake；覆盖过窄 |
| 安全 | web SSRF/response 上限、worktree 注入/逃逸 |
| C16/C17 | spec draft；tool-fs 重复路径逻辑 |
| 证据/矩阵 | 缺 generated matrix、docs/33 prompts |

## 7. 明确未验证

- 真实模型 provider（OpenAI/DeepSeek/Anthropic）与企业 gateway Responses tools：仅 fake/OpenAI 兼容。
- Windows/PowerShell、macOS 下终端/持久 shell。
- 真实 subagent 隔离/预算/深度控制（仅 fixture/薄 runner）。
- 真实 Tauri IPC（e2e 用 `--serve`；IPC 审批/问题仅通过 serve 路径）。
- 真实工具安全红队（SSRF/命令注入仅代码审查，无负向 e2e）。

## 8. 结论

- **P0：4**：spec/矩阵文档、全矩阵缺口、web SSRF、worktree 注入。
- **P1：12**：取消、审批超时、max turns、rig 能力、薄工具、上下文隔离、Playwright 覆盖、每工具测试、C17、docs/38、prompt parity、事件实时性。
- **M 实现不能 APPROVED**；修 P0/P1，补真实矩阵/负向 e2e 后复评。

- 关联 spec：`specs/changes/2026-10-30-agent-dsh-parity`。
- 评审结论：**CHANGES-REQUESTED**

---

## 9. 第二轮复评（R2，2026-10-30）

> 对象：R1 修复后的 agent rig 实现。只读复跑；未改产品代码/spec（仅本记录）。
> 仅列剩余 P0/P1。

### 9.1 结论

**CHANGES REQUIRED（CHANGES-REQUESTED）**

- **P0：1**：`web.fetch` 仍可被 DNS rebinding 绕过（先解析检查、请求时再解析）。
- **P1：6**：`review-check` 红、agent-e2e 2 flaky false green、acceptance 仍写“3 条”/未完成项与 matrix 冲突、rig 多 provider/hooks/vector/loader 未落、Playwright 仍缺 max/unknown/provider、`call-timeout` 无独立包/角色。
- **已关闭**：spec approved；矩阵 `AGENT-TOOL-MATRIX.md` 生成且 `--check` 绿；docs/39 prompts；SSRF 私网/redirect/1MB 初检；worktree argv/safe_path/branch；实时 ToolCallStarted；取消非流式；审批 timeout。

### 9.2 命令与结果

| 命令 | 结果 |
| --- | --- |
| `bash scripts/agent-e2e.sh` | **EXIT 0，但 4 passed + 2 flaky**：write approval、bash 各重试 1 次；不是干净 6 passed |
| `cargo test --workspace` / clippy | PASS |
| `pnpm --dir apps/desktop/web test` / typecheck | PASS（45 tests） |
| `zk validate` | PASS（100 packages） |
| `python3 scripts/agent-tool-matrix.py --check` | PASS |
| `python3 scripts/review-check.py` | **FAIL**：`proposal.md` missing `- 评审：`（C19） |
| `docs-check` / `spec-check` | PASS |
| `docs/39-agent-prompts.md` | 存在 |
| spec 状态 | `approved` |

### 9.3 剩余 P0

#### P0-1 · `web.fetch` DNS rebinding 仍可绕过

- `host_allowed` 在请求前 `to_socket_addrs` 校验解析 IP；但 `reqwest` 发送时会再次解析 DNS。攻击者 DNS 可在两次解析间返回公网→私网；redirect 策略同样只做一次预检。
- 现有测试只覆盖静态私网 IP，不覆盖 DNS rebinding/多 A 记录/解析竞态。
- 修：解析后 pin IP（自定义 resolver/连接目标），或对实际连接 IP 校验；补 rebinding/多地址负向；矩阵状态改回 partial。

### 9.4 剩余 P1

1. **`review-check` 红**：`specs/changes/2026-10-30-agent-dsh-parity/proposal.md` 缺 `- 评审：` 行（C19），当前 docs/review 门禁失败；补字段。
2. **agent-e2e false green/flaky**：6 个测试中 2 个 flaky（write approval/bash），重试才过；脚本仍 `agent e2e ok`、exit 0。补根因（会话切换/审批 race）或 failOnFlaky，验收不得写 6 passed。
3. **acceptance 自相矛盾/过期**：
   - `acceptance.md` 仍写“当前 3 条 agent e2e”，实际 6；
   - “仍未完成”列 `call-timeout 策略包`，但 matrix 把 `call-timeout` 标 implemented（agent-rig）；
   - 状态 `in-progress` 与“已落地证据/独立评审 APPROVED”混用；需校正。
4. **rig 能力仍缺**：`hooks/extractor/vector/loaders/多 provider` 未落；`llm-rig` 仍仅 OpenAI；acceptance 自述未完成，但 spec §0/§5 要求 capability 包/`CAPABILITY_UNAVAILABLE`。
5. **Playwright 仍窄**：6 条覆盖审批/拒绝/取消/ask/bash/present/多步，但无“最大步数、未知工具、provider 设置、workflow/subagent”真实 UI；spec §5 的 Playwright 矩阵未全。
6. **`call-timeout` 无独立包/角色**：matrix 用 `role "-"`/package agent-rig；spec §3 要求每工具独立包/角色；补 `tool-call-timeout` 或明确豁免并写入矩阵。
7. **C17 重复**：`tool-fs` 自带 `read_confined` 与 `zk-kit-tools::safe_path` 仍重复（若未统一）。
8. **e2e retry=1 掩盖 flaky**：Playwright 配置 retry=1 让 2 条失败被吞；应记录 flaky 为红或修复。

### 9.5 剩余 P2

- matrix 中 `web.fetch` 仍标 “SSRF guarded”，需改 `partial` 直到 IP pin。
- `web_search` 的 `ZEOKEEL_SEARCH_URL` 仅 https 校验，未禁私网/自定义 endpoint（本地配置，P2）。
- `docs/38` 的“每个工具包 behavior”与部分 conformance-only 测试需对齐。
- 工具名与 DSH 对齐（`web.fetch`/`web_search`、`create_worktree`/`worktree`）。
- `acceptance` 的“6 scenarios”与 Playwright 实际 test 数保持一致。
- 生成物 `docs/generated/AGENT-TOOL-MATRIX.md` 的 `@generated` 检查进 CI。

### 9.6 已关闭/验证

- spec approved；matrix/prompts 文档存在；matrix --check 绿。
- SSRF 私网/redirect/1MB 初检、worktree argv/safe_path/branch、实时 ToolCallStarted、取消非流式、审批 timeout、6 条 e2e 场景。
- cargo/clippy、web tests/typecheck、zk validate 绿。

### 9.7 结论

- **P0：1（SSRF DNS rebinding）；P1：6 组**。
- **agent 实现仍不可 APPROVED**：修 P0/P1 + review-check 绿 + e2e 无 flaky 后复评。

- 关联 spec：`specs/changes/2026-10-30-agent-dsh-parity`。
- 复评结论（第二轮）：**CHANGES-REQUESTED**

---

## 10. 第三轮复评（R3，2026-10-30）

> 对象：R2 修复后的 agent rig 实现。只读复跑；未改产品代码/spec（仅本记录）。
> 仅列剩余 P0/P1；按用户目标“all DSH tools + all rig capabilities”评估。

### 10.1 结论

**CHANGES REQUIRED（CHANGES-REQUESTED）**

- **P0：0**：SSRF/worktree/proposal/e2e 已关闭。
- **P1：4 组（阻断用户目标）**：rig 全能力未落、Playwright 矩阵缺项、tool/capability 包角色缺口、acceptance 自相矛盾。故仍不能 APPROVED。
- **e2e 已干净 6 passed**；cargo/clippy/docs/spec/matrix 绿；**review-check 因当前最新评审为 CHANGES 而红**（审批前正常，但验收不能写绿）。

### 10.2 已关闭

- **SSRF**：`resolve_public` 单次解析后 `reqwest .resolve(host, addr)` pin；手动 redirect（≤3）逐跳 resolve/pin；1MB 上限；私网/redirect 测试。
- **worktree**：argv-only、`safe_path`、branch 白名单。
- **proposal**：已有 `- 评审：docs/reviews/...`。
- **e2e**：`retries: 0`；实测 **6 passed (16.2s)**，无 flaky。
- **acceptance**：状态改为 `implementation-in-progress`；e2e 6 已写入（但“当前 3 条”仍残留，见 P1）。

### 10.3 剩余 P1（阻断目标）

1. **rig 全能力未落**：`hooks`、`vector`、`loaders`、多 provider 仍未实现；`llm-rig` 仅 OpenAI。用户目标明确“all rig capabilities”，spec §0/§5 的 capability 包/`CAPABILITY_UNAVAILABLE` 未齐；acceptance 自述“仍未完成”。这直接阻断 APPROVED。
2. **Playwright 仍缺 max turns/unknown tool/provider 设置**：现有 6 条覆盖审批/拒绝/取消/ask/bash/present/多步，但 spec §5 要求“工具+审批+provider 设置”与 max/unknown；缺项未补。
3. **工具/能力包角色缺口**：
   - `call-timeout` 仍只在 agent-rig env，matrix role 为 `-`；spec §3 要求每工具独立包/role（或明确豁免）。
   - `schedule` 仅定义无 worker；`workflow` DAG 自述未完成；`subagent` 无 fork/异步 control；`agent-team` 实验禁用。目标“all DSH tools”未闭环。
4. **acceptance 自相矛盾**：仍写“当前 3 条 agent e2e”（实际 6）；“仍未完成”列 `workflow DAG`/`call-timeout` 与 matrix 的 implemented/partial 状态不一致；验收清单未勾选但已写“已落地证据”。

### 10.4 非阻塞 P2

- `review-check` 因本轮 CHANGES 红是审批流程状态，不是代码缺陷；APPROVED 后应转绿。
- matrix `web.fetch` status 仍可写 implemented（DNS pin 后）。
- `docs/38` “每个工具 behavior”与少量 conformance-only 测试对齐。
- `web_search` 的 `ZEOKEEL_SEARCH_URL` 仅 https，未禁私网 endpoint（本地配置）。
- 工具名 DSH 对齐（`web.fetch`/`web_search`、`create_worktree`/`worktree`）。

### 10.5 证据

| 命令 | 结果 |
| --- | --- |
| `bash scripts/agent-e2e.sh` | **PASS：6 passed (16.2s)，retries=0，无 flaky** |
| `cargo test --workspace` / clippy | PASS |
| `pnpm --dir apps/desktop/web test` / typecheck | PASS（前轮 45 tests） |
| `python3 scripts/agent-tool-matrix.py --check` | PASS |
| `docs-check` / `spec-check` | PASS |
| `review-check` | **FAIL**：latest review CHANGES-REQUESTED (need APPROVED)；APPROVED 前预期红 |
| `zk validate` | PASS（100 packages） |

### 10.6 结论

- **P0：0；P1：4 组**（rig 能力、Playwright、工具/包角色、acceptance）。
- **仍不能 APPROVED**：用户目标“all DSH tools + all rig capabilities”未满足；修 P1 后复评。
- **review-check 当前红**属于“最新评审 CHANGES”的流程状态，修完并 APPROVED 后应变绿。

- 关联 spec：`specs/changes/2026-10-30-agent-dsh-parity`。
- 复评结论（第三轮）：**CHANGES-REQUESTED**
