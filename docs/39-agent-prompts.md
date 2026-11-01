# 39 · Agent prompts（DSH 参考）

> DSH 只作参考；提示词来自本地安装 `@deepseek-ai/dsh` 0.2.1-rc.2（MIT）。逐字清单见 `research/dsh-agent-inventory.md` §3。

## 1. 已移植（逐字/近似）

| 来源 | 文本 | 移植位置 |
| --- | --- | --- |
| `dsh-system-prompt/lib/index.js:229` | `You are an AI agent powered by DeepSeek Harness.` | `agent-rig::default_preamble` |
| `dsh-web-app/cordis.patch.yml:19` | `You are a coding agent powered by the {{model}} model.` | 同上（当前用 DeepSeek 模型名） |
| `dsh-agent-instructions` | AGENTS.md/CLAUDE.md 基线 | `agent-rig::workspace_instructions`（≤64KB） |
| `dsh-tool-web:256/731` | web 不可信内容/引用 URL | `tool_guidance` + web tool |
| `dsh-tool-bash:228` | 删除/移动前验证绝对路径；cwd | `tool_guidance` |
| `dsh-tool-fs` | read-before-write | `tool_guidance` |
| `dsh-tool-ask-user` | 问题工具语义 | `tool-ask-user` |

## 2. 未逐字移植/替代

- DSH profile persona 由远端/配置注入，本地 bundle 不完整；当前固定为“DeepSeek model”。
- DSH compaction、goal/ralph/subagent/teams 的独立提示段未全量移植；当前只保留 identity/persona/workspace/tool guidance，另加 ZeoKeel 自有的客户端 UI 交互段（§3）与直觉交互段（§3.1，均非 DSH 来源）。
- 工具描述来自各 ZeoKeel 包 `ToolSpec.description`，不逐字复制 DSH（避免与实现不符）。

## 3. 客户端 UI 交互段（2026-11-01）

> 来源：ZeoKeel 自有提示词（**非** DSH 逐字移植），依据 `specs/changes/2026-11-01-ui-interaction-verification/spec.md` R7；交互规范真源是 `docs/11` §8.6、`docs/19` §11、`docs/27` §2.5/§2.6，本文件只记录提示词原文与落位，不复制规范。

- 位置：`agent-rig::default_preamble`（`packages/client/agent-rig/src/lib.rs`）。`RigAgent::run_rig` 用 `self.preamble + tool_guidance + workspace_instructions` 组装每个 turn 的 preamble，所以改默认 preamble 会对所有 turn 生效（显式 `with_preamble` 覆盖仍优先）。
- 插入点：DSH identity/persona 两行与 ZeoKeel 运行说明之后、`web_fetch` 不可信内容警告之前；identity/persona 与 `web_fetch` 警告保持逐字不变。
- 原文（逐字）：

```text
Client UI changes (React/Tauri webview, docs/27 patterns) are contract-first: write the interaction contract in specs/changes/<id>/interaction.md with unique IDs, then browser tests tagged @interaction:<ID>, then the smallest implementation slice. Done means a real-browser click-through on the serve carrier, axe 0 serious/critical, aria and visual baselines, and evidence under target/evidence/. Reuse ui-kit primitives and tokens; never add a second focus/toast/dialog/menu implementation. Forbidden shortcuts: CSS-selector tests, waitForTimeout, force clicks, auto-accepting snapshots, and claiming "manually verified". Loop: contract, tests first, smallest slice, browser test, read trace/screenshot/aria diff, fix (max 3), evidence; if a test exposes a design gap, update the contract first.
```

- 理由：
  - 客户端 UI 是包化 React/Tauri（`docs/16`、`docs/27`、`docs/28`），交互缺陷过不了类型/lint，只能靠交互契约 + 真实浏览器证据兜底。
  - agent 的默认倾向是“写完即报完成”；没有契约与证据约束时会出现 CSS 选择器测试、`force: true`、`waitForTimeout`、自动接受快照、自述“手动点过”等捷径（正是 spec §2 门禁反模式）。
  - 先测试后实现、失败先读 trace/截图/aria diff，能把修复锁在真实行为上，并把“完成”从自述变成可执行证据。
  - 复用 ui-kit 原语与 token、禁止第二套 focus/toast/dialog/menu，是 C17 单一实现的提示词层表达。
- 规则：改提示词必须更新本文件与 `research/dsh-agent-inventory.md` 引用（见 §4）；不得把 DSH 运行时代码/依赖引入仓库（C2/C15）；本段关键片段由 `zk-agent-rig` 单测 `default_preamble_mentions_ui_interaction` 锁定。

### 3.1 直觉交互段（2026-11-01）

> 来源：ZeoKeel 自有提示词（**非** DSH 逐字移植），依据 `specs/changes/2026-11-01-ui-intuition-heuristics/spec.md` R3 与 `design.md` §2.4；直觉规范真源是 `docs/27` §2.7/§2.8、`docs/19` §11.1、`docs/11` §8.6，本文件只记录提示词原文与落位，不复制规范。

- 位置：同一 `agent-rig::default_preamble`，紧接 §3 交互段之后、`web_fetch` 不可信内容警告之前；两段由 agent-rig 单测同时锁定。
- 原文（逐字）：

```text
New client UI features or interaction changes need a cognitive walkthrough before code: 3 personas (first-time, high-frequency, keyboard/screen-reader or error recovery) x their top tasks, answering 4 questions (know the goal? see the control? predict the feedback? recover from error?). Write intuition.md (INT-* rows: scenario, user expectation, verification) and intuition-walkthrough.md next to interaction.md, then the interaction contract, tests, and code. e2e cannot prove intuition; the interaction suite is only a proxy. Never claim a surface is "intuitive" without the walkthrough, a fresh-context review, and a real user test with at least 5 users (think-aloud; success rate, first click, time, errors); resolve all P0 friction first.
```

- 理由：
  - 交互契约 + e2e 只能证明“行为按约定发生”，证明不了“真人第一次就能用”；把直觉判定拆成 INT 表 + 走查 + 用户测试三条证据，才符合 `docs/11` 的可执行证据政策。
  - 认知走查（3 persona × 3 top task × 4 问）成本低，能在写代码前暴露 P0 摩擦，避免实现完成后返工。
  - agent 默认把“功能可用”当成“符合直觉”；提示词明确 e2e 不是直觉证明，并禁止无证据的 “intuitive” 结论（评审记录/acceptance 必须引用 INT ID 与用户测试路径，未执行标 UNVERIFIED）。
  - 用户测试（≥5 人 think-aloud）是唯一发布证据；自动化门禁只查直觉文件存在性，不冒充质量判定。
- 测试锁：`default_preamble_mentions_ui_interaction` 断言新增关键词（`cognitive walkthrough`、`intuition.md`、`intuition-walkthrough.md`、`user test`、`P0 friction`）与既有 identity/`web_fetch` 片段同时存在；改段必须同步改测试。
- 规则：改提示词必须更新本文件与 `research/dsh-agent-inventory.md` 引用（见 §4）；不得把 DSH 运行时代码/依赖引入仓库（C2/C15）；不得在 `tool_guidance` 复制第二份直觉提示词（避免第二份规范）。

## 4. 规则

- 改提示词必须更新本文件与 `research/dsh-agent-inventory.md` 引用。
- 不得把 DSH 运行时代码/依赖引入仓库（C2/C15）。
- 提示词变更须同步更新 `zk-agent-rig` 中锁定关键片段的单测断言（如 `default_preamble_mentions_ui_interaction`）。
