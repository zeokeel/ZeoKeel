# 38 · Agent runtime（rig）

> 真源：`agent.loop` = `packages/client/agent-rig`，模型 = `llm.provider.default`，工具 = `tool.*` 包。DSH 只作参考（`research/dsh-agent-inventory.md`），不运行 DSH。
>
> **组合状态（ADR-064，2026-10-31）**：`editions/personal.toml`（enterprise 继承）映射
> `agent.loop = zeokeel/agent-rig`、个人版 `llm.provider.default = zeokeel/llm-rig`；
> `packages/client/agent-loop` 只保留为最小演示/契约对拍，不进入 shipped 组合。未配置 provider 时
> 桌面以占位模型启动，随后从 settings 热切换（失败输出诊断日志；`settings.set` 把错误回传 UI）。

## 1. 内核

- `rig-agent` AgentBuilder/Agent 是唯一循环：preamble、DynamicTool（JSON Schema）、max turns、hooks、extractor、memory/vector 由 rig 提供。
- 每个工具是独立包（`zk-tool-*`），角色 `tool.<名>`；共享路径/进程/截断逻辑只在 `crates/zk-kit-tools`（C17）。
- 工具通过 `TurnContext` 获得 workspace/conversation；DynamicTool 回调闭包捕获同一 context，禁止把工具绑定到全局 cwd。
- 提示词：DSH identity + persona（`You are an AI agent powered by DeepSeek Harness.` / `You are a coding agent powered by the … model.`）+ 工作区 `AGENTS.md`/`CLAUDE.md`（≤64KB）+ 工具说明。来源见 `research/dsh-agent-inventory.md` §3。

## 2. 审批

- `ToolSpec.requires_approval` 的工具在 rig DynamicTool 回调内暂停：发 `ApprovalRequested` → `ApprovalGate.decide` 阻塞 → 发 `ApprovalResolved` → 继续/拒绝。
- 默认 `DenyAll`（fail-closed）；桌面在 `AppState` 安装 `ApprovalBroker`，UI `approval.decide` 解除等待；超时/无通道 = 拒绝。
- 这是 DSH `ask` 的等价语义；sandbox 升级不在弱管控范围。

## 3. 事件/取消/上下文

- `EventSink` 通过 `run_stream_in_shared(Arc<dyn EventSink>)` 传入，工具与审批可在执行中发事件；旧 `&dyn` 路径用缓冲 sink 兼容。
- 每轮事件：TurnStarted → ApprovalRequested/Resolved → ToolCallStarted/Finished → TextDelta → TurnCompleted/Cancelled/Error。
- 取消：rig 流式循环每项检查 `RigAgent.cancel()`；工具执行前检查取消；桌面 `chat.cancel` 调用 `Runtime::cancel_agent()`。工具超时优先取设置 `AppSettings.tools.tool_timeout_ms`（默认 120s、范围 1000..600000），未保存设置时回退 `ZEOKEEL_TOOL_TIMEOUT_MS`。非流式 provider 通过 50ms 轮询取消标志也可中断；审批超时优先取设置 `AppSettings.tools.approval_timeout_ms`（默认 300s、范围 1000..1800000），未保存设置时回退 `ZEOKEEL_APPROVAL_TIMEOUT_MS`。
- 委派：subagent/workflow/ralph 复用同一 model/tools，但子会话 id 隔离为 `<conversation>-sub-<depth>`（异步子代理追加 `-<id>` 保证唯一）；最大深度 3。深度按会话 id 传递（子代理在新线程执行，不能只依赖线程本地状态），工具执行闭包内恢复深度后 `MAX_DEPTH` 才真正生效。异步子代理（`subagent_start`/`subagent_status`/`subagent_interrupt`）在后台线程运行同一套 run 逻辑，进程内并发上限 8；取消通过父/子共享的 `Arc<AtomicBool>` 传递（`RigAgent::with_cancel_flag`/`cancel_flag`），完成/失败/取消的状态与结果保存在 runner 注册表，list_agents 可枚举 async 子代理。

## 4. 工具矩阵

- 已实现：`fs.read`、`fs.write`、`fs.edit`、`fs.search`、`bash`、`bash_persistent`、`job_output/list/kill`、`glob`、`grep`、`skill`、`web.fetch`、`working_directory`、`todo`、`present`、`load_workspace_dependencies`、`create_worktree`、`terminal`、`goal`、`schedule`、`pwsh`、`read_image`、`subagent`、`subagent_start`、`subagent_status`、`subagent_interrupt`、`subagent_fork`、`workflow`、`ralph`、`ask_user_question`、`extract_json`、`read_document`（PDF/EPUB）、`session_search`、`web_search`、`pwsh_persistent`、`subagent_control`、`cordis`（禁用）、`agent_team`（实验禁用）；UI 已有审批卡、问题卡、provider 流式/取消；每个工具包至少一条行为测试，`scripts/agent-e2e.sh` 覆盖 9 条人机流程（审批/拒绝/取消/ask-user/bash/present/多步；retries=0）。
- 部分/未实现：`agent-team`（未启用）、`call-timeout`（agent 级策略，超时已可经 `AppSettings.tools` 配置）、workflow 并行（DAG 已支持）、subagent fork 的实时历史共享（当前为上下文快照）、pwsh persistent 平台支持。`schedule` 执行器为 opt-in：仅当 `ZEOKEEL_ENABLE_SCHEDULE_AUTORUN=1` 时读取工作区 `.zeokeel/schedules.json` 并自动起 turn；条目必须带 `schedule` 工具写入的 `origin` 标记、按 `interval_seconds` 重复（cron 在创建时校验但不解释）。
- 完整矩阵与测试状态以 `docs/generated/AGENT-TOOL-MATRIX.md` 为准。
- 平台不支持必须返回明确错误，不得静默缺失。

## 5. rig 能力释放矩阵（2026-10-31 round 3）

### 5.1 已用/已接

- Agent 循环：AgentBuilder、DynamicTool（JSON Schema）、preamble、max turns、streaming（text/tool-call/tool-result）、取消标志、审批阻塞、工具超时；`RigAgent::configure_model` 热切换 model（`configure_model_hot_swaps`）。
- 多 provider：openai/deepseek/anthropic/ollama（`llm-rig`）；enterprise 由 `llm-gateway` 的 rig Responses 适配（`model()`/`model_from_config`）。
- Extractor：`extract_json` 工具（`rig_agent::extractor`，retries=2）。
- Loaders：`read_document`（PDF/EPUB，独立实现，`tool-read-document`）。workspace 只启用 rig 的 `audio`/`image` feature；workspace 的 `lopdf` 固定 `=0.42.0`（tool-read-document 用），rig `pdf` feature 要求 `lopdf 0.44` 且其依赖未缓存；`epub` 是 GPL-3.0（与项目 MIT 冲突）——两者均不启用（见 §5.3）。

### 5.2 本轮新增（round 3）

| 能力 | 实现 | 配置/接口 |
| --- | --- | --- |
| Completion 参数 | `agent-rig` 每轮把配置写进 rig `AgentBuilder`（`temperature`/`max_tokens`/`tool_choice`）与 `GenerationOptions`（`top_p`/`seed`/`stop`/`parallel_tool_calls`/`reasoning`/`cache`） | `AgentRuntimeConfig`（`agent-rig::config`）：`with_agent_config` / `configure_agent` / 进程级 `set_default_agent_config`；env 兜底 `ZEOKEEL_AGENT_TEMPERATURE/_MAX_TOKENS/_TOP_P/_SEED/_STOP/_TOOL_CHOICE/_PARALLEL_TOOL_CALLS/_REASONING_EFFORT/_PROMPT_CACHE`。优先级：显式配置 > env > 内建默认（不强制 sampling，保持 provider 默认）。契约已落地：`AppSettings.agent`（schema 1，`#[serde(default)]`）+ `Agent::configure_agent`；桌面启动与 `settings.set` 后通过契约 runtime settings 快照应用（失败回传 UI；`set_default_agent_config` 仍可供直接嵌入）。 |
| Usage | 流式 `FinalResponse` 与非流式 `PromptResponse` 都取 rig `Usage`（input/output/total/cached/cache_creation/reasoning/cost） | `RigAgent::last_usage() -> Option<UsageSnapshot>`；hook `TurnCompleted{usage}`。``AgentReply.usage` 契约已落地（`AgentUsage`：input/output/total/cached/reasoning，未报告归一为 0）；桌面把 usage 持久化到 session `Message`（SQLite v4 `usage_json`），重启加载后 UI 经 `ChatMessage.usage` 恢复展示。 |
| 多轮 memory | `rig-memory` DirStore + conversation 持久化；每轮自动回放该 conversation 历史 | `AgentRuntimeConfig.memory{enabled,dir}`；目录优先级 `dir` > `ZEOKEEL_AGENT_MEMORY_DIR` > `$ZEOKEEL_HOME/agent-memory`；无目录时发 `MemoryUnavailable` hook（不静默）。 |
| Hooks | `HookSink` 结构化事件：TurnStarted/TurnCompleted(usage/cancelled)/ToolStarted/ToolFinished/ApprovalRequested/ApprovalResolved/MemoryUnavailable/TurnFailed；可选 JSONL 追加 | `RigAgent::with_hook_sink`；`AgentRuntimeConfig.hooks{enabled,log}` / `ZEOKEEL_AGENT_HOOKS_ENABLED/_LOG`；`cap-hooks` 动态 availability（host 可 `set_available` 覆盖）。 |
| Vector/embeddings | `cap-vector`：OpenAI-compatible embedding model + cosine 索引（insert/insert_many/search/clear/len）。rig 适配在共享 `crates/zk-kit-media`（`kit.media` 的 `EmbeddingConfig`/`EmbeddingModel`/`embed_text`），cap-vector 不再直接依赖 rig-core/llm-rig（ADR-067）。**可选持久化**：配置 `ZEOKEEL_VECTOR_DIR`（或 `configure_with_dir`）后索引存 `vector-index.json`（versioned `{version, model, dims, width, documents:[{id,text,embedding}]}`），`insert_many` 为批量原子提交（全部 embedding 成功后一次 save），insert/clear 原子写（temp `0600` → rename，目录 `0700`，已存在目录也收紧为 `0700`）；上限 10k docs / 16MB（`VectorError::Capacity`，文档数在发起 embedding 前预检）；缺失/损坏/不可读/超限/**模型或维度指纹不匹配**时降级为空索引并暴露 `persistence_reason()` + stderr 日志（不 panic），下次成功保存修复文件。 | `configure(EmbeddingModel)`（读 `ZEOKEEL_VECTOR_DIR`；用持久化状态替换内存索引，无 dir 时为空）、`configure_with_dir(model, dir)`、`configure_from_env()`（`ZEOKEEL_EMBEDDING_BASE_URL/_API_KEY/_MODEL/_DIMS` + `ZEOKEEL_VECTOR_DIR`）、`unconfigure()`、`persistence_dir()`/`persistence_reason()`；未配置时 `available()==false` + `unavailable_reason()` + `CAPABILITY_UNAVAILABLE`。personal boot 经 `zk_composed::configure_capabilities_from_env()` 自动配置；enterprise 不配置（C7/ADR-031，网关无 embeddings 路由）。 |
| 媒体/rerank/转写 | round 3 为「诚实不可用」占位；round 4 已用 rig 媒体模型实现，见 §5.3 | `cap-audio/cap-image/cap-rerank/cap-transcription`；错误码 `CAPABILITY_UNAVAILABLE`。 |
| 异步子代理 | `RigSubagentRunner` 后台线程执行子代理（复用同一 model/tools），`subagent_start` 立即返回 id；注册表保存 `running/done/failed/cancelled` + 最终结果；`subagent_status` 查询、`subagent_interrupt` 置共享取消标志；并发上限 8、深度上限 3；完成/退出清理会话深度，list_agents 枚举 async 子代理 | 工具：`subagent_start{prompt}`、`subagent_status{id}`、`subagent_interrupt{id}`（`tool-subagent`/`tool-subagent-control` 经生成 glue 的 `extra_tools()` 追加）；kit `SubagentRunner::{start,status,result,interrupt}` + `AsyncAgentStatus`；`RigAgent::{with_cancel_flag,cancel_flag}`；无异步能力的 runner 诚实返回 "async subagents are not supported"。 |
| 工具/审批超时 | `agent-rig` 每轮把已保存的 `tool_timeout_ms` 作为 DynamicTool 执行超时；桌面 `ApprovalBroker::set_timeout_ms` 在启动与 `settings.set` 后热更新审批等待超时（不再只读 env） | `AppSettings.tools`（`ToolSettings{tool_timeout_ms=120000 clamp 1000..600000, approval_timeout_ms=300000 clamp 1000..1800000}`）。优先级：显式 settings > `ZEOKEEL_TOOL_TIMEOUT_MS`/`ZEOKEEL_APPROVAL_TIMEOUT_MS` env > 默认；fresh install / 默认-only 文件不发布（env 生效，同 agent settings 的 boot 语义），保存后立即生效；越界值 `settings.set` 回传 UI 错误。 |

### 5.3 媒体能力（round 4，2026-10-31）

| 能力 | 实现 | 配置/接口 | 可用性 |
| --- | --- | --- | --- |
| 图像生成 `media.image` | `cap-image`：rig `image_generation`（OpenAI 兼容）→ bytes + 编码探测（PNG/JPEG/WebP/GIF） | `configure[_from_config/_from_env]` / `unconfigure`；`ZEOKEEL_IMAGE_{PROVIDER,BASE_URL,API_KEY,MODEL}`（默认 `openai` / `gpt-image-1`）；`generate(prompt,width,height)` | 配置模型后可用；未配置 `available()==false` + `unavailable_reason()` + `CAPABILITY_UNAVAILABLE` |
| 语音合成 `media.audio` | `cap-audio`：rig `audio_generation`（TTS）→ 音频字节 | `ZEOKEEL_AUDIO_*`（默认 `openai` / `tts-1`）；`synthesize(text,voice)` | 同上 |
| 转写 `media.transcription` | `cap-transcription`：rig `transcription` | `ZEOKEEL_TRANSCRIPTION_*`（默认 `openai` / `whisper-1`）；`transcribe(bytes,filename)`、`transcribe_file(path)` | 同上 |
| rerank `media.rerank` | `cap-rerank`：rig `rerank`（Voyage AI wire） | `ZEOKEEL_RERANK_*`（默认 `voyageai` / `rerank-2.5`）；`rerank(query,documents)` | 同上；`provider=openai` 明确报配置错误（rig-core 0.45 OpenAI wire 无 rerank 端点，源码 `RerankQuirks::unsupported`） |

- 实现位置（C4/C17，round-2 评审 P1-4）：rig 适配只在共享库 `crates/zk-kit-media`（`kit.media`）——`MediaKind`/`MediaConfig`（**Debug 脱敏 api_key**）/`MediaError`/四个 builder（返回不透明句柄 `ImageModel`/`AudioModel`/`TranscriptionModel`/`RerankModel`）/`block_on`/`ModelSlot`，以及 embedding 适配（`EmbeddingConfig`/`EmbeddingModel`/`embedding_model[_from_config]`/`embed_text`，供 `cap-vector`）。`cap-*` 只依赖该 kit + 自己的错误/结果 DTO，**不再直接依赖 `rig-core` 或 `zk-llm-rig`**。capability index 已登记 `kit.media`、`vector.store` 与 `media.{image,audio,transcription,rerank}`（`@capability`）。
- 同步桥：cap-* 是同步 API、rig 模型是 async，`zk_kit_media::block_on(future)` 是唯一共享 runtime；cap-* 的 model slot / configure / available 结构由 `kit.media::ModelSlot` 统一。
- **宿主接线**：personal boot 调用生成 glue 的 `zk_composed::configure_capabilities_from_env()`（`zk compose` 按组合生成，内部逐个调 cap-* 的 `configure_from_env()`；未配置 provider 的 cap 保持不可用并记录原因）。enterprise boot **不调用**它（C7/ADR-031：企业 AI 只走网关，网关无 image/audio/transcription/rerank 端点），故 enterprise 媒体能力保持 `available()==false` + `unavailable_reason()`，不得假装可用。
- rig features：workspace 只启用 `audio`+`image`。`pdf` 不启用：workspace 为 `tool-read-document` 固定 `lopdf = "=0.42.0"`，而 rig 0.45 的 `pdf` feature 要求 `lopdf 0.44`（连带 `ecb`/`md-5`/`weezl`，离线未缓存）；`epub` 不启用：其依赖 `epub 2.1.5` 是 GPL-3.0，与项目 MIT 冲突。PDF/EPUB 解析由 `tool-read-document` 自身实现承担。
- 测试：每个 cap 包用 `zk_kit_media::test_support` 的确定性进程内假模型（feature `test-support`，底层 rig `Local<Op>` + fake transport）覆盖「配置后真实调用」与「未配置」两条路径，无网络；真实 provider 为 UNVERIFIED（见 acceptance）。
- ADR：ADR-067（accepted）已把 rig 适配点由 ADR-066 的三处扩展为 `llm-rig`/`agent-rig`/`llm-gateway` + `kit-media`（kit 非包，满足 C4/C17 单一实现）；媒体与 embedding 的 rig 实现都在 kit，`cap-*` 不再直接依赖 rig-core。

### 5.4 仍未接（明确 gap）

- **契约与桌面接线（已落地）**：`AppSettings.agent`/`AgentSettings`、`AgentReply.usage`（`AgentUsage`，未报告计数器归一 0）、`Agent::configure_agent` 默认实现已入契约；桌面启动与 `settings.set` 后按 section 发布非密 `RuntimeSettings{agent?,tools?}` 并应用 agent 配置（失败回传 UI 错误）；media/vector 配置由 personal boot 调用生成 glue 的 `zk_composed::configure_capabilities_from_env()`（enterprise 生成/调用 no-op，C7/ADR-031）。**剩余**：`AgentEvent` 无独立 usage 事件（随 `TurnCompleted.reply` 携带）；`Agent::configure_agent` 在 agent-rig 生效，其它实现仍为默认 no-op；usage 已持久化并在加载后恢复展示。
- **真实历史回放 vs 会话 DB**：当前多轮靠 rig-memory 目录日志；`SessionStore::messages_in` 回放仍缺（需桌面把历史注入 prompt 或统一由 memory 承担）。
- rig 的 run-state 序列化/`Recorder`/`observe`/`telemetry`/`effect`、`ModelCatalog` 上下文窗口/压缩仍未接；rig loaders 的 `pdf`/`epub` feature 均未启用（原因见 §5.1），`read_document` 仍用独立实现。
- `cap-vector` 可选 JSON 持久化已实现（`ZEOKEEL_VECTOR_DIR`，原子写/0600/0700、10k docs/16MB 上限、损坏降级 + reason）；**剩余**：无跨进程写锁/跨设备同步（本地文件，最后一写者胜），无 SQLite/FTS，enterprise 仍无 embeddings 路由（C7 不提供）；指纹只校验 model id/dims/width（不含 provider 端模型版本号）。

### 5.5 能力可用性契约

- 每个能力包必须提供动态 `available()`；不可用时给出 `unavailable_reason()`，host 侧统一映射 `CAPABILITY_UNAVAILABLE`，不得静默缺失或假装可用。
- 参考实现：`cap-hooks`（默认可用，env/host 可关）、`cap-vector`（配置 embedding 后可用）、`cap-{audio,image,rerank,transcription}`（配置 rig 媒体模型后可用，未配置诚实不可用 + 原因）。
