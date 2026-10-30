# rig 模型层调研（ZeoKeel / 2026-10-30）

> 调研人：rig-researcher（task-3）
> 方法：crates.io API + docs.rs + 下载 0.45.0 crate 源码逐文件核对 + `/tmp` 两个最小可编译工程（`rig-core + rig-agent` 与 `rig` facade）实测编译。
> 结论先行：**`rig = "0.45.0"`（core/agent/memory 同版本）是真实存在且可编译的组合**；但 rig 处于"每 1–3 周一次 breaking"的高频演进期（0.46.0-rc.1 已发布），**必须把 rig 类型挡在包契约之外**，用 `zk-llm`（契约）+ `zk-llm-rig`（适配，唯一允许 `use rig::*` 的 crate）双层隔离。

---

## 0. TL;DR

| 项 | 结论 |
|---|---|
| 版本 | `rig` / `rig-core` / `rig-agent` / `rig-memory` 稳定版均为 **0.45.0**（2026-10-19 发布）；**0.46.0-rc.1** 已于 2026-10-23 发布；0.45 真实存在 ✅ |
| 组合 | `rig = "=0.45.0"` 会拉入 `rig-core 0.45.0`、`rig-agent 0.45.0`、`rig-memory 0.45.0`（agent 强制依赖 memory），版本自洽 ✅ |
| Tool trait | 不是"name/description/definition/call"四件套：`const NAME` + `type Args/Output/Error` + `description()` + `parameters()`（返回 `serde_json::Value`，不是 `ToolDefinition`）+ `annotations()/footprint()/map_error()` + `async fn call(&self, &mut ToolContext, Args)`；`ToolDefinition` 由 `tool_definition(&tool)` 生成 |
| Agent | **长生命周期对象**（`Clone + Send + Sync`，实测编译断言）；`agent.prompt(..)` 返回 `#[must_use] AgentRunner`（一次 run），`.await` / `.stream()` / `.run_channel()` 三选一驱动 |
| max_turns | **默认 1**（一次模型调用，不做工具回环），必须显式 `.max_turns(n)` |
| Provider | deepseek / openai / ollama / openrouter 均有；OpenAI 兼容自定义走 `OpenAIConfig::new(key).with_base_url(..)`，也可自定义 `Dialect` 常量；另有 `Catalog::connect("deepseek/deepseek-chat")` + 可序列化 `ProviderConfig` |
| Memory | 0.45 把"会话持久化"独立成 `rig-memory`（append-only 日志、CAS 写入、压缩、token 计数），**不是向量库**；向量库在 rig-core（trait）+ 独立后端 crate（lancedb/qdrant/postgres/...） |
| 可观测 | `tracing` gen_ai span + 细粒度 `AgentHook`（可改写请求/结果、可拦截工具派发）+ `observe::ObservationLog` + `Recorder`/`rig-cassette`（JSONL 录制回放） |
| 结论 | 契约层零 rig 依赖；`zk-llm-rig` 独占 rig；`zk-agent-rig` 独占 rig-agent。`Agent/AgentBuilder/AgentRunner/Handlers/Serve/Tool/Model/PromptError/ProviderError/ToolExecutionError/Message` 一律不进出包契约 |

---

## 1. 版本事实（实测，非记忆）

### 1.1 crates.io 实测（2026-10-30）

```bash
# crates.io API 需要 User-Agent；proxychains 可用，直连亦可用
proxychains -q curl -s -H "User-Agent: zeokeel-research/1.0" \
  https://crates.io/api/v1/crates/rig
```

| crate | 稳定最新 | 预发布最新 | 0.45.0 发布日期 | 备注 |
|---|---|---|---|---|
| `rig` | **0.45.0** | 0.46.0-rc.1 | 2026-10-19 | facade + 可选 companion 集成 + 自带 launcher 二进制 |
| `rig-core` | **0.45.0** | 0.46.0-rc.1 | 2026-10-19 | 模型/工具/流/向量库等契约与内置 provider |
| `rig-agent` | **0.45.0** | 0.46.0-rc.1 | 2026-10-19 | 经典 agent runtime（`rig::agent`） |
| `rig-memory` | **0.45.0** | 0.46.0-rc.1 | 2026-10-19 | 会话存储/压缩（`rig::memory`，feature `memory`） |
| `rig-macros` | 不存在 | — | — | 派生宏 crate 叫 **`rig-derive`**（`#[rig_tool]`、`Embed`、`ContextValue`） |

版本线（crates.io `versions`，倒序）：

```
0.46.0-rc.1 2026-10-23
0.45.0      2026-10-19   <- ZeoKeel-Next 钉的版本
0.44.0      2026-10-07
0.43.0      2026-09-30
0.42.0      2026-08-17
0.41.0      2026-07-28   <- 从这里开始拆成 rig-core + rig-agent + rig facade
0.40.0      2026-07-10
0.39.0      2026-06-19
0.38.x      2026-06          ...
```

- `rig` crate 的 `default_version = 0.45.0`，`max_stable_version = 0.45.0`，`max_version = 0.46.0-rc.1`。**0.45 是当前最新稳定线**，不是未来版本，也不是旧版本。
- `rig-agent` 是 2026-07-19 才从 rig 拆出的独立 crate（0.0.0 → 0.41.0），`rig-memory` 2026-05 起独立。三者互相用 `0.45.0`（caret）约束；`rig-agent` 无条件依赖 `rig-memory`。
- 包元数据：`edition = 2024`，**`rust-version = 1.97.1`**（0.45 起提升，changelog 明示 breaking），本机 `rustc 1.99.1` / `cargo 1.99.1` 满足。
- docs.rs 已发布 0.45.0 文档（HTTP 200）：<https://docs.rs/rig/0.45.0/rig/>、<https://docs.rs/rig-core/0.45.0/rig_core/>。

### 1.2 `rig` 0.45 是 facade（不是单 crate）

`rig` 0.45.0 的直接依赖（crates.io `/dependencies` + 实测 `cargo tree`）：

```
normal:  rig-core 0.45.0, rig-harness-protocol 0.45.0
optional companion（feature 按需）：
  rig-agent(agent 默认开), rig-memory(memory), rig-derive(derive 默认开),
  rig-reqwest(reqwest 默认开), rig-tungstenite, rig-rmcp,
  rig-bedrock, rig-candle, rig-fastembed, rig-gemini-grpc, rig-helixdb,
  rig-lancedb, rig-milvus, rig-mongodb, rig-neo4j, rig-postgres, rig-qdrant,
  rig-s3vectors, rig-scylladb, rig-sqlite, rig-surrealdb, rig-typesafeai,
  rig-vectorize, rig-vertexai, rig-cassette(cassette)
```

实测（`rig = "=0.45.0"` 默认 feature，`cargo tree --prefix none | sort -u | wc -l`）：
**实际编译进图的唯一包 216 个**；但 `Cargo.lock` 解析出 **663 个包**（所有 optional companion 都被写进 lock），首次 `cargo fetch` 会把 lancedb/lance/datafusion/fastembed/ort 等一并下载（本机实测），**CI 缓存/`cargo vendor` 会因此明显变重**。若只需要模型层，直接依赖 `rig-core` + `rig-agent`：`Cargo.lock` 244 包、213 个编译包、冷编译约 20s（本机）。

> 结论：**不要依赖 `rig` facade**，直接依赖 `rig-core` + `rig-agent`（+ 需要会话持久化时 `rig-memory`）。这样 lock 图小 2.7 倍，也避开 facade 未来再拆 companion 的扰动。

### 1.3 ZeoKeel-Next 的钉版核实

`rig = "=0.45.0"`（core/agent/memory 同版本）——**真实、可编译、自洽**。实测工程（第 2 节）：

- `rig-core = "=0.45.0"`，`features = ["reqwest","rustls","derive"]`；
- `rig-agent = "=0.45.0"`，`features = ["derive"]`；
- 二者互相约束 `0.45.0`，`rig-agent` 自带 `rig-memory 0.45.0`；
- 用 `rig = "=0.45.0"` facade 的根路径版本也单独编译通过。

**但需要注意两点**：

1. **0.45.0 只发布了 11 天**（2026-10-19），0.46.0-rc.1 已在 5 天后出现。按 0.41→0.45 的节奏（约 1–3 周一个 breaking 版本），0.46 大概率会再破坏 API。钉 `=` 是对的，但必须把升级成本限制在适配层。
2. 0.45 的 breaking 是**语义级**的，不只是路径：消息改为 `Arc<Message>`、`Tool` 合并为唯一 contextual trait、effect bus 换成 inline `Handlers`、持久化 run 升级到 `RUN_FORMAT=5`（旧 run 直接拒绝恢复）、`serde_json` 打开 `preserve_order` + `float_roundtrip`（Cargo feature 全局统一，**会改变整个二进制的 JSON map 顺序与浮点序列化**）。

### 1.4 0.44 → 0.45 中与 ZeoKeel 直接相关的 breaking（摘自官方 CHANGELOG / MIGRATING）

| 变更 | 影响 |
|---|---|
| `rig::core` 移除，统一根路径 `rig::` | 依赖 facade 的代码要改 import |
| effect bus → inline `Handlers`（`rig::handlers`）；`over_bus` → `over_handlers` | 插件注册模型要用 `Handlers::register` |
| `AgentBuilder` 去掉泛型参数（不再有 `WithToolServerHandle` 等状态类型） | 可以直接 `let b: AgentBuilder = ...` 存字段 |
| 唯一 contextual `Tool` trait 取代 `PortableTool`/`ErasedTool`；`ToolDefinition` 增加 `annotations` | 自定义工具直接实现第 3.4 节签名 |
| 消息/请求/工具定义共享 `Arc`（`Arc<Message>`、`Arc<ToolDefinition>`） | 适配层与 GUI 缓存要注意 `Arc::unwrap_or_clone` |
| 会话持久化迁到 `rig-memory`（`AsyncStore`），`RUN_FORMAT=5` | run 存档不向前兼容 |
| `Catalog` 重构：`Resolved`、`get_exact`、`from_overrides`、`ProviderConfig` 可序列化（密钥 `[redacted]`） | 配置层可用它做"provider 包配置" |
| 新的 Bevy ECS kernel `rig-ecs`（与经典 `rig-agent` 并存，**不兼容** 0.44 ECS） | ZeoKeel 不要用 `rig-ecs`，它是 rig 自家 coding agent 的 kernel |

补充（0.44）：工具参数**流式增量**（`StreamEvent::Arguments`，每 delta 一片，`on_tool_call_delta` 每片回调）；typed `GenerationOptions`（reasoning/cache/service_tier/...，不支持的选项默认报错）；工具/消息类型大量 `#[non_exhaustive]`；`PromptError` 统一（`StreamingError` 删除）。

### 1.5 工具链与全局副作用

- 需要 **Rust ≥ 1.97.1**、edition 2024（ZeoKeel 桌面端工具链须满足；本机 1.99.1 OK）。
- rig-core 打开 `serde_json` 的 `preserve_order` 与 `float_roundtrip`。这两个 feature 是 **Cargo 全局统一**的：ZeoKeel 自己的 JSON 序列化（IPC、配置、缓存 key）也会变成"保序 map + 精确浮点"。**契约层/插件 IPC 的稳定性测试要覆盖这一点**。
- rig-core 默认 `rustls`；`reqwest` feature 打开共享 `ReqwestClient`（全局连接池 + 系统代理支持）。若桌面端要接入用户代理/自签证书，优先用 `rig_reqwest::ReqwestClient::from(reqwest::Client)` 注入而不是改 rig。
- `rig-agent` 不强制 tokio：`tokio` 仅 `test-utils` 可选。但 `rig-reqwest` 依赖 tokio 运行时上下文（`rig_http::runtime` 里有 lazy fallback；宿主必须驱动 I/O/timer）。GUI 侧建议统一 tokio runtime。

---

## 2. 最小可编译验证（实测）

### 2.1 环境

```
$ rustc --version   # rustc 1.99.1 (58cf8f9c3 2026-10-13)
$ cargo --version   # cargo 1.99.1
```

网络：crates.io 直连可用（带 User-Agent），海外站点走 `proxychains -q`。为加速解析，probe 工程内放了 **项目级** `.cargo/config.toml`（只影响 /tmp 工程，不动共享配置）：

```toml
[source.crates-io]
replace-with = "rsproxy-sparse"

[source.rsproxy-sparse]
registry = "sparse+https://rsproxy.cn/index/"
```

### 2.2 Probe A：`rig-core + rig-agent`（推荐的最小依赖面）

`/tmp/rigprobe/Cargo.toml`：

```toml
[package]
name = "rigprobe"
version = "0.1.0"
edition = "2024"

[dependencies]
rig-core = { version = "=0.45.0", default-features = false, features = ["reqwest", "rustls", "derive"] }
rig-agent = { version = "=0.45.0", default-features = false, features = ["derive"] }
futures = "0.3"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
schemars = "1"
```

`src/main.rs`（可编译骨架，节选；完整文件见 `/tmp/rigprobe/src/main.rs`）：

```rust
use std::convert::Infallible;
use rig_agent::{Agent, AgentBuilder};
use rig_core::completion::ToolDefinition;
use rig_core::providers::openai::{self, OpenAI};
use rig_core::tool::{tool_definition, IntoToolOutput, Tool, ToolContext, ToolOutput};

#[derive(Debug, serde::Deserialize)]
struct AddArgs { a: i64, b: i64 }

struct Add;

impl Tool for Add {
    const NAME: &'static str = "add";
    type Args = AddArgs;
    type Output = i64;               // 任何 Serialize 值自动实现 IntoToolOutput
    type Error = Infallible;

    fn description(&self) -> String { "Add two integers and return their sum.".into() }
    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": { "a": {"type":"integer"}, "b": {"type":"integer"} },
            "required": ["a","b"],
            "additionalProperties": false
        })
    }
    async fn call(&self, _ctx: &mut ToolContext, args: AddArgs) -> Result<i64, Infallible> {
        Ok(args.a + args.b)
    }
}

fn add_definition() -> ToolDefinition { tool_definition(&Add) }

fn build_agent() -> Agent {
    let model = OpenAI::new("sk-compile-only").completion(openai::GPT_5_2);
    AgentBuilder::new(model)
        .name("calculator")
        .preamble("You are a calculator. Use tools for arithmetic.")
        .temperature(0.0)
        .tool(Add)
        .default_max_turns(8)       // 默认只有 1 次模型调用！
        .build()
}

async fn run_once(agent: &Agent) -> Result<String, rig_agent::completion::PromptError> {
    let response = agent.prompt("What is 2 + 3?").max_turns(4).await?;
    Ok(response.output().to_string())
}

fn stream_run(agent: &Agent) -> rig_agent::prelude::StreamingResult {
    agent.prompt("Count to three.").max_turns(2).stream()
}

fn assert_agent_is_shareable() {
    fn assert_send_sync_clone<T: Send + Sync + Clone + 'static>() {}
    assert_send_sync_clone::<Agent>();
}
```

### 2.3 编译与运行输出（实测）

```
$ cd /tmp/rigprobe && cargo check
    Updating `rsproxy-sparse` index
     Locking 244 packages to highest Rust 1.99.1 compatible versions
   Checking rig-reqwest v0.45.0
   Checking rig-core v0.45.0
   Checking rig-memory v0.45.0
   Checking rig-agent v0.45.0
   Checking rigprobe v0.1.0 (/tmp/rigprobe)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 19.90s

$ cargo run -q
tool: add
json schema: {"type":"object","properties":{"a":{"type":"integer","description":"left operand"},"b":{"type":"integer","description":"right operand"}},"required":["a","b"],"additionalProperties":false}
agent built; streaming type checked
RunSpec::new().effective_max_turns() = 1
RunSpec::new().max_turns = None
```

**验证到了**：真实 `Tool` trait 签名、`ToolDefinition` 由三字段（name/description/parameters）+ annotations 组成、`AgentBuilder::new(model).tool(Add).build()`、`agent.prompt(..).max_turns(..)`、`.stream()` 返回类型、`Agent: Send + Sync + Clone`、`i64` 自动 `IntoToolOutput`，以及**默认 `max_turns` 语义 = 1**（运行期断言）。

### 2.4 Probe B：`rig = "=0.45.0"` facade 根路径（验证 ZeoKeel-Next 的钉法）

`/tmp/rigfacade` 只写 `rig = "=0.45.0"` + `serde_json`，代码全部使用 **0.45 根路径**（`rig::agent::AgentBuilder`、`rig::providers::openai`、`rig::tool::Tool`）：

```
   Checking rig-core v0.45.0
   Checking rig-memory v0.45.0
   Checking rig-agent v0.45.0
   Checking rig v0.45.0
   Checking rigfacade v0.1.0
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 29.18s
```

facade 默认 feature 的 `cargo tree` 直接依赖只有：`rig-agent, rig-core, rig-derive(proc-macro), rig-harness-protocol, rig-reqwest`（+`rig-memory` 经 agent）。**facade 本身不引入 Bevy/launcher**，Bevy 插件体系在 `rig-basics`/`rig-tui`/`rig-harness` 等 companion crate 里，ZeoKeel 不需要。

---

## 3. 核心抽象与真实签名

### 3.1 消息、工具结果与多模态

`rig_core::message`（0.45 起历史消息全部 `Arc<Message>`）：

```rust
pub enum Message { System { content: String }, User { content: Vec<UserContent> }, Assistant(AssistantMessage) }

pub enum UserContent { Text(Text), ToolResult(ToolResult), Image(Image), Audio(Audio), Video(Video), Document(Document) }
pub enum AssistantContent { Text(Text), ToolCall(ToolCall), Reasoning(Reasoning), Image(Image), Opaque(Opaque) }
pub enum DocumentData { Text(..), File(..) }   // 0.44 起
```

- 多模态现状：**图片/音频/视频/PDF 都能进请求**（`UserContent::Image/Audio/Video/Document` + `Document` 的 text/base64），assistant 侧可回传图片；生成方向有 `image_generation`/`audio_generation`/`transcription` 模块（provider 能力而定）。feature：`image`、`audio`、`pdf`、`epub` 可选。
- 但**能力差异由 catalog 描述**：`ModelSpec.input: Modalities { text, image, audio, video, pdf }`、`ModelSpec.tools`、`structured_output`、`caching`、`reasoning`；`Model::check(&request)` 可在**不发请求**的情况下列出"该模型会拒绝的选项"。适配层应把"该模型是否吃图"暴露为能力查询。
- 工具输出也支持富媒体：`ToolOutput` 内部是 `Vec<ToolResultContent>`（Text / Json / Image），工具可以返回图片给模型。
- 注意：`ToolResult` 里的 `is_error` 由 `ToolResult::success/failed` 决定；schema 校验只覆盖 JSON Schema 子集（`type/enum/const/properties/required/additionalProperties/items/minItems/maxItems/minLength/maxLength/minimum/maximum/allOf/anyOf/oneOf/$ref`），`pattern`/`format` 不校验。

### 3.2 Model / Wire / Transport / DynModel（provider 的地基）

```rust
// rig_core::driver
pub struct Model<W, T = DynHttpClient> { pub wire: W, pub transport: T, .. }
pub struct DynModel<Op: Operation> { .. }   // 类型擦除，AgentBuilder::new 接收 Into<DynModel<Completion>>
impl<W, T> From<Model<W, T>> for DynModel<W::Op>  // Model::erase()

// rig_http::http_client（可替换的传输层，只有 3 个方法）
pub trait HttpClientExt: Send + Sync {
    fn send<T, U>(&self, req: Request<T>) -> impl Future<Output = Result<Response<LazyBody<U>>>> ..;
    fn send_multipart<U>(&self, req: Request<MultipartForm>) -> impl Future<...>;
    fn send_streaming<T>(&self, req: Request<T>) -> impl Future<Output = Result<StreamingResponse>> ..;
}
```

- **Wire** = 说什么/怎么解析；**Transport** = 怎么发。`Model = wire + transport`，可 `Model::new(wire, custom_transport)` 或 `.connect(impl HttpClientExt)`。
- 这层对"CI 可替换"极友好：`rig_http::test_utils::{RecordingHttpClient, SequencedHttpClient, MockStreamingClient}` 可以完全离线；`rig_core::test_utils::MockCompletionModel`/`MockScript`/`MockTurn` 可以脚本化模型输出。
- `rig_core::driver::Local::<Op>` 是"透传 wire"，用于本地/进程内 runtime，但**没有 completion 版本**（工具/嵌入类 operation 有）。

### 3.3 Agent / AgentBuilder / AgentRunner

```rust
#[derive(Clone)] pub struct Agent { /* AgentConfig(Arc 化) + ToolServerHandle */ }

impl Agent {
    pub fn prompt(&self, prompt: impl Into<Message>) -> AgentRunner;   // 一次 run，#[must_use]
    pub fn resume(&self, run: AgentRun) -> AgentRunner;                // 断点续跑
    pub async fn chat(&self, prompt: impl Into<Message>, history: &mut Vec<Message>) -> Result<PromptResponse, PromptError>;
    pub fn prompt_typed<T: JsonSchema + DeserializeOwned>(&self, ..) -> TypedRun<T>; // = AgentRunner<TypedOutput<T>>
    pub async fn tool_definitions(&self, prompt: Option<String>) -> Result<Vec<Arc<ToolDefinition>>, ToolServerError>;
    pub fn into_tool(self) -> Result<DynamicTool, EmptyToolName>;      // agent 变 subagent 工具
    pub fn set_model(&mut self, ..) / register_model(..) / required_row() / handlers() ...
}
```

**生命周期**：`Agent` 是**长生命周期对象**，构建成本 = 注册 handler（默认模型/路由/动态上下文索引）+ 工具注册表；`#[derive(Clone)]` 共享 `Handlers`/`ToolServerHandle`，实测 `Agent: Send + Sync + Clone`。**每次请求构造的是 `AgentRunner`**（run 级 overrides：history、max_turns、tool_concurrency、hooks、tool_context、memory、model 选择……），`.await`/`.stream()`/`.run_channel()` 三选一驱动；runner 上的设置不污染 agent。

`AgentBuilder` 关键方法（0.45 实测签名，节选）：

```rust
AgentBuilder::new(model: impl Into<DynModel<Completion>>) -> Self
  .named_model(label, model) / over_handlers(Handlers, owner, HandlerKey)
  .name(..) .description(..)
  .preamble(..) .append_preamble(..) .without_preamble(..) .context(doc)
  .dynamic_context(samples, VectorStoreIndex) / .dynamic_context_handler(..)
  .tool(Tool) .dynamic_tool(DynamicTool) .dynamic_tools(Vec<..>)
  .retrieved_tools(samples, index, ToolSet) / .retrieved_tools_handler(..)
  .tool_server_handle(ToolServerHandle)
  .default_max_turns(usize) .temperature(f64) .max_tokens(u64) .stop(..)
  .tool_choice(ToolChoice) .options(GenerationOptions) .provider_option(..) .additional_params(json)
  .output_schema::<T>() / .output_schema_raw(Schema) / .output_mode(OutputMode)
  .model_route(label, model) / .model_route_handler(..)
  .memory(impl AsyncStore) .conversation(log_name) .record_to(Recorder)
  .add_hook(AgentHook) .record_content_telemetry(bool) .accept_unknown_finish_reasons(bool)
  .build() -> Agent
```

`AgentRunner` 关键方法（一次 run 的配置面）：

```rust
.max_turns(usize) .history(..) .preamble(..) .without_preamble(..) .document(s) / .documents(..)
.temperature(..) .max_tokens(..) .tool_choice(..) .options(..) .provider_options(..) .merge_additional_params(..)
.tool_concurrency(usize)          // 默认 1（严格顺序）
.max_invalid_tool_call_retries(usize) .max_consecutive_malformed_tool_calls(Option<usize>)
.tool_context(ToolContext) .add_hook(AgentHook) .using_model(label) .using_model_value(..)
.memory(store) / .without_memory() / .conversation(log)
.unhandled_invalid_tool_call(UnhandledInvalidToolCall) .record_content_telemetry(bool) .accept_unknown_finish_reasons(bool)
.run() -> impl Future<Output = Result<PromptResponse, PromptError>>
.stream() -> StreamingResult
.run_channel() -> (impl Future<Output = Result<PromptResponse, PromptError>>, RunEvents)
```

**`max_turns` 语义（重要坑）**：`RunSpec.max_turns: Option<usize>`，"total model-call budget for the run. `None` means the protocol default of one call"，`effective_max_turns()` 返回 `max_turns.unwrap_or(1)`。而 `AgentConfig::new` 初始化为 `Some(1)`。**即：默认只发一次模型请求，模型要调工具就会因预算耗尽失败（`PromptError::MaxTurns`）**。工具回环必须显式设 `default_max_turns(n)` 或每次 `.max_turns(n)`。

`PromptResponse`：

```rust
pub struct PromptResponse {
    pub usage: Usage,                          // 整轮聚合；含 cached_input_tokens / cache_creation_input_tokens
    pub completion_calls: Vec<CompletionCall>,  // 每次模型请求的 usage/finish_reason/response_id/raw
    pub messages: Vec<Message>,                 // 本次 run 的增量 transcript（prompt + 已接受 turn + 工具结果）
    pub content: Vec<AssistantContent>,         // 最后一轮内容（结构化输出在线径 Tool 模式下会被塞成单个 text）
}
// output() 是派生方法（取 content 的文本），不是字段
```

### 3.4 `Tool` trait 完整签名（rig-core 0.45.0，`src/tool/contextual.rs`）

```rust
pub trait Tool: Sized + WasmCompatSend + WasmCompatSync {
    /// Unique registration and provider-facing name.
    const NAME: &'static str;
    /// Typed JSON arguments.
    type Args: for<'de> Deserialize<'de> + WasmCompatSend + WasmCompatSync;
    /// Output convertible into Rig's canonical model presentation.
    type Output: IntoToolOutput;                     // 任何 Serialize 的 owned 值自动满足
    /// Typed error returned by direct calls to this tool.
    type Error: std::error::Error + WasmCompatSend + WasmCompatSync + 'static;

    /// Model-facing description.
    fn description(&self) -> String;

    /// JSON Schema for arguments.   <-- 返回 serde_json::Value，不是 ToolDefinition
    fn parameters(&self) -> serde_json::Value;

    /// MCP 风格注解（readOnly/destructive/idempotent/openWorld/title），永不发给模型，
    /// 只给宿主做审批/plan/并发策略；默认空。
    fn annotations(&self) -> ToolAnnotations { ToolAnnotations::default() }

    /// 一次调用可能读写什么（用于同批工具并发调度）；默认从 annotations 推导。
    fn footprint(&self, args: &Self::Args) -> Touches { let _ = args; Touches::from_annotations(&self.annotations()) }

    /// 把作者侧错误归一化为运行时策略/遥测用的 ToolExecutionError。
    fn map_error(&self, error: Self::Error) -> ToolExecutionError { ToolExecutionError::from_error(error) }

    /// Execute the tool.
    fn call(&self, context: &mut ToolContext, args: Self::Args)
        -> impl Future<Output = Result<Self::Output, Self::Error>> + WasmCompatSend;
}
```

要点：

- **`name` 是关联常量 `NAME`，不是方法**；`description`/`parameters` 都是 `&self` 方法，schema 直接是 `serde_json::Value`。
- **`call` 多了 `&mut ToolContext`**（入参上下文 + 宿主侧结果元数据；`ToolContext` 里可 `insert`/`require` 带 `ContextValue` 的强类型值，`insert_result` 发布调用结果元数据）。
- 错误有两层：作者侧 `type Error` 直接传播；到 erased 分发边界统一归一化为 `ToolExecutionError { kind: ToolErrorKind, message, model_output, retryable, code, http_status, refusal, source }`。`ToolErrorKind` = `InvalidArgs | Timeout | Cancelled | NotFound | PermissionDenied | RateLimited | Provider | Network | Other`，每种有默认可重试性与**模型可见反馈**（默认"不透传原始 error 文本"，要透传就 `map_error` 自定义）。
- `ToolEmbedding: Tool` 另有 `embedding_docs/context/init`，用于把工具塞向量库做 RAG 检索。
- 派生宏：`#[rig_tool]` 从普通函数生成工具类型 + `Args` + 大写静态常量，支持 `name/description/params/required`；`#[rig(ContextValue)]` 派生上下文键。运行时工具用 `DynamicTool::{new,new_with_context}` + `run/run_with`。

### 3.5 `ToolDefinition`（发给模型的 wire 形状）

```rust
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct ToolDefinition {
    pub name: ToolName,                 // 校验过合法字符的 newtype
    pub description: String,
    pub parameters: serde_json::Value,  // JSON Schema
    #[serde(default, skip_serializing_if = "ToolAnnotations::is_empty")]
    pub annotations: ToolAnnotations,   // 宿主策略用，不发给模型
}
impl ToolDefinition {
    pub fn new(name: ToolName, description: impl Into<String>, parameters: serde_json::Value) -> Self;
    pub fn with_annotations(self, ToolAnnotations) -> Self;
}
// 辅助函数（rig_core::tool）：
pub fn tool_name<T: Tool>() -> ToolName;         // NAME 非空在编译期断言
pub fn tool_definition<T: Tool>(&T) -> ToolDefinition;
```

实测生成的 JSON Schema（`tool_definition(&Add).parameters` 原样打印）：

```json
{"type":"object","properties":{"a":{"type":"integer","description":"left operand"},"b":{"type":"integer","description":"right operand"}},"required":["a","b"],"additionalProperties":false}
```

rig 对 provider 侧的 schema 会做 **sanitize**（OpenAI 系：补 `additionalProperties:false`、去掉 `$ref` 兄弟、`oneOf` 并入 `anyOf` 等），所以**不要假设你写入的 schema 会 1:1 出现在请求体**。

### 3.6 工具注册：静态 / 动态 / 检索 / agent-as-tool

- `ToolSet`：插入序容器（`add_tool/add_dynamic_tool/add_retrieved_tool/remove_tool`，`iter()` 有稳定顺序 → **工具定义顺序稳定，prefix cache 友好**）。
- `ToolServerHandle`（`Agent::tool_server_handle() -> &ToolServerHandle`）支持**运行时热插拔**：
  ```rust
  pub fn add_tool<T: Tool + 'static>(&self, tool: T);
  pub fn add_dynamic_tool(&self, tool: DynamicTool);
  pub fn add_registered_tool(&self, tool: RegisteredTool);
  pub fn add_tools(&self, toolset: ToolSet);
  pub fn remove_tool(&self, tool_name: &str);
  pub fn snapshot(&self) -> ToolCatalog;     // 只读目录
  pub fn toolset(&self) -> ToolSet;
  pub async fn tool_defs(&self, prompt: Option<String>) -> ...;
  ```
  契约：**模型的某一轮会 pin 住它开始时看到的注册**；运行中替换/删除对**下一轮**生效（已开始的 dispatch 保持原 handler）。这正是插件包热插拔的语义。
- `RegisteredTool::execute(args: String, &mut ToolContext) -> ToolResult`（不经 agent 直接执行）；`ToolCatalog`/`ToolSet::execute` 同理。
- `Agent::into_tool()` 把 agent 变成 `DynamicTool`（子 agent 组合）；`AgentBuilder::model_route` 可按 label 做运行时模型路由；`model_select` hook 可每轮选模型。

### 3.7 Streaming 与增量

```rust
pub type StreamingResult = WasmBoxedStream<'static, Result<MultiTurnStreamItem, PromptError>>;

#[non_exhaustive]
pub enum MultiTurnStreamItem {
    StreamAssistantItem(Item<StreamEvent>),  // provider 增量：Start/Text/Reasoning/Arguments/End
    ToolCall { tool_call },                  // 已提交到历史的调用（按提交顺序）
    ToolExecutionCommitted { tool_call },    // 实际执行并提交（批量结算后一次性出现，不是实时开始）
    ToolResult { tool_result },
    CompletionCall(CompletionCall),          // 每次模型请求的 usage/finish_reason/raw
    ModelTurnRetried { turn },               // 该轮文本/推理增量是"临时"的，消费方应丢弃
    FinalResponse(PromptResponse),           // 终止项
}

pub enum StreamEvent {                       // rig_core::streaming
    Start { part: Part, kind: PartKind, name: Option<ToolName> },
    Text { part, text: String },
    Reasoning { part, text: String },
    Arguments { part, json: String },       // 工具参数增量片段（0.44 起）
    End { part, content: AssistantContent },
}
```

- 流式工具调用的增量：每个 provider delta 一片 `Arguments`，拼起来才是模型发的 JSON；`rig_core::streaming::parse_partial_arguments` 可解析不完整前缀（best-effort）。**执行发生在该轮结束、参数校验通过之后**，不会边流边执行。
- 工具事件不是实时生命周期：`ToolExecutionCommitted`/`ToolResult` 是**整批结算成功后**才投影出来（`tool_concurrency` 各值一致）；实时 start/result 观测要用 hooks（`on_dispatch`/`on_outcome`）。
- 中断：
  - **drop stream / drop future 即取消**（`Handlers` 文档明示 "dropping the future or stream cancels it"；`rig-reqwest` 说明 "dropping an operation cancels local work, not accepted remote work"）。
  - 取消表现为 `PromptError::Cancelled { reason, chat_history }`（history 已用 `pairing::close_pending` 补齐 dangling tool call）；`on_run_settled` 也会拿到 outcome。
  - 需要"暂停后继续"用 `AgentRun`（可序列化）+ `Agent::resume(run)`；`RUN_FORMAT=5`，同 rig 版本内可恢复，**跨版本不可读**。
  - `run_channel()` 返回 `(future, RunEvents)`，适合 GUI 的事件循环（有界 channel，事件推送与 run future 分开驱动）。
- `stream_to_stdout`、`RunEvents`、`MultiTurnStreamItem: Serialize+Deserialize`（0.44 恢复）→ 可以把流式事件**逐条转 IPC**给前端。

### 3.8 Memory（会话持久化，不是向量库）

0.45 的"memory"分两层：

1. **`rig_core::conversation`**：`Conversation`/`Entry`/`Aside`（带稳定 id、作者、fork 语义）+ `pairing` 不变量（tool call ↔ result 配对校验）。
2. **`rig-memory`**（`rig::memory`，feature `memory`；`rig-agent` 强制依赖）：
   - `recent`（`LastMessages`/窗口视图）、`CompactionPolicy`/`Summarizer`/`apply_summary`/`undo_summary`（长对话压缩）、`estimate_tokens`/`TokenCounter`（请求 token 预算、output_room）、`history/as_of`、`fork`、`rewrite`。
   - **存储**：`MemoryStore`（内存） / `DirStore`（本地目录，跨进程锁） / 实现 `AsyncStore`（DB、IndexedDB 等）。`AsyncStore` 只有 5 个 async 方法：`read_log_async / append_log_async(len, bytes) / replace_log_async(len, bytes) / read_file_async / write_file_async`；append/replace 带 **CAS 长度校验**（并发两个 save，第二个 `SessionError::Changed` 且不写入）。
   - Agent 接入：`AgentBuilder::memory(store).conversation("log-name")`；每次 run 前 load、成功后一次性 append；run 失败/被取消则不保存；load/save 失败会让 run 失败（`PromptError::Memory { error, response }`）。
   - 文件格式单一（`FORMAT`/`SESSION`），旧格式不兼容（0.45 明确删除旧 memory policy 体系，无兼容层）。
3. **没有**内置的"向量记忆/长期记忆"：0.45 把旧 `MemoryPolicy`/`ConversationMemory`/向量检索式 memory 全删了；动态上下文检索走 `AgentBuilder::dynamic_context(samples, VectorStoreIndex)`（每轮检索 N 条文档塞进请求）。

### 3.9 Vector store / embeddings

```rust
pub trait VectorStoreIndex: Send + Sync {
    type Filter: SearchFilter + Send + Sync;
    fn top_n<T: DeserializeOwned + Send>(&self, req: VectorSearchRequest<Self::Filter>)
        -> impl Future<Output = Result<Vec<VectorSearchResult<T>>, VectorStoreError>> + Send;
    fn top_n_ids(&self, req: VectorSearchRequest<Self::Filter>)
        -> impl Future<Output = Result<Vec<VectorSearchIdResult>, VectorStoreError>> + Send;
}
pub trait InsertDocuments: Send + Sync { fn insert_documents<Doc: Serialize + Embed + Send>(..) -> ...; }
```

- 内置：`InMemoryVectorStore`（BruteForce / LSH 两种索引策略）、`EmbeddingsBuilder`、`Embed` derive。
- 后端全部是**独立 crate**（facade feature）：`rig-lancedb / rig-qdrant / rig-postgres / rig-sqlite / rig-mongodb / rig-neo4j / rig-surrealdb / rig-milvus / rig-helixdb / rig-s3vectors / rig-scylladb / rig-vectorize`；本地 embedding 是 `rig-fastembed`。
- `VectorStoreIndex` + `DynamicSearchFilter` 自动实现 `Tool`（`search_vector_store`），也可作为 `dynamic_context` 检索源 —— 这是"知识包"最自然的接缝。

### 3.10 Provider 层

**内置（rig-core 0.45 `src/providers/` 目录实测）**：

```
anthropic azure chatgpt cohere copilot deepseek gemini groq huggingface hyperbolic
llamacpp minimax mira mistral moonshot ollama openai openrouter perplexity together
venice voyageai xai xiaomimimo zai (+ 独立 crate: bedrock/vertexai/gemini-grpc/candle/typesafeai)
```

- **deepseek**：`deepseek::from_env()/new()`，常量 `DEEPSEEK_FLASH`、`DEEPSEEK_V4_FLASH`、`DEEPSEEK_V4_PRO`；dialect 特性 = `supports_response_format:false`（json_object 走 additional_params）、回传 `reasoning_content`、verify `/user/balance`。
- **ollama**：`Ollama::new()`（本地 `http://localhost:11434`）；**0.44 起 `completion` 走 OpenAI 兼容 `/v1`**，要 `num_ctx/options/think/keep_alive` 用 `native_completion`（原生 `/api/chat`）。
- **openrouter**：`providers/openrouter`（completion/audio/transcription/extension），dialect 在 `openai::wire::OPENROUTER`。
- **openai 兼容自定义**（最常用）：
  ```rust
  use rig_core::providers::openai::{OpenAIConfig, OpenAI};

  // 1) 简单：沿用 OpenAI dialect，只换 base_url（vLLM / one-api / LiteLLM / 自建网关）
  let model = OpenAIConfig::new("sk-xxx")
      .with_base_url("http://127.0.0.1:8000/v1")
      .completion("my-model");           // 或 .chat("my-model") 强制 Chat Completions

  // 2) 完整：自定义 Dialect（字段全 pub；Dialect::gateway 是 const fn；Quirks 可定制）
  use rig_core::providers::openai::wire::{Dialect, Quirks};
  static MY_GW: Dialect = Dialect {
      base_url_env: Some("MY_GW_BASE_URL"),
      ..Dialect::gateway("my-gw", "https://gw.example/v1", "MY_GW_API_KEY")
  };
  let model = OpenAIConfig::from_env_with(&MY_GW)?.completion("my-model");
  ```
  另外 `OpenAIConfig::with_route(Route::Chat|Route::Responses)` 选端点；`with_system_instructions_placement` 处理 Responses 指令位置；`with_strict_tools()`/`with_tool_result_array_content()` 处理兼容性开关。
- **配置/连接**：`catalog::Catalog::builtin().connect("deepseek/deepseek-chat")` 用 `DEEPSEEK_API_KEY` 直接建 `DynModel<Completion>`；`ProviderConfig`（OpenAi/Anthropic/Gemini 三族）可序列化，`Secret` 序列化为 `[redacted]`、反序列化得到空密钥（**持久化配置不能存密钥**，需要宿主 keychain 注入）。`ConnectOptions` 可换 dialect/transport。
- **catalog**：内置 1,492 行模型事实（context window、output limit、多模态、reasoning、caching、pricing、deprecated、以及 wire 兼容 quirks），可 `Model::check` 预检、`Usage::cost` 计价。

### 3.11 可观测性

三层，互相独立：

1. **tracing / OpenTelemetry**：agent/chat/tool 各有 span（`gen_ai.*` 语义约定；流式记 `gen_ai.request.stream`），run 有 RunId；`record_content_telemetry(bool)` 决定是否把 prompt/response/tool 内容写进 span（默认 false → 不泄露用户内容）。`rig_core::telemetry::SpanCombinator`；测试有 `test_utils::TraceCapture` 捕获 span/event。
2. **`AgentHook`（细粒度、可改写、可拒绝）**：
   ```rust
   pub trait AgentHook: Send + Sync {
       fn name(&self) -> Option<String>;
       async fn on_run_start(&self, &HookContext, RunStart) -> RunStartAction;                  // 可改写 prompt / 停止
       async fn on_run_settled(&self, &HookContext, RunSettled);
       fn on_model_select(&self, &HookContext, ModelSelection) -> ModelSelectionAction;        // 每轮选模型
       async fn on_completion_call(&self, &HookContext, CompletionCallEvent) -> CompletionCallAction; // RequestPatch：换 preamble/temperature/tool_choice/active_tools/extra_context/history
       async fn on_model_turn_finished(&self, &HookContext, ModelTurnFinished) -> ModelTurnAction;    // 接受/重试/停止
       async fn on_invalid_tool_call(&self, &HookContext, &InvalidToolCallContext) -> Option<InvalidToolCallAction>;
       async fn on_text_delta / on_reasoning_delta / on_tool_call_delta(..) -> ObservationAction;      // 流式增量观测，可 stop
       async fn on_dispatch(&self, &HookContext, DispatchEvent) -> DispatchAction;   // Proceed/Patch/Deny/Skip/Stop/rewrite_tool_args —— 审批门、参数脱敏
       async fn on_outcome(&self, &HookContext, OutcomeEvent) -> OutcomeAction;      // 改写 tool output / tool result / completion
       fn observes(&self, StepEventKind) -> bool;   // 裁剪高频 delta 噪声
   }
   pub struct HookStack; // 注册顺序、first-stop-wins、patch 后写覆盖
   ```
   `HookContext` 提供 `run_id/turn/is_streaming/agent_name/scratchpad`（类型化共享状态）、`bind/index/model/tool` 句柄、`append_entry`（可序列化、随 run 走的 aside）。
3. **`observe` 与录制/回放**：
   - `rig_core::observe::{ObservationLog, ObservationSink, Witness, Clock}`：有界、带 seq/stage/emitter 的**类型化事实流**，适合 GUI 状态栏/审计，不依赖 tracing。
   - `Recorder`（handler 表级）+ `rig-cassette`：JSONL effect 日志、按 by-id 回放、checkpoint、shape-matched 回放（`RIG_CASSETTE_MATCHING`）、request snapshot 校验；**这是 "CI 可替换" 最现成的能力**——录制真实 provider 会话，在 CI 回放。

---

## 4. 与 cordis 插件模型的接缝

### 4.1 意外收获：rig 自带的 dispatch 层非常"cordis"

0.45 把旧的 effect bus 换成了 **inline `Handlers`**（key → `ErasedHandler`），核心概念：

```rust
// rig_core::effect
HandlerKey          // 字符串键，形如 [owner/][kind:]label[#generation]，可序列化
Key<F: Family>      // 带 family 类型证明的 HandlerKey
EffectKind          // Completion / ToolCall / Embed / Retrieve / Rerank / Custom
Answered            // 处理结果
EffectRow           // 一组 key→family，声明"这次 run 需要谁在场"
// rig_core::serve
trait Serve { type Family; fn descriptor() -> HandlerDescriptor; async fn serve(&self, EffectKind, Dispatch) -> Reply; }
trait Intercept { fn name(); async fn before(id, &EffectKind) -> BeforeServe;   // Proceed/Patch/Deny
                  async fn after(id, &EffectKind, &Result<Answered, ErrorReport>) -> AfterServe; } // Keep/Replace
Handlers::register(impl Into<HandlerKey>, impl Serve) / deregister / bind::<F>(key) / record_to(Recorder)
```

- 分发是 **inline** 的：在调用方 future 里执行 → **drop future 即取消**；没有后台 driver task/backpressure。
- 替换/删除只影响**之后开始**的 dispatch；模型一轮的工具注册被 pin 住。
- `Intercept` 就是中间件/策略层（审批、plan mode、脱敏），`Recorder` 就是录制回放层。

这与 cordis 的 service/plugin/中间件高度同构。**建议**：cordis 拥有权威注册表，`zk-llm-rig` 把 cordis 的服务映射成 rig 的 handler/table 或反向包装；**不要让 rig 的 `HandlerKey`/`Serve`/`EffectKind` 进入包契约**，否则内核与 rig 深度耦合。

### 4.2 cordis 包 → rig 扩展点映射

| cordis 包角色 | rig 侧接缝 | 备注 |
|---|---|---|
| 模型/provider 包 | 注册 `family::Completion` handler（`Handlers::register` + `AgentBuilder::over_handlers`）；或实现契约 `ChatModel` 由适配层包成 `DynModel` | rig 的 provider 不是 trait 插件，而是 `Wire + Transport` + 静态 `Dialect`；自定义供应商走 `Dialect`/`with_base_url` |
| 工具包 | 契约 `ToolSpec + handler`；适配层转 `DynamicTool` 或 typed `Tool`，注册进 `ToolServerHandle`（可热插拔）或 `ToolSet` | `ToolServerHandle::add/remove` 运行时生效（下一轮）；`announcements/footprint` 可驱动审批与并发 |
| 传输/代理包 | 实现 `HttpClientExt`（3 方法），`OpenAIConfig::connect(client)` 注入 | CI 用 `rig_http::test_utils` 的 Recording/Mock client |
| 记忆/会话包 | 实现 `rig_memory::AsyncStore`（5 个 async 方法）或直接实现契约 `ConversationStore` | 保存是 CAS append；`DirStore` 可参考 |
| 知识/向量库包 | 实现 `VectorStoreIndex`（2 方法）+ `InsertDocuments`；或实现契约 `Retriever` | 可作为 `dynamic_context` 或 `retrieved_tools` |
| 策略/审批包 | 实现 `Intercept` 或 `AgentHook::on_dispatch`（Deny/Skip/Patch/rewrite args） | rig 的 `annotations/footprint` 提供 MCP 风格 hint |
| 观测包 | `tracing::Layer` / `AgentHook` / `Observer`(`ObservationSink`) / `Recorder` | 录制回放 = rig-cassette（可选依赖，不一定要用） |
| UI/渲染包 | 消费 `MultiTurnStreamItem`（可 serde）或 `RunEvents` | 事件逐条过 IPC；注意 `ModelTurnRetried` 要回滚临时输出 |

### 4.3 生命周期、并发、取消（结论）

- **Agent 长生命周期、可克隆、可共享**（编译期已断言 `Send+Sync+Clone`）；每次请求 new 一个 `AgentRunner`。GUI 建议 `Arc<Agent>`（或 `Arc<RwLock<Agent>>` 仅当要 `set_model`），run 状态放 runner/前端。
- 同一 `Agent` 上并发 run 是受支持的：run 级配置都在 runner，`Handlers`/`ToolServerHandle` 内部 `RwLock`/`Arc` 共享；工具并发由 `.tool_concurrency(n)` 控制（默认 1 = 严格按调用顺序串行；`0` 与 `1` 同义）。工具批**整体原子提交**：任一 sibling 终止/失败 → 不再启动新调用、已启动的 drain 掉、按**最小 call index 的确定性错误**返回；成功才按 call 顺序提交结果。**没有 per-call 取消 API**（取消粒度 = run/stream，或工具自己实现超时并映射 `ToolErrorKind::Timeout`）。
- 取消 = drop stream/future（或 hook stop）；得到 `PromptError::Cancelled` + 规范化 history；要续跑，持久化 `AgentRun`（同版本）再 `resume`。
- 并发要点：`on_completion_call`/`on_model_select` 是同步/短异步、不能阻塞 I/O；hook 里的 stop 会终止整个 run；`HookStack` 依次执行、first-stop-wins。

### 4.4 prefix cache 友好性

rig 给的材料：

- system prompt 静态：`preamble` + `static_context` 每轮重发、顺序稳定；工具定义按 `ToolSet` 插入序生成 → **稳定**。
- **破坏 prefix cache 的写法**：① `preamble` 里塞时间戳/随机 id；② 用 `on_completion_call` 的 `RequestPatch::preamble` 每轮改 system；③ `dynamic_context`/`retrieved_tools` 每轮插入不同文档（位置在请求前部）；④ 工具热插拔导致 tools 数组变化（只在变化那一轮破坏）。
- 显式 cache 控制：`GenerationOptions::cache(CacheRetention)` + `cache_key(conversation_id)`（部分 provider 的 prompt cache 路由键），`Usage.cached_input_tokens`/`cache_creation_input_tokens` 可在 `CompletionCall` 逐次观测；catalog 有 `ModelSpec.caching`（哪些 retention 被模型支持），未列 cache 费率时成本会标为不完整。
- 建议：system prompt/工具集按"包版本"生成并哈希（作为缓存 key 的一部分），把动态内容一律放到 **user message 尾部**或显式标注为每轮变化；把 `AgentRunner.history` 用完整历史（同模型回合 verbatim replay，rig 0.44 起的 `history::adapt` 就是干这个的）。

---

## 5. 已知坑（按踩中概率排序）

1. **默认 `max_turns = 1`**：模型一调工具就到顶，报 `PromptError::MaxTurns`。必须 `default_max_turns(n)` 或每 run `.max_turns(n)`。（源码：`RunSpec::effective_max_turns`、`AgentConfig::new`）
2. **`AgentBuilder::tool()` 只把工具注册进 agent 自带表**；如果同时传了 `tool_server_handle(shared)`，builder 的工具会在 `build()` 时**并入** shared。宿主自管注册表时用 `over_handlers`，否则会拿到两套 owner/key 语义。
3. **0.45 的 tool 事件不是实时的**：`ToolExecutionCommitted`/`ToolResult` 在整批结算后才投影；做进度条/审批要用 `AgentHook::on_dispatch/on_outcome`（或自己包一层工具）。
4. **参数校验是"宽容子集"**：`validate` 只查 JSON Schema 子集（不查 `pattern`/`format`），且 `additionalProperties:false` 未声明时未知字段会被接受（`arguments_refusal` 的 extra 检查依赖 schema 声明）；工具内部要自己二次校验。错误必须映射成清晰的 `ToolExecutionError`，否则模型看到的是泛化反馈。
5. **流式工具参数是碎片**：`Arguments` 逐片到达，片段拼接才是完整 JSON；中途取消/截断会得到 `unfinished_calls`（有 `output_limit_feedback` 之类的官方提示文案可复用）。
6. **工具并发默认串行**；`tool_concurrency(n)` 时结果仍按调用顺序提交，且**失败快、批原子**——不要假设"部分成功"可被模型看到。
7. **错误类型大量 `#[non_exhaustive]`**（`ProviderError`、`PromptError`、`ToolAnnotations`、`MultiTurnStreamItem`、`FinishReason`…）：适配层 match 必须带 `_ =>`，否则 0.46 升级必炸。
8. **`PromptError` 是唯一 agent 错误**（含 provider/memory/max_turns/cancelled/unknown_tool）；`ToolExecutionError` 是工具层错误；`ErrorReport` 是 handler/effect 层错误。三层不要混着透出给 UI。
9. **消息共享 `Arc`**：`PromptResponse.messages`、`CompletionRequest.chat_history`、hook 事件里都是 `Arc<Message>`；要改先 `Arc::unwrap_or_clone`/`Arc::make_mut`，别克隆整段历史。
10. **`serde_json` feature 全局副作用**（`preserve_order`/`float_roundtrip`）：会影响 ZeoKeel 自身的 JSON 序列化顺序/浮点表示；IPC/对齐测试要防回归。
11. **MSRV 1.97.1 + edition 2024**：CI 镜像、Windows/macOS 交叉编译工具链都要 ≥1.97.1；`cargo` 版本锁定。
12. **lockfile 膨胀**：依赖 `rig` facade 时 `Cargo.lock` 663 包（含所有 optional 集成），`cargo fetch` 会下载 lance/datafusion/fastembed/ort；用 `rig-core`+`rig-agent` 直接依赖可降到 244。`cargo vendor` 体积/CI 缓存要预算。
13. **0.46.0-rc.1 已在线上**：别用 `cargo update` 让它漂；`=` 钉死 + 定期手动升级适配层。`cargo install rig`（launcher）与库无关，别装。
14. **持久化 run 不向前兼容**：`RUN_FORMAT=5`，app 升级 rig 后旧 `AgentRun` 存档一律拒绝——存档要么版本化，要么不带 rig 类型（推荐后者，用契约层自己的 transcript）。
15. **`rig-reqwest` 需要 tokio 驱动**：宿主必须驱动 runtime（I/O+timer），否则 I/O 失败/panic；桌面端建议统一 tokio 多线程 runtime，不要在 UI 线程跑 run。
16. **`Agent` 没有实现 `Debug`**（只有 `Clone`）、`Handlers` 不可序列化（只有 key/descriptor 可序列化）；不要把 `Agent` 塞进需要 serde/Debug 的状态桥。
17. **`max_invalid_tool_call_retries` 默认 0**、`unhandled_invalid_tool_call` 默认 `Fail`：模型发错工具名会直接失败而不是"模型自己纠错"，需要的话显式开 retry/ignore 策略。
18. **`PromptResponse.output()` 是派生**（取 `content` 文本），结构化输出在 `content` 里可能是一个 text part 的 JSON；不要读不存在的字段。

---

## 6. 备选方案对比（若 rig 0.45 出现阻塞）

| 方案 | 优点 | 缺点 | 建议用途 |
|---|---|---|---|
| **rig-core + rig-agent 0.45（本方案）** | Rust 生态最完整的 provider/流式/工具/多模态/向量库；handler/拦截/录制回放 可直接支撑"一切皆包"；自带 mock 与 cassette | 破坏式发版（1–3 周）；类型面巨大；MSRV/edition 新；memory/agent 语义仍在演进 | 默认实现；用适配层隔离 |
| `async-openai` | 稳定、贴近 OpenAI API、类型清晰、breaking 少 | 只有 OpenAI 兼容；没有 agent 循环/工具调度/多 provider；流式与工具需自己写 | 作为**第二实现**验证契约（zk-llm-openai）；或 provider 降级路径 |
| `genai` | 多 provider、API 简洁、维护活跃 | 抽象层偏薄（无 agent 循环/记忆/向量库）；版本也常有 breaking；工具流式细节不如 rig | 备选 provider 层实现 |
| 自写薄 HTTP（reqwest + serde） | 完全可控、体积最小、零 breaking；完全贴合桌面端需求 | 要自己实现 SSE 解析、工具增量、重试、多 provider 方言、结构化输出、缓存……约数千行+长期维护 | 仅当只支持 1–2 家且需求极窄时 |
| `rig-ecs` / rig-harness 插件体系 | rig 自家 coding agent 的 Bevy ECS kernel，插件化彻底 | 与 ZeoKeel 的 cordis 内核重复；Bevy 依赖重；0.45 新 kernel 与 0.44 不兼容；非通用库定位 | **不要用**（除非想换掉 cordis） |

**结论**：保持 rig 作为**默认实现**，但契约层必须可被 `async-openai`/`genai`/mock 实现，否则"CI 可替换"落空。

---

## 7. 结论：适配层与 crate 边界

### 7.1 哪些 rig 类型**不进**包契约（必须隔离在 `zk-llm-rig` / `zk-agent-rig` 内）

| rig 类型/模块 | 为什么隔离 |
|---|---|
| `Agent`, `AgentBuilder`, `AgentRunner`, `AgentConfig`, `AgentRun`, `RunSpec`, `PromptResponse`, `TypedRun` | 0.45 刚重构过（去泛型、runner、run spec），0.46 大概率再变；且携带 handler 表/工具服务等 rig 特有句柄 |
| `Tool`（含 `NAME/Args/Output/Error/call(&mut ToolContext)`）、`DynamicTool`, `ToolDefinition`, `ToolName`, `ToolAnnotations`, `ToolOutput`, `ToolResult`, `ToolExecutionError`, `ToolSet`, `ToolServerHandle`, `RegisteredTool` | trait 形态在 0.40→0.45 变过多次（PortableTool→Tool）；`RPITIT + ToolContext` 不可对象化；annotations 仍在演进 |
| `Handlers`, `Serve`, `Intercept`, `EffectKind/EffectId/HandlerKey/Key<F>/EffectRow`, `Recorder`, `Dispatch`, `Reply` | rig 的 effect/dispatch 体系；与 cordis 的 service 注册表是**两套元模型**，暴露会锁死内核 |
| `Model<W,T>`, `Wire`, `Transport`, `DynModel`, `OpenAIConfig`, `Dialect`, `ProviderConfig`, `Catalog`, `ModelSpec` | provider 架构 0.44/0.45 两次大改（catalog/registry/typed options）；`Wire` 是巨型 trait |
| `Message/UserContent/AssistantContent/Text/Image/Document/ToolCall/ToolResult/DocumentData` | 0.44 改 `Arc` + item-shaped history + DocumentData；且会被序列化进持久化，跨版本不可读 |
| `PromptError`, `ProviderError`, `ErrorReport`, `ToolErrorKind`, `FinishReason`, `UnsupportedOption` | 全 `#[non_exhaustive]`，每版都可能加 variant；UI 契约不能直接映射 |
| `AsyncStore`, `Session`, `CompactionPolicy`, `Summarizer`, `TokenCounter`, `Conversation`, `Entry` | 0.45 刚重做，旧 memory 体系**无兼容层** |
| `VectorStoreIndex`, `VectorSearchRequest`, `Embedding`, `Embed`, `VectorStoreError` | filters 在每个后端 crate 里都有专属类型；`Filter` 关联类型泄露后端 |
| `HttpClientExt`, `rig_http::*`, `rig_reqwest::*` | 传输实现可替换，契约层不需要知道 |
| `StreamEvent`, `Item<E>`, `MultiTurnStreamItem`, `Part`, `Streamed`, `Relayed` | 流式事件 0.44 改过 `Arguments` 增量、`ToolCallDelta` 去 call_id；IPC 事件要自己定义 |

### 7.2 可以"暴露形状"的东西 → **自己定义等价类型**

| 概念 | 建议契约类型（零 rig 依赖） |
|---|---|
| 消息 | `zk_llm::Message { role, blocks: Vec<ContentBlock> }`，`ContentBlock = Text | Image{..} | Document{..} | ToolCall | ToolResult | Reasoning` |
| 模型能力 | `ModelCapabilities { context_window, max_output, modalities, tools, structured_output, cache }` |
| 工具 | `ToolSpec { name, description, parameters: serde_json::Value, annotations }`；`trait ToolHandler { fn spec(&self)->&ToolSpec; fn call(...) -> BoxFuture<Result<ToolOutcome, ToolFailure>> }` |
| 流事件 | `StreamEvent = Start | TextDelta | ReasoningDelta | ToolCallDelta | ToolCallReady | ToolResult | Usage | Done`（serde 稳定 tag） |
| 错误 | `LlmError { kind, retryable, code, http_status, provider_message, source, model_visible }`；`kind` 自己定义（Mapping rig `ToolErrorKind`/`ProviderError`） |
| 会话/记忆 | `ConversationStore`（load/append/replace）+ `Conversation`/`Entry` 自有类型（serde 版本化） |
| 检索 | `trait Retriever { async fn search(query, k) -> Vec<Document> }` |
| Agent | `trait AgentRuntime { async fn run(&self, AgentRequest, EventSink) -> Result<AgentOutcome, AgentError> }`，带取消 token（`tokio_util::CancellationToken` 或自有） |

### 7.3 建议 crate 图

```
zk-contract-llm        (仅 serde/thiserror/futures；模型/消息/工具/流/错误的契约类型 + trait)
zk-contract-agent      (AgentRuntime/AgentRequest/EventSink/CancelToken；工具执行契约)
        ▲                        ▲
        │                        │
zk-llm-rig             (唯一 use rig_core::*；Model/Provider/Tool/Stream/Error 双向映射)
zk-agent-rig           (唯一 use rig_agent::*；Agent/AgentRunner/Hook/Memory/会话编排)
zk-llm-openai          (可选第二实现：async-openai 或薄 HTTP；用于契约对拍/降级)
zk-llm-mock            (测试包：脚本化模型/工具；可选用 rig-core 的 MockCompletionModel 但只在 dev-dependencies)
zk-llm-cassette        (可选：录制回放；基于 rig-cassette 或自研 JSONL)
zk-tool-*              (工具包：只依赖 zk-contract-llm + zk-tool-sdk；不依赖 rig)
zk-provider-*          (provider 包：只依赖 zk-contract-llm；由 zk-llm-rig 装配)
```

要点：

- **`rig-core` 只出现在 `zk-llm-rig`，`rig-agent` 只出现在 `zk-agent-rig`**；`zk-contract-*` 的 `Cargo.toml` 里不能出现 `rig`。
- 工具包（zk-tool-*）实现 `ToolHandler`，由 `zk-agent-rig` 在装配期转成 `DynamicTool`/typed Tool；这样工具包**不需要重新编译 rig 版本**，也能被非 rig runtime 复用。
- provider 包贡献 `ChatModel` 契约实现；`zk-llm-rig` 可以把契约实现包成 `DynModel`，或者反过来把 rig 的 `DynModel` 包成 `ChatModel`（推荐后者：rig 做能力提供方，契约做门面）。
- 会话存储包实现 `ConversationStore`；`zk-agent-rig` 提供 `AsyncStore` 适配器（或直接在适配层用 `rig_memory::DirStore` 作为默认实现，但**存档格式由契约层决定**，避免 `RUN_FORMAT` 升级导致用户会话不可读）。
- 录制回放：**优先自研/封装在 `zk-llm-cassette`**，不要把 `rig_cassette` 的 `EffectRecord` 暴露；CI 只要求 "provider 调用可回放" 这个行为。
- 版本策略：`rig-core = "=0.45.0"`、`rig-agent = "=0.45.0"`、`rig-memory = "=0.45.0"` 三件套在 `zk-llm-rig`/`zk-agent-rig` 的 `[dependencies]` 里精确钉死（workspace 统一），并提供 `zk-llm-rig/compat-0.45` 之类 feature；升级 = 只改两个 crate + 跑契约对拍（mock/cassette）。
- CI 矩阵（最小）：
  1. `zk-contract-*` 单测（无网络、无 rig）；
  2. `zk-llm-rig` 用 `MockCompletionModel`/`RecordingHttpClient` 跑工具循环、流式、取消、审批拦截；
  3. 录制回放对拍（cassette/JSONL）；
  4. 真实 provider 冒烟（可选、密钥隔离、按需跑）。

### 7.4 契约伪代码（建议直接落地）

```rust
// zk-contract-llm
#[async_trait::async_trait]
pub trait ChatModel: Send + Sync {
    fn capabilities(&self) -> ModelCapabilities;
    async fn complete(&self, req: ChatRequest) -> Result<ChatResponse, LlmError>;
    fn stream(&self, req: ChatRequest) -> Result<ChatStream, LlmError>; // Stream<Item = Result<StreamEvent, LlmError>>
}

pub trait ToolHandler: Send + Sync {
    fn spec(&self) -> &ToolSpec;
    fn call<'a>(&'a self, ctx: &'a mut ToolCallCtx, args: serde_json::Value)
        -> futures::future::BoxFuture<'a, Result<ToolOutcome, ToolFailure>>;
}

#[async_trait::async_trait]
pub trait AgentRuntime: Send + Sync {
    async fn run(&self, req: AgentRequest, cancel: CancellationToken, sink: EventSink)
        -> Result<AgentOutcome, AgentError>;
}

#[derive(Serialize, Deserialize)]
pub struct ToolSpec {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,   // JSON Schema
    pub annotations: ToolAnnotations,    // 自有镜像，不引用 rig
}
```

`zk-llm-rig` 内部映射（示意）：

```rust
// rig DynModel<Completion> -> 契约 ChatModel
struct RigChatModel { model: DynModel<Completion>, spec: ModelSpec }
// 契约 ToolHandler -> rig DynamicTool（或 typed Tool 的 DynamicTool 包装）
fn to_rig_tool(handler: Arc<dyn ToolHandler>) -> DynamicTool { ... }
// rig AgentHook -> 契约 EventSink（obs/审批）
struct BridgeHook { sink: EventSink, policy: Arc<dyn Policy> }
```

**只要这两层存在，rig 的版本演进（甚至换 genai/薄 HTTP）只影响适配层，包契约和 cordis 内核不需要动。**

---

## 8. 附录

### 8.1 实测命令与证据

```bash
# 版本（crates.io API，需 UA；proxychains 可用）
proxychains -q curl -s -H "User-Agent: zeokeel-research/1.0" https://crates.io/api/v1/crates/rig
proxychains -q curl -s -H "User-Agent: zeokeel-research/1.0" https://crates.io/api/v1/crates/rig-core
proxychains -q curl -s -H "User-Agent: zeokeel-research/1.0" https://crates.io/api/v1/crates/rig-agent
proxychains -q curl -s -H "User-Agent: zeokeel-research/1.0" https://crates.io/api/v1/crates/rig-memory

# 源码（0.45.0 实际内容；本机解包在 /tmp/rigsrc/）
curl -sL https://static.crates.io/crates/rig-core/rig-core-0.45.0.crate | tar xz
# 关键文件：
#   rig-core-0.45.0/src/tool/contextual.rs   Tool trait / DynamicTool / tool_definition
#   rig-core-0.45.0/src/completion/request.rs ToolDefinition / ToolAnnotations
#   rig-core-0.45.0/src/tool/validate.rs      JSON Schema 校验子集
#   rig-core-0.45.0/src/serve/{handler,layer}.rs  Serve / Intercept
#   rig-core-0.45.0/src/effect/{mod,key}.rs       HandlerKey / Key<F> / EffectKind
#   rig-core-0.45.0/src/driver.rs, driver/dyn_model.rs
#   rig-core-0.45.0/src/driver/local.rs, streaming/event.rs
#   rig-core-0.45.0/src/providers/openai/wire.rs, wire/dialects.rs
#   rig-core-0.45.0/src/catalog/spec.rs        ModelSpec（能力/缓存/价格）
#   rig-agent-0.45.0/src/agent/{builder,completion,runner,hook,streaming}.rs
#   rig-agent-0.45.0/src/run/{spec,output,response,patch}.rs
#   rig-agent-0.45.0/src/handlers.rs          Handlers 注册表
#   rig-agent-0.45.0/src/tool/{registry,server}.rs
#   rig-memory-0.45.0/src/{store,session,compaction}.rs
#   rig-http-0.45.0/src/http_client/mod.rs    HttpClientExt（可替换传输）

# 最小编译验证
cd /tmp/rigprobe  && cargo check   # rig-core+rig-agent, 244 packages, 19.90s
cd /tmp/rigfacade && cargo check   # rig facade =0.45.0, 29.18s
cargo run --manifest-path /tmp/rigprobe/Cargo.toml   # 打印 ToolDefinition
```

### 8.2 参考链接

- crates.io：<https://crates.io/crates/rig> · <https://crates.io/crates/rig-core> · <https://crates.io/crates/rig-agent> · <https://crates.io/crates/rig-memory>
- docs.rs 0.45：<https://docs.rs/rig/0.45.0/rig/> · <https://docs.rs/rig-core/0.45.0/rig_core/> · <https://docs.rs/rig-agent/0.45.0/rig_agent/>
- 仓库：<https://github.com/0xPlaygrounds/rig>
- 变更记录：`CHANGELOG.md`（0.45.0 条目，2026-10-19）· `MIGRATING.md#044--045`
- 官方文档站：<https://rig.rs/docs>（含 `docs/agent.md` 编码 agent 参考、插件指南）

### 8.3 交付物

- 本文档：`/data/ZeoKeel/research/rig.md`
- 验证工程（临时，不属于仓库）：`/tmp/rigprobe`（rig-core+rig-agent）、`/tmp/rigfacade`（rig facade）
- 已解包的 0.45.0 源码（临时）：`/tmp/rigsrc/`

> 注：`/tmp` 内容不纳入仓库；若后续需要复现，按 8.1 的命令重建即可（镜像配置只写在 `*.cargo/config.toml` 的 probe 目录内，不污染共享环境）。
