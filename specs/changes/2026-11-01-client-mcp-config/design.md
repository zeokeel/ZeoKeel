# 2026-11-01-client-mcp-config · 技术设计

> 规范细节链接 `docs/`，不在这里复制第二份规范。组合/manifest 以 `24` 为准，API 以 `04` 为准。
> 范围：MCP 本地配置（personal）+ 两级校验 + 无缓存测试连接 + personal-only UI pane + 企业 catalog-only 口径。
> 审批策略/`connectors.*` 见父 spec；本 spec 不写审批卡/工具行/设置 IA。

## 1. 影响面

| 类型 | 变更 |
| --- | --- |
| 包 | 新增 `zeokeel/ui-settings-mcp`（kind `ui`，role `ui.slot.settings-mcp`，editions `["personal"]`，贡献 `ui.settings.pane` order 70）；修改 `zeokeel/connector-mcp`（本地源/探测 API）；`zeokeel/ui-settings` 不改（pane 由贡献注入）；无新 Rust 业务包 |
| 契约 | `zk-contract-connector`：`McpServerConfig::validate()`（从 client 包提升，规则原样）；`zk-contract-config`：`McpSettings { servers }` + `AppSettings.mcp` + `McpSettings::validate()`（本地上限）+ 依赖 `zk-contract-connector`；`zk-contract-ipc`：`MCP_TEST`/`MCP_TEST_STATUS` 常量 + `McpTestPayload`/`McpTestResult`/`McpTestStatusResult`；**`packages/contracts/ipc/Cargo.toml` 新增 `zk-contract-connector` path 依赖**（`McpTestPayload.server` 引用其 DTO；无环，connector 契约无契约依赖） |
| 路由 | 无（本地 IPC；服务端 catalog 形状不改） |
| 迁移 | 无 DB 迁移；设置 JSON additive（`schema` 保持 1）。env 迁移：不自动导入（M-4 已定），仅 release note |
| 生成物 | `shared/contracts/schema/config-*.json`、`connector-*.json`、`ipc-*mcp-test*.json`、`shared/contracts/ts/contracts.ts`、`docs/generated/CONTRACT-INDEX.md`；生成器写（C10/C11） |
| edition/profile/channel | `editions/personal.toml` 映射 `ui.slot.settings-mcp`；`editions/enterprise.toml` exclude；**`editions/personal-only.toml:3` 的 `packages` 数据列表**加入新包（P1-15）；profile/channel 不变 |
| 文档 | 任务 1 先行：`docs/15`（D 编号或 ADR，C1/C6）、`docs/07` §6.1、`docs/16`（新包/槽位）、`docs/24` §1 与 `docs/04` §1.5（契约依赖登记 P2-8）、`docs/38`（MCP 能力注记）；`docs/README.md` 不新增主题 |

## 1.5 复用检查（C17，必填）

| 要实现的点 | 现有能力（capability id / 路径） | 结论 | 归属 |
| --- | --- | --- | --- |
| MCP 配置 DTO | `zk-contract-connector::McpServerConfig`（`packages/contracts/connector/src/lib.rs:170-199`） | 复用（不新建 DTO） | 契约 |
| MCP 校验规则 | `zk_connector_mcp::validate_server`（`packages/client/connector-mcp/src/lib.rs:102-148`）+ `valid_server_id`（`:307-313`） | **原样提升**到 `zk-contract-connector::McpServerConfig::validate()`；client 委托 | 契约 |
| catalog→servers 映射/应用 | `servers_from_catalog`（`:186-218`）、`update_from_catalog`（`:220-232`）、`clear_catalog_config`（`:236-243`）、`reload_connectors`（`:288-294`）、`configured_server_ids`（`:276-285`） | 复用（catalog 路径行为不变） | `connector-mcp` |
| 会话/transport 创建 | `with_session`/`SESSIONS`（`:261-294,406-413`；transport 构造 `:415-422`） | 扩展（抽出一次性 transport 创建用于无缓存探测；禁止复用全局缓存） | `connector-mcp` |
| 测试连接展示用 tool 列表 | `list_server_tools`（`:442-451`） | **不复用**于探测（走全局缓存，评审 F22）；仅作为 UI 既有展示语义参考 | `connector-mcp` |
| settings 存储/校验/失败不落盘 | `SettingsStore`（`packages/contracts/config/src/lib.rs:653-658`）、`store-settings-json`、`settings.set`（`apps/desktop/src/ipc.rs:531-592`）、`masked_settings`（`:1469-1481`） | 复用 | 契约 + desktop |
| enterprise 能力清理先例 | `configure_persisted_capabilities` 的 embedding/media 清理（`apps/desktop/src/ipc.rs:269-295`） | 复用模式（MCP 段清理） | desktop |
| 两段式授权/轮询先例 | `oauth.authorize`/`oauth.status`（`packages/contracts/ipc/src/lib.rs:125-133`；`apps/desktop/src/ipc.rs:1282-1330`（authorize）/`:1340-1364`（status）） | 复用模式（`mcp.test`/`mcp.test.status`） | desktop |
| settings pane 扩展 | settings-ia 的 `ui.settings.pane` 多贡献 slot、`SettingsPaneProps { t, draft, update, errors, saving }` | 复用（本 spec 只贡献 pane） | ui-settings |
| personal-only UI 包 | `zeokeel/ui-settings-provider` 四件套 + editions exclude（`packages/ui/ui-settings-provider/zk.toml`、`editions/enterprise.toml:94`） | 复用先例（结构复制、代码不复用） | ui |
| 生成物 | `cargo test --workspace export_schema`、`scripts/schema-ts.py`、`zk compose` | 复用 | 生成器 |

- 搜索证据（2026-11-01，`./target/debug/zk`）：`zk find mcp` → **0**（不新增 capability id）；`zk find connector` → `contract.connector`；
  `zk find settings` → `store.settings.json`；`zk find config` → `contract.config`；`rg -n "validate_server|configured_servers|list_server_tools" packages apps` 确认唯一实现位置。
- 若「新建」：唯一新建包 = `zeokeel/ui-settings-mcp`（personal-only pane；理由见 §3.4）。不新建 Rust 能力包/kit。
- 新增可复用能力：`McpServerConfig::validate()`（`contract.connector` 既有 capability）、`McpSettings::validate()`（`contract.config`），
  不新增 capability id；`connector-mcp` 新增 `update_local_servers`/`clear_local_servers`/`set_env_fallback_enabled`/`test_server`（同包内 API）。

## 2. 方案

### 2.1 契约与两级校验（P1-8）

```rust
// packages/contracts/connector/src/lib.rs —— 现有规则原样搬移（语义逐字节不变；不含本地上限）
impl McpServerConfig {
    /// Validate one MCP server config (same rules as the previous
    /// `zk_connector_mcp::validate_server`; catalog and local paths both call this).
    pub fn validate(&self) -> Result<(), String> { /* id/name 非空、transport、url(https|loopback http)、url 长度/字符、secret-like env */ }
}
```

```rust
// packages/client/connector-mcp/src/lib.rs —— 委托（保持既有 pub API，测试继续可用）
pub fn validate_server(server: &McpServerConfig) -> Result<(), String> { server.validate() }
```

```rust
// packages/contracts/config/src/lib.rs —— 本地上限只在这里（catalog 不受影响）
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(default)]
pub struct McpSettings {
    /// personal 本地 MCP server（非 secret，ADR-060；enterprise 不允许本地段）
    pub servers: Vec<zk_contract_connector::McpServerConfig>,
}
impl McpSettings {
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.servers.len() > 16 { return Err(ConfigError::Invalid("mcp.servers exceeds 16".into())); }
        // 同 id 去重、id 字符集 [A-Za-z0-9._-]、id/name ≤128、args≤32/≤2048、env≤32/键≤128/值≤2048、url≤2048、timeout≤600000
        // oauth_required=true → 拒绝（本地无 OAuth 元数据）
        // 每个 server 先 McpServerConfig::validate()
    }
}
pub struct AppSettings { /* ... */ #[serde(default)] pub mcp: McpSettings }
```

> **关键**：不把本地上限放进 `McpServerConfig::validate()`。catalog 的 `servers_from_catalog` 继续调用 `validate_server`
> （= `validate()`），因此「catalog 旧合法但超本地上限」的 server **仍可用**；只有 `settings.set` 路径调用 `McpSettings::validate()`。

### 2.2 来源优先级与 edition（T4/P1-10）

```rust
// connector-mcp：新增 LOCAL_SERVERS 槽 + env 开关
static LOCAL_SERVERS: OnceLock<Mutex<Option<Vec<McpServerConfig>>>> = OnceLock::new();
static ENV_FALLBACK: AtomicBool = AtomicBool::new(true);

pub fn update_local_servers(servers: &[McpServerConfig]) -> Result<(), String> {
    // 每个 server.validate()（不是 McpSettings::validate，上限在 config 契约已校验）
    // 同 id 拒绝；写槽；reload_connectors()
}
pub fn clear_local_servers() -> Result<(), String> { /* 槽 = None；reload */ }
pub fn set_env_fallback_enabled(enabled: bool) { ENV_FALLBACK.store(enabled, SeqCst); }

fn configured_servers() -> Result<Vec<McpServerConfig>, String> {
    if let Some(catalog) = catalog_slot().lock()?.as_ref().cloned() { return Ok(catalog); } // 含空列表
    if let Some(local) = local_slot().lock()?.as_ref().cloned() { return Ok(local); }         // 含空列表
    if ENV_FALLBACK.load(SeqCst) { return servers_from_env(); }
    Ok(Vec::new()) // enterprise：catalog-only，无 catalog/未登录 = 无 MCP
}
```

**desktop 装配**（`apps/desktop/src/mcp.rs` + `ipc.rs`）：

| 时机 | personal | enterprise |
| --- | --- | --- |
| boot（`apply_persisted_settings` 旁） | 设置文件存在 → `update_local_servers(&settings.mcp.servers)`；文件不存在 → `clear_local_servers()`（保持 env fallback） | `clear_local_servers()` + `set_env_fallback_enabled(false)`；若有持久化本地段 → 清空并保存回写（对齐 embedding/media 先例 `ipc.rs:269-295`） |
| `settings.set` | `McpSettings::validate` → `update_local_servers`（失败则不落盘） | 非空 `mcp.servers` → 拒绝（`settings/invalid: mcp.local-forbidden-in-enterprise`）；空 → 允许 |
| `auth.logout` | 不清本地配置（personal 无登录） | 沿用现有 `clear_catalog_config`（父 spec 接线），本地槽本就为空 |

> edition 判定在桌面用 `zk_composed::EDITION`（既有用法 `ipc.rs:276,1496`），`connector-mcp` 不感知 edition（保持包内纯逻辑）。
> C1/C6 映射：企业差异来自**构建期包集合 + 客户端来源策略**，不是服务端下发/管控；需 `docs/15` D 或 ADR 落定（任务 1）。

### 2.3 无缓存测试连接（P1-9）

**transport 抽象**（`connector-mcp` 内重构出可复用的一次性连接）：

```rust
// 现有 with_session 内部创建 transport 的逻辑抽为：
fn connect(server: &McpServerConfig) -> Result<ClientTransport, String> { /* stdio spawn / http client；含 initialize */ }

// 直接测试：不复用全局 SESSIONS，drop 即关闭（stdio 子进程 kill + reap，既有 drop 语义）
pub fn test_server(server: &McpServerConfig) -> Result<Vec<String>, String> {
    server.validate()?;
    let mut transport = connect(server)?;
    let tools = transport.list_tools()?;   // 既有 tools/list 解析
    Ok(tools.into_iter().map(|t| t.name).collect())
}
```

**IPC 两段式**（沿用 oauth 先例）：

```rust
// zk-contract-ipc
pub const MCP_TEST: &str = "mcp.test";
pub const MCP_TEST_STATUS: &str = "mcp.test.status";
pub struct McpTestPayload { pub server: zk_contract_connector::McpServerConfig }
pub struct McpTestResult { pub test_id: String }
/// 探测状态（封闭枚举，C10 风格；不用裸 String）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum McpTestState { Pending, Ok, Error }
pub struct McpTestStatusResult {
    pub state: McpTestState,
    pub tools: Vec<String>,   // ok 时 ≤200 条、单条 ≤128
    pub error: Option<String>, // error 时封闭错误码
}

// desktop：与 oauth 状态表同构（`ipc.rs:1282-1330` / `:1340-1364`）
// mcp.test：校验 → 生成 test_id → 线程跑 test_server（超时 clamp 1..=15s）→ 立即返回 {test_id}
// mcp.test.status：返回 McpTestState::{pending|ok{tools}/error{code}}；完成后保留短期供轮询，下一次 test 覆盖
```

- 超时：探测线程在 `timeout_ms`（默认 10s，clamp ≤15s）内完成；超时返回 `mcp/timeout`，transport drop 关闭子进程。
- 并发：同一时刻只允许**一个**探测在飞；`pending` 期间再次调用 `mcp.test` 返回封闭错误码 **`mcp/test-in-flight`**
  （不覆盖、不排队）；完成后下一次 `mcp.test` 覆盖旧结果（旧线程自然结束）。
- 断言：`cargo test -p zk-connector-mcp test_server_no_cache` 比较探测前后 `SESSIONS` 长度（需测试用只读访问器 `session_count()`）；无新增全局副作用。

### 2.4 UI pane（personal-only）

```toml
# packages/ui/ui-settings-mcp/zk.toml
schema = 1
id = "zeokeel/ui-settings-mcp"
kind = "ui"
version = "0.1.0"
license = "MIT"
summary = "Personal-only MCP server settings pane"

[role]
name = "ui.slot.settings-mcp"
contract = "zk-contract-ui"
cardinality = "singleton"
editions = ["personal"]

[facets]
rust = { crate = "zk-ui-settings-mcp" }
ui = { package = "@zeokeel/ui-settings-mcp" }

[[ui.contributions]]
slot = "ui.settings.pane"
component = "settings.mcp"
order = 70
```

- 包结构复制 `ui-settings-provider`：`zk.toml` + `Cargo.toml`(`zk-ui-settings-mcp`) + `src/lib.rs`（`#![forbid(unsafe_code)]` + `@capability` 注记可省）+ `tests/conformance.rs` + `README.md` + `web/{package.json,tsconfig.json,src/index.tsx,tests/contract.test.ts}`。
- props 用 settings-ia 的 `SettingsPaneProps { t, draft, update, errors, saving }`（**不含 edition**；personal/enterprise 差异由构建期包集合表达，C2）。
- pane 内部状态：`servers: McpServerConfigDraft[]`（来自 `draft.mcp.servers`）、行内编辑、`testStateByRow`（idle/pending/ok/error + tools 列表）。
- 测试连接：`mcp.test` → `test_id` → 每 300ms 轮询 `mcp.test.status`，最多 20s / 可取消；结果内联显示（不写入 settings）。
- 校验：前端先做基本校验（必填/url scheme/重复 id），保存时以 Rust 错误为准并映射到 server id 内联显示；i18n 键（zh/en）全新增。
- 六态见 `spec.md` R6 表；axe/键盘/Esc/焦点沿用 ui-kit（`useDialogFocus` 由外壳提供）。

### 2.5 组合与 personal-only 清单（P1-15）

```toml
# editions/personal.toml（新增一行）
"ui.slot.settings-mcp" = "zeokeel/ui-settings-mcp"

# editions/enterprise.toml（exclude 追加）
exclude = ["zeokeel/client-update-github", "zeokeel/ui-settings-provider", "zeokeel/ui-settings-mcp"]

# editions/personal-only.toml（数据清单，`zk validate` 读取，`:3`）
packages = ["zeokeel/client-update-github", "zeokeel/ui-settings-provider", "zeokeel/ui-settings-mcp"]
```

- 断言：`zk validate`（enterprise 闭包不含 personal-only 包）+ `zk compose --edition personal ... --check`（`SettingsPanes` 含 `settings.mcp` order 70）。
- **list slot 语义（P2-N5）**：enterprise 不选中该包 → `zk compose` 生成 `SettingsPanes = []`（该贡献不生成，
  **不存在 single-component `null` stub**）；`ui-settings` 外壳按 settings-ia 的空列表语义渲染，不显示该 pane（见 settings-ia `spec.md` 空态/`SettingsPanes = []` 规则）。

### 2.6 逐文件改动清单（约 21 个工作项）

| # | 文件 | 改动 |
| --- | --- | --- |
| 1 | `packages/contracts/connector/src/lib.rs` | `McpServerConfig::validate()`（规则原样搬移）+ 私有 url/loopback/secret 辅助 |
| 2 | `packages/contracts/connector/tests/conformance.rs` | 规则逐条测试（与旧行为一致） |
| 3 | `packages/contracts/config/src/lib.rs` | `McpSettings`（上限/重复/id/OAuth 拒绝）、`AppSettings.mcp`、`validate()` 调用 |
| 4 | `packages/contracts/config/Cargo.toml` | 依赖 `zk-contract-connector`（无环） |
| 5 | `packages/contracts/config/tests/{conformance,schema}.rs` | 默认/回读/上限/失败路径 + schema |
| 6 | `packages/client/connector-mcp/src/lib.rs` | 委托 `validate`；`LOCAL_SERVERS`/`ENV_FALLBACK`/`update_local_servers`/`clear_local_servers`/`set_env_fallback_enabled`；`connect` 抽取；`test_server`；`session_count()`（测试） |
| 7 | `packages/client/connector-mcp/tests/*` | 优先级/precedence/无缓存探测/catalog 两级校验回归 |
| 8 | `packages/contracts/ipc/src/lib.rs` + `packages/contracts/ipc/Cargo.toml` | `MCP_TEST`/`MCP_TEST_STATUS` + payload/result DTO；Cargo 新增 `zk-contract-connector` path 依赖（P1-13.1） |
| 9 | `packages/contracts/ipc/tests/schema.rs` | 新 DTO schema |
| 10 | `apps/desktop/src/mcp.rs`（新） | `apply_settings_mcp`/boot/enterprise 清理/`test_server` 线程 + 状态表 |
| 11 | `apps/desktop/src/ipc.rs`（含内联 tests） | `settings.set` 应用/拒绝、`mcp.test`/`mcp.test.status` dispatch、boot 调用 |
| 12 | `packages/ui/ui-settings-mcp/zk.toml`（新） | 包 manifest + 贡献（order 70） |
| 13 | `packages/ui/ui-settings-mcp/Cargo.toml` + `src/lib.rs`（新） | Rust facet crate |
| 14 | `packages/ui/ui-settings-mcp/tests/conformance.rs`（新） | 四件套 conformance |
| 15 | `packages/ui/ui-settings-mcp/README.md`（新） | 功能/配置/限制说明 |
| 16 | `packages/ui/ui-settings-mcp/web/{package.json,tsconfig.json}`（新） | pnpm workspace 包 |
| 17 | `packages/ui/ui-settings-mcp/web/src/index.tsx`（新） | MCP pane 组件（列表/CRUD/测试连接/六态/a11y） |
| 18 | `packages/ui/ui-settings-mcp/web/tests/contract.test.ts`（新） | props/交互单测 |
| 19 | `editions/{personal,enterprise,personal-only}.toml` | 映射 + exclude + **packages 数据列表**（P1-15） |
| 20 | `e2e/mcp-config.spec.ts`（新）+ `e2e/serve-m1.sh` | fake stdio MCP server、seeded settings；`test.describe` 内 `test.skip(process.env["ZEOKEEL_E2E_SKIP_SERVE"] === "1", "mock stage")`（P2-N4：第一段 mock 跳过）；并把新 spec 加入 `scripts/m1-e2e.sh` 第二段显式清单 |
| 21 | `packages/ui/ui-kit/web/src/types.ts` | 手写镜像加 `McpSettings`/`McpServerConfig`（T22 过渡期例外；Ajv schema 仍由生成物校验） |

> 生成物（schema/TS/registry/CONTRACT-INDEX）由任务 9 生成器写，不计入上表。文档 6（任务 1/11）另计。

### 2.7 关键流程

```text
(A) 保存配置
设置 pane 编辑 → 外壳合并 draft → settings.set {settings, expected_revision}
  → McpSettings::validate（上限/重复/id/OAuth）→ update_local_servers → reload_connectors
  → 失败：返回错误且不落盘；成功：settings 返回，UI 重置基线

(B) 测试连接
点击测试 → mcp.test {server} → 校验 → 线程 connect→initialize→tools/list→drop
  → 立即返回 {test_id}；轮询 mcp.test.status → pending/ok{tools}/error{code}
  → 超时/失败：内联错误，不写 settings、不污染全局会话缓存

(C) 装载（启动/登录）
personal boot：settings 文件存在 → update_local_servers(settings.mcp.servers)；否则 clear_local_servers（env fallback 可用）
enterprise boot：clear_local_servers + set_env_fallback_enabled(false)；auth.configure → catalog（父 spec）> 一切
```

## 3. 备选与取舍

### 3.1 MCP 存储（M-1，已定 A）

| 方案 | 优点 | 缺点 | 结论 |
| --- | --- | --- | --- |
| A. `AppSettings.mcp.servers` | 复用 settings 存储/校验/失败不落盘/revision/pane 草稿；零新增 CRUD IPC；备份统一 | settings.json 变大（上限约束）；无 per-server revision；契约耦合（P2-8 登记） | **采用（M-1 已定）** |
| B. 独立 store + `mcp.*` CRUD IPC | 关注点分离；per-server 原子写/revision；企业只读视图 | 新包 + 第二套校验/掩码/并发管线（C17）；与 settings-ia「单文档 + revision」冲突 | 放弃（Phase 1） |

### 3.2 校验位置（P1-8）

| 方案 | 优点 | 缺点 | 结论 |
| --- | --- | --- | --- |
| A. 现有规则提升 `McpServerConfig::validate`；本地上限只在新 `McpSettings::validate` | catalog 零回归；单点复用 | 契约 crate 变厚（+~90 行含 url 辅助） | **采用** |
| B. 把本地上限也放进共享 validate | 代码更少 | 企业 catalog 被收紧（评审 F23/P1-8 回归） | 放弃 |
| C. 桌面再写一份校验 | 不动契约 | 第二份规则（C17），catalog/本地漂移 | 放弃 |

### 3.3 测试连接（M-3，已定 A）

| 方案 | 优点 | 缺点 | 结论 |
| --- | --- | --- | --- |
| A. 两段式 `mcp.test` + `mcp.test.status`，无缓存 transport | 不阻塞 IPC、可取消、不污染 SESSIONS（P1-9） | 多一条命令/状态表 | **采用** |
| B. 单命令同步 `list_server_tools` | 少一条命令 | 复用全局缓存（草稿会话驻留）+ 阻塞 15s（评审反对） | 放弃 |
| C. 复用 `connectors.reload` 后间接验证 | 无新命令 | 会改变生效配置，语义不对 | 放弃 |

### 3.4 pane 归属（M-5，已定 A）

| 方案 | 优点 | 缺点 | 结论 |
| --- | --- | --- | --- |
| A. 新 personal-only 包 `ui-settings-mcp` | 构建期 edition 差异（C2）；可独立替换；enterprise 不渲染 | 新增四件套包 | **采用** |
| B. 放 `ui-settings` 内运行期 `edition` 分支 | 少一个包 | 违反「构建期差异优先」（评审 P2-7） | 放弃 |

### 3.5 env 迁移（M-4，已定 A）

| 方案 | 优点 | 缺点 | 结论 |
| --- | --- | --- | --- |
| A. 不自动导入（env 仅 personal 且无 settings 文件） | 不擅自改写用户配置；不出现「清空又被灌回」 | 已有 settings 文件+env 用户需按 release note 重建 | **采用（M-4 已定）** |
| B. 首次启动导入 env 并持久化 | 迁移无感 | 需要一次性标记/回滚语义；用户清空后不应再导入（复杂度） | 备选 |

## 4. 安全与隐私

- **本地命令执行**：stdio MCP = 用户可控的本地进程（与 `bash` 同一信任级别），argv 数组无 shell 拼接；`env` 含 secret-like 键在保存时拒绝（ADR-060 规则沿用）。
- **SSRF**：http(s) 只允许 https 或 loopback 明文（既有规则，两级校验不变）；`mcp.test` 同样走该校验，不能当 SSRF 跳板。
- **无缓存探测**：草稿配置不进入全局会话缓存；结束后 transport drop（stdio 子进程 kill+reap），避免残留进程/会话。
- **企业口径**：catalog-only（本地段拒绝、env 关闭、boot 清理）；不在客户端落任何上游 secret；OAuth token 仍只在 keychain。
- **错误信息**：不回显 env 值/令牌；`mcp.test` 错误码封闭。
- **审计**：本地能力无服务端审计；失败 fail-closed（保存失败不落盘、探测失败不改配置）。

## 5. 发布与回滚

- 无灰度/无 server 变更；随客户端版本发布。
- 回滚：回退 commit；设置 JSON additive（旧版忽略 `mcp`）；`editions/personal-only.toml` 回退即可。
- 客户端兼容：旧前端不调 `mcp.test` 正常工作；旧 catalog 不受两级校验影响（R2）；env 用户升级行为见 M-4 与 CHANGELOG。

## 6. 测试计划

| 层 | 用例 | 命令 |
| --- | --- | --- |
| T0 组合 | 新包/角色/edition exclude/personal-only 清单；SettingsPanes 快照 | `zk validate && zk compose --edition personal --profile default --channel official --check && zk capability --check` |
| T1 契约 | `McpServerConfig::validate` 规则一致；`McpSettings` 上限/重复/id/OAuth；schema 导出 | `cargo test -p zk-contract-connector -p zk-contract-config -p zk-contract-ipc && cargo test --workspace export_schema && bash scripts/schema-check.sh` |
| T1 client | 优先级（catalog>local>env，含空）；catalog 两级校验回归；`test_server` 无缓存；enterprise env 关闭 | `cargo test -p zk-connector-mcp` |
| T1 desktop | boot 装配/企业清理/`settings.set` 拒绝/`mcp.test`+status 状态机/超时 | `cargo test -p zk-desktop mcp` |
| T2/T3 e2e | personal `serve` + fake stdio MCP server：pane 添加→测试连接→保存→刷新保留；六态/axe | `bash scripts/m1-e2e.sh --grep "MCP"`（spec 的 `test.describe` 内 `test.skip(process.env["ZEOKEEL_E2E_SKIP_SERVE"] === "1", "mock stage")`：第一段 mock 自动 skip，第二段真实 serve 执行） |
| T4 矩阵 | enterprise 组合不含 `ui-settings-mcp`；`cargo check` | `zk compose --edition enterprise --profile default --channel official && cargo check --workspace` |
| 前端 | typecheck/lint/unit/build/i18n/新包单测 | `pnpm -r typecheck && pnpm -r lint && pnpm -r test:unit && pnpm -r build && pnpm --dir packages/ui/ui-settings-mcp test:unit && bash scripts/i18n-check.sh` |
| T5 交付 | 无（无镜像/安装包变化） | — |

## 7. 文档更新（任务 1 先行）

- [ ] `docs/15` §C：C1/C6 映射 **D 编号**（**M-6 已定：不新增 ADR**）——企业 catalog-only 属构建期 edition 差异；同时 `docs/24` §1 / `docs/04` §1.5 登记依赖（P2-8）
- [ ] `docs/24` §1：**`zk-contract-config → zk-contract-connector` 与 `zk-contract-ipc → zk-contract-connector`** 依赖方向 + 「settings/IPC DTO 内含 connector DTO」兼容责任与 public-api 影响（P2-8 / P1-13.1）
- [ ] `docs/04` §1.5：契约依赖/客户端能力措辞对齐（含两条跨契约依赖与 public-api diff 要求）
- [ ] `docs/07` §6.1：MCP 配置来源（personal settings / enterprise catalog-only / env 仅无 settings 文件）
- [ ] `docs/16` §2：新 UI 包 `ui-settings-mcp` 与 `ui.settings.pane` 贡献；personal-only 包清单
- [ ] `docs/26` §5：IPC 命令表补 `mcp.test`/`mcp.test.status` 两条命令（输入/输出/错误码，P2-N6）
- [ ] `docs/38` §4：MCP 能力注记（`mcp.test` 两段式、无缓存探测、超时上限）
- [ ] `CHANGELOG.md`：`ZEOKEEL_MCP_SERVERS` 生效条件变化（M-4）+ 企业 catalog-only
- [ ] `docs/README.md` 主题登记：否

## 8. 风险

| 风险 | 概率 | 影响 | 缓解 | 验证 |
| --- | --- | --- | --- | --- |
| 提升 validate 导致 catalog 回归 | 中 | 高 | 规则原样搬移 + catalog 兼容 golden | `cargo test -p zk-connector-mcp catalog_limits` |
| `test_server` 仍污染缓存/泄漏子进程 | 中 | 高 | 独立 transport + drop kill + `session_count` 断言 | `cargo test -p zk-connector-mcp test_server_no_cache` |
| 两段式状态表泄漏/堆积 | 低 | 中 | test_id 覆盖 + 完成态短期保留 + 单测 | `cargo test -p zk-desktop mcp_test` |
| enterprise env 关闭导致既有用户「MCP 消失」 | 中 | 中 | T4 已定；CHANGELOG + 诚实不可用提示 | `cargo test -p zk-connector-mcp precedence` |
| personal-only 清单遗漏 | 中 | 中 | 任务显式改数据列表 + `zk validate` 断言 | `zk validate` |
| settings.json 膨胀 | 低 | 低 | 上限（16/32/…）+ 单测 | `cargo test -p zk-contract-config mcp` |
| 契约耦合（config→connector）后续破坏性变更 | 中 | 中 | docs/24 登记兼容责任；破坏性变更走 v2/ADR | `bash scripts/schema-check.sh` |
| settings-ia / config spec 未合并就开 UI 任务 | 中 | 高 | tasks 依赖列 + §0 串行 | 评审/CI |
