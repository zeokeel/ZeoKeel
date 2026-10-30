# 05 · 客户端运行时：内核宿主、两种版本、Agent 循环、WASM 宿主

> 内核研究结论（`research/kernel.md`）：`zeokeel-cordis` 只提供**组合原语与生命周期语义**，
> 加载/权限/沙箱/运行时/配额/包身份**全部归宿主**。本文件说明客户端宿主由哪些包组成、边界在哪、
> 两种版本（个人/企业）如何在同一份代码里共存，以及哪些内核坑必须绕。
> 服务端运行时见 `06`；API 见 `04`。页面/审批/无障碍见 `19`；窗口 capability/本地载体安全见 `21`；会话存储见 `24` §7。

## 1. 进程与分层

```text
┌─────────────────────────── 桌面进程（单二进制）───────────────────────────┐
│ WebView（React，由 UI 包组装）                                             │
│        ▲  IPC: 命令表 + 流式事件（Transport 抽象：tauri-ipc / http-ws）    │
│ ┌──────┴──────────────────────────────────────────────────────────────┐ │
│ │ apps/desktop  ── bootstrap（唯一非包代码）                            │ │
│ │   读配置 → 装 BlockingExecutor/Clock/LogExporter → Context::root()     │ │
│ │   → zk_composed::register_all(ctx) → 起 tokio → 开窗口/监听           │ │
│ ├─────────────────────────────────────────────────────────────────────┤ │
│ │ zeokeel-cordis（外部 crate，pin 0.6.x）                                │ │
│ │   Context 树 / ServiceKey / Fiber / Effect / Events / Registry        │ │
│ ├─────────────────────────────────────────────────────────────────────┤ │
│ │ 被 profile 选中的包（静态链接）                                        │ │
│ │   platform-tauri · host-* · store-* · llm-* · agent-loop · tool-*     │ │
│ │   · approval-* · telemetry-* · ui-*（声明 crate）                     │ │
│ └─────────────────────────────────────────────────────────────────────┘ │
└───────────────────────────────────────────────────────────────────────────┘
```

- **单进程、单二进制**。不做多进程（除未来 L3 不可信包的进程边界，见 ADR-013）。
- **服务端地址来自构建期渠道默认值**：`channel.json` 的 `defaults.server_url` 非空 → 锁定，登录页**只有账号/密码**；
  为空（官方企业版默认）→ 首配页显示地址输入，首次填写并保存到本地（不写回渠道；设置中可改，需重新登录）。
- **企业版 = 个人版 + 企业管控包**（ADR-043）：客户端基线只有一份（个人版）；企业版在其上追加登录/网关/商店客户端/服务端，
  覆盖 provider 与更新源，并**排除个人版专属包**（本地 provider 设置 UI、GitHub 更新器）。
- **两种版本，同一个二进制形态**（构建期裁剪，ADR-020）：
  - `personal`：无登录、无服务端包、本地 provider（key 存系统密钥库）、**无应用中心**；
  - `enterprise`：登录 + 服务端配置同步 + `llm.provider.default`（不可自配 provider）+ 技能/连接器/应用客户端；
    **其余客户端能力与个人版完全一致**（工具全开、网络不限、本地审批可配）。
  企业版二进制里**不存在** provider 设置包与本地 key 存储包——这是裁剪，不是隐藏入口。
- 内核在**同步**语义下运行 `apply`；异步能力（HTTP、流式、定时）在宿主侧 tokio 上跑，
  但**必须包成 effect**，否则内核回收不到（kernel.md §7.2 H4）。
- 运行时只允许一个 `BlockingExecutor`（进程级 `OnceLock`，先装者赢）——所以它由 bootstrap 装，
  包不得再装。测试进程天然隔离（nextest 每个测试独立进程），这点写进 testkit 说明。

## 2. 宿主责任 → 包归属

内核研究列出的 H1–H14 是"宿主必须自己实现"的清单。逐条落到包：

| 内核研究编号 | 宿主机制 | 归属 | 说明 |
| --- | --- | --- | --- |
| H1 | 组合根 / 包清单 | `crates/zk-composed`（生成）+ bootstrap | profile → 注册表，编译期固定 |
| H2 | 配置来源与分层 | `config-store` 包（`host.config`） | 文件/env/默认值；schema 由包的 `[config]` 提供；内核只做校验合并 |
| H3 | 运行时/定时器/事件循环 | `runtime` 包 + `timer` 包 | tokio 归 runtime；定时器用 effect 包 `JoinHandle::abort` |
| H4 | 任务所有权 | `task-service` 包（`host.tasks`） | `spawn` 返回 handle，cleanup 里 abort/await；**禁止裸 `tokio::spawn`**（CI 源码扫描） |
| H5 | 关闭编排 | bootstrap/runtime | 固定顺序：`when_idle` → `dispose_all`（重试 `LifecycleBusy`）→ `drain_orphan_async_cleanups` → 关 tokio → 关窗口 |
| H6 | 权限/审计/能力策略 | `approval` + `audit` 包 | 审批保护**用户**不受 agent 越界，不是包隔离；审计事件落 store |
| H7 | 资源计量/处置 | `supervisor` 包（`supervision` feature） | 配额、heartbeat、stalled fiber 观测 + cancel；不做"强杀线程" |
| H8 | 不可信包隔离 | 无（不承诺插件隔离） | 信任边界是构建输入（ADR-012）；未来 L3 进程 / L4 WASM 由宿主自建（§7） |
| H9 | 可观测性 | `telemetry` 包 | 装 `LogExporter`（**不装就看不到插件日志**）；lifecycle → 结构化事件；指标命名在宿主 |
| H10 | 时间与确定性 | `host.clock` 包 | `set_clock` 只能 root 一次；测试用假 clock 重放 |
| H11 | 包静态检查 | `tools/zk lint` + CI | 依赖白名单（只许契约）、禁止 IO/线程/全局状态、公共 API 上限——照抄内核自身护栏思路 |
| H12 | 契约测试矩阵 | CI | 见 02 §6 |
| H13 | 持久化/迁移 | `store-*` 包 | schema 版本 + 迁移器；不存 `PluginKey`/`TypeId`（跨构建不稳定） |
| H14 | 产品/UI/Agent 契约 | 各能力包 | IPC 命令表、UI 贡献、agent 循环 |
| ★H15 | 登录与令牌 | `client-auth` 包 | JWT access/refresh 存储、刷新、登出、401 处理；个人版不包含 |
| ★H16 | 服务端配置同步 | `client-bootstrap` 包 | 登录后拉 provider/模型目录/技能源/连接器源/应用源/更新源，写本地配置 |
| ★H17 | 技能客户端 | `client-skill` 包 | 安装/卸载/更新/强制覆盖/来源展示 |
| ★H18 | 连接器客户端 | `client-connector` 包 | 启用/停用、OAuth 令牌（本地）、MCP 连接管理 |
| ★H19 | WASM 应用宿主 | `client-wasm-host` 包 | 每应用独立 origin、窗口、请求信封、错误页 |
| ★H20 | 自动更新 | `client-update` 包 | 企业版只从登录服务端取清单（1:1）；个人版只从 GitHub Releases |

`[requires]` 的角色正好对应这些服务：包声明需要 `host.tasks`、`host.clock`、`host.config`，
组合期据此拓扑排序，运行期断言服务存在。

## 3. 注册与启动：必须绕过的内核坑

### 3.1 `ctx.plugin()` 返回 `Ok` 不等于成功

内核研究实测：`apply` 失败时 `ctx.plugin()` **仍返回 `Ok(fiber)`**，失败只在
`fiber.state()==Failed` / `error_detail()` / `await_settled()` 里。宿主的启动逻辑必须：

```rust
// 包不持有 Context：它只产出一个 Plugin 适配器（见 02 §5.1）
let inst = pkg_tool_fs::install();                       // Installed { id, plugin, required }
let fiber = ctx.plugin(inst.plugin, config_for(inst.id))?;
fiber.await_settled().await?;                            // 关键：看状态，不看 Result
if fiber.state() != FiberState::Active {
    // required=true → dispose_all 后进诊断页退出；false → 记 degraded，继续启动
}
```

**没有"整体回滚"**：内核只保证单 fiber 的激活事务自回滚；多 key 原子替换做不到。启动失败时由
bootstrap `dispose_all()` 重建/退出，而不是指望"撤销已成功的包"。

启动策略：**逐包注册 + 逐包判定**，失败时按 profile 的 `required` 语义处理：

| 包的 `required` | 加载失败时 |
| --- | --- |
| `true`（默认） | `dispose_all()` → 诊断页（包 id、角色、错误链、最近 lifecycle 事件）→ 退出 |
| `false` | 记录 degraded；依赖它的包因 `inject` 门控不激活，相关 UI 显示 fallback |

注：`required` **只是启动失败策略，不是权限分级**（ADR-012）。

### 3.2 同名插件共享 runtime

内核 `Registry` 按 `Plugin::name()` 归并 runtime；两个包 `name()` 相同会被一起替换/卸载。
因此**契约强制命名空间**：`Plugin::name()` 必须是 `<role>/<package-id>`；包 id 是 `zk.toml` 的
`<vendor>/<name>`（如 `zeokeel/tool-fs`），所以完整示例是 `tool.fs/zeokeel/tool-fs`。`zk lint` 检查每个包的
`name()` 与清单一致。

### 3.3 包身份在宿主，不在内核

`PluginKey` 只有 `Named`/`TypeId`，`TypeId` 跨构建不稳定。所以：

- 持久化、诊断、崩溃报告里**只出现包 id + 版本**（来自 `zk.toml`），永不出现 `PluginKey`。
- 生成的 `PACKAGES: &[PackageDescriptor]` 是唯一权威的"我由什么组成"。
- 启动日志第一行打印 profile 名 + 包清单哈希（可复现性证据）。

### 3.4 加载顺序

内核**没有拓扑排序 API**，只有 `inject` 门控与有界 64 轮 refresh。所以顺序由生成器决定：

```
第 1 层 contract / runtime / host    （无 requires）
第 2 层 platform / store / telemetry
第 3 层 llm / tool / approval
第 4 层 agent-loop
第 5 层 ui 声明包（只声明贡献，无运行逻辑）
```

生成器按 `[requires]` DAG 做拓扑排序，同层按包 id 字典序（确定性）。层内不保证互相可见，
所以**跨层依赖必须显式写 `[requires]`**；`zk validate` 检查"requires 的角色由更早层的包提供"。

### 3.5 关闭顺序（固定 recipe）

```rust
runtime.block_on(root.when_idle());              // 等生命周期操作收敛
loop { match root.dispose_all() { Ok(_) => break, Err(LifecycleBusy) => retry, Err(e) => log } }
root.drain_orphan_async_cleanups();              // 0.6.0 的孤儿 async cleanup 队列
shutdown_tokio();                                 // 之后才关 tokio
```

窗口关闭/退出/崩溃前的最后一次机会都要走这条路径；否则 orphan 队列不 drain，可能出现
"退出后还有日志/写盘"的悬空任务。

## 3.5 工作区（Workspace）绑定

M1 规范，**代码待实现**：每个会话必须绑定一个 Workspace；无 Workspace 不建会话/不启动 turn；agent cwd 与工具默认根 =
Workspace 根。完整规则、schema、契约/包影响见 **`26`**（唯一权威），本文不复述。

## 4. Agent 循环的分层

```text
ui-chat ──ipc──► agent-loop（agent.loop，singleton）
                   │  读 SessionStore / 拼 system prompt（prompt 片段包）
                   │  调 LlmProvider（llm.*）
                   │  收到 tool_call → ToolRegistry（tool.*）→ 过 ApprovalGate → 执行
                   │  流式事件 → ipc 事件通道 → UI
                   └  checkpoint / compact / token meter（各自包）
```

- `agent-loop` 只依赖**契约**：`LlmProvider`、`ToolRegistry`、`SessionStore`、`ApprovalGate`、
  `PromptSection`。换 provider/工具/存储不需要改它。
- **provider 的来源随版本不同**：个人版 `llm.provider.default` 是本地 provider（DeepSeek/OpenAI 兼容/
  Ollama）；企业版是 `llm.provider.default`（把请求发到登录服务端的 `/v1/responses`），本地不持有上游 key。
  两条实现共用 `zk-contract-llm`，因此 agent-loop/工具/UI 完全不知道区别。
- 一轮对话的状态机（`Turn`）由 `zk-contract-agent` 定义，UI 与 store 共用同一 DTO。
- 流式：`LlmProvider::stream()` 返回事件流（token / tool-call-delta / usage / done）；
  `agent-loop` 转成 `IpcEvent`，同时落 store。**取消**用 per-consumer token，
  由 `task-service` 持有 handle，effect 里 abort。
- 审批：`ApprovalGate` 在工具调用前异步等待用户决策；超时/退出应用 → 拒绝（fail-closed）；窗口隐藏不拒绝。
- 压缩/上下文管理是**包**（`compact-*`），不是 loop 内硬编码——这是"一切皆包"的试金石。

### 4.1 并发、取消与审批挂起（M1 必须定义，不能拖到 M2）

| 场景 | 规则 |
| --- | --- |
| 同一会话并行多个 turn | **不允许**。每个会话一个 turn 串行执行；新输入进入队列（UI 显示"排队"），可升级为"打断并重开" |
| 不同会话并行 | 允许；每会话一个 `task-service` 任务，独立取消 token |
| 审批挂起时 | turn 保持 `AwaitingApproval` 状态，**不占用执行线程**（等待 channel/notify）；超时（可配置，默认 10 min）→ 拒绝；**退出应用** → 全部拒绝；窗口隐藏不拒绝（通知+超时） |
| 取消（用户主动） | 取消 token → 停止流式读取 → 已产生的部分输出落库，turn 置 `Cancelled`；工具批按"未开始的不再开始、已开始的上报已执行"处理 |
| 中断（进程退出/断流/崩溃） | 已生成部分落库，turn 置 `interrupted`；重启后扫描未完成 turn，可继续/重试（见 `24` §7） |
| 关进程 | 先取消所有 turn（等 ≤2s）→ `when_idle` → `dispose_all` → drain orphan → 关 tokio |
| turn 状态 | `Pending / Streaming / AwaitingApproval / RunningTools / Done / Cancelled / Interrupted / Failed`，是 `zk-contract-agent` 的 DTO（持久化字段 `turn.status` 为 `interrupted`，见 `24` §7），UI 与 store 共用 |

这些规则要写成契约测试：审批挂起时取消、退出应用时拒绝（窗口隐藏不拒绝）、双 turn 排队、断流后重订阅。

## 5. rig 适配层（已按 0.45.0 实测核实）

**版本事实**（`research/rig.md`，crates.io + 两个最小工程实测）：`rig` / `rig-core` / `rig-agent` /
`rig-memory` 稳定版均为 **0.45.0**（2026-10-19），`0.46.0-rc.1` 已于 10-23 发布；节奏是**每 1–3 周
一次 breaking**。

**M1 只依赖 `rig-core`**：`agent-loop` 用自有状态机，不需要 `rig-agent`；后者会无条件拉入
`rig-memory` 与约 100+ 个包，只为「可选第二实现」不值得（评审 S10）。`zk-agent-rig` 与
`rig-agent`/`rig-memory` 放到 M5 的可选实现。直接依赖 `rig-core`（而不是 `rig` facade，后者 lock
≈663 包含全部 optional 集成）也把 lock 控制在约 150 包以内。

### 5.1 crate 边界

```text
zk-contract-llm      模型/消息/工具/流/错误契约（Cargo.toml 里不出现 rig）
zk-contract-agent    AgentRuntime / AgentRequest / EventSink / CancelToken
        ▲
zk-llm-rig           唯一 use rig_core::*：completion / streaming / error 映射
zk-agent-rig         （M5 可选）唯一 use rig_agent::*：AgentRunner/Hook/Memory 编排
zk-llm-openai        第二实现（async-openai 或薄 HTTP），用于契约对拍与降级
zk-llm-mock          测试实现（脚本化模型/工具）
zk-tool-*            只依赖 zk-contract-tool；工具调度由 agent-loop 自己做（不经 rig）
```

铁律：

1. `zk-contract-*` 与 `zk-tool-*` **不得依赖 rig**——这样升级 rig 不需要重编工具包。
2. rig 的 `Agent`/`Builder`/`Runner`/`Handlers`/`Serve`/`EffectKind`/`HandlerKey`/`Tool`/
   `Model`/`Message`/各 `Error` **一律不进契约**（它们 0.44/0.45 连续重构过）。
3. 会话存档格式由契约层决定，**不用 rig 的 `AgentRun` 格式**（`RUN_FORMAT=5` 不向前兼容，
   升级 rig 后旧存档会被拒绝）。
4. `rig-core`/`rig-agent`/`rig-memory` 在 workspace 里 `=0.45.0` 精确钉死，升级只改适配层两个包 +
   跑契约对拍。

### 5.2 适配层职责

1. **类型转换**：自有 `Message`/`ContentBlock` ↔ rig `Message`/`UserContent`/`AssistantContent`。
2. **流式增量拼装**：rig 的工具参数是**碎片**（`Arguments` 逐片到达），必须拼接成完整 JSON；
   中断会产生 `unfinished_calls`，要转成结构化错误而不是静默丢弃。
3. **错误映射**：三层错误不要混着透出——`PromptError`（agent 级）、`ToolExecutionError`（工具级）、
   `ErrorReport`（handler 级）→ 统一映射到自有 `LlmError { kind, retryable, code, model_visible }`。
   注意这些类型大量 `#[non_exhaustive]`，match 必须带 `_ =>`，否则 0.46 升级必炸。
4. **装配**：M1 的模型调用路径是 `agent-loop → zk-contract-llm::LlmProvider → zk-llm-rig →
   rig-core`；工具执行与调度完全在 `agent-loop` + `zk-contract-tool` 内完成，不经过 rig 的
   `ToolServerHandle`。只有将来用 `agent-rig` 时，才需要把契约工具转成 rig 的 `DynamicTool`——
   那是 M5 的实现细节，现在不设计。

### 5.3 必须显式处理的 0.45 行为（都是实测坑；标 🤖 的仅在使用 `agent-rig` 时相关）

| 行为 | 影响 | 处理 |
| --- | --- | --- |
| 🤖 **`max_turns` 默认 1** | 模型一调工具就到顶，报 `MaxTurns` | 用 `agent-rig` 时必须显式设；自有 agent-loop 不受影响，但要自己实现回合上限 |
| 🤖 Agent 长生命周期、`Clone+Send+Sync`；`prompt()` 返回一次性 `AgentRunner` | 不能把 `Agent` 塞进 serde 状态 | 运行时持 `Arc<Agent>`；run 状态放 runner/UI |
| 🤖 工具默认**串行**、批**原子提交**；无 per-call 取消 | 无法"部分成功"给模型看 | 用 hook 或自管工具调度；取消粒度定为 turn/stream |
| 🤖 0.45 的 `ToolExecutionCommitted` **不是实时事件** | 做不了实时进度条 | 自有调度直接产生实时事件，不受此限 |
| 参数校验是**宽容子集**（不查 `pattern`/`format`；未声明 `additionalProperties:false` 时多余字段被接受） | 模型幻觉参数能进来 | 工具内部二次校验；契约 `ToolSpec` 自带 schema，校验在适配层 |
| `serde_json` 的 `preserve_order`/`float_roundtrip` 是**全局 feature 副作用** | 影响 IPC 序列化顺序/浮点表示 | workspace 统一决策 + IPC 契约测试防回归 |
| MSRV **1.97.1 + edition 2024** | 工具链/CI 镜像不匹配 | `rust-toolchain.toml` 钉 ≥1.97.1；CI 镜像同步 |

### 5.4 agent 循环归属

- **默认**：`agent-loop` 包在契约上实现自己的回合状态机（审批、流式事件、压缩、取消完全可控）。
- **可选第二实现**：`agent-rig`（基于 `rig-agent` 的 AgentRunner/Hook/Memory）作为 `agent.loop`
  角色的另一个实现——这是证明"最复杂的角色也能替换"的绝佳样本，放 M3 之后。
- 两条实现共用 `zk-contract-agent`，用同一套 conformance 对拍。

### 5.5 prefix cache 友好性（直接影响成本）

- system prompt 静态；工具定义按`包版本`生成并哈希，作为缓存 key 的一部分。
- 动态内容（时间、工作区状态、检索结果）一律放 **user message 尾部**，不放 preamble。
- 禁止：preamble 里塞时间戳/随机 id；每轮热插拔工具导致 tools 数组变化。
- 观测：`cache_key(conversation_id)` + `Usage.cached_input_tokens` 进指标，缓存命中率作为回归项。

### 5.6 弱管控边界

企业版/个人版能力差异见 `01` §2；弱管控原则、服务端不做清单与责任声明见 [`00` §2–§3](00-design-principles.md)。本节不重复。

provider 配置/模型目录/keychain 键名的 schema 在 M1 spec 冻结（索引见 [15 §C](15-open-questions.md#c-待定清单)）；本节不展开设计。

## 6. IPC：单一分发入口（本地）+ 服务端 API（远程）

> 客户端有两个边界：**本地 IPC**（WebView ↔ Rust 宿主，见本节）与**远程 API**（Rust 宿主 ↔ 服务端，
> 见 `04-contracts-and-api.md`）。不要把两者混成一个协议。

```toml
# 包的 zk.toml：命令静态声明（唯一真源，24 §4）
[[commands]]
name = "chat.send"
args_schema = "contracts/chat/send.schema.json"
permission = "chat:write"
```

- **命令不在源码手写第二份**：`zk compose` 从 `[[commands]]` + 契约 schema 生成 `commands.gen.ts` 与主进程
  handler 表；运行期/构建期只做一致性比对（缺/多/签名不一致 → 拒启或构建失败，见 `24` §4）。
- 参数/返回 schema 由契约 crate 的 schemars 生成，`ipc-commands.json` 由生成器合并。
- Tauri 侧只有**一个** `#[tauri::command] async fn ipc_call(name: String, payload: Value)`，
  内部查表分发；未知命令 → `IpcError::UnknownCommand`（不是 panic）。
- 流式事件：`ipc://event` 通道，统一信封 `{ stream, seq, kind, payload }`，前端按 stream 订阅。
- 传输无关：同一份命令表可由 `platform-tauri`（Tauri IPC）或 `host-webserver`（WS）承载，
  这是 e2e 与远程访问的前提（ADR-006）。
- 参数校验用生成的 JSON Schema（`jsonschema` crate），拒绝时返回结构化错误，便于 UI 提示。
- **Schema 单源**：命令参数/返回类型定义在契约 crate，用 `schemars` 生成 schema/TS；不再手写参数类型与 handler 类型两套。
- **固定平台命令白名单（按窗口 capability）**：主窗口 = `ipc_call`（业务总入口）+ `updater`/`window`/`dialog`/`store`/`open_url` 最小集；WASM 窗口 = 仅 `app_request`；登录/门禁窗口 = `auth_*` + `open_url`。包不得新增 `#[tauri::command]`；详见 `19` §7.1。
- **事件协议**（`zk-contract-ipc` 定义，M1 必须有）：
  ```
  Event { stream_id, seq: u64, kind, payload, ts }
  ```
  - `seq` 单调递增、每条流独立；前端检测断档 → 请求重放。
  - 宿主为每条活跃流保留环形缓冲（默认 512 条 / 30 s），支持 `subscribe(stream_id, from_seq)` 重订阅。
  - 流结束发 `kind = "done" | "error"`，之后 seq 停止。
  - 取消是 per-stream：`ipc_call("stream.cancel", { stream_id })`。
- **serve 载体（M1 就上线）**：只绑定 `127.0.0.1`，启动时生成一次性 token 注入页面并通过
  `Authorization`/cookie 校验；拒绝无 Origin 或跨源写入（防 CSRF）。远程访问（0.0.0.0 + TLS）
  留到 M5，且必须显式开启。
- **丢失的 Tauri 编译期类型安全，用三件事补回来**：① 生成的 `commands.gen.ts` 让前端调用有类型；
  ② 每个命令在 CI 里有一条"契约驱动"的集成测试（从 `ipc-commands.json` 生成用例，正向 + 参数缺失）；
  ③ Rust 侧命令处理函数签名统一为 `(Json) -> Result<Json, IpcError>`，序列化失败在边界处转结构化错误，
  不允许 panic。
- **工作区/会话命令（M1，权威表见 `26` §5）**：`workspace.list/create/rename/set_default/remove`、
  `session.create/list/messages`（均带 `workspace_id`）、`chat.send/cancel`（带 `workspace_id` 并校验归属）；
  `chat.send` 不自动建会话；无 workspace 一律报错。
- 命令全集（name/args/result/permission）、事件 kind 枚举、IPC 错误码在 M1 spec 冻结（索引见
  [15 §C](15-open-questions.md#c-待定清单)）；本节只冻结协议形状。

## 7. 安全与信任模型（写死）

| 层 | 机制 | 不是什么 |
| --- | --- | --- |
| 包信任 | 构建输入可信：源码评审 + 依赖锁定 + 可复现构建 + 契约测试 | 不是运行时签名/市场白名单 |
| Agent 行为 | 审批门（危险命令、工作区外写入、网络）、计划先只读、checkpoint 可回滚、审计日志 | 不是插件沙箱 |
| 数据 | API key 进系统密钥库（`host.secrets`），不落明文配置 | 不是全盘加密 |
| 隔离 | 同进程静态链接，无隔离；未来 L3 进程/L4 WASM 由宿主自建 | 不要宣称"插件沙箱" |

## 8. 可观测性与诊断

- 启动：profile 名、包清单哈希、每包加载状态与耗时、服务就绪表。
- 运行：结构化日志（`LogExporter` → tracing）、lifecycle 事件、fiber 状态快照（**注意 Registry
  快照不是事务视图**，只在空闲时取）、token/工具耗时指标。
- 诊断面板：包清单（角色/版本/来源/状态）、缺失角色、降级包、`zk explain <role>` 结果链接。
- 崩溃报告：只含包 id/版本/角色，不含 `PluginKey`；附最近 N 条 lifecycle 事件。
