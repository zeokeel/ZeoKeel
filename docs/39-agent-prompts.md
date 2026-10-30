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
- DSH compaction、goal/ralph/subagent/teams 的独立提示段未全量移植；当前只保留 identity/persona/workspace/tool guidance。
- 工具描述来自各 ZeoKeel 包 `ToolSpec.description`，不逐字复制 DSH（避免与实现不符）。

## 3. 规则

- 改提示词必须更新本文件与 `research/dsh-agent-inventory.md` 引用。
- 不得把 DSH 运行时代码/依赖引入仓库（C2/C15）。
