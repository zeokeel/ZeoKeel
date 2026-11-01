# 2026-11-01-client-config-approval-mcp · 技术设计

> 规范细节链接 `docs/`，不在这里复制第二份规范。组合/manifest 以 `24` 为准，API 以 `04` 为准。
> 范围：审批策略消费链 + `connectors.*` 契约接线。MCP 配置见 `2026-11-01-client-mcp-config`；
> 审批卡/工具行/`always-allow`/决策 wire 见 `2026-11-01-client-approval-tool-rendering`；
> pane slot/revision 见 `2026-11-01-client-settings-ia`。跨 spec 顺序与单 owner 见 `tasks.md` §0。

## 1. 影响面

| 类型 | 变更 |
| --- | --- |
| 包 | 不新增包；修改 `zeokeel/agent-rig`（`packages/client/agent-rig`）、`zeokeel/ui-settings`（审批控件，依赖 settings-ia 的 pane）；desktop web 接线 |
| 契约 | `zk-contract-config`：`ApprovalPolicy` 枚举、`ToolSettings.approval_policy: Option<ApprovalPolicy>`、`normalize_approval_policy()`/`effective_approval_policy()`；`zk-contract-ipc`：`CONNECTORS_CONFIGURE/RELOAD/CLEAR` + payload/result DTO。**本 spec 不新增 cross-contract 依赖**（`zk-contract-config → zk-contract-connector` 由 MCP spec 引入并登记，P2-8） |
| 路由 | 无（本地 IPC；不含 server HTTP 路由） |
| 迁移 | 无 DB 迁移；设置 JSON additive（`schema` 保持 1）。启动（文件存在时）与 `settings.set` 时把旧 `auto_approve_read_only` 规范化为显式策略（幂等） |
| 生成物 | `shared/contracts/schema/config-*.json`（如 `AppSettings` 变更）、`shared/contracts/schema/ipc-*.json`、`shared/contracts/ts/contracts.ts`、`docs/generated/CONTRACT-INDEX.md`；由 `cargo test --workspace export_schema` + `python3 scripts/schema-ts.py` 生成（C10/C11），禁止手改 |
| edition/profile/channel | 不变（审批策略两版通用；`connectors.*` 只影响企业登录流程） |
| 文档 | `docs/38` §2（审批策略，任务 1 先行）、`docs/19`（设置/审批段）、`docs/26` §5（IPC 命令表）、`docs/07` §6.1 与 `KNOWN-LIMITATIONS.md` §3.5（web 侧接线口径修正）、`CHANGELOG.md`（deprecated 说明） |

## 1.5 复用检查（C17，必填）

| 要实现的点 | 现有能力（capability id / 路径） | 结论 | 归属 |
| --- | --- | --- | --- |
| 审批阻塞/超时/fail-closed | `kit.approval`（`crates/zk-kit-approval/src/lib.rs`）；`zk-contract-tool::ApprovalGate`（`packages/contracts/tool/src/lib.rs:55-75`） | 复用（不改 gate 语义） | kit / 契约 |
| 审批触发点与 `ToolSpec` 可见性 | `agent-rig::dynamic_tools`（`packages/client/agent-rig/src/lib.rs:246-350`；`spec.requires_approval` `:272`、`if requires_approval` `:303`、`gate.decide` `:322`） | 扩展（加策略求值，不改事件顺序） | `agent-rig` |
| 每轮热配置读取 | `AgentRuntimeConfig`（`packages/client/agent-rig/src/config.rs:222-237`）、`effective_config()`（`lib.rs:163-169`）、`default_agent_config()`（`config.rs:455-475`） | 复用（策略随 `tools` 段每轮读取） | `agent-rig` |
| settings 存储/校验/掩码/失败不落盘 | `SettingsStore`（`packages/contracts/config/src/lib.rs:653-658`）、`store-settings-json`、`settings.set` 管线（`apps/desktop/src/ipc.rs:531-592`）、`masked_settings`（`:1469-1481`） | 复用 | 契约 + desktop |
| 连接器 catalog 应用/OAuth token 复原 | `apply_connector_catalog`/`restore_connector_oauth_tokens`/`sync_connector_catalog`（`apps/desktop/src/ipc.rs:338-410`） | 复用（命令改用契约常量，行为不变） | desktop |
| 连接器 catalog 拉取（Web） | `apps/desktop/web/src/store.ts:59-85`（`fetchConnectors`） | 扩展（返回原始 catalog，供 `connectors.configure`） | desktop web |
| 设置 pane 扩展点与共享表单类型 | settings-ia 的 `ui.settings.pane` 多贡献 slot、`SettingsPaneProps`（`{t,draft,update,errors,saving}`，无 edition 分支） | 复用（本 spec 只加控件） | ui-settings |
| 生成物 | `cargo test --workspace export_schema`、`scripts/schema-ts.py`、`zk compose` | 复用 | 生成器 |

- 搜索证据（2026-11-01，`./target/debug/zk`）：`zk find approval` → `kit.approval`（1）；`zk find connector` → `contract.connector`（1）；
  `zk find settings` → `store.settings.json`（1）；`zk find config` → `contract.config`（1）。`rg -n "auto_approve_read_only|requires_approval|connectors\." packages apps` 用于确认无第二份实现。
- 若「新建」：本 spec **不新建包/能力**；`ApprovalPolicy::should_ask` 是契约内单一求值实现（登记在既有 `@capability contract.config`），
  `connectors.*` DTO 挂在既有 `@capability contract.ipc`。
- 为什么不能复用/扩展：审批策略需要 `ToolSpec.mutating` + 逐轮热读，只有 agent-rig 的 DynamicTool 回调同时拥有 `spec` 与 `ApprovalGate`
  （`lib.rs:260-350`）；桌面 broker 只收到 name/args，放那里需要第二份「工具是否只读」知识（C17）。`connectors.*` 宿主已实现，缺契约与 Web 接线。

## 2. 方案

### 2.1 审批策略（A）

**契约**（`packages/contracts/config/src/lib.rs`）：

```rust
/// 本地工具审批策略（封闭三值；客户端本地，C6：服务端不下发）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalPolicy { #[default] Ask, AutoReadOnly, Strict }

impl ApprovalPolicy {
    /// 单一求值实现：是否需要询问用户。
    /// 弱不变式：requires_approval == true 时任何策略都返回 true。
    #[must_use]
    pub fn should_ask(self, requires_approval: bool, mutating: bool) -> bool {
        match self {
            Self::Ask => requires_approval,
            Self::AutoReadOnly => requires_approval || mutating, // C-1 已定 A（只读自动批准，mutating 全询问）
            Self::Strict => true,
        }
    }
    /// 未知值降级（P1-7）：deserialize 用；未知 → Ask。
    #[must_use]
    pub fn parse(value: &str) -> Self {
        match value { "auto_read_only" => Self::AutoReadOnly, "strict" => Self::Strict, _ => Self::Ask }
    }
}

// 未知值不得让 settings.load() 失败（P1-7）：自定义 Deserialize（字符串先行，未知→Ask）
impl<'de> serde::Deserialize<'de> for ApprovalPolicy { /* String::deserialize → Self::parse */ }

pub struct ToolSettings {
    pub tool_timeout_ms: u64,
    pub approval_timeout_ms: u64,
    /// None = ask（默认，不发布）；Some(...) 才视为显式非默认。
    #[serde(default)]
    pub approval_policy: Option<ApprovalPolicy>,
}
impl ToolSettings {
    #[must_use] pub fn effective_approval_policy(&self) -> ApprovalPolicy {
        self.approval_policy.unwrap_or_default()
    }
}
```

**真值表（3 策略 × 4 组合，C-1 已定 A）**：

| 策略 \ (requires_approval, mutating) | (true,true) | (true,false) | (false,true) | (false,false) |
| --- | --- | --- | --- | --- |
| `ask`（默认） | 询问 | 询问 | **不询问**（4 个工具作者声明） | 不询问 |
| `auto_read_only` | 询问 | 询问 | **询问**（C-1 A：mutating 全询问） | 不询问 |
| `strict` | 询问 | 询问 | 询问 | 询问 |

- **弱不变式**（T1，**策略求值范围内**）：`requires_approval == true` ⇒ 询问（三种策略都成立；负向测试断言）。
  唯一例外是 S4「始终允许本会话」：用户先前的显式授权使后续调用不再询问（approval spec `design.md §2.0` S4），
  不属 `should_ask` 的自动批准，避免逐字反驳。
- **4 个 `mutating && !requires_approval` 工具**与理由见 `spec.md` R1 表（`schedule`/`job_kill`/`goal`/`mcp_call`）。
  默认 `ask` 下不询问是既有行为；`auto_read_only`/`strict` 会询问它们（用户显式选择才生效）。

**事件序列**：统一引用 approval spec **`specs/changes/2026-11-01-client-approval-tool-rendering/design.md §2.0`《审批事件序列表》行 S1/S2/S3a–S3c/S4/S5**（唯一真源，不复制）——
策略求值 = 不需要询问 → **S5**（无任何 `approval_*`）；需要询问 → **S1/S2/S3a/S3b/S3c**；`always-allow` 命中 → **S4**（成对 `auto:true`，不阻塞）。
S3 组按 approval 权威表逐条区分：S3a 显式取消（cancel + turn_cancelled）/S3b 超时（reject+timed_out）/S3c 断线或窗口关闭（reject，不当 cancel）。
本 spec 只负责「S5 判定」（`should_ask == false`），不定义 S1/S2/S3a–S3c/S4 的字段/决策语义。

**运行时读取**（`packages/client/agent-rig`）：

1. `AgentRuntimeConfig` 增字段 `approval_policy: ApprovalPolicy`；`from_contract` 取 `settings.effective_approval_policy()`；
   `default_agent_config()`（`config.rs:455-475`）在 `published.tools` 存在时用 `published.tools.effective_approval_policy()`，
   否则用 env/默认（可加 `ZEOKEEL_APPROVAL_POLICY=ask|auto_read_only|strict` 供测试/e2e 使用，未知→Ask，与契约 parse 一致）。
2. `ToolRunContext`（`lib.rs:69-76`）增 `approval_policy: ApprovalPolicy`；`run_stream_in_shared_with_history` 构造处
   （`lib.rs:464-478`）从 `config.approval_policy` 填入。
3. `dynamic_tools`（`lib.rs:246-350`）把 `if requires_approval`（`:303`）改为
   `if policy.should_ask(requires_approval, mutating)`；其中 `mutating` 取自 `spec.mutating`（当前 `:260` 已取 `spec`）。
   **事件顺序不变**：需要询问时仍 `ApprovalRequested → gate.decide → ApprovalResolved`；不需要询问时不发任何审批事件。

**规范化**（`AppSettings::normalize_approval_policy(&mut self)`，单点、幂等）：

```text
effective = tools.approval_policy.unwrap_or(if auto_approve_read_only { Ask } else { Strict })   // C-2 已定（false→strict / true→ask）
match effective {
  Ask          => { tools.approval_policy = None;                auto_approve_read_only = true  }
  AutoReadOnly => { tools.approval_policy = Some(AutoReadOnly);  auto_approve_read_only = true  }
  Strict       => { tools.approval_policy = Some(Strict);        auto_approve_read_only = false }
}
```

调用点：boot `apply_persisted_settings`（`apps/desktop/src/ipc.rs:302-330`，**仅设置文件已存在时保存回写**，
避免 fresh install 被写成「已保存」而抑制 env fallback，`settings_file_exists()` `:164-168`）与 `settings.set`
（`ipc.rs:531-592`，反序列化后、`validate_settings` 前）。`ask` 存 `None` ⇒ `ToolSettings` 仍为默认 ⇒
`runtime_snapshot_for` 不发布 tools 段 ⇒ `ZEOKEEL_TOOL_TIMEOUT_MS`/`ZEOKEEL_APPROVAL_TIMEOUT_MS` fallback 语义保持（`docs/38` §3）。

### 2.2 `connectors.*` 契约与接线（B）

**契约**（`packages/contracts/ipc/src/lib.rs`）：

```rust
pub const CONNECTORS_CONFIGURE: &str = "connectors.configure";
pub const CONNECTORS_RELOAD: &str = "connectors.reload";
pub const CONNECTORS_CLEAR: &str = "connectors.clear";

pub struct ConnectorsConfigurePayload { pub catalog: serde_json::Value }
pub struct ConnectorsConfigureResult { pub configured: u64 }
// reload/clear 用既有 EmptyPayload；结果复用 json!({ "reloaded": true }) / { "cleared": true } 形状（沿用宿主现状）
```

**宿主 dispatch**（`apps/desktop/src/ipc.rs:630-650`）：把字符串字面量改为 `commands::CONNECTORS_*` 常量；
行为不变：`configure` 接受对象或 JSON 字符串（现分支 `Value::String(text) => text.clone(), other => other.to_string()`），
调用 `apply_connector_catalog`（`:338-340`，内部 `update_from_catalog` + `restore_connector_oauth_tokens`）；
错误前缀收敛为 spec §2 的封闭码（`connectors/catalog-invalid`、`connectors/apply-failed`、`connectors/reload-failed`、`connectors/clear-failed`），
保留原错误文本作为细节（`format!("connectors/catalog-invalid: {error}")`），不破坏既有日志。

**Web 接线**（`apps/desktop/web/src`）：

| 文件 | 改动 |
| --- | --- |
| `store.ts` | `fetchConnectors` 返回 `{ items: ConnectorItem[], catalog: Record<string, unknown> }`（catalog = 服务端原始 JSON；不解析 `config`，解析归宿主 `servers_from_catalog`）；`toggleConnector` 不变 |
| `App.tsx` | 企业 session effect（`:118-190`）在目录拉取成功后 `connectors.configure({catalog})`；`onToggleConnector`（`:428-434`）重新拉取后同样 configure；`logout`（`:104-115`）先 `connectors.clear({})`；新增「重新加载连接器」回调 → `connectors.reload({})` |
| `i18n.tsx` | 新增错误/成功文案键（zh-CN/en-US），无硬编码 |
| `apps/desktop/web/tests/*` | fake transport 断言调用序列与失败可见（P2-5/verify） |

说明：初次登录仍由 host `auth.configure` 内 `sync_connector_catalog`（`:651-687`）同步；Web 的 configure 是「目录变化后」的显式同步，
两者幂等、不冲突（host 侧 `update_from_catalog` 会重载会话并 retain OAuth token）。

### 2.3 逐文件改动清单（14 个实现/契约/测试文件）

| # | 文件 | 改动 |
| --- | --- | --- |
| 1 | `packages/contracts/config/src/lib.rs` | `ApprovalPolicy`（含自定义 `Deserialize`）、`ToolSettings.approval_policy`、`effective_approval_policy()`、`AppSettings::normalize_approval_policy()`、`validate()` 复用 |
| 2 | `packages/contracts/config/tests/conformance.rs` | 真值表/弱不变式/迁移 4 形态/未知值/幂等（不新增 cross-contract 依赖） |
| 3 | `packages/contracts/config/tests/schema.rs` | `ApprovalPolicy`/`AppSettings` schema 导出更新 |
| 4 | `packages/contracts/ipc/src/lib.rs` | 3 命令常量 + `ConnectorsConfigurePayload/Result` |
| 5 | `packages/contracts/ipc/tests/schema.rs` | 新 DTO schema 导出 |
| 6 | `packages/client/agent-rig/src/lib.rs` | `ToolRunContext.approval_policy`、`dynamic_tools` 求值替换、事件顺序保持 |
| 7 | `packages/client/agent-rig/src/config.rs`（+ 其 tests） | `AgentRuntimeConfig.approval_policy`、`from_contract` 映射、`default_agent_config` 读取（含 `ZEOKEEL_APPROVAL_POLICY` 测试用 fallback） |
| 8 | `apps/desktop/src/ipc.rs`（含内联 tests） | 规范化调用点、connectors 命令常量/错误码、dispatch 单测 |
| 9 | `apps/desktop/web/src/store.ts` | `fetchConnectors` 返回原始 catalog |
| 10 | `apps/desktop/web/src/App.tsx` | 4 处接线（登录/启停/重新加载/登出） |
| 11 | `apps/desktop/web/src/i18n.tsx` | 新文案键（zh/en） |
| 12 | `apps/desktop/web/tests/*.test.ts(x)` | 调用序列/失败可见单测 |
| 13 | `packages/ui/ui-settings/web/src/panes/tools.tsx` + `packages/ui/ui-kit/web/src/types.ts` | 审批策略三选一控件（替换旧「自动批准只读」）；`SettingsForm` 加 `approvalPolicy`（依赖 settings-ia 产物） |
| 14 | `e2e/agent-rig.spec.ts` + `e2e/m3-client-content.spec.ts` | strict/ask 审批策略用例（真实 personal rig）；连接器 configure/reload/clear 链路 |

> 生成物（schema/TS/registry/CONTRACT-INDEX）由任务 9 的生成器写，不计入上表。文档 5 + CHANGELOG 另计（任务 1/10）。

### 2.4 关键流程

```text
(A) 审批策略
settings.json → AppSettings::normalize_approval_policy (启动/保存，幂等)
  → RuntimeSettings.tools（仅非默认策略发布）→ AgentRuntimeConfig.approval_policy（每轮重读）
  → dynamic_tools: should_ask(ra, mutating)
      true  → ApprovalRequested → gate.decide（fail-closed）→ ApprovalResolved → 执行/拒绝
      false → 直接执行（不发审批事件）

(B) 连接器
登录/刷新/启停后 → Web fetchConnectors（items + catalog）
  → connectors.configure {catalog} → dispatch → update_from_catalog + reload_connectors + restore OAuth
  → 失败：保留旧配置并返回 connectors/* 错误 → Web 可见错误
重新加载按钮 → connectors.reload {}；登出 → connectors.clear {}
```

## 3. 备选与取舍

| 决策点 | 方案 | 结论 |
| --- | --- | --- |
| `auto_read_only` 语义（C-1） | A. `ra` 或 `mutating`（只读自动、mutating 全问）；B. 等同 `ask`；C. 删除该档 | **已定 A**（Lead 2026-11-01）；B/C 不再考虑 |
| 策略求值位置 | A. agent-rig `dynamic_tools`（有 `ToolSpec` + gate）；B. 桌面 broker（缺 `mutating` 元数据，需第二份知识）；C. 契约 gate 装饰器（`decide` 签名无 spec） | 采用 A |
| 未知枚举降级（P1-7） | A. 自定义 `Deserialize`（字符串先行→`parse`，未知→`Ask`）；B. 普通 derive（加载失败，违反 R1）；C. 改 R1 为「未知值报错」 | 采用 A（C 会让手改/未来版本文件阻止启动） |
| `connectors.configure` payload | A. 沿用现状 `{catalog}`（对象或字符串）；B. Web 侧解析 `config` 后传结构化 servers（引入 Web 解析重复）；C. 新命令 `connectors.install` | **C-3 已定 A**（Lead 2026-11-01） |
| 契约依赖 | 本 spec 不新增 `zk-contract-config → zk-contract-connector`（MCP spec 引入并登记 P2-8） | 采用（保持本 spec 契约面最小） |

## 4. 安全与隐私

- **弱不变式**（策略求值范围内）保证任何策略都不会自动批准 `requires_approval == true` 的工具；唯一例外是 S4
  「始终允许本会话」（用户显式授权后的 always-allow，归 approval spec）；`ask` 与 `auto_read_only`/`strict` 的差异只体现在
  「是否额外询问未声明审批的工具」，不削弱工具作者声明。
- 4 个 `mutating && !requires_approval` 工具在默认 `ask` 下不询问是**既有行为**；本 spec 不改变它，只在文档/spec 显式披露，
  并在 `auto_read_only`/`strict` 下提供更严格的选项。
- `connectors.*`：只接受服务端已 sanitize 的 catalog（ADR-060，无 static secret）；非法 catalog 不覆盖旧配置（fail-safe）；
  命令幂等，重复调用不重复安装 token/会话。
- 审批仍 fail-closed（超时/无通道/退出 = 拒绝，`docs/38` §2）；本 spec 不改 gate/超时语义。
- 无新增网络调用（Web 只调本地 IPC；catalog 拉取是既有行为）；无新增敏感持久化。

## 5. 发布与回滚

- 灰度/强制更新：无（随客户端版本发布；无 server/DB 迁移）。
- 回滚：回退 commit；设置 JSON additive（回滚后旧版忽略 `approval_policy`，用 `auto_approve_read_only` 镜像值，语义不变）。
- 客户端兼容：旧前端不调用 3 条新命令可正常工作；旧设置文件自动规范化；`connectors.*` 为 additive 命令。
- 数据影响：仅设置文件新增可选字段；无数据库/服务端改动。

## 6. 测试计划

| 层 | 用例 | 命令 |
| --- | --- | --- |
| T0 组合 | edition/闭包/生成物不变 | `zk validate && zk compose --edition personal --profile default --channel official --check && zk capability --check` |
| T1 契约 | 真值表（3×4）+ 弱不变式负向；4 工具 `ask` 不询问；迁移 4 形态 + 幂等；未知值降级；`ApprovalPolicy` schema | `cargo test -p zk-contract-config && cargo test --workspace export_schema && bash scripts/schema-check.sh` |
| T1 IPC | 3 命令 DTO schema；dispatch 成功/非法 catalog 保留旧配置/幂等 | `cargo test -p zk-contract-ipc && cargo test -p zk-desktop connectors_dispatch` |
| T1 agent-rig | `should_ask` 真值表 + fake tool/记录 sink 事件链（`ask`/`auto_read_only`/`strict`）+ 热生效 | `cargo test -p zk-agent-rig approval_policy` |
| T2/T3 e2e | personal 真实 rig：`strict` 下 `fs.read` 出审批卡、`ask` 不出、切换后下一轮生效 | `bash scripts/agent-e2e.sh` |
| T3 企业 | 连接器 toggle→configure、重新加载→reload、登出→clear（真实 server + PG） | `bash scripts/login-e2e.sh`（无 PG 记 UNVERIFIED） |
| T4 矩阵 | enterprise 组合构建（不含 personal-only 包；本 spec 不改组合，回归即可） | `zk compose --edition enterprise --profile default --channel official` + `cargo check --workspace` |
| T5 交付 | 无（无镜像/安装包变化） | — |
| 前端 | typecheck/lint/unit/build/i18n | `pnpm -r typecheck && pnpm -r lint && pnpm -r test:unit && pnpm -r build && bash scripts/i18n-check.sh` |

## 7. 文档更新

- **任务 1 先行（C12/P1-2）**：把审批策略（封闭枚举 + 弱不变式 + 4 个免审批工具清单）写入 `docs/38` §2；
  把 `connectors.*` 契约与触发时机写入 `docs/26` §5（IPC 命令表）+ `docs/07` §6.1；`docs/19` 设置段补审批策略；
  `docs/15` §C 登记 D 编号 + `docs/07` §6.1 + `docs/38` §2；**C-6 已定：不新增 ADR**。
- [ ] `docs/07` §6.1：删「web 侧接线」未完成口径 → 已接线 + 触发时机
- [ ] `KNOWN-LIMITATIONS.md` §3.5：同上（保留远端生命周期 UNVERIFIED）
- [ ] `docs/26` §5：3 条新命令（输入/输出/错误码）
- [ ] `docs/38` §2：策略枚举/求值点/热生效/弱不变式；§4 注记 4 个免审批 mutating 工具
- [ ] `docs/19`：设置段补「审批策略」；审批卡字段仍归 approval spec
- [ ] `CHANGELOG.md`：`auto_approve_read_only` deprecated + 迁移映射
- [ ] `docs/README.md` 主题登记：否（不新增主题）

## 8. 风险

| 风险 | 概率 | 影响 | 缓解 | 验证 |
| --- | --- | --- | --- | --- |
| 规范化与「默认-only 不发布」交互写错，env timeout fallback 失效 | 中 | 中 | `ask` 存 `None`；4 形态单测 | `cargo test -p zk-desktop settings_normalize` |
| `ToolSettings` 新字段影响既有 conformance golden | 高 | 低 | 更新 golden（新增字段） | `cargo test -p zk-contract-config` |
| 未知值降级实现错误导致整份设置加载失败 | 中 | 高 | 自定义 `Deserialize` + 未知值/类型错测试 | `cargo test -p zk-contract-config approval_unknown` |
| 与 approval spec 在 `agent-rig`/`ipc.rs` 同文件竞写 | 中 | 高 | 串行顺序 + 单 owner（tasks §0） | code review |
| Web 接线重复 configure（登录内 host 已同步） | 中 | 低 | 命令幂等 + 单测调用序列 | `pnpm --dir apps/desktop/web test:unit` |
| 企业 e2e 依赖本机 PG | 中 | 中 | 无 PG 标 UNVERIFIED、用 dispatch/web 单测兜底 | `bash scripts/login-e2e.sh` |
| strict 双重交互 UX | 中 | 低 | C-5 记录已知行为；e2e 断言不崩溃 | `bash scripts/agent-e2e.sh` |
