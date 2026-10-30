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
- 取消：rig 流式循环每项检查 `RigAgent.cancel()`；工具执行前检查取消；桌面 `chat.cancel` 调用 `Runtime::cancel_agent()`。工具超时可由 `ZEOKEEL_TOOL_TIMEOUT_MS` 配置（默认 120s）。非流式 provider 通过 50ms 轮询取消标志也可中断；审批超时由 `ZEOKEEL_APPROVAL_TIMEOUT_MS` 配置（默认 300s）。
- 委派：subagent/workflow/ralph 复用同一 model/tools，但子会话 id 隔离为 `<conversation>-sub-<depth>`；最大深度 3。深度按会话 id 传递（子代理在新线程执行，不能只依赖线程本地状态），工具执行闭包内恢复深度后 `MAX_DEPTH` 才真正生效。

## 4. 工具矩阵

- 已实现：`fs.read`、`fs.write`、`fs.edit`、`fs.search`、`bash`、`bash_persistent`、`job_output/list/kill`、`glob`、`grep`、`skill`、`web.fetch`、`working_directory`、`todo`、`present`、`load_workspace_dependencies`、`create_worktree`、`terminal`、`goal`、`schedule`、`pwsh`、`read_image`、`subagent`、`subagent_fork`、`workflow`、`ralph`、`ask_user_question`、`extract_json`、`read_document`（PDF/EPUB）、`session_search`、`web_search`、`pwsh_persistent`、`subagent_control`、`cordis`（禁用）、`agent_team`（实验禁用）；UI 已有审批卡、问题卡、provider 流式/取消；每个工具包至少一条行为测试，`scripts/agent-e2e.sh` 覆盖 9 条人机流程（审批/拒绝/取消/ask-user/bash/present/多步；retries=0）。
- 部分/未实现：`agent-team`（未启用）、`call-timeout`（固定 120s 策略）、workflow 并行（DAG 已支持）、subagent fork/异步 control、pwsh persistent 平台支持。`schedule` 执行器为 opt-in：仅当 `ZEOKEEL_ENABLE_SCHEDULE_AUTORUN=1` 时读取工作区 `.zeokeel/schedules.json` 并自动起 turn；条目必须带 `schedule` 工具写入的 `origin` 标记、按 `interval_seconds` 重复（cron 在创建时校验但不解释）。
- 完整矩阵与测试状态以 `docs/generated/AGENT-TOOL-MATRIX.md` 为准。
- 平台不支持必须返回明确错误，不得静默缺失。

## 5. rig 能力释放矩阵（2026-10-31 round 3）

### 5.1 已用/已接

- Agent 循环：AgentBuilder、DynamicTool（JSON Schema）、preamble、max turns、streaming（text/tool-call/tool-result）、取消标志、审批阻塞、工具超时；`RigAgent::configure_model` 热切换 model（`configure_model_hot_swaps`）。
- 多 provider：openai/deepseek/anthropic/ollama（`llm-rig`）；enterprise 由 `llm-gateway` 的 rig Responses 适配（`model()`/`model_from_config`）。
- Extractor：`extract_json` 工具（`rig_agent::extractor`，retries=2）。
- Loaders：`read_document`（PDF/EPUB，独立实现，`tool-read-document`）。workspace 只启用 rig 的 `audio`/`image` feature；`pdf` 需要未缓存的 `lopdf 0.44`（连带 `ecb`/`md-5`/`weezl`）且 `epub` 是 GPL-3.0（与项目 MIT 冲突），两者均不启用（见 §5.3）。

### 5.2 本轮新增（round 3）

| 能力 | 实现 | 配置/接口 |
| --- | --- | --- |
| Completion 参数 | `agent-rig` 每轮把配置写进 rig `AgentBuilder`（`temperature`/`max_tokens`/`tool_choice`）与 `GenerationOptions`（`top_p`/`seed`/`stop`/`parallel_tool_calls`/`reasoning`/`cache`） | `AgentRuntimeConfig`（`agent-rig::config`）：`with_agent_config` / `configure_agent` / 进程级 `set_default_agent_config`；env 兜底 `ZEOKEEL_AGENT_TEMPERATURE/_MAX_TOKENS/_TOP_P/_SEED/_STOP/_TOOL_CHOICE/_PARALLEL_TOOL_CALLS/_REASONING_EFFORT/_PROMPT_CACHE`。优先级：显式配置 > env > 内建默认（不强制 sampling，保持 provider 默认）。**契约字段待 T22 解冻后搬入 `AppSettings.agent`**（schema 已报 Lead）。 |
| Usage | 流式 `FinalResponse` 与非流式 `PromptResponse` 都取 rig `Usage`（input/output/total/cached/cache_creation/reasoning/cost） | `RigAgent::last_usage() -> Option<UsageSnapshot>`；hook `TurnCompleted{usage}`。**`AgentReply.usage` / UI 持久化待 T22 契约**。 |
| 多轮 memory | `rig-memory` DirStore + conversation 持久化；每轮自动回放该 conversation 历史 | `AgentRuntimeConfig.memory{enabled,dir}`；目录优先级 `dir` > `ZEOKEEL_AGENT_MEMORY_DIR` > `$ZEOKEEL_HOME/agent-memory`；无目录时发 `MemoryUnavailable` hook（不静默）。 |
| Hooks | `HookSink` 结构化事件：TurnStarted/TurnCompleted(usage/cancelled)/ToolStarted/ToolFinished/ApprovalRequested/ApprovalResolved/MemoryUnavailable/TurnFailed；可选 JSONL 追加 | `RigAgent::with_hook_sink`；`AgentRuntimeConfig.hooks{enabled,log}` / `ZEOKEEL_AGENT_HOOKS_ENABLED/_LOG`；`cap-hooks` 动态 availability（host 可 `set_available` 覆盖）。 |
| Vector/embeddings | `cap-vector`：rig embedding model + 进程内 cosine 索引（insert/insert_many/search/clear/len） | `configure(DynModel<Embedding>)`、`configure_from_env()`（`ZEOKEEL_EMBEDDING_BASE_URL/_API_KEY/_MODEL/_DIMS`）、`unconfigure()`；未配置时 `available()==false` + `unavailable_reason()` + `CAPABILITY_UNAVAILABLE`。`llm-rig::embedding_model()` 提供个人版 embedding model。**注：cap-vector 成为第三个 rig 适配包，ADR-007「只有两个适配层」的措辞需补 ADR。** |
| 媒体/rerank/转写 | round 3 为「诚实不可用」占位；round 4 已用 rig 媒体模型实现，见 §5.3 | `cap-audio/cap-image/cap-rerank/cap-transcription`；错误码 `CAPABILITY_UNAVAILABLE`。 |

### 5.3 媒体能力（round 4，2026-10-31）

| 能力 | 实现 | 配置/接口 | 可用性 |
| --- | --- | --- | --- |
| 图像生成 `media.image` | `cap-image`：rig `image_generation`（OpenAI 兼容）→ bytes + 编码探测（PNG/JPEG/WebP/GIF） | `configure[_from_config/_from_env]` / `unconfigure`；`ZEOKEEL_IMAGE_{PROVIDER,BASE_URL,API_KEY,MODEL}`（默认 `openai` / `gpt-image-1`）；`generate(prompt,width,height)` | 配置模型后可用；未配置 `available()==false` + `unavailable_reason()` + `CAPABILITY_UNAVAILABLE` |
| 语音合成 `media.audio` | `cap-audio`：rig `audio_generation`（TTS）→ 音频字节 | `ZEOKEEL_AUDIO_*`（默认 `openai` / `tts-1`）；`synthesize(text,voice)` | 同上 |
| 转写 `media.transcription` | `cap-transcription`：rig `transcription` | `ZEOKEEL_TRANSCRIPTION_*`（默认 `openai` / `whisper-1`）；`transcribe(bytes,filename)`、`transcribe_file(path)` | 同上 |
| rerank `media.rerank` | `cap-rerank`：rig `rerank`（Voyage AI wire） | `ZEOKEEL_RERANK_*`（默认 `voyageai` / `rerank-2.5`）；`rerank(query,documents)` | 同上；`provider=openai` 明确报配置错误（rig-core 0.45 OpenAI wire 无 rerank 端点，源码 `RerankQuirks::unsupported`） |

- builder 在 `llm-rig`：`image_model()` / `audio_model()` / `transcription_model()` / `rerank_model()`（及 `*_from_config(&MediaConfig)`），`MediaKind` + `MediaConfig` 统一解析 env；未配置返回 `MediaError::Unconfigured`，非法配置返回 `MediaError::Config`。cap-* 只做错误映射 + 模型槽 + 结果 DTO，不保存资产。
- 同步桥：cap-* 是同步 API、rig 模型是 async，`llm-rig::block_on(future)` 是唯一共享 runtime（cap-* 不再各自复制 tokio runtime）。
- rig features：workspace 只启用 `audio`+`image`；`pdf`（依赖未缓存、离线构建失败）与 `epub`（GPL-3.0 与 MIT 冲突）都不启用，PDF/EPUB 解析只由 `tool-read-document` 自身实现承担。
- **企业版保持不可用**：企业网关只允许 OpenAI Responses API（C7/ADR-031），不提供 image/audio/transcription/rerank 端点；这些能力在 enterprise 组合下保持 `available()==false` 并给出原因，不得假装可用。
- 测试：每个 cap 包用 rig `Local<Op>` + fake transport 的确定性进程内模型覆盖「配置后真实调用」与「未配置」两条路径，无网络；真实 provider 为 UNVERIFIED（见 acceptance）。

### 5.4 仍未接（明确 gap）

- **契约层**（T22 冻结中，本轮不改 contracts）：`AppSettings.agent` 字段、`AgentReply.usage`、`AgentEvent` usage/hook 事件、`Agent::configure_agent` trait 方法；桌面 settings loader 调 `zk_agent_rig::set_default_agent_config(...)`，登录/设置后调 `Runtime::configure_llm` + `configure_agent`；cap-vector 需桌面在 provider 配置后调用 `zk_cap_vector::configure(zk_llm_rig::embedding_model()?)`（enterprise 网关无 embeddings 路由，保持不可用）。
- **真实历史回放 vs 会话 DB**：当前多轮靠 rig-memory 目录日志；`SessionStore::messages_in` 回放仍缺（需桌面把历史注入 prompt 或统一由 memory 承担）。
- rig 的 run-state 序列化/`Recorder`/`observe`/`telemetry`/`effect`、`ModelCatalog` 上下文窗口/压缩仍未接；rig loaders 的 `pdf`/`epub` feature 均未启用（原因见 §5.1），`read_document` 仍用独立实现。
- `cap-vector` 索引为进程内（重启丢失）；SQLite 持久化待后续。

### 5.5 能力可用性契约

- 每个能力包必须提供动态 `available()`；不可用时给出 `unavailable_reason()`，host 侧统一映射 `CAPABILITY_UNAVAILABLE`，不得静默缺失或假装可用。
- 参考实现：`cap-hooks`（默认可用，env/host 可关）、`cap-vector`（配置 embedding 后可用）、`cap-{audio,image,rerank,transcription}`（配置 rig 媒体模型后可用，未配置诚实不可用 + 原因）。
