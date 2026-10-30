# zeokeel-cordis 0.6.0 内核研究：能力、边界、以及「一切皆包 + CI 可替换」的硬约束

> **研究对象**：`/tmp/cordis-ref`（`zeokeel-cordis` v0.6.0，tag `v0.6.0`，commit `c8adbe9`）。
> 文中所有 `file:line` 都相对该目录；文档行号同样相对该 clone。
> **对照参考**：TypeScript `@deepseek-ai/cordis` 4.0.5（`/usr/local/lib/node_modules/@deepseek-ai/dsh/node_modules/@deepseek-ai/cordis/`）。
> **方法**：先读 `docs/`（DESIGN / KERNEL-BOUNDARY / KERNEL-ISOLATION / GUIDE / CORDIS-RUST）→ 再读 `spec/CORDIS-CORE.md` → 再用 `src/**` 核实每条结论 → 跑 4 个 examples + 8 个语义探针。**文档与代码冲突时以代码为准**，差异列在 §7.4。

---

## 0. 结论先行（TL;DR）

| # | 结论 | 证据 |
|---|---|---|
| 1 | 内核只提供**组合原语**：Context 作用域、类型化 Service、Fiber 生命周期、可逆 Effect、Events/waterfall、Registry。它**不含**任何 IO、运行时、加载器、权限、沙箱。 | `src/lib.rs:57-105`；`docs/KERNEL-BOUNDARY.md:47-65` |
| 2 | 内核的语义承诺核心是两条正交维度：**时间可组合**（effect 逆序、单次、幂等、激活失败回滚）与**空间可组合**（`inject` 门控激活、服务变化驱动 refresh）。 | `docs/DESIGN.md:26-82`；`src/effect.rs:36-53`；`src/fiber.rs:2091-2199` |
| 3 | **`ctx.plugin()` 在 `apply` 失败时仍返回 `Ok(Fiber)`**；失败体现在 `fiber.state()==Failed`、`fiber.error_detail()`，以及 `fiber.await_settled().await` 的 `Err`。宿主绝不能用 `Result` 判定加载成功。 | `src/context.rs:1393-1395`（`let _ = activate_fiber(...)`）、`src/fiber.rs:3917-3944`；探针验证 |
| 4 | 内核**没有 ABI、没有动态加载、没有 dylib/cdylib**：插件就是 `Arc<dyn Plugin>`，必须静态链接进宿主二进制。⇒ 「CI 阶段可替换」只能实现为**构建期替换（按 profile 改 Cargo 依赖后重编译；不要用 Cargo feature 换包，见 §5）**；运行时只能在同一二进制内替换**已链接**实现。 | `docs/DESIGN.md:208-213`；`docs/CORDIS-RUST.md:645-655`；`src/lib.rs:46` |
| 5 | 包身份极弱：`PluginKey` 只有 `Named(&'static str)` 和 `Type(TypeId)`；同名插件共享一个 `PluginRuntime`，`Registry::replace(key, ..)` 作用于该 key 下的**所有** fiber；`TypeId` 跨构建不稳定。**包 id / 版本 / 来源 / 签名必须由宿主自建。** | `src/registry.rs:61-70`、`1044-1052`、`1101`；探针 §9 |
| 6 | 多 root 完全独立（每个 `Context::root()` 新建自己的 Registry/Events/Logger/service store），但进程级单例只有三个：`BlockingExecutor`（`OnceLock`，先装者赢）、orphan async cleanup 队列、若干单调计数器。 | `src/context.rs:128-195`；`src/exec.rs:207`；`src/async_effects.rs:281-294`；探针 §8.1 |
| 7 | panic 遏制是 **unwinding-panic 级**，不是沙箱：`panic="abort"` 被编译期拒绝；但栈溢出/OOM/UB/死循环/双重 panic/外部 kill 进程内无法遏制 ⇒ 不可信包必须 L3 进程隔离或 L4 WASM，且这层完全由宿主实现。 | `src/lib.rs:52-55`；`docs/KERNEL-ISOLATION.md:13-21`、`110-126`、`201-211` |
| 8 | 内核**不强制**插件依赖声明：`inject` 只是激活门，不是能力授权；拿到 `Context` 的插件可以 `ctx.get` 作用域内任何服务、发任何事件。依赖声明是**契约**，不是权限。 | `spec/CORDIS-CORE.md:142-155`；`src/context.rs:550-706` |
| 9 | 「一切皆包」在 Rust 侧的落地形态是：**契约 crate（trait + ServiceKey + 事件名）+ 多个实现 crate + 组合根（宿主二进制）**；profile→crate 的映射（`experiments/composition-poc/profiles/*.toml`）属于宿主/CI，内核不感知。 | §5；`docs/KERNEL-BOUNDARY.md:130-150` |
| 10 | 对架构的**唯一硬约束**：包替换是**构建期**事件；任何「运行时装/卸第三方代码」的需求都必须由宿主自建进程/WASM 边界（内核不提供、也不会提供）。内核能给的只有「同一二进制内已链接实现的语义安全替换」。 | 同 #4；§7.3 |

---

## 1. 内核是什么 / 不是什么（一句话版）

- **是**：一个纯 Rust、`#![forbid(unsafe_code)]`、进程内、静态链接的**组合内核**；对插件提供「注册即拥有、卸载即撤销、依赖变化即重算、panic 不越界」的运行时语义。
- **不是**：框架、loader、HMR、权限系统、沙箱、运行时、调度器、插件市场、包管理器。
- 三层模型（`docs/KERNEL-BOUNDARY.md:28-43`）：

```
业务/宿主 crate ──► 插件（impl Plugin / AsyncPlugin）──► zeokeel-cordis（内核）
       │                                                      │
       └──────────────► serde_json / tracing / futures-util ◄─┘
```

依赖方向单向：插件/宿主 → 内核；内核运行时依赖白名单**只有** `serde_json`、`tracing`、`futures-util`（`Cargo.toml:52-55`；`docs/KERNEL-BOUNDARY.md:96-113`）。这条白名单是**可执行护栏**，不是口号：`tests/kernel_boundary.rs` 里有 11 个测试（4 真实断言 + 7 防绕过自测），CI 以 `cargo test --test kernel_boundary` 强制（`.github/workflows/ci.yml:75`、`235`）。公共 API 还有 `PUBLIC_ITEM_CAP = 420` 绊线（`tests/kernel_boundary.rs:142`）。

> 对本项目的含义：宿主要想在插件侧引入 tokio/reqwest/rig 等，**只能放在宿主或插件 crate**，不能碰内核；而内核的这层护栏可以被宿主**照抄**成「包边界护栏」（同样的思路：白名单依赖 + 源码扫描 + 公共 API 上限）。

---

## 2. 精确 API 面（含签名与源码位置）

> `src/lib.rs:75-105` 是公共 re-export 的权威清单；以下按模块列关键项。

### 2.1 Context 与作用域

```rust
pub struct Context { /* Arc<ContextInner>，Clone + Send + Sync */ }

impl Context {
    pub fn root() -> Self;                                  // src/context.rs:128
    pub fn fiber(&self) -> Fiber;                           // :218
    pub fn registry(&self) -> Arc<Registry>;                // :222
    pub fn events(&self) -> Arc<Events>;                    // :226
    pub fn extend(&self) -> Self;                           // :231  共享父作用域
    pub fn isolate(&self, name: &'static str) -> Self;      // :241  某个服务名开新 scope
    pub fn isolate_labeled(&self, name: &'static str, label: &'static str) -> Self; // :252
    pub fn intercept(&self, name: &'static str, config: Value) -> Self;             // :314
    pub fn clock(&self) -> Arc<dyn Clock>;                  // :345
    pub fn set_clock(&self, clock: Arc<dyn Clock>) -> Result<()>; // :380 仅 root fiber
}
```

关键实现事实（`src/context.rs:128-195`）：**每个 root 自己新建** `Events`、`Registry`、`LoggerService`、`service_store`、`accessors`、clock 槽；scope id 分配器在 `Registry` 内（`src/registry.rs:404`、`1022-1040`）。
⇒ **一个进程里可以跑多个互不干扰的 root**（探针已验证，见 §8.1），这是宿主做「每窗口/每会话/每 agent 一个容器」的基础。

`isolate` 的语义（`src/context.rs:241-312`、`docs/DESIGN.md:91-103`）：
- 只隔离**指定服务名**与**事件派发映射**，不是命名空间；
- 服务解析沿子链向外、在**第一个定义该名字的 scope 停下**，即使该 scope 的 availability check 为 false 也**不回退**祖先（`spec/CORDIS-CORE.md:70-71`，CORE-014）；
- 事件过滤是「isolate **映射相等**」，不是「子树」；`global` 监听器例外（`docs/GUIDE.md:500-527`）。

### 2.2 Plugin / AsyncPlugin

```rust
pub trait Plugin: Send + Sync + 'static {
    fn name(&self) -> &'static str;
    fn key(&self) -> PluginKey { PluginKey::Named(self.name()) }
    fn inject(&self) -> &'static [&'static str] { &[] }
    fn config_schema(&self) -> Option<Arc<dyn ConfigSchema>> { None }
    fn apply(&self, ctx: Context, config: serde_json::Value) -> Result<()>;
}
// src/registry.rs:120-157

pub trait AsyncPlugin: Send + Sync + 'static {
    fn name(&self) -> &'static str;
    fn key(&self) -> PluginKey;
    fn inject(&self) -> &'static [&'static str];
    fn config_schema(&self) -> Option<Arc<dyn ConfigSchema>>;
    fn apply_async<'a>(&'a self, ctx: Context, config: Value)
        -> BoxFuture<'a, Result<()>>;
}
// src/registry.rs:167-...；BoxFuture = Pin<Box<dyn Future<Output=T> + Send + 'a>>（src/exec.rs:27）
```

加载入口（`src/context.rs`）：

```rust
pub fn plugin(&self, plugin: Arc<dyn Plugin>, config: Value) -> Result<Fiber>;           // :1187
pub fn plugin_typed<P: Plugin>(&self, plugin: Arc<P>, config: Value) -> Result<Fiber>;   // :1200
pub async fn plugin_async(&self, plugin: Arc<dyn AsyncPlugin>, config: Value) -> Result<Fiber>; // :1402
pub async fn plugin_async_typed<P: AsyncPlugin>(...) -> Result<Fiber>;
pub fn inject<F>(&self, deps: &'static [&'static str], callback: F) -> Result<Fiber>;   // :1454
```

**语义要点（代码事实）**：
1. `plugin()` 是**立即返回**：fiber 初始 `Pending`，若依赖已满足则**同步激活**，否则停在 `Pending`（`src/context.rs:1186-1188`、`1393-1395`）。
2. 激活失败（`apply` 返回 `Err` / panic）**不会**让 `plugin()` 返回 `Err`；它设置 `state=Failed` + `error`，返回 `Ok(fiber)`（`src/context.rs:1394-1395`；`src/fiber.rs:2141-2199`）。要拿到结构化错误：`fiber.await_settled().await`（`src/fiber.rs:3917-3944` 在无 in-flight op 后返回 `Err(fiber.error)`）。
3. 插件元数据回调（`name/key/inject/config_schema`）panic 会被遏制并**确定性降级**（`name`→`"<panicked plugin>"`、`key`→共享 `Named`、`inject`→`&[]`、schema→`None`），见 `src/registry.rs:229-290`、`docs/KERNEL-ISOLATION.md:104`。
4. 同一个 `PluginKey` 可以被加载多次：**一个 runtime、多个 fiber**（`src/registry.rs:1044-1052`；注释明确「同一个插件启动两次只有一个 runtime」）。探针验证：同名两次 → `registry.size()==1, fiber_count()==2`。

### 2.3 Fiber（生命周期句柄）

```rust
pub enum FiberState { Pending, Loading, Active, Failed, Unloading, Disposed } // src/fiber.rs:118-126 (#[non_exhaustive])

impl Fiber {
    pub fn name(&self) -> String;            // :343
    pub fn id(&self) -> u64;                 // :353（root = 0）
    pub fn uid(&self) -> Option<u64>;        // :358（Disposed 后 None）
    pub fn parent(&self) -> Option<Context>; // :366
    pub fn get_effects(&self) -> Vec<EffectMeta>; // :372
    pub fn state(&self) -> FiberState;       // :412
    pub fn is_active(&self) -> bool;         // :419
    pub fn assert_active(&self) -> Result<()>;// :425
    pub fn config(&self) -> Value;           // :503（schema 解析后）
    pub fn raw_config(&self) -> Value;       // :518（调用方原样传入）
    pub fn replace_plugin(&self, plugin: Arc<dyn Plugin>) -> Result<()>;      // :590
    pub fn replace_plugin_async(&self, plugin: Arc<dyn AsyncPlugin>) -> Result<()>; // :628
    pub fn error(&self) -> Option<String>;        // :759
    pub fn error_detail(&self) -> Option<CordisError>; // :766
    pub fn dispose(&self) -> Result<()>;          // :781  同步阻塞边
    pub async fn dispose_async(&self) -> Result<()>; // :793
    pub fn restart(&self) -> Result<()>;          // :844
    pub async fn restart_async(&self) -> Result<()>; // :859
    pub fn update(&self, config: Value) -> Result<()>;  // :947
    pub fn update_with_no_save(&self, config: Value, no_save: bool) -> Result<()>; // :962
    pub async fn update_async(&self, config: Value) -> Result<()>; // :973
    pub async fn await_settled(&self) -> Result<()>;   // :400
    pub fn settled(&self) -> Settled<'_>;              // :405（future 形态）
    pub fn cancel(&self) -> ();                        // :483（协作取消）
    pub fn is_cancelled(&self) -> bool;                // :490
    // supervision feature:
    pub fn heartbeat(&self); pub fn last_activity_ms(&self) -> u64; pub fn idle_for(&self) -> Duration;
}
```

**`Ok` ≠ `Active`（代码确认）**：`restart`/`update`/`replace_plugin` 返回 `Ok(())` 只表示操作完成；随后 reload 失败会让 fiber 落到 `Failed`，结构化错误保留在 `error_detail()`（`docs/GUIDE.md:545-549`）。`ctx.plugin` 同理（§2.2）。**宿主必须同时检查 `Result` 和 `state()`。**

### 2.4 Effect / Disposer（时间可组合性的载体）

```rust
pub type Cleanup = Box<dyn FnOnce() + Send>;      // src/effect.rs:9
pub type Effect  = Vec<Cleanup>;                  // src/effect.rs:12
pub fn cleanup<F: FnOnce() + Send + 'static>(f: F) -> Cleanup; // :15
pub struct Disposer { /* Arc<Mutex<Option<Vec<Cleanup>>>> */ } // :25 单次幂等
impl Disposer { pub fn dispose(&self); pub fn is_disposed(&self) -> bool; } // :36,55
impl Drop for Disposer { /* 最后一份句柄 drop = 兜底 dispose；panic 吞掉并记日志 */ } // :60-79
```

```rust
impl Context {
    pub fn effect<F: FnOnce() -> Effect>(&self, setup: F) -> Result<Disposer>;             // :1132
    pub fn effect_labeled<F>(&self, label: &'static str, setup: F) -> Result<Disposer>;    // :1141
    pub fn effect_async<F: FnOnce() -> AsyncEffect>(&self, setup: F) -> Result<AsyncDisposer>; // :1170
    pub fn effect_future<F, Fut>(&self, body: F, label: &'static str) -> Result<AsyncDisposer>; // :1648
    pub fn effect_stream<S>(&self, stream: S, label: &'static str) -> Result<AsyncDisposer>;    // :1687
    pub fn attach_effect(&self, disposer: Disposer, label: &'static str) -> Result<()>;         // :798
}
```

保证：
- **逆序 + 单次 + 幂等**：一个 fiber 一条有序 effect 列表，同步/异步共用同一顺序（`docs/DESIGN.md:41-49`；`src/effect.rs:36-53` 弹出逆序执行）；
- **清理 panic 不跳过剩余清理**：逐个 `catch_unwind`，全部跑完后**重抛第一个 payload**（`src/effect.rs:41-52`）；
- 卸载/激活失败时逆序回滚（`src/fiber.rs:1391-1410`、`1935-1975`）；
- 取消安全：`AsyncDisposer` 的 future 被 drop 时，未启动的 cleanup 会被恢复并在下次 dispose 重试（`docs/KERNEL-ISOLATION.md:75-86`）；
- `effect_future` 的 body **在注册时被内核 `block_on` 同步驱动完**（`src/context.rs:1653-1659`），不要在 reactor 线程上做 reactor-bound 初始化。

### 2.5 Service（空间可组合性的载体）

```rust
pub type ServiceValue = Arc<dyn Any + Send + Sync>;                 // src/service.rs:15
pub type AvailabilityCheck = Arc<dyn Fn() -> bool + Send + Sync>;   // :73
pub struct ServiceKey<T: ?Sized + Send + Sync + 'static>;           // :152
impl ServiceKey<T> { pub const fn new(name: &'static str) -> Self; }// :163

impl Context {
    pub fn provide<T>(&self, key: ServiceKey<T>, value: Arc<T>) -> Result<Disposer>;              // :406
    pub fn provide_checked<T, F: Fn() -> bool + ...>(&self, key, value, check: F) -> Result<Disposer>; // :420
    pub fn provide_gated<T>(&self, key, value, gate: &ServiceGate) -> Result<Disposer>;           // :435
    pub fn get<T>(&self, key: ServiceKey<T>) -> Option<Arc<T>>;                                   // :565
    pub fn require<T>(&self, key: ServiceKey<T>) -> Result<Arc<T>>;                               // :578
    pub fn provide_any<T: ?Sized + ...>(&self, key, value: Arc<T>) -> Result<Disposer>;           // :589
    pub fn get_any<T: ?Sized + ...>(&self, key) -> Option<Arc<T>>;                                // :598
    pub fn require_any<T: ?Sized + ...>(&self, key) -> Result<Arc<T>>;                            // :612
    pub fn set_any<T: ?Sized + ...>(&self, name: &'static str, value: Arc<T>) -> Result<()>;       // :627
    pub fn service_available(&self, name: &str) -> bool;                                          // :666
    pub fn set(&self, name: &'static str, value: ServiceValue) -> Result<()>;                     // :1993
    pub fn set_revertible(&self, name: &'static str, value: ServiceValue) -> Result<Disposer>;     // :2024
}
pub trait Service: Send + Sync + 'static { name/check/config_schema/resolve_config } // src/service.rs:283-310
pub fn provide_service<S: Service>(ctx: &Context, service: Arc<S>) -> Result<Disposer>; // :313
```

语义要点（`spec/CORDIS-CORE.md:76-141`、`docs/GUIDE.md:170-238`）：
- **服务归提供者 fiber 所有**：`Disposer` 撤销、fiber 卸载自动撤销、`ctx.set` 只能改自己提供的（否则 `ServiceWriteForbidden`）；
- **两条查询通道**：
  - **严格注入门**（`inject` / `service_available`）：服务已存储 + availability check 通过 + **provider fiber 必须 `Active`**（`src/service.rs:120-141`；`src/context.rs:703-706`）；
  - **宽松读通道**（`get` / `get_named` / `get_any`）：可以看到 `Loading` 中的 provider 的值，且 accessor / `internal/get` 虚拟服务只对读通道可见（`spec/CORDIS-CORE.md:132-140`）；
- **accessor 不参与 inject**（v0.4.2）；
- **`ctx.set` 是可逆的分类变更**：每次成功 `set` 在提供者 fiber 上注册一个逆操作 → 热路径上 O(n) 增长（XT-07，`spec/CORDIS-CORE.md:118-131`）。高频更新应改服务内部可变状态；
- **类型安全**：`set`/`set_any` 用 `TypeId` 校验，不匹配返回 `ServiceTypeMismatch`（`src/error.rs:51-55`）；`get` 用 `ServiceKey<T>` 的 `TypeId` 取，类型不匹配表现为 `None`/`UnknownService`（不是 UB，但宿主应把它当「契约被破坏」处理）；
- **provider epoch**：服务条目带单调 `version`（`src/service.rs:23-27`、`76-90`），fiber 激活时 `record_dep_versions`，`dependencies_satisfied` 比对版本（`src/fiber.rs:1091-1129`）→ **同 key 换 provider 或 provider 重启，依赖方会重新评估/重启**。

### 2.6 Events / waterfall / internal hooks

```rust
pub enum EventResult { Continue, Bail(Value) }                 // src/events.rs:22
pub struct EventOptions { pub prepend: bool, pub global: bool, pub once: bool } // :42
pub struct Events { ... }                                      // :165

impl Context {
    pub fn on<F: Fn(&[Value]) -> EventResult + Send + Sync + 'static>(&self, name: &'static str, listener: F) -> Result<Disposer>; // :1480
    pub fn on_with / on_with_options / once                                                  // :1487/:1510/:1539
    pub fn on_async / on_async_with                                                          // :1584/:1594
    pub fn on_waterfall / on_waterfall_async                                                 // :1745/:1767
    pub fn emit(&self, name: &str, args: &[Value]);                                          // :1785
    pub fn bail(&self, name: &str, args: &[Value]) -> Option<Value>;                         // :1791
    pub fn waterfall(&self, name: &str, args: &[Value], terminal: Option<WaterfallTerminal>) -> Value; // :1812
}
```

保证：
- **两条 lane 严格分离**：sync（`emit/bail/parallel/serial`）不会 poll async listener；async（`parallel_async/serial_async`）不会降级（`src/lib.rs:14-19`、`docs/CORDIS-RUST.md:33-36`）；
- 监听器归注册 fiber 所有，fiber 卸载自动移除；
- panic 遏制：listener / hook / exporter / accessor / schema 记录后遏制；**waterfall 中间件是唯一例外**——fail-closed，中止整条链并重抛给调用方（`docs/KERNEL-ISOLATION.md:104-108`）；
- 9 条 typed `internal/*` 中间件通道：`plugin / status / service / config / update / get / set / listener / dispatch`（`src/context.rs:2202-2333`；`src/internal.rs:178-191`；`docs/KERNEL-BOUNDARY.md:139`）。这是宿主做审计/策略/观测的**唯一**内核级扩展点，但**不是安全边界**（见 §7.3）；
- 生命周期广播：`Context::on_lifecycle(LifecycleListener) -> Result<Disposer>`（`src/context.rs:2252`），`LifecycleEvent { fiber_id, name, from, to, seq }`（`src/internal.rs:243-295`，`#[non_exhaustive]`）。**订阅不归 fiber 所有**，drop `Disposer` 即退订——宿主做 Tauri 事件桥/指标时应保存该 handle。

### 2.7 Registry

```rust
pub struct Registry { /* runtimes: Mutex<HashMap<PluginKey, Arc<PluginRuntime>>> */ } // src/registry.rs:402-420

impl Registry {
    pub fn size(&self) -> usize;                     // :1048（runtime 数，不是 fiber 数）
    pub fn fiber_count(&self) -> usize;              // :1056
    pub fn get(&self, key: PluginKey) -> Option<Arc<PluginRuntime>>; // :1061
    pub fn has(&self, key: PluginKey) -> bool;       // :1069
    pub fn delete(&self, key: PluginKey) -> Option<Arc<PluginRuntime>>; // :1078
    pub fn replace(&self, old: PluginKey, new: Arc<dyn Plugin>) -> Result<Vec<Fiber>>; // :1101
    pub async fn replace_async(&self, old: PluginKey, new: Arc<dyn AsyncPlugin>) -> Result<Vec<Fiber>>;
    pub fn dispose_all(&self) -> Result<()>;         // :1409
    pub fn keys(&self) -> Vec<PluginKey>;            // :1466
    pub fn values(&self) -> Vec<Arc<PluginRuntime>>; // :1474
    pub fn entries(&self) -> Vec<(PluginKey, Arc<PluginRuntime>)>; // :1482
    pub fn fibers(&self) -> Vec<Fiber>;              // :1507
    pub fn is_idle(&self) -> bool;                   // :871
    pub fn when_idle(&self) -> IdleWait<'_>;         // :1004（future；Context::when_idle 是 async 封装 :775）
    // supervision:
    pub fn set_quota(&self, quota: FiberQuota); pub fn stalled_fibers(&self, timeout: Duration) -> Vec<StalledFiber>;
}
```

**`Registry::replace` 的精确语义**（`src/registry.rs:1101-1140`、`1334-1400`）：
- 复用**已有 FiberRef**（id/uid/raw config 不变），只把 runtime 绑定迁到新实现，然后逐个 `restart()` 重新激活；
- 任何 fiber 拒绝（如 `LifecycleBusy`）→ 用旧实现回滚已 reload 的 fiber；回滚自身失败返回 `CordisError::Rollback`；
- reload 期间旧实现的 cleanup panic 被遏制为 `Err(PluginFailed("… reload panicked: …"))` 并触发回滚；
- **只作用于一个 key**，多个 key 的替换**不是原子的**（无跨 key 2PC）。

### 2.8 `supervision` feature（默认关闭）

| 能力 | API | 位置 |
|---|---|---|
| 能力自省 | `IsolationCapabilities::detect()/summary()`（`panic_strategy/panic_containment/unsafe_forbidden/process_isolation/wasm_isolation`；后两者**恒 false**） | `src/l2.rs:25-70` |
| 配额 | `FiberQuota::unlimited().max_effects(n).max_listeners(n).max_child_fibers(n)`；`Context::set_quota`/`Registry::set_quota`；超限 `QuotaExceeded` 并回滚 | `src/l2.rs:75-150`；`src/context.rs:916`/`929` |
| 心跳 | `Fiber::heartbeat()` / `Context::heartbeat()` | `src/fiber.rs:440`；`src/context.rs:910` |
| watchdog 观测 | `Registry::stalled_fibers(timeout) -> Vec<StalledFiber>`（**只观测，不强杀**） | `src/registry.rs:828` |
| 协作取消 | `Fiber::cancel()` + `Context::checkpoint() -> Result<()>` | `src/fiber.rs:483`；`src/context.rs:713` |

注意：配额只覆盖 **effect / listener / child fiber** 三种**内核可见**资源；线程、fd、CPU、内存、子进程**不在内核视野**（`docs/KERNEL-ISOLATION.md:272-289` 路线图第 8 条）。

### 2.9 exec / logger / clock / config

- `BlockingExecutor`：`fn block_on(&self, future: BoxFuture<'static, ()>)`（`src/exec.rs:202-205`）；`set_blocking_executor` 是**进程级 `OnceLock`，先装者赢**（`src/exec.rs:207-215`）。未安装时同步销毁边使用内核自带 thread-park `block_on`（`src/exec.rs:57-72`，`docs/GUIDE.md:838-843`）。
- `LoggerService` + `Logger` facade + `LogExporter`（`src/logger.rs:33/101/117`）：**默认不安装 exporter**，记录只进 1000 条 ring buffer（README:276；`docs/GUIDE.md:662-693`）。
- `Clock` / `SystemClock`（`src/clock.rs:31-70`）：`set_clock` **只能在 root fiber（fiber_id==0）的 context 上设置**（`src/context.rs:380`），root 派生 scope 共享同一树 clock 槽；运行期不要切换。
- `ConfigSchema` / `JsonConfigSchema`（`src/config.rs:97-111/166`）：`validate(Value) -> Result<Value, Vec<String>>` + `merge(base, overrides)`；schema 由**插件**声明，内核只做校验/合并机制。`update` 链路：`internal/config` waterfall → schema 校验 → `internal/update` waterfall（`docs/GUIDE.md:552-566`）。

---

## 3. 语义保证与限制（宿主可以直接依赖 vs 不能依赖）

### 3.1 激活是事务（代码流程）

`activate_fiber`（`src/fiber.rs:2091-2195`）的实际顺序：

```
state_rejects_activation → begin_operation(await_settled 可见)
→ begin_activation_epoch(epoch = 新 activation id)
→ state = Loading
→ internal/config waterfall → config_schema.validate → 失败: Failed + refresh_dependents
→ catch_unwind(Plugin::apply)          ← 用户代码 panic 在这里被捕获
→ Ok(Ok(())):
     stale epoch?          → discard 本 epoch 的注册
     disposed/restarted?   → discard 注册
     依赖在 apply 期间消失? → rollback 到 Pending（XT-02，epoch 精确）
     otherwise             → record_dep_versions + Active + refresh_dependents
→ Ok(Err(e)): fail_fiber_sync → 逆序回滚 + Failed
→ Err(panic): fail_fiber_sync(PluginFailed("panic: ...")) → 回滚 + Failed
```

子 fiber（`ctx.plugin`）也是父 fiber 的 effect：父激活失败会一并回滚；父 dispose 会级联整棵子树（`docs/DESIGN.md:123-136`，CORE-047）。

### 3.2 失败回滚 / 级联卸载 / 生命周期互斥

| 保证 | 说明 | 证据 |
|---|---|---|
| 激活失败全回滚 | effect/service/listener 逆序撤销；fiber `Failed`，不污染依赖方 | `src/fiber.rs:2141-2199`、`1391-1410`；CORE-026/027/028 |
| 级联卸载 | 父 dispose → 子 fiber 级联；两阶段迭代级联（begin provider-before-dependent → 逆序 drain），调用栈 O(1) | `src/fiber.rs:2840`（begin_cascade）、`3099`（drain_cascade_dependents）、`docs/KERNEL-ISOLATION.md:87-93` |
| 生命周期互斥 | `dispose/restart/update/replace` 对同一 fiber 互斥；跨线程/异步挂起期间返回 `LifecycleBusy`；**不排队**，调用方自己重试 | `src/error.rs:30-51`；`docs/GUIDE.md:529-577` |
| Disposed 终态 | disposed fiber 不能 restart，`transition` 对 Disposed 早退 | `src/fiber.rs:1123-1134`（transition） |
| 静默检测 | `is_idle()` 严格；`when_idle()` 只排除**调用线程自己**的注册窗口；跨线程环形注册不检测 | `src/registry.rs:871/1004`；`docs/KERNEL-ISOLATION.md:142-146` |
| 关闭 | `dispose_all()` 对每个 fiber 都尝试；panic 优先于 Err 重抛；遇到 busy fiber 会返回 `LifecycleBusy` 且可能留下未清理 fiber，需重试 | `src/registry.rs:1409-1464`；CORE-051 |

### 3.3 provider epoch 与 refresh

- 服务条目 version 单调（`src/service.rs:23`），依赖方记录版本并在 refresh 时比对（`src/fiber.rs:1091-1129`）→ provider 更换/重启会重启依赖方，避免陈旧 `Arc`。
- `Context::refresh()` / `refresh_async()` 是**全量重算**，但**有界**：最多 64 轮、按注册顺序扫描，重入用 dirty flag 合并（`src/context.rs:775-810`）。**反向依赖链**（C 依赖 B 依赖 A，先加载 C）约每轮推进一层，极深链需要多次 refresh 才稳定 ⇒ 宿主**不要**假设一次 refresh 收敛。
- `provide_gated` + `ServiceGate` 会自动 refresh（`src/service.rs:197-225`）。

### 3.4 orphan async cleanup 队列（0.6.0）

- 未显式 dispose、未安装 `BlockingExecutor`（或正在 panicking）时，`FiberData::drop` 只跑同步 cleanup；被跳过的 async/deferred cleanup 进入**进程级** orphan 队列，容量 256（`src/async_effects.rs:276-330`）。
- 观测/回收：`orphan_async_cleanups_pending()`、`orphan_async_cleanup_count()`、`orphan_async_cleanup_failure_count()`、`drain_orphan_async_cleanups()`（`src/lib.rs:75-79`）。
- 宿主必须在关闭时 drain，且最好先安装 `BlockingExecutor`（否则 reactor-bound cleanup 可能失败并计入 failure count）。

### 3.5 panic 遏制矩阵（精简自 `docs/KERNEL-ISOLATION.md:95-108`）

| 边界 | 行为 |
|---|---|
| `apply`/`apply_async` panic | 遏制：激活失败 + 回滚，fiber `Failed` |
| 直接 `dispose` 的 cleanup panic | 收尾完成后重抛首个 panic |
| refresh/retire 触发的依赖方 unload panic | 记录并遏制；provider 仍到目标态 |
| 父 sweep 子 fiber 的 panic | 逐子遏制，父落目标态后重抛首个 payload |
| `Registry::replace`/`replace_plugin` reload panic | 遏制为 `Err(PluginFailed("… reload panicked"))` + 回滚 |
| 元数据回调（name/key/inject/schema）panic | 遏制 + 确定性降级 |
| listener/hook/exporter/accessor/schema | 记录后遏制 |
| **waterfall 中间件 panic** | **fail-closed：中止链并重抛**（唯一例外） |
| `dispose_all` 同时有 Err+panic | panic 优先，error 只记日志 |
| `Drop` 中的 cleanup panic | 尽力清理；unwind 期间双 panic 会 abort；跳过的 async 进 orphan 队列 |

### 3.6 进程内无法遏制（内核明确声明为设计边界）

`panic=abort`（编译期已被拒绝）、栈溢出、OOM/allocator abort、`process::abort/exit`、SIGSEGV/UB、死循环、死锁、线程/fd 泄漏、双重 panic、外部 kill、取消时正在 await 的那个 async cleanup（`docs/KERNEL-ISOLATION.md:110-126`）。

---

## 4. 内核边界：哪些能力**明确不在内核**

### 4.1 「绝不属于内核」清单（`docs/KERNEL-BOUNDARY.md:69-90`）

| 类别 | 应归属 |
|---|---|
| 业务领域逻辑（订单/支付/用户/租户/库存…） | 插件层 |
| 领域配置 schema（DB URL、超时、业务开关） | 插件层（用内核 `ConfigSchema` 机制） |
| 网络/传输（TCP/UDP/HTTP/WS/gRPC、代理、路由） | 插件层 + 宿主运行时 |
| 持久化/存储（DB/ORM/对象存储/文件布局/迁移） | 插件层 |
| 缓存/中间件（Redis/Kafka/NATS…） | 插件层 |
| 进程/环境（`std::process`、env、信号、daemon、线程池、**tokio/async-std/smol**） | 宿主 |
| 时钟/随机（chrono/time/rand 作为内核运行时依赖） | 插件/宿主（通过 `Clock`/effect 扩展点） |
| 业务观测（指标名/标签/告警规则） | 插件/宿主 |
| 业务策略（重试/退避/限流/熔断/鉴权/路由/多租户/优先级/计费） | 插件/宿主 |
| **加载与隔离**（模块加载器、HMR 文件监听、权限审批、进程/WASM 隔离） | **宿主** |
| 产品契约（UI/Agent 工具契约、API 版本、业务协议） | 宿主/插件 |

判定口诀（原文）：「如果一条能力需要知道具体业务实体、具体传输/存储/配置来源或具体运行时，它就不属于内核。」

### 4.2 内核 vs 插件 vs 宿主 责任矩阵（对本项目）

| 机制 | 内核 | 插件 | 宿主（Tauri 后端） |
|---|---|---|---|
| 作用域/服务/fiber/effect/events | ✅ 全部 | 使用 | 使用 + 编排 |
| 依赖门控/refresh/provider epoch | ✅ | 声明 `inject` | 触发 `refresh`/决定加载顺序 |
| 生命周期互斥/静默/取消 | ✅ 机制 | 协作 `checkpoint` | 决定重试/串行化/超时 |
| panic 遏制（unwind） | ✅ L1 | 不 abort/不 UB | 选择 L3/L4 处理不可信包 |
| 资源配额 | ⚠️ 仅 effect/listener/child fiber | 声明 | 线程/fd/CPU/内存/子进程计量与处置 |
| 组合/加载/热更新 | ❌ | ❌ | ✅（构建期 + 运行时编排） |
| 权限/审计/沙箱 | ❌（hooks 只是通道） | 不拥有 | ✅ |
| 运行时（tokio）/定时器/事件循环 | ❌ | 可用（自管生命周期） | ✅ 拥有 |
| 日志后端/指标命名/OTel | ❌（只给 ring buffer + `LogExporter`） | 可用 | ✅ |
| UI/Agent 契约/持久化 | ❌ | 领域实现 | ✅ |

---

## 5. 无 unsafe / 无动态加载 / 静态链接 ⇒ 宿主如何做「构建期组合」

**代码事实**：
- `#![forbid(unsafe_code)]`（`src/lib.rs:46`），且 `panic="abort"` 构建被 `compile_error!` 拒绝（`src/lib.rs:48-55`）；
- 插件就是 `Arc<dyn Plugin>` 值，随宿主一起编译；**没有 `cdylib` ABI、没有 `dlopen`、没有模块 URL、没有配置文件加载器**（`docs/CORDIS-RUST.md:645-655`；`docs/KERNEL-ISOLATION.md:3-6`）；
- Rust ABI 不稳定 ⇒ 插件**不能**以 `.so` 形式跨编译器/crate 版本分发（原文明确）。

**因此「一切皆包 + CI 可替换」的正确落地形态**（与 `experiments/composition-poc` 的 profile→crate 映射一致）：

```
packages/<contract-crate>      // 只含 trait、ServiceKey 常量、事件名常量、DTO —— 无实现
packages/<impl-a>              // impl Plugin/AsyncPlugin for Xxx，依赖 contract
packages/<impl-b>              // 同一 contract 的另一种实现（如 sandboxed 变体）
apps/<host-binary>             // 组合根：读 profile，显式注册选中的 impl
profiles/<name>.toml           // CI 输入：package id → crate 名/path（构建期）
```

- **替换 = 按 profile 改 `Cargo.toml` 依赖（path/crate）+ 重编译**（CI 矩阵可以一次构建多套并跑同一套契约测试）；**不要用 Cargo feature 表达换包**：feature 会被 Cargo 做并集统一、组合不可穷举，feature 只适合**单个包内部**的可选依赖裁剪；
- **运行时**只能在同一二进制内、用 `Fiber::replace_plugin` / `Registry::replace` 在**已链接实现之间**切换（保持 fiber id/raw config，走 restart 管线）；或者用 `update(config)` 改配置重启；
- **跨进程/WASM 插件**：内核完全不支持；宿主必须自建 HTTP/stdio IPC 或 wasmtime 宿主，把这些外部插件**适配成**一个本地 `Plugin`（内部转发）。
- **包身份**：内核的 `PluginKey` 不能当包 id（`Named` 是 `Plugin::name()`，不是包名；`TypeId` 跨构建不稳定）。宿主必须在 profile/清单里维护 `package id → crate → impl`，并在注册时用**命名空间化的 name**（如 `tool.fs`）或 `plugin_typed`（仅限同二进制内区分）避免撞车。
- **同名插件共享 runtime**（`src/registry.rs:1044-1052`）⇒ 两个包 `name()` 相同会被 `Registry::replace` 一次替换掉全部实例；**契约 crate 应强制 name 前缀**。
- 依赖白名单机制值得照抄：宿主可用「插件 crate 依赖白名单 + 源码禁用 API 扫描」的 CI 护栏，保证包不会偷偷摸 IO/全局状态（内核自己的护栏见 `tests/kernel_boundary.rs`）。

---

## 6. 与 TS 版 cordis 4.0.5 的差异（对宿主有影响的）

`docs/CORDIS-RUST.md:9-49` 有权威清单，下面是「影响宿主设计」的部分：

| 维度 | TS `@deepseek-ai/cordis` 4.0.5 | Rust `zeokeel-cordis` 0.6.0 | 影响 |
|---|---|---|---|
| 服务访问 | `ctx.foo` 属性 + Proxy，`declare module` 扩展 | `ServiceKey<T>` + `ctx.get(KEY)`，编译期类型安全 | 包契约必须显式常量，不能靠字符串约定 |
| 插件形态 | 可调用函数 / class extends Service / `ctx.mixin` | 只有 `Plugin` / `AsyncPlugin` trait | 实现包必须 newtype + impl |
| `inject` | name → intercept config map | `&'static [&'static str]` 名字数组 | 不能通过 inject 传配置 |
| 内部钩子 | 字符串 key 的 metadata | typed 方法（9 条） | 宿主审计要写类型化 hook |
| 事件 | 同步/异步混合，Promise listener | **两条 lane 严格分离**，`EventResult` | UI/事件桥要选 lane |
| 事件作用域 | 子树 walk？ | **isolate 映射相等**（非子树） | 跨包事件必须靠 `global` 或同一作用域 |
| 动态加载/loader | **核心不含，但生态有** `@cordisjs/plugin-loader`、`plugin-include`、`plugin-group`、`plugin-hmr`、`plugin-timer`、`logger-console`（TS README + `package.json` peerDependencies） | **完全没有**，且明确列为宿主职责 | Rust 侧宿主要自建 loader/HMR/timer；HMR 最多做到「配置/实现替换 + restart」 |
| Rust-only 扩展 | 无 | 结构化 `CordisError::Plugin{plugin,kind,message}`、`on_lifecycle` 广播、可注入 `Clock`、`BlockingExecutor`、L2 supervision | 宿主可用的额外观测/确定性能力 |

> 关键结论：**TS 的「一切皆包」是生态层的，不是 core 的**；Rust 侧连生态层都还没有。所以本项目的 loader/包管理/替换编排**全部要从零写在宿主**，内核只保证替换时的语义安全。

---

## 7. 对「一切皆包 + CI 阶段可替换」的约束

### 7.1 由**内核保证**、宿主可以直接依赖的不变量

`docs/DESIGN.md:156-172` 列出 I1–I12，逐条对应 `spec/CORDIS-CORE.md` 的 CORE-0xx 用例与 `tests/`：

| # | 不变量 | 内核证据 | 对「可替换」的用处 |
|---|---|---|---|
| I1 | effect 逆序、单次清理；fiber 卸载后不残留服务/监听器 | `src/effect.rs:36-53`；`src/fiber.rs` unload | 换包后无残留 |
| I2 | 激活失败必须回滚已注册的一切 | `src/fiber.rs:2141-2199` | 新包 apply 失败不会留半初始化 |
| I3 | panic 不跨越内核边界（waterfall 例外 fail-closed） | `src/lib.rs:48-55` + 各 catch_unwind | 新包崩溃不会拖垮宿主 |
| I4 | 依赖变化驱动激活/停用与重算 | `src/fiber.rs:1091-1129`；`src/context.rs:775` | 换 provider 自动影响依赖方 |
| I5 | 服务身份由 `(scope, name)` 唯一；类型错误报错而非 UB | `src/service.rs:120-141`；`src/error.rs:51-55` | 契约破坏可检测 |
| I6 | `ctx.set` 只能改自己的服务；撤销逆序 | `src/context.rs:1993-2040` | 包不能越权改别人的服务 |
| I7 | 生命周期互斥；Disposed 终态 | `src/fiber.rs:3656-3700`（token）；`src/error.rs:30-51` | 并发替换不会撕裂 |
| I8 | 子 fiber 属父 fiber，父卸载级联回收 | `src/fiber.rs:2408`/`3015` | 子树整体替换原子性（单 key） |
| I9 | 替换/重载保持 fiber 身份，失败原子回滚 | `src/registry.rs:1101-1140`、`1334-1400` | 换实现保留配置/身份 |
| I10 | 静默可在无生命周期操作在飞时立即返回，不漏唤醒 | `src/registry.rs:1004` | 安全的时间点做替换/关闭 |
| I11 | 取消是协作式且 per-fiber，不误伤兄弟 | `src/fiber.rs:483`；`src/context.rs:713` | 单包取消可隔离 |
| I12 | `dispose_all` 遇 panic 仍尝试所有 fiber，最后抛出 | `src/registry.rs:1409-1464` | 关闭不因一个坏包中断 |

**注意 I1–I12 的边界**：这些不变量只对**通过内核原语注册的资源**成立。插件自己 `thread::spawn`、开 fd、起 tokio task 而不包成 effect，内核**看不见、也不会回收**（`docs/GUIDE.md:853-856`）。这正是宿主必须加「包实现规范 + 静态检查 + 验收测试」的原因。

### 7.2 **必须由宿主实现**的机制（清单 + 原因 + 建议）

| # | 宿主机制 | 内核为什么不管（证据） | 建议实现 |
|---|---|---|---|
| H1 | **组合根 / 包清单**：profile → crate → impl 的映射、注册顺序、启用/禁用 | 内核无 loader/manifest（`PluginManifest` 仍是路线图，`docs/KERNEL-ISOLATION.md:287`）；`PluginKey` 不是包 id | 编译期 include 生成的 registry 表；每包一个 `install(ctx)` 适配器；name 用命名空间 |
| H2 | **配置来源**（文件/env/DB/远程）+ 默认值分层 + 迁移 | 内核只做 `ConfigSchema` 校验/合并；文件/环境归宿主（`docs/KERNEL-BOUNDARY.md:81-84`、`130-150`） | 宿主 loader 解析后 `ctx.plugin(impl, cfg)`；用 `fiber.raw_config()` 持久化 |
| H3 | **运行时**：tokio、定时器、事件循环、Tauri 主循环 | 内核零运行时依赖，`BlockingExecutor` 是宿主 seam（`src/exec.rs:189-234`） | 启动时 `set_blocking_executor`（**OnceLock 先装者赢，全进程只能一次**）；定时器做成 plugin，用 effect 包 `JoinHandle::abort` |
| H4 | **任务所有权**（tokio spawn → effect） | 内核不跟踪线程/任务（`docs/GUIDE.md:853-856`） | 提供 `TaskService` 基础插件：`spawn` 返回的 handle 在 cleanup 中 abort/await；禁止裸 `tokio::spawn` |
| H5 | **关闭编排** | `dispose_all()` 是同步阻塞边且可能 `LifecycleBusy`；无 `dispose_all_async`（`src/context.rs:738`、`src/registry.rs:1409`） | 在非 reactor 线程执行：`block_on(when_idle())` → `dispose_all()`（失败重试至 `is_idle`）→ `drain_orphan_async_cleanups()` → 关 tokio |
| H6 | **权限/审计/能力策略** | 内核明确不做权限；`inject` 不是能力授权（§0 #8）；`internal/*` 只是通道，插件仍可直接 IO | 用「独立 root / isolate + 只暴露受控服务」做软隔离；硬隔离必须进程/WASM；审计用 `on_lifecycle` + typed hooks + logger exporter |
| H7 | **资源计量与处置策略** | 内核只给 effect/listener/child 配额与 heartbeat；线程/fd/CPU/内存不给（`docs/KERNEL-ISOLATION.md:272-289`） | L2 supervisor：`stalled_fibers` + `cancel` + 自定义计量；L3 每包组子进程 + SIGKILL/rlimit/cgroup；L4 wasmtime fuel/epoch |
| H8 | **不可信包隔离** | 内核不是沙箱（`docs/KERNEL-ISOLATION.md:13-21`） | 进程隔离（推荐信任边界弱时）或 WASM；内核侧只保留「远端包适配器插件」 |
| H9 | **可观测性**：日志导出/指标/trace/UI 事件桥 | 默认无 exporter；指标命名归宿主（README:276） | `ctx.log_exporter(TracingExporter)`；`on_lifecycle` → Tauri event；指标命名在宿主 |
| H10 | **时间源与确定性** | `Clock` 只能 root 设置一次，日志 facade 快照 clock（`docs/GUIDE.md:688-693`） | 启动时 `set_clock`；测试用假 clock 重放 |
| H11 | **包/实现的静态检查**（禁止 IO/线程/全局状态、依赖白名单） | 内核护栏只管自己 `src/**`，不覆盖插件 | 照抄 `tests/kernel_boundary.rs` 思路做宿主侧 `packages/**` 扫描 + CI 门禁 |
| H12 | **契约测试矩阵**（每个 package id 至少 N 套实现跑同一套测试） | 内核只保证语义，不保证「换实现后业务契约不变」 | CI：`profiles/*.toml` × `tests/contracts/*`；失败的 profile 即阻断发布 |
| H13 | **持久化/状态迁移** | 内核只有 raw config；无版本化状态存储 | 宿主 store；`update_with_no_save` 的 `no_save` 只是给 hook 的提示，内核不落盘（`docs/GUIDE.md:566-570`） |
| H14 | **产品/UI/Agent 契约** | 明确不属于内核（`docs/KERNEL-BOUNDARY.md:86`） | Tauri commands/events；rig 适配器做成插件提供 LLM 服务 |

### 7.3 **根本做不到**（不要写进架构假设）

| 目标 | 根因（证据） | 可行替代 |
|---|---|---|
| **运行时热替换 crate / 加载 .so 插件** | 无 ABI、无 dylib、静态链接、`forbid(unsafe_code)`；Rust ABI 不稳定（`docs/CORDIS-RUST.md:645-655`） | 构建期替换 + 重编译；运行时只换已链接实现（`Registry::replace`）；或进程/WASM 边界 |
| 进程内沙箱 / 不可信第三方代码安全执行 | 内核只做 unwinding panic 遏制，明确声明不是 sandbox（`docs/KERNEL-ISOLATION.md:13-21`） | L3 每包组子进程（IPC + seccomp/rlimit/cgroup）或 L4 wasmtime |
| 强杀卡死/死循环的插件 | Rust 无安全机制杀死线程；watchdog 只观测不强杀（`docs/KERNEL-ISOLATION.md:120-126`、`206`） | 子进程 + SIGKILL；或 WASM fuel/epoch 中断 |
| 进程 abort / OOM / 栈溢出 / UB 的进程内恢复 | `catch_unwind` 只处理 unwinding panic；`panic=abort` 已被编译期拒绝（`src/lib.rs:52-55`） | 外层 supervisor 进程重启；L3/L4 |
| 用 `inject` 做权限/能力控制 | `inject` 只是激活门；`ctx.get` 读通道更宽松，且 accessor/内部服务只对读通道可见（`spec/CORDIS-CORE.md:132-140`） | 能力边界必须靠进程/WASM 或「每包独立 root + 最小服务暴露」 |
| 内核级线程/fd/内存配额 | 内核不跟踪线程/fd/内存（`docs/KERNEL-ISOLATION.md:272-289` 第 8 条） | 宿主计量 + cgroup/rlimit；L4 内存上限 |
| **多 key 原子替换**（全部成功或全部回滚） | `Registry::replace` 只覆盖一个 key；回滚自身可能失败并返回 `Rollback`（`src/registry.rs:1101-1140`） | 宿主用「新 root 全量构建 → 切换 → 旧 root dispose」做蓝绿；或用 `when_idle` 串行化 |
| 依赖拓扑排序 / 声明式加载顺序 | 内核只有 `inject` 门控 + 有界 64 轮 refresh，没有拓扑排序 API（`src/context.rs:775-810`） | 组合根按层加载（contract → base → feature）；或加载后 `refresh` 直到 `when_idle` 且全 Active |
| 包版本/来源/签名/供应链 | `PluginKey` 只有 name/TypeId；无 manifest、无版本、无签名（§2.7、§5） | 宿主清单 + Cargo lock + SBOM（内核仓库自己发布 CycloneDX SBOM，可借鉴） |
| 跨构建的稳定包身份 | `TypeId` 跨编译不稳定；`Named` 只是 `Plugin::name()` | 宿主 profile 的 `package id` 作为唯一身份；不要在持久化里存 `PluginKey`/`TypeId` |
| 插件崩溃后自动重启（语义级） | 内核无重启策略，只有 `restart` 原语；策略归宿主（`docs/KERNEL-BOUNDARY.md:60`） | 宿主 supervisor：`on_lifecycle` 监听 Failed → 决策 restart/隔离/告警 |
| 插件互相调用不经过内核 | 内核无法阻止插件直接 `use` 对方 crate（同二进制内） | 包规范 + CI 依赖图检查（只允许依赖 contract crate）；或进程边界 |

### 7.4 文档 vs 代码：差异与文档缺口（以代码为准）

1. **`ctx.plugin` 的返回语义未写清**：文档只说「激活失败 → fiber `Failed` 并回滚」（`docs/DESIGN.md:119-121`、`docs/GUIDE.md:492`），没有明说**调用仍返回 `Ok`**。代码 `src/context.rs:1393-1395` 是 `let _ = activate_fiber(...); Ok(fiber)`。**以代码为准**：宿主必须 `fiber.await_settled().await` 或检查 `state()`。这是文档缺口，不是矛盾。
2. **生命周期重入的表述过简**：`docs/DESIGN.md:145` 说「同线程重入被允许（如清理中再次调用 dispose）」，但 0.6.0 起是**操作级 token**：同线程但在某个 async lifecycle 挂起期间的 sync 调用会返回 `LifecycleBusy`（`docs/KERNEL-ISOLATION.md:173-179`、`docs/GUIDE.md:551-555`；实现 `src/fiber.rs:3656-3700`）。以代码/0.6.0 文档为准。
3. **`DESIGN.md:121` 的「看到互斥错误或等待」**：代码里没有「等待」，只有 `LifecycleBusy` 且**不排队**（`src/error.rs:39-43`）。宿主必须自己重试/串行化。
4. **`KERNEL-ISOLATION.md` 的测试数字/性能声明**（466/481 tests、2 MiB 线程 depth 800 等）属于文档声明，本次只跑了 `hello/services/events/async_plugin` 与 `kernel_boundary`，未复跑全量 perf 断言。标为**未独立验证**。
5. **`PluginManifest` 不存在**：只在 `docs/KERNEL-ISOLATION.md:287` 的「路线图」里出现；`src/**` 无此类型（grep 确认）。不要按 manifest 设计宿主接口。
6. **`README` 里 `ctx.logger()` 默认无 exporter** 与代码一致（`src/context.rs:1908`、`logger.rs`），但这是极易踩的坑：不装 exporter 就完全看不到插件日志。
7. **`Registry` 快照不是事务视图**：`size/keys/values/fibers` 各自加锁，跨调用可能不一致（`docs/CORDIS-RUST.md:584-590`）。宿主诊断代码不能把它当一致快照。

---

## 8. 实践注意（宿主落地要点）

### 8.1 一个进程多个 root / 子作用域（已探针验证）

```rust
let root_a = Context::root();   // 独立 Registry/Events/Logger/service_store
let root_b = Context::root();   // 与 root_a 完全隔离
```

探针实测（见 §9 复现）：

```
3 root_a sees=Some("a") root_b sees=None
3 after a.dispose_all: b fiber state=Active b.registry.fiber_count=1
```

结论：**可以一个进程跑多个独立容器**，适合「每窗口/每会话/每 agent 一个 root」。
但注意：
- **进程级单例**：`BLOCKING_EXECUTOR`（`src/exec.rs:207`，先装者赢）、orphan 队列（`src/async_effects.rs:281`）、若干计数器（`src/service.rs:23`、`src/fiber.rs:3656/3897`、`src/registry.rs:33`）。BlockingExecutor 只能装一次 ⇒ 宿主启动时统一安装；多 root 共享它。
- **root 之间没有事件/服务互通**（各自 Events/service store）⇒ 跨窗口通信必须走宿主自己的总线（Tauri event / channel），不能指望内核。
- **同一个 root 内**：`isolate` 只隔离服务/事件，**不隔离 Registry**——插件注册表是 root 全局的，所有 scope 的插件都在同一 registry（`src/registry.rs:402-420`）。所以「按包隔离注册表」需要独立 root。
- 子作用域：`extend()`/`isolate()`/`intercept()` 只派生 Context（不新建 fiber，`src/context.rs:231-334`），fiber_id 不变；`ctx.plugin` 的子 fiber 才计入 ownership 树和 `MAX_PLUGIN_DEPTH`。

### 8.2 插件依赖排序（内核没有拓扑排序）

- `inject` 只决定**能不能激活**，不决定**先加载谁**：先加载消费者 → `Pending`；provider 后到 → 自动 `Active`（探针验证：`pre-provider=Pending` → `post-provider=Active`）。
- `refresh()` 有界 64 轮、按注册顺序扫描；**反向依赖链**每轮推进一层 ⇒ 深链需要多次 refresh（`src/context.rs:775-810`）。
- 实用策略：**组合根按层加载**（contract/base → infra → feature → UI），加载完 `block_on(when_idle())`，再断言「所有期望包 `Active`」；失败包用 `error_detail()` 报给用户。
- `MAX_PLUGIN_DEPTH = 256`（`src/fiber.rs:1644`）：递归 `ctx.plugin` 超过即 `InvalidPlugin`。
- **不要**制造跨线程环形注册等待：内核不检测（`docs/KERNEL-ISOLATION.md:125-126`、`146`）。

### 8.3 错误类型速查（`src/error.rs:9-88`）

| 变体 | 何时 | 宿主处理 |
|---|---|---|
| `PluginFailed(msg)` | apply 返回 Err / panic / reload panic | 显示 + 决定重启；细节看 `fiber.error()` |
| `Plugin { plugin, kind, message }` | 插件用 `CordisError::plugin(...)` 主动分类 | **推荐插件用**；宿主 match `kind` 做策略 |
| `Validation(err)` | schema 校验失败 | 把校验信息回显给配置界面 |
| `LifecycleBusy` | 生命周期操作并发/挂起 | 串行化或重试（勿丢弃） |
| `Disposed` | fiber/context 已销毁 | 停止后续操作 |
| `DuplicateService(name)` | 同 scope 重复 provide | 包冲突 → 组合根命名空间问题 |
| `UnknownService(name)` | `require` 不存在 | 依赖缺失（inject 没声明/没加载） |
| `ServiceWriteForbidden` / `ServiceTypeMismatch` / `AccessorReadOnly` / `AccessorWriteRejected` | 服务写权限/类型/accessor | 视为包契约违规 |
| `AsyncPluginNeedsAsync(name)` | 对 async 插件用了 sync API | 全部改用 `*_async` |
| `Cancelled` | checkpoint 观测到取消 | 走正常失败/回滚路径 |
| `QuotaExceeded(kind)` | supervision 配额 | 拒绝该包并告警 |
| `Rollback(msg)` | `Registry::replace` 回滚失败 | 宿主状态可能不一致：重建 root |
| `InactiveEffect` | 非 active context 注册 effect | 检查激活顺序 |

### 8.4 async 支持程度

- 两条 lane 严格分离（§2.6）；`AsyncPlugin` 的所有生命周期操作必须用 `*_async`（否则 `AsyncPluginNeedsAsync`）；
- **没有 `dispose_all_async`**：`dispose_all()` 是同步阻塞边（内部 `Fiber::dispose()` → `block_on`），在 tokio current_thread 上会死锁，除非已安装 `BlockingExecutor`（`docs/GUIDE.md:838-843`）；
- 同步 `dispose()` 也会阻塞等待 async cleanup；async 宿主优先 `dispose_async()/restart_async()/update_async()`；
- 内核 `block_on` 只是 park/unpark，不能驱动 reactor I/O；`effect_future`/`effect_stream` 注册时也用它（§2.4）；
- clippy `await_holding_lock = deny` 说明内核自己避免持锁跨 await；插件侧宿主应加同样 lint；
- 取消是协作式：长任务必须 `ctx.checkpoint()`；否则只能靠 L3 杀进程。

### 8.5 关闭顺序（推荐 recipe）

```rust
// 在独立阻塞线程执行（非 reactor）
block_on(ctx.when_idle());                 // 1) 等注册/激活/卸载窗口结束（recovery-aware）
loop {                                     // 2) 逐步 dispose_all，处理 LifecycleBusy
    match ctx.dispose_all() {
        Ok(()) => break,
        Err(CordisError::LifecycleBusy) => { block_on(ctx.when_idle()); }
        Err(e) => { log::warn!(...); break; } // 3) 其它错误记录后继续收尾
    }
}
let drained = drain_orphan_async_cleanups(); // 4) 回收 Drop 跳过的 async cleanup
// 5) 最后再关 tokio runtime / Tauri
```

（依据：`src/registry.rs:1409-1464`、`src/context.rs:738`、`docs/KERNEL-ISOLATION.md:185-195`、`docs/GUIDE.md:579-637`。）

### 8.6 Tauri2 / React / rig 的落点建议

- **Tauri 后端 = 组合根 + 宿主服务**：启动时建 root、`set_clock`、`set_blocking_executor`、装 logger exporter、加载 profile 选中的包；`tauri::State` 持有 `Context`/`Fiber` 句柄（`Context` 是 `Clone + Send + Sync`）。
- **React 前端不是 cordis 插件**：内核是 Rust-only。UI 的「包化」是前端构建期的事（例如按 profile 选择前端模块）；Rust 与前端之间只有 Tauri commands/events。`on_lifecycle`/typed hooks 是桥接观测的最佳点。
- **rig（LLM）适配器做成插件**：网络/传输属于插件层（`docs/KERNEL-BOUNDARY.md:74-86`）；它 `provide` 一个 `ServiceKey<dyn Llm>`（trait object 服务走 `provide_any`/`get_any`，`src/context.rs:589-627`），其他包只依赖 contract crate 的 key，这样 CI 可以替换成 mock/本地/远端实现。
- **包替换验收测试**：每个可替换包至少两类 CI——(a) 契约测试（同一套测试对每个 impl）；(b) 替换测试（profile A↔B 切换后，宿主启动、`when_idle`、全 Active、关键用户路径、关闭无残留）。
- **禁止清单**（放进包规范/CI）：插件里禁止裸 `tokio::spawn`、裸 `std::thread`、全局 `static mut`、直接读 env/文件/网络（除宿主注入的受控服务）、`std::process`；这些都应交由宿主基础服务或显式声明。

---

## 9. 本报告的验证记录（可复现）

环境：cargo 1.99.1 / rustc 1.99.1，离线。

```bash
cd /tmp/cordis-ref
cargo run --offline --example hello        # hello, Cordis / bye, Cordis
cargo run --offline --example services     # uptime: … / consumer is now pending / uptime: …
cargo run --offline --example events       # sync/once/bail/waterfall 输出符合 §2.6
cargo run --offline --example async_plugin # async plugin active / async cleanup ran
cargo test  --offline --test kernel_boundary   # 11 passed; 0 failed（4 真实断言 + 7 防绕过自测）
```

另用一个临时 `rustc` 探针（不写入任何仓库，已删除）连编 `target/debug/deps/libzeokeel_cordis*.rlib`，验证 8 条代码事实：

```
1 failing-plugin: returned_ok=true state=Failed error_detail=Some("plugin error: failing (config): boom")
2 pre-provider consumer state=Pending
2 post-provider consumer state=Active provider=Active
2 after-provider-dispose consumer state=Pending
3 root_a sees=Some("a") root_b sees=None
3 after a.dispose_all: b fiber state=Active b.registry.fiber_count=1
4 first dispose=true second dispose=true state=Disposed
5 duplicate provide -> Some("service already provided: who")
6 root sees=Some("root") iso sees=Some("iso")
7 registry.size=1 fiber_count=2
8 typed registry.size=1 fiber_count=2
```

未验证项：全量测试数量（466/481）、性能/深度回归声明、Miri、跨平台 CI —— 这些是文档声明，本文不作为结论依据。

---

## 10. 关键文件索引（供继续深挖）

| 主题 | 位置 |
|---|---|
| 公共 API 权威清单 | `src/lib.rs:57-105` |
| 设计思想/不变量 I1–I12 | `docs/DESIGN.md:26-82`、`156-172` |
| 内核/插件/宿主边界 + CI 护栏 | `docs/KERNEL-BOUNDARY.md:28-90`、`153-241` |
| 故障隔离/panic 矩阵/L1–L4 | `docs/KERNEL-ISOLATION.md:50-126`、`201-211`、`259-270` |
| 使用指南（服务/effect/事件/隔离/生命周期/关闭/取消/L2） | `docs/GUIDE.md:170-660`、`801-904` |
| 与 TS 的差异 | `docs/CORDIS-RUST.md:9-49`、`645-655` |
| 规范性语义/用例 | `spec/CORDIS-CORE.md:22-52`、`76-141`、`142-376`、`696-969` |
| 激活事务实现 | `src/fiber.rs:2091-2199`、`2201-2330` |
| 依赖版本/epoch | `src/fiber.rs:1091-1129`；`src/service.rs:23-141` |
| 卸载/级联/收尾 | `src/fiber.rs:2408`（finish_unload）、`2509`（CascadeFrame）、`2840`（begin_cascade）、`3099`（drain_cascade_dependents）、`3306`（begin_unload_activity） |
| 同步/异步 effect | `src/effect.rs`、`src/async_effects.rs`、`src/context.rs:1132-1200`、`1648-1745` |
| 事件/waterfall | `src/events.rs`、`src/waterfall.rs`、`src/context.rs:1480-1830` |
| Registry/replace | `src/registry.rs:1044-1140`、`1334-1400`、`1409-1464` |
| 边界护栏测试 | `tests/kernel_boundary.rs`（`PUBLIC_ITEM_CAP` 在 `:142`）|
| 宿主可注入 seam | `src/exec.rs:202-234`（BlockingExecutor）、`src/logger.rs:101`（LogExporter）、`src/clock.rs:31`（Clock）、`src/internal.rs:178-191`（typed hooks） |
| examples | `examples/hello.rs`、`services.rs`、`events.rs`、`async_plugin.rs` |

---

### 一句话给架构决策

> 内核能保证的是「**语义上的安全替换**」：换掉一个已链接的包实现、重启它的 fiber、回滚失败的激活、撤销它注册的一切；它**不能**保证「**代码层面的可替换性**」——那必须由宿主的构建期组合（Cargo/profile/CI）和运行时编排（组合根/loader/supervisor）承担。把「包」定义在宿主仓库的 contract + profile + 契约测试上，内核只当运行库用；这样「一切皆包、CI 可替换」才是可执行的，而不是一个内核 API 承诺。
