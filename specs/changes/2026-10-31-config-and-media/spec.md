# 2026-10-31-config-and-media · 规格

- 状态：`approved`

## 1. 契约

- `zk-contract-config` 新增：
  - `AgentSettings { #[serde(default)] max_steps: usize = 50; completion: CompletionSettings; memory: MemorySettings; hooks: HookSettings }`
  - `CompletionSettings { temperature: Option<f64>, max_tokens: Option<u64>, top_p: Option<f64>, seed: Option<u64>, stop: Vec<String>, tool_choice: ToolChoiceSetting = Auto, parallel_tool_calls: Option<bool>, reasoning_effort: Option<String>, prompt_cache: Option<String> }`
  - `MemorySettings { enabled: bool = true, dir: String = "" }`、`HookSettings { enabled: bool = true, log: String = "" }`
  - `AppSettings { #[serde(default)] agent: AgentSettings }`（schema 仍为 1，向后兼容）
- `zk-contract-agent` 新增 `AgentUsage { input_tokens, output_tokens, total_tokens, cached_tokens, reasoning_tokens, cost_micros? }`；`AgentReply { #[serde(default)] usage: Option<AgentUsage> }`。
- `Agent` trait 增加默认空实现 `configure_agent(&AgentSettings)`（不破坏既有实现）。

## 2. 桌面/agent 行为

- 启动：读取 settings → `zk_agent_rig::set_default_agent_config(settings.agent)`；settings.set 成功后重新应用；失败回传 UI 错误。
- agent-rig 每轮读取 default config（已有），usage 填充到 AgentReply。
- 登出：gateway provider 重置为未配置占位模型（`model_from_config` 全空返回 placeholder）。

## 3. 媒体能力（个人版）

- `cap-image`：rig-core `image_generation`（OpenAI 兼容）生成图片，返回 bytes/encoding；provider 未配置 → available=false+reason。
- `cap-audio`：rig-core `audio_generation`（TTS）；`cap-transcription`：rig-core `transcription`；`cap-rerank`：rig-core rerank 模型。
- 配置来自 `llm-rig` 的 media model builders（base_url/api_key/model），env 前缀 `ZEOKEEL_{IMAGE,AUDIO,TRANSCRIPTION,RERANK}_*`，settings 后续接线。
- 企业网关不提供这些端点，能力保持不可用并给出原因。

## 4. T22 前端边界

- 桌面引入 Ajv + 生成 `SCHEMAS`；`parseCommandResponse`/`parseUiEvent`/`parseAgentEvent`/`parseSettings` 先按 schema 校验，再进入现有安全映射；失败拒绝并 console.warn。
- 负向单测：缺字段、错类型、未知事件 type 被拒绝；合法样本通过。

## 5. 验收

见 acceptance.md。
