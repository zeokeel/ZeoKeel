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

- **单 owner = agent-rig**：策略求值（config spec R1 的封闭枚举 `ApprovalPolicy {ask|auto_read_only|strict}`，经
  `ToolSettings.approval_policy` 发布到 runtime settings；`ApprovalPolicySource` 只作测试/嵌入的实例覆盖）与
  always-allow 会话记忆都只在 `packages/client/agent-rig` 求值；宿主 `ApprovalBroker` 只做 IPC 阻塞与超时，**不持有**会话允许集合。
- **策略真值表（C-1=A）**：`ask` = 只询问声明 `requires_approval` 的工具；`auto_read_only` = 只读工具自动 + **所有 mutating** 询问；
  `strict` = 全部询问（含只读/交互工具，C-5 不豁免）。**弱不变式 T1**：任何策略都不自动批准 `requires_approval == true` 的工具
  （唯一例外是 S4 always-allow，属用户先前显式授权，不是策略判定）。当前 4 个 `mutating && !requires_approval` 工具
  （`schedule`/`job_kill`/`goal`/`mcp_call`）在 `ask` 下不询问、在 `auto_read_only`/`strict` 下询问；新增此类工具必须同步更新本清单。
- **旧字段迁移（C-2）**：`auto_approve_read_only` 兼容映射 `false→strict`、`true→ask`；启动（设置文件存在）与 `settings.set`
  幂等规范化为 `tools.approval_policy`（`ask` 规范化为缺省字段，保证默认-only 文档不发布、env fallback 仍生效）；
  未知枚举值/非法类型降级 `ask`，不阻止启动（R5）。
- 契约：`zk-contract-tool` 定义 `ApprovalMetadata`/`ApprovalRisk`/`ApprovalDecision`（agent 用 `pub use` 保持旧路径）；`ApprovalGate::decide_scoped`
  返回 typed 决策（默认实现转调旧 `decide -> bool`，第三方实现不破）。
- 事件序列表（唯一真源 = `specs/changes/2026-11-01-client-approval-tool-rendering/design.md` §2.0）：
  S1 ask→allow_once（`requested → resolved{allow_once} → started → finished{success}`）；S2 ask→reject（`resolved{reject}` → `finished{denied}`，无 started）；
  S3a 显式 cancel（`resolved{cancel}` → `finished{cancelled}` → `TurnCancelled`）；S3b 超时（`resolved{reject,timed_out:true}` → `finished{denied}`）；
  S3c 断线/窗口关闭（`resolved{reject,timed_out:false}` → `finished{denied}`）；S4 always-allow 命中（成对 `requested{auto:true} → resolved{allow_session}`，不阻塞、不弹决策）；
  S5 策略短路（不发任何 `approval_*`，直接执行）。
- 决策 wire：`allow_once | allow_session | reject | cancel`（canonical），宿主反序列化兼容旧值 `approved`→`allow_once`、`rejected`→`reject`。
- 拒绝/取消也发终态 `ToolCallFinished{status}` 并落库；`cancel` 与 `chat.cancel` 共用单一取消路径：broker 释放等待者(Reject) + `cancel_agent()`，
  由 agent-rig 统一发一次 `TurnCancelled`（宿主不再手发）。
- **执行中取消**（P1-13.2）：取消置位后，无论 gate 前还是工具 outcome 返回后都发 `ToolCallFinished{cancelled}`；result 原样保留工具已产生文本（不伪造失败），
  UI 据此显示「已取消（副作用可能已发生）」；无已产生结果时 result 为空。
- 默认 `DenyAll`（fail-closed）；超时/无通道 = 拒绝。这是 DSH `ask` 的等价语义；sandbox 升级不在弱管控范围。

## 3. 事件/取消/上下文

- `EventSink` 通过 `run_stream_in_shared(Arc<dyn EventSink>)` 传入，工具与审批可在执行中发事件；旧 `&dyn` 路径用缓冲 sink 兼容。
- 每轮事件：TurnStarted → ApprovalRequested/Resolved → ToolCallStarted/Finished → TextDelta → Usage(可选) → TurnCompleted/Cancelled/Error。
- 工具终态：`ToolCallStatus = success|failed|denied|cancelled`；终态事件携带已脱敏 `args` 与 `status`，`BridgeSink` **先落库再判断 cancelled 短路**
  （bus 只抑制非终态事件；`TurnCancelled` 与终态 `ToolCallFinished` 仍发）；会话 `Message.turn_id` 记录归属 turn（SQLite v5 仅加列），
  assistant 行 status(`done|cancelled`) 由 BridgeSink 观察到的 `TurnCancelled` 决定；取消/失败/拒绝的工具卡刷新后仍可恢复。
- 取消：rig 流式循环每项检查 `RigAgent.cancel()`；工具执行前检查取消；桌面 `chat.cancel` 调用 `Runtime::cancel_agent()`。工具超时优先取设置 `AppSettings.tools.tool_timeout_ms`（默认 120s、范围 1000..600000），未保存设置时回退 `ZEOKEEL_TOOL_TIMEOUT_MS`。非流式 provider 通过 50ms 轮询取消标志也可中断；审批超时优先取设置 `AppSettings.tools.approval_timeout_ms`（默认 300s、范围 1000..1800000），未保存设置时回退 `ZEOKEEL_APPROVAL_TIMEOUT_MS`。
- 委派：subagent/workflow/ralph 复用同一 model/tools，但子会话 id 隔离为 `<conversation>-sub-<depth>`（异步子代理追加 `-<id>` 保证唯一）；最大深度 3。深度按会话 id 传递（子代理在新线程执行，不能只依赖线程本地状态），工具执行闭包内恢复深度后 `MAX_DEPTH` 才真正生效。异步子代理（`subagent_start`/`subagent_status`/`subagent_interrupt`）在后台线程运行同一套 run 逻辑，进程内**存活线程**并发上限 8（RAII 计数，取消后线程未退出仍占槽；线程 panic 由 `catch_unwind` 捕获为 failed 并释放槽）；取消通过父/子共享的 `Arc<AtomicBool>` 传递（`RigAgent::with_cancel_flag`/`cancel_flag`），状态机为 `running→done|failed|cancelled`（cancelled 胜出且不被完成路径覆盖），完成/失败/取消的状态与结果保存在 runner 注册表，list_agents 可枚举 async 子代理。**保留策略**：runner children 与 kit 子代理注册表各上限 64 条；超出时淘汰最旧**已完成**记录（evicted id 的 `subagent_status`/结果视为 unknown→NotFound），`running` 记录永不淘汰，避免长会话无界增长。
- agent team：`agent_team {objective, tasks[], max_workers?, timeout_ms?}` 让父 agent 作为 coordinator，经同一 runner 的 `start/status/result/interrupt` **有界扇出** ≤8 个 worker（`max_workers` 1..=8，与 `subagent_start` 共享存活线程上限 8、深度上限 3）。同步聚合：50ms 轮询直至全部终态或超时（默认 60s，范围 100..300000ms；到时中断仍在运行的 worker），返回 `{status ∈ done|partial|failed|timeout, workers[{id,task,status,result|error}], completed/failed/cancelled/total, elapsed_ms}`；单 worker 失败为 `partial`、全失败为 `failed`，runner 缺失/深度/并发上限为明确 `Denied` 错误，启动失败会中断已启动 worker 并返回 `Io`。不做动态团队 UI、不做 LLM 综述（父 agent 从 per-worker 结果综合）；父轮次取消仍在工具边界生效（ADR-070）。

## 4. 工具矩阵

- 已实现：`fs.read`、`fs.write`、`fs.edit`、`fs.search`、`bash`、`bash_persistent`、`job_output/list/kill`、`glob`、`grep`、`skill`、`web.fetch`、`working_directory`、`todo`、`present`、`load_workspace_dependencies`、`create_worktree`、`terminal`、`goal`、`schedule`、`pwsh`、`read_image`、`subagent`、`subagent_start`、`subagent_status`、`subagent_interrupt`、`subagent_fork`、`workflow`、`ralph`、`ask_user_question`、`extract_json`、`read_document`（PDF/EPUB）、`session_search`、`web_search`、`pwsh_persistent`、`subagent_control`、`agent_team`（coordinator + 有界 worker 扇出）、`cordis`（禁用）；UI 已有审批卡、问题卡、provider 流式/取消；每个工具包至少一条行为测试，`scripts/agent-e2e.sh` 覆盖 9 条人机流程（审批/拒绝/取消/ask-user/bash/present/多步；retries=0）。
- **B15 后最终状态（2026-10-31）**：无 `partial`/`unsupported`/`experimental` 工具行（矩阵生成物为准，
  `scripts/agent-tool-matrix.py --check` 绿）。B1–B5 已收敛：
  `terminal` 每会话多会话（`open/status/send/close/list`，session id 校验、每会话上限 8）；
  `pwsh_persistent` 真实长驻 pwsh 会话（pwsh 缺失/非 Windows 诚实返回 `TOOL_UNSUPPORTED_PLATFORM`）；
  `goal` 会话级目标 + 有界 round driver（`set/status/run/cancel`；每轮一次 agent turn；`rounds`/`max_rounds`
  持久化，默认 10、clamp 1..100；重启恢复同一轮不重复计数；无可用 turn runtime 时显式报错）；
  `ralph` 每轮 `run_fresh`（独立 child conversation id，不共享 memory）；
  `agent-team` coordinator + 有界 async worker 扇出（cap 8/depth 3、bounded aggregate wait、超时中断；
  ADR-070；单 worker 失败→`partial`）。
  设计边界（非缺口）：`call-timeout` 为 agent 级策略（`AppSettings.tools`，C3 豁免、无独立包）；workflow 为
  线性/DAG 拓扑执行；`subagent_fork` 共享上下文快照而非实时历史；`tool-cordis` 由 C2 禁用。
- `schedule` 执行器默认关闭；`AppSettings.agent.schedule {enabled, scan_seconds}`（设置页“调度/自动化”，
  scan 5..3600s）配置并在 settings.set 后热生效。优先级：**已存在 settings 文件/显式保存的 settings（含默认
  `{enabled:false, scan_seconds:10}`）> env**；`ZEOKEEL_ENABLE_SCHEDULE_AUTORUN=1` 仅在**没有 settings 文件的
  新装/首启**时作为兜底。执行器读取工作区 `.zeokeel/schedules.json` 并自动起 turn；条目必须带 `schedule` 工具
  写入的 `origin` 标记、按 `interval_seconds` 重复（cron 在创建时校验但不解释）。
- 完整矩阵与测试状态见 `docs/generated/AGENT-TOOL-MATRIX.md`（由 `scripts/agent-tool-matrix.py` 生成，为对照参考、**不是产品 SSOT**；权威 writer 见 `docs/24` §3）。
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
| Usage | 流式 `FinalResponse` 与非流式 `PromptResponse` 都取 rig `Usage`（input/output/total/cached/cache_creation/reasoning/cost） | `RigAgent::last_usage() -> Option<UsageSnapshot>`；hook `TurnCompleted{usage}`。`AgentReply.usage` 契约已落地（`AgentUsage`：input/output/total/cached/reasoning，未报告归一为 0）；**B7（2026-10-31）**：新增 additive `AgentEvent::Usage{usage}`，在 terminal 事件（`TurnCompleted`/`TurnCancelled`）前恰好发出一次；`TurnCompleted.reply.usage` 保留兼容。桌面把 usage 持久化到 session `Message`（SQLite v4 `usage_json`），重启加载后 UI 经 `ChatMessage.usage` 恢复展示。 |
| 多轮 memory / 历史回放 | **B8（2026-10-31）**：桌面会话 DB 是回放权威——`run_turn` 调 `SessionStore::messages_in(workspace, conversation)` → `Agent::run_stream_in_shared_with_history` → rig `AgentRunner::history`；只回放 user/assistant 文本（tool 行是纯文本、缺配对 tool-call id，不回放），去掉刚持久化的当前 prompt，尾部上限 40 条。有回放历史时**不**再挂 rig-memory（避免双份上下文）；无历史（独立 `RigAgent` 用法）时仍用 `rig-memory` DirStore。 | `AgentRuntimeConfig.memory{enabled,dir}`；目录优先级 `dir` > `ZEOKEEL_AGENT_MEMORY_DIR` > `$ZEOKEEL_HOME/agent-memory`；无目录时发 `MemoryUnavailable` hook（不静默）。 |
| Hooks | `HookSink` 结构化事件：TurnStarted/TurnCompleted(usage/cancelled)/ToolStarted/ToolFinished/ApprovalRequested/ApprovalResolved/MemoryUnavailable/TurnFailed；可选 JSONL 追加 | `RigAgent::with_hook_sink`；`AgentRuntimeConfig.hooks{enabled,log}` / `ZEOKEEL_AGENT_HOOKS_ENABLED/_LOG`；`cap-hooks` 动态 availability（host 可 `set_available` 覆盖）。 |
| Vector/embeddings | `cap-vector`：OpenAI-compatible embedding model + cosine 索引（insert/insert_many/search/clear/len）。rig 适配在共享 `crates/zk-kit-media`（`kit.media` 的 `EmbeddingConfig`/`EmbeddingModel`/`embed_text`），cap-vector 不再直接依赖 rig-core/llm-rig（ADR-067）。**可选持久化**：配置 `ZEOKEEL_VECTOR_DIR`（或设置页 `AppSettings.embedding {base_url,api_key,model,dims,dir}` / `configure_with_dir`）后索引存 `vector-index.json`（versioned `{version, model, dims, width, documents:[{id,text,embedding}]}`），`insert_many` 为批量原子提交（全部 embedding 成功后一次 save），insert/clear 原子写（temp `0600` → rename，目录 `0700`，已存在目录也收紧为 `0700`）；上限 10k docs / 16MB（`VectorError::Capacity`，文档数在发起 embedding 前预检）；缺失/损坏/不可读/超限/**模型或维度指纹不匹配**时降级为空索引并暴露 `persistence_reason()` + stderr 日志（不 panic），下次成功保存修复文件。 | `configure(EmbeddingModel)`（读 `ZEOKEEL_VECTOR_DIR`；用持久化状态替换内存索引，无 dir 时为空）、`configure_with_dir(model, dir)`、`configure_from_env()`（`ZEOKEEL_EMBEDDING_BASE_URL/_API_KEY/_MODEL/_DIMS` + `ZEOKEEL_VECTOR_DIR`）、`unconfigure()`、`persistence_dir()`/`persistence_reason()`；未配置时 `available()==false` + `unavailable_reason()` + `CAPABILITY_UNAVAILABLE`。personal boot 顺序：先 `configure_capabilities_from_env()`（env fallback），有保存设置时再用 `AppSettings.embedding` 覆盖 env；优先级 **显式 settings > env > 未配置（capability unavailable）**。`api_key` 只写：`settings.get` 掩码为空，保存空值保留已存 key；schema `dims` 边界 1..=65536。配置失败（缺 model、base_url 非 http(s)、vector 配置错误）由 `configure_embedding_from_settings` 返回 `Result` 向上传播：`settings.set` 拒绝且不保存，boot 记录到 `meta.get.agent_config_error`（不再静默）。enterprise 不配置（C7/ADR-031，网关无 embeddings 路由）：设置 UI 隐藏、`settings.set` 拒绝任何本地 embedding 配置、boot 忽略并清除持久化的 embedding/key。 |
| 媒体/rerank/转写 | round 3 为「诚实不可用」占位；round 4 已用 rig 媒体模型实现，见 §5.3 | `cap-audio/cap-image/cap-rerank/cap-transcription`；错误码 `CAPABILITY_UNAVAILABLE`。 |
| 异步子代理 | `RigSubagentRunner` 后台线程执行子代理（复用同一 model/tools），`subagent_start` 立即返回 id；注册表保存 `running/done/failed/cancelled` + 最终结果；`subagent_status` 查询、`subagent_interrupt` 置共享取消标志；**存活线程**并发上限 8（RAII，取消/panic 后线程退出才释放）、深度上限 3；children/kit 记录各上限 64、淘汰最旧已完成（running 保留；evicted→NotFound）；完成/退出清理会话深度，list_agents 枚举 async 子代理 | 工具：`subagent_start{prompt}`、`subagent_status{id}`、`subagent_interrupt{id}`（`tool-subagent`/`tool-subagent-control` 经生成 glue 的 `extra_tools()` 追加）；kit `SubagentRunner::{start,status,result,interrupt}` + `AsyncAgentStatus`；`RigAgent::{with_cancel_flag,cancel_flag}`；无异步能力的 runner 诚实返回 "async subagents are not supported"。 |
| 工具/审批超时与策略 | `agent-rig` 每轮把已保存的 `tool_timeout_ms` 作为 DynamicTool 执行超时、把 `approval_policy` 作为策略求值输入；桌面 `ApprovalBroker::set_timeout_ms` 在启动与 `settings.set` 后热更新审批等待超时（不再只读 env） | `AppSettings.tools`（`ToolSettings{tool_timeout_ms=120000 clamp 1000..600000, approval_timeout_ms=300000 clamp 1000..1800000, approval_policy: Option<ApprovalPolicy>}`）。优先级：显式 settings > `ZEOKEEL_TOOL_TIMEOUT_MS`/`ZEOKEEL_APPROVAL_TIMEOUT_MS`/`ZEOKEEL_APPROVAL_POLICY` env > 默认；fresh install / 默认-only 文件不发布（env 生效，同 agent settings 的 boot 语义），保存后立即生效；越界值 `settings.set` 回传 UI 错误。 |
| Web search | `tool-web-search` 每次 `call_json` 重新解析端点与超时（`resolve_endpoint()`），运行时仍强制 https（设置校验 + 运行时兜底）；无需重启 | `AppSettings.web_search`（`WebSearchSettings{base_url="", timeout_ms=15000 clamp 1000..120000}`，**无凭据**）；桌面 boot 与 `settings.set` 后发布非密 `RuntimeSettings.web_search`（默认值不发布）；优先级：显式 settings > `ZEOKEEL_SEARCH_URL` env > 内置 `https://api.duckduckgo.com/`；空 `base_url` = 不覆盖（env/默认生效），超时无 env fallback；非 https/非法 URL/越界超时由 `settings.set` 拒绝且不保存，Ajv schema 同步 bound（1000..=120000）。企业版同样生效：这是工具出口端点、不是 AI provider，C7 不适用（C6 弱管控不限制出口）。 |

### 5.3 媒体能力（round 4，2026-10-31）

| 能力 | 实现 | 配置/接口 | 可用性 |
| --- | --- | --- | --- |
| 图像生成 `media.image` | `cap-image`：rig `image_generation`（OpenAI 兼容）→ bytes + 编码探测（PNG/JPEG/WebP/GIF） | `configure[_from_config/_from_env]` / `unconfigure`；env `ZEOKEEL_IMAGE_*`（默认 `openai` / `gpt-image-1`）或 `AppSettings.media`（设置页“媒体能力”，共享 base_url/api_key + model；settings > env；api_key 只写/masking；企业版隐藏/拒绝）；`generate(prompt,width,height)` | 配置模型后可用；未配置 `available()==false` + `unavailable_reason()` + `CAPABILITY_UNAVAILABLE` |
| 语音合成 `media.audio` | `cap-audio`：rig `audio_generation`（TTS）→ 音频字节 | `ZEOKEEL_AUDIO_*`（默认 `openai` / `tts-1`）或 `AppSettings.media.audio_model`；`synthesize(text,voice)` | 同上 |
| 转写 `media.transcription` | `cap-transcription`：rig `transcription` | `ZEOKEEL_TRANSCRIPTION_*`（默认 `openai` / `whisper-1`）或 `AppSettings.media.transcription_model`；`transcribe(bytes,filename)`、`transcribe_file(path)` | 同上 |
| rerank `media.rerank` | `cap-rerank`：rig `rerank`（Voyage AI wire） | `ZEOKEEL_RERANK_*`（默认 `voyageai` / `rerank-2.5`）或 `AppSettings.media.rerank_model`；`rerank(query,documents)` | 同上；`provider=openai` 明确报配置错误（rig-core 0.45 OpenAI wire 无 rerank 端点，源码 `RerankQuirks::unsupported`） |

- 实现位置（C4/C17，round-2 评审 P1-4）：rig 适配只在共享库 `crates/zk-kit-media`（`kit.media`）——`MediaKind`/`MediaConfig`（**Debug 脱敏 api_key**）/`MediaError`/四个 builder（返回不透明句柄 `ImageModel`/`AudioModel`/`TranscriptionModel`/`RerankModel`）/`block_on`/`ModelSlot`，以及 embedding 适配（`EmbeddingConfig`/`EmbeddingModel`/`embedding_model[_from_config]`/`embed_text`，供 `cap-vector`）。`cap-*` 只依赖该 kit + 自己的错误/结果 DTO，**不再直接依赖 `rig-core` 或 `zk-llm-rig`**。capability index 已登记 `kit.media`、`vector.store` 与 `media.{image,audio,transcription,rerank}`（`@capability`）。
- 同步桥：cap-* 是同步 API、rig 模型是 async，`zk_kit_media::block_on(future)` 是唯一共享 runtime；cap-* 的 model slot / configure / available 结构由 `kit.media::ModelSlot` 统一。
- **宿主接线**：personal boot 先调用生成 glue 的 `zk_composed::configure_capabilities_from_env()`（`zk compose` 按组合生成，内部逐个调 cap-* 的 `configure_from_env()`），有保存设置时再以 `AppSettings.embedding` / `AppSettings.media` 覆盖（生成 glue 的 `configure_embedding_from_settings` / `configure_media_from_settings` 返回 `Result`；优先级 **显式 settings > env > 未配置**；空 api_key = 不覆盖/保留 env；`settings.get` 掩码 key、空值保留已存 key）。配置失败会阻断 `settings.set`（不落盘）并在 boot 时记录到 `meta.get.agent_config_error`；媒体为逐能力配置，错误聚合返回，部分成功后设置为未保存状态需重试修正。enterprise boot **不调用**本地配置（C7/ADR-031：企业 AI 只走网关，网关无 image/audio/transcription/rerank 端点）：设置 UI 隐藏媒体/embedding 段、`settings.set` 拒绝本地配置、boot 清理任何 stale 持久化凭据；故 enterprise 媒体能力保持 `available()==false` + `unavailable_reason()`，不得假装可用。
- rig features：workspace 只启用 `audio`+`image`。`pdf` 不启用：workspace 为 `tool-read-document` 固定 `lopdf = "=0.42.0"`，而 rig 0.45 的 `pdf` feature 要求 `lopdf 0.44`（连带 `ecb`/`md-5`/`weezl`，离线未缓存）；`epub` 不启用：其依赖 `epub 2.1.5` 是 GPL-3.0，与项目 MIT 冲突。PDF/EPUB 解析由 `tool-read-document` 自身实现承担。
- 测试：每个 cap 包用 `zk_kit_media::test_support` 的确定性进程内假模型（feature `test-support`，底层 rig `Local<Op>` + fake transport）覆盖「配置后真实调用」与「未配置」两条路径，无网络；真实 provider 为 UNVERIFIED（见 acceptance）。
- ADR：ADR-067（accepted）已把 rig 适配点由 ADR-066 的三处扩展为 `llm-rig`/`agent-rig`/`llm-gateway` + `kit-media`（kit 非包，满足 C4/C17 单一实现）；媒体与 embedding 的 rig 实现都在 kit，`cap-*` 不再直接依赖 rig-core。

### 5.4 最终状态与残余（B7–B10 收敛，2026-10-31 B15）

> 本节为最终口径；以上 §5.2/§5.3 中的历史“剩余/未接”表述以本节为准。B1–B5 工具状态见 §4。

- **B7 Usage 事件（已落地）**：`AgentEvent::Usage{usage}` 为独立 additive 事件（terminal 事件前一次；契约 schema/TS
  已由生成器重新生成）；usage 同时随 `TurnCompleted.reply` 携带、持久化到 session 并在重启加载后恢复展示。
  **无残余**。
- **契约与桌面接线（已落地）**：`AppSettings.agent`/`AgentSettings`、`AgentReply.usage`（`AgentUsage`，未报告计数器
  归一 0）、`Agent::configure_agent` 默认实现已入契约；桌面启动与 `settings.set` 后按 section 发布非密
  `RuntimeSettings{agent?,tools?,web_search?}` 并应用 agent 配置（失败回传 UI 错误）；media/vector 配置由 personal boot
  先跑 `configure_capabilities_from_env()`（env），再应用 `AppSettings.media`/`AppSettings.embedding`（settings > env；
  密钥只写/masking；失败阻断保存并记录 agent_config_error）；enterprise 生成/调用 no-op、UI 隐藏、settings.set 拒绝并
  清理 stale 凭据（C7/ADR-031）。`Agent::configure_agent` 在 agent-rig 生效，其它实现仍为默认 no-op。
- **B8 历史回放（已落地）**：桌面 `SessionStore::messages_in` 成为回放权威并注入 prompt
  （`run_stream_in_shared_with_history` + rig `AgentRunner::history`）；有历史时禁用 rig-memory 以免双份上下文。
  **残余**：tool-call 历史不回放（纯文本行缺 tool-call id）；跨 workspace 隔离由 `messages_in` 校验承担。
- **B9 effect log（子集已落地）**：`AgentRuntimeConfig.effects` + `ZEOKEEL_AGENT_EFFECT_LOG[_ENABLED]` 控制的可选
  JSONL effect log（`packages/client/agent-rig/src/effect_log.rs`，rig `Recorder`+`RecordBuffer`，每条 `EffectRecord`
  经 `rig_core::redact::json_text` 脱敏后追加；`enabled=false` 不落盘，写失败只降级日志不影响 turn）。
  **剩余（known limitation，已确证不可达）**：live run checkpoint/resume（rig 0.45 `AgentRun`/`build_run` 为
  `pub(crate)`，宿主拿不到可恢复 run state，证据 `target/evidence/release-1.0-rc1/spikes/rig-0.45.md`）；
  telemetry 仅 tracing spans、无自定义 sink；`observe` 全量语义未接。
- **B10 上下文窗口（窗口子集已落地）**：`AgentRuntimeConfig.context.window_tokens`（env
  `ZEOKEEL_AGENT_CONTEXT_WINDOW_TOKENS`；缺省 128k 窗口 − 4k 输出预留）在每次 turn 前预算；超预算时**确定性丢弃
  最旧 history**，仍超则 `CONTEXT_WINDOW_EXCEEDED` 诚实报错。**剩余（设计待定）**：rig-memory
  `CompactionPolicy`/summarizer 全量压缩未接（需 summarizer 模型/成本/阈值决策）。
- **B4 ralph fresh-context（已落地）**：`SubagentRunner::run_fresh`（默认委托 `run`）；`RigSubagentRunner` 为每轮生成
  唯一 child conversation id `<conv>-sub-<depth>-fresh-<seq>`（不共享 memory）；`tool-ralph` 每轮改用 `run_fresh`。
  测试：ralph 每轮 fresh 调用计数、fresh id 唯一性/非 fresh 稳定 id。
- **文档 loader**：rig loaders 的 `pdf`/`epub` feature 均未启用（原因见 §5.1/ADR-067），`read_document` 仍用独立实现。
- `cap-vector` 可选 JSON 持久化已实现（`ZEOKEEL_VECTOR_DIR`，原子写/0600/0700、10k docs/16MB 上限、损坏降级 +
  reason）；**剩余**：无跨进程写锁/跨设备同步（本地文件，最后一写者胜），无 SQLite/FTS，enterprise 仍无 embeddings
  路由（C7 不提供）；指纹只校验 model id/dims/width（不含 provider 端模型版本号）。

### 5.5 能力可用性契约

- 每个能力包必须提供动态 `available()`；不可用时给出 `unavailable_reason()`，host 侧统一映射 `CAPABILITY_UNAVAILABLE`，不得静默缺失或假装可用。
- 参考实现：`cap-hooks`（默认可用，env/host 可关）、`cap-vector`（配置 embedding 后可用）、`cap-{audio,image,rerank,transcription}`（配置 rig 媒体模型后可用，未配置诚实不可用 + 原因）。
