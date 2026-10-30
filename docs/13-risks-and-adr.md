# 13 · 架构决策记录（ADR）与风险登记

> ADR 一旦写下就是评审基线；推翻需要新 ADR 说明"为什么原来那条不成立了"。
> 状态：`accepted` 已定 · `proposed` 待评审 · `open` 未决。

## ADR 主题索引（快速定位）

| 组 | ADR |
| --- | --- |
| 架构/组合 | 001–006、010–016、020–026、045–047 |
| 契约/内核 | 007–009、017–019 |
| 认证/安全 | 027–030、033、037、040、049–050、054 |
| 钱包/计费 | 023、034、036、048、055 |
| 更新/交付 | 041、052 |
| 客户端 | 006、014、050、053 |
| 运维/备份 | 022、051 |
| 业务口径 | 055–056 |

> 索引只做导航；决策正文仍以对应 ADR 为准，实现细节链接 `19–24`。


## A. 架构决策记录

### ADR-001 · 采用构建期组合，不做运行时加载 `accepted`

**背景**：DSH 用 Node 的运行时装配，换包不重建；Rust 的 `zeokeel-cordis` 明确不做动态加载、无 ABI、
静态链接、`#![forbid(unsafe_code)]`。
**决策**：一切组合在构建期完成：profile → `zk compose` → 生成粘合代码 → 原生静态链接。
**后果**：换实现必须重建；换来单二进制、编译期契约校验、可穷举的 CI 矩阵。
**推翻条件**：产品必须支持"用户装插件不重启/不重装"——届时走 ADR-013（WASM 叶子包），而不是改回 dylib。

### ADR-002 · 契约 crate 是唯一稳定缝，一契约一 major 一 crate `accepted`

**决策**：包之间只允许通过 `zk-contract-*` 的 trait/DTO 耦合；同一 profile 内每个契约 crate 恰好一个版本；
major 变更发新 crate（`zk-contract-tool2`），不与旧版共存。
**理由**：Rust trait 身份由 crate 实例决定，两份副本会产生"看起来实现、实际没实现"的延迟错误。
**后果**：契约变更成本高，因此契约必须窄（见 R2）；符合性套件是契约的一部分。
**证据（设计论证，非实测）**：契约身份由 crate 实例决定，自带私有契约副本会在编译期失败（如 `E0599`）；
POC 尚未入库该负向 fixture（见 `experiments/composition-poc/README.md` §未验证）。

### ADR-003 · `zk.toml` 是包元数据唯一真源 `accepted`

**决策**：包身份/版本/角色/契约/依赖/facet 只写在 `zk.toml`；`Cargo.toml`、`package.json` 中对应字段
由 `zk validate` 交叉校验一致，而不是反过来生成。
**理由**：同一事实两处手写必然漂移（先例的返工点之一）；但 Cargo/npm 的清单是工具链硬要求，不能省。
**后果**：新增/改包要同步两处，校验器兜底；`zk init` 生成骨架减少手工。

### ADR-004 · `requires` 只写角色，不写包名 `accepted`

**决策**：包依赖声明为"我需要角色 X 已被提供"，而非"我需要包 Y"。
**理由**：写包名会让替换失效（换掉 Y 就编译不过），直接违背"一切皆包"。
**后果**：需要组合期做角色解析与拓扑排序；缺角色在组合期报错而非编译期。

### ADR-005 · 前端是 pnpm 包 + 构建期生成静态注册表 `accepted`

**决策**：每个 UI facet 是 pnpm workspace 成员包，导出 `components` + `contribute`；`zk compose` 生成
`apps/desktop/web/src/generated/registry.gen.ts`，通过 vite 虚拟模块暴露给 shell；shell 只认 `componentId` / `slotId`。
**理由**：静态 import 保证 tree-shaking、React 单实例、单 bundle；运行时动态 import 会破坏 CSP 与
单二进制模型，也带不回"热换 UI"的真正价值（仍需重建 bundle）。
**后果**：换 UI 包 = 改 profile + 重新 compose + 重建；`componentId` 覆盖率由 composer 在构建期校验。

### ADR-006 · 桌面壳 Tauri 2，同源 dist 提供 HTTP 副载体 `accepted`

**决策**：主载体 Tauri 2（系统 WebView）；同一份 `dist` 可由 `serve` 包（axum）以本地 HTTP 提供，
用于浏览器调试、远程访问与 CI 无头 e2e；前端通过 `Transport` 抽象（tauri-ipc / http-ws / mock）切换，
由 profile 选择实现。
**理由**：Tauri 的 WebDriver 在 macOS 不可用（需付费 fork），e2e 必须有不依赖 WebDriver 的载体；
DSH 的 `host-webserver` + `host-frontend-static` 正是该形态。
**后果**：IPC 契约必须是传输无关的（生成命令表 + JSON 编解码），不能直接用 Tauri 特有类型。

### ADR-007 · rig 隔离在适配层包，不进契约 `accepted`

**决策**：`zk-contract-llm` / `zk-contract-agent` 只定义自有 `Message`/`ContentBlock`/`ToolSpec`/
`StreamEvent`/`LlmError`/`AgentRuntime`；`zk-llm-rig` 独占 `rig-core`，`zk-agent-rig` 独占
`rig-agent`；工具包/provider 包只依赖契约。
**理由**（已实测，`research/rig.md`）：rig 0.45.0 三件套真实可用，但处于每 1–3 周一次 breaking 的
演进期（0.46.0-rc.1 已发布），且类型面巨大（`Agent`/`Tool`/`Handlers`/`Message`/各 Error 均在重构）；
把 rig 写进契约等于把 40+ 个包钉死在某个小版本上。
**后果**：需要一层薄适配（类型转换、流式碎片拼装、三层错误映射）；这层成本刻意付。升级 rig 只改
两个适配包 + 跑契约对拍。工具包不依赖 rig，升级不需要重编。
**附加约束**：workspace 精确钉 `rig-core/rig-agent/rig-memory = "=0.45.0"`；`rust-toolchain.toml`
≥ 1.97.1（rig MSRV，edition 2024）；会话存档格式由契约层决定，不用 rig 的 `AgentRun` 格式
（`RUN_FORMAT` 不向前兼容）。

### ADR-008 · 生成物入库，`--check` 防漂移 `accepted`

**决策**：`crates/zk-composed`、`apps/desktop/web/src/generated`、`shared/contracts`、文档索引、ts-rs bindings 的 **default 产物**入库；
CI `zk compose --check` + `git diff --exit-code`。
**理由**：新人 clone 即可构建；生成物变更在 PR 里可审；避免"生成器坏了 CI 才发现"。
**后果**：生成器改动会产生大 diff，需用 `--check` 与 golden 测试约束格式。

### ADR-009 · 平台引导不是包（非包代码清单）`accepted`

**决策**：以下不是包：`crates/zk-bootstrap`、`apps/desktop/src/main.rs`、web 的 `main.tsx`、
`crates/zk-composed`（生成物）、`tools/zk`（构建工具）、`zk-testkit`（测试基建）。它们被约束为
"只做创建/配置/注册/生成"，不含业务。
**理由**：图灵完备的组合系统需要一个不参与组合的入口；把它包化会产生无限回归。
**后果**：bootstrap 的代码审查按"平台代码"标准，任何业务逻辑进入即 review 拒绝。

### ADR-010 · 单一 IPC 分发入口 + schema 单源 `accepted`

**决策**：包不写 `#[tauri::command]`；每个包在 `zk.toml` 声明命令 + 在 `register` 里注册处理函数；
生成器产出命令表，host 只暴露一个 `ipc_call(name, payload)` + 流式事件通道。
**理由**：Tauri 的 `generate_handler!` 是静态宏列表，无法按 profile 裁剪；包化后命令表面必须可生成。
**后果**：Tauri 边界只有一处类型断言。**schema 必须单源**：命令参数/返回类型定义在契约 crate，
用 `schemars` 生成 JSON Schema 与 TS 类型，禁止"手写命令表 + 手写 handler 类型"两套（评审盲点 7）。
平台包可保留少量原生命令（窗口/主题），列入白名单并由 `zk lint` 校验。

### ADR-011 · 包的 Rust facet 与 UI facet 原子替换 `accepted`

**决策**：一个包的 `[facets]` 同时声明 rust/ui；组合时一起选中、一起替换。UI 贡献在 `zk.toml` 里
**静态声明**（`[[ui.contributions]]`），运行期只按"已激活包集合"过滤，不动态产生贡献（ADR-017）。
**理由**：UI 假定实现的数据形状，分开替换会产生"新实现 + 旧面板"的隐性不兼容。
**后果**：纯 UI 包也要带极小 Rust 声明 crate，用于 `UiContribution` 编译期校验。

### ADR-012 · 信任边界是构建输入，不做运行时插件市场 `accepted`

**决策**：第三方包通过 vendor/git/crates.io 引入并经源码评审、契约符合性、换包矩阵；不提供运行时安装器、
不承诺插件沙箱；`[permissions].declares` 只驱动统一审批策略，不是安全边界。
**理由**：同进程静态链接下，插件隔离是修辞；真正的安全投入在供应链与可复现构建。
**后果**：文档与 UI 措辞必须一致（诊断页可显示包来源，但不暗示"沙箱"）。

### ADR-013 · WASM 运行时包是 M5 扩展点，不是当前范围 `open`

**思路**：叶子能力（工具、解析器）编译为 wasm，由 `host-wasm` 包用 wasmtime 加载，带显式 ABI + 权限 +
资源配额，从而获得"不重建即可换"的能力，同时不触碰内核静态链接前提。
**待决**：ABI 稳定性、性能、调试体验、包分发。现在只保留目录与角色命名空间（`tool.wasm.*`），不实现。

### ADR-014 · 用户数据格式只增不改 `accepted`

**决策**：本产品自有数据目录（`$ZEOKEEL_HOME` 下 `config.json` / `workspaces.json` / conversations），
新增字段一律 `#[serde(default)]`，前端类型同步可选；破坏性变更需迁移器 + 备份。
**理由**：桌面应用升级不可回滚用户数据；这是长期兼容性要求，不承接任何先例数据格式（D1）。
**后果**：会话/配置包（`store-*`）必须提供 schema 版本字段与迁移入口。

### ADR-015 · 锁文件分两层：`zk.lock` 组合锁 + 每 profile 解析 `Cargo.lock` `accepted`

**背景**（POC 实测）：生成物的依赖边随 profile 变化，而 Cargo.lock 记录依赖边，所以一份锁文件不可能
对所有 profile `--locked`（切换后 `exit 101`）；`--offline` 可构建但会重写锁文件。
**决策**：`zk.lock` 是跨 profile 的组合锁（角色→包/版本/源/哈希，入库）；`Cargo.lock`/`pnpm-lock.yaml`
是 **default 的解析结果**（入库，default job `--locked`）；其他 profile 在 CI 内 `--offline` 重解析 +
`zk verify-lock` 断言与 `zk.lock` 一致，产物不提交。pnpm 依赖使用"候选包并集"保持锁文件稳定。
**后果**：矩阵 job 的锁文件是临时产物；可复现性由 `zk.lock` 保证而非 Cargo.lock。
**被否方案**：每 profile 一个临时 workspace——`workspace = true` 继承失效、冷构建成本 × profile 数。

### ADR-016 · 契约发布 + `[patch]` 强制单实例，第三方包才能接入 `accepted`

**背景**（评审）：外部包若依赖不同来源的契约副本，trait 身份不同 → 编译失败；POC 的"外部包"其实在
仓库内。没有稳定契约身份，"第三方可接入"不成立。
**决策**：`zk-contract-*` 按 semver 发布（crates.io 或内网 registry）；根 `Cargo.toml` 由 `zk compose`
维护 `[patch.crates-io]` 把每个契约重定向到工作区内的唯一实例；`zk validate` 用完整 `cargo metadata`
断言每个契约只有一个 package id，**fail closed**。
**后果**：M0 前需 S0-P1 spike 验证；若发布不可行，退化为"外部包必须使用与你完全相同的 git rev"，
并把"第三方接入"在文档里降级为 vendor-only，不假装支持。
**被否方案**：只靠 manifest 字符串声明契约——无法阻止两个 crate 实例（评审实测三种骗法）。

### ADR-017 · UI 贡献静态声明，运行期只过滤 `accepted`

**背景**（评审）：贡献若在 `register()` 运行时产生，构建期就无法校验 component id 是否存在，
"构建失败"的承诺与设计自相矛盾，M1 第一个 settings 槽位就撞上。
**决策**：贡献写在包 `zk.toml` 的 `[[ui.contributions]]`，生成 `ui-contributions.json` + Rust const；
composer 构建期校验 component ⊆ providers、slot 存在、props schema 兼容；运行期按"已激活包集合"过滤。
**后果**：贡献不能依赖运行期计算的动态参数；需要动态数据的组件通过 props 的 `data` 字段 + IPC 获取。
**代价**：写贡献要在 `zk.toml` 与 `contribute.ts` 各写一次（信息不同：slot/order vs 组件实现），
由 `zk validate` 做一致性校验。

### ADR-018 · 平台壳是契约角色，`apps/desktop` 清单由生成器产出 `accepted`

**背景**（评审）：初版只让 profile 选 `platform-tauri`，但 `apps/desktop/Cargo.toml` 手写依赖具体包，
换壳要改应用代码——"任意单例可替换"在最重要的角色上失效；且 serve 载体没有角色可挂。
**决策**：新增 `zk-contract-platform`（`ShellHost`/`HttpHost`）；角色 `platform.shell`、`platform.http`；
`apps/desktop/Cargo.toml` 由 `zk compose` 生成（只依赖被选中的 platform 包），`main.rs` 固定为
`zk_bootstrap::run()`。
**后果**：换壳只改 profile；代价是 `apps/desktop` 的清单是生成物，tauri 配置也要分段生成。

### ADR-019 · POC 的验证范围必须如实标注 `accepted`

**背景**（评审）：POC 被当成"机制已全部验证"，实际只覆盖 trait 身份 + 依赖替换；不含 cordis、UI、
角色基数、requires、锁、CI。过度背书会误导排期。
**决策**：`experiments/composition-poc/README.md` 明确列出"证明了什么/没证明什么"；规划中所有引用
POC 的结论都要标注边界；M0 的退出条件不用"POC 已证明"作为依据。
**后果**：文档信噪比提高，评审可复现地反驳。

### ADR-020 · 双版本 = 同库两个 edition `accepted`

**决策**：企业版与个人版同一代码库；`editions/enterprise.toml` / `personal.toml` 选择包集合与产物形态，
构建期裁剪。企业版不含本地 provider 设置/密钥包；个人版不含登录/网关/服务端包。
**理由**：避免双仓同步；功能差异是"包组合差异"，可由 CI 矩阵证明。
**后果**：CI 必须同时构建两版；任何新包要声明属于哪个 edition（`[editions]` 字段）。

### ADR-021 · 服务端也是 cordis 包 + 构建期组合 `accepted`

**决策**：服务端与客户端共用 `Package` 契约与 `zk` 组合器；服务端包额外实现 `ServerPackage`
（路由/迁移/作业/配置 schema）。服务端产物 `apps/server` 的清单由生成器产出。
**理由**：同一套"一切皆包"心智；OEM 可替换整域；组合矩阵复用。
**后果**：单 Cargo workspace；服务端包必须遵守路由唯一真源与迁移不可变（ADR-022）。

### ADR-022 · 迁移不可变 + 内容哈希 + 只前向修复 `accepted`

**决策**：每个服务端包的迁移文件入库即不可变；`migrations.lock` 记录内容哈希；启动时与
`schema_migrations.checksum` 对账，不一致拒绝启动；修复只能新增前向迁移。
**理由**：参考实现发生过"历史迁移被就地改写导致崩溃循环且不可恢复"的真实事故。
**后果**：开发期也要走新迁移；`zk db lock` 是唯一更新哈希的入口。

### ADR-023 · 钱只有一本账；网关结算与 usage 同事务 `accepted`（v3.1 细化见 ADR-034）

**决策**：只有个人钱包余额（不做部门预算，不做 token 配额/金额配额并行）；月发放是"给钱包充值"的一种（v3.1：未折天，全额）；
网关转发前做准入（余额/限流/模型定价），转发后结算与 usage 在同一事务；未定价模型默认拒绝。
**理由**：参考实现四套并行额度互相打架且长期遗留死列；这是最大技术债之一。
**后果**：所有计费路径必须经 wallet 服务；禁止任何包直接改余额列。

### ADR-024 · OEM = 私有索引仓 + 每客户 submodule，构建期注入渠道 `accepted`

**决策**：OEM 客户列表在私有索引仓；每客户一个子仓（submodule，pin commit）；子仓含品牌/文案/配置，
深度 OEM 可含代码包；渠道 id 在构建期注入，运行期不可改。
**理由**：客户隔离、可复现、可审计；品牌不可被线上篡改；与参考实现一致。
**后果**：CI 需 deploy key；每渠道一套产物；OEM 产物必须过品牌残留门禁与契约矩阵。

### ADR-025 · 交付面自包含 `accepted`

**决策**：企业版交付一个 docker 镜像导出包，内含 compose/Caddyfile/管理台/门户/三平台客户端安装包；
员工从门户下载客户端；客户端从登录的服务端取更新清单。
**理由**：内网零外网依赖、版本天然配套、升级路径单一。
**后果**：镜像体积变大；发布流程必须"先出客户端安装包 → 再打镜像"；镜像内容需签名/校验和。

### ADR-026 · WASM 应用：服务端执行 + 客户端独立 origin + 每应用 SQLite；不可调 AI `accepted`（v3.1：员工可上传）

**决策**：wasmtime + wasip1；每应用一个 SQLite（限额）；客户端按 `<app_origin_scheme>://<app_id>`
开独立窗口，请求转成 JSON 信封发服务端；应用不能调用 AI、不能访问其他应用数据、无网络/文件。
**v3.1 追加**：员工可以上传应用（管理员审核/上架）；开发语言集见 ADR-035（Go/Rust wasip1，已定）。
**理由**：参考实现已验证形态；"不给自由度只给铺好的路"降低攻击面；AI 能力由客户端 agent 提供。
**后果**：需要帧 ABI、应用 DB 限额、生命周期（App/Release/Deployment）与退役调度。

### ADR-027 · 认证单一主源 + 本地应急超管；JWT + refresh `accepted`

**决策**：企业同一时间只启用一个主认证源（local/OIDC/LDAP），本地应急超管始终可用；员工客户端用
短效 JWT + refresh；钉钉/企微/飞书后续作为独立包（各按官方形态，优先级低）。
**v3.1 追加**：员工被禁用/降权后**立即失效**——每请求校验 `token_epoch`（写操作强制查库），不等 access 过期。
**理由**：多源同时启用会产生身份/用户数据冲突；桌面端不适合 cookie session。
**后果**：切换主源需要迁移/绑定流程（待确认）；JWT 吊销以 `token_epoch` 每请求校验为准（≤5s 缓存，写操作查库），不再依赖短 TTL。

### ADR-028 · Agent 循环在客户端，服务端不执行用户工具 `accepted`

**决策**：agent-loop、工具执行、审批、会话存储都在客户端；服务端只做认证、用户、钱、网关、内容分发、
WASM 应用执行、审计与管理。企业版 AI 请求经服务端网关，但"谁在调工具、调什么"由客户端决定并上审计事件。
**理由**：本地文件/终端/工作区能力天然在客户端；服务端跑 agent 会引入远程工具执行、沙箱、会话状态的巨大复杂度；
参考实现同样把 agent 放在客户端（服务端只是网关 + 管控面）。
**后果**：服务端无法强制"工具级"策略，只能强制模型/额度/内容分发；需要客户端上报审计事件（可被篡改，
故只作运营指标，不作合规证据）。

### ADR-029 · 企业版以可控性/可预期性为第一优先 `superseded → ADR-030`

**决策（已废弃）**：曾要求默认拒绝、白名单、员工不可覆盖能力。**v3.1 已改为弱管控**：服务端只管认证、
钱包/网关、商店内容、审计、构建期版本；工具默认全开、网络不限制、不做企业审批。本 ADR 仅作历史记录。
**理由**：企业采购的第一诉求是"不出事、可解释、可追责"，不是功能多；员工乱操作的成本远高于少一个功能。
**后果（历史，已被 ADR-030 取代）**：客户端企业版设置页显著变薄；管理台无自由 SQL/脚本；
新增功能必须同时给出白名单、权限点、失败语义、审计记录与测试，否则不进入企业版——**这些要求已废弃**。
与"可插拔"不冲突：可插拔是**构建期**由厂商/客户决定，不是员工运行期决定。

### ADR-030 · 服务端弱管控：只管身份/钱/商店/记录 `accepted`（取代 ADR-029）

**决策**：服务端只做认证与会话、用户、钱包与计费、AI 网关、技能/连接器/应用的分发与审批、审计、管理台；
**不下发**工具白名单/网络策略/审批策略；工具默认全部允许；网络默认开启、允许所有地址；企业不做审批流。
员工拥有客户端全部能力。个人版更自由（无服务端、无登录、无应用中心）。
**理由**：需求方明确"服务端弱管控，能力工具都下发，只管认证相关和商店功能"；企业采购要的是身份/费用/
内容/审计可控，不是限制员工操作。
**后果**：服务端不为员工行为兜底；安全责任在客户端与客户网络边界；文档/销售不得宣称"可限制员工操作"。
构建期仍固定 edition/渠道/品牌（可替换性不受影响）。

### ADR-031 · AI 网关只支持 OpenAI Responses API `accepted`

**决策**：企业版网关只实现 `POST /v1/responses`（流式 SSE + 工具调用透传）；不做 Chat Completions。
上游仍是 OpenAI 兼容 provider。
**理由**：需求方明确"企业版只支持 Responses"；减少协议面与测试矩阵。
**后果**：客户端网关适配只做 Responses；第三方只兼容 Chat Completions 的模型需要上游网关转换（或后续再加）。

### ADR-032 · 删除部门/用户组；角色为超管/审计员/员工 `accepted`

**决策**：删除组织树、部门归属、部门可见范围、部门管理员；技能/应用可见性 = **全员 / 指定用户**
（可见性部分由 ADR-036 进一步确认）。
管理台新增**只读审计员**角色（审计/用量/登录/内容变更，只读）。
**理由**：需求方"去掉用户组，现在企业都不怎么做这种东西"；审计员满足合规查看需求。
**后果**：认证源不再需要组映射；技能授权模型简化。

### ADR-033 · 不做 Windows 代码签名 `accepted`

**决策**：Windows 安装包不签名；保留 SHA256 校验和与更新清单签名；文档提示 SmartScreen 风险。
macOS 公证见 ADR-037（需要，证书后配）。
**理由**：需求方明确"Windows 代码签名证书不做"。
**后果**：Windows 首次安装有 SmartScreen 警告；企业分发需在文档中说明。

### ADR-034 · 钱包细则：月全额发放、仅管理员充值、峰谷价、流式透支 `accepted`

**决策**：月发放每月直接全额到账（不折天，幂等键 `grant:<user>:<yyyy-mm>`）；充值只有管理员手动加；
价格表支持峰/谷时段倍率，结算按请求发生时段取价；流式已输出部分内容后允许负余额（记欠费），
下次请求准入阻断；未定价模型拒绝。
**理由**：需求方逐条确认。
**后果**：`prices`/`price_periods` 成为必要表；对账需处理负余额与欠费；管理员充值必须审计。

### ADR-035 · WASM 应用：v1 语言集与帧 ABI `accepted`

**决策**（依据 `research/wasm-languages.md` 实测）：
- 运行时：Wasmtime embedding，锁 **LTS 48.x 或 50.x**（普通版本只支持 2 个月），每月跟进；
  用 epoch/fuel + StoreLimits 做超时/内存，不依赖 CLI。
- 目标：**wasip1 core module**（p2 暂缓）。v1 Tier-1 语言 = **Go 1.27 wasip1 + Rust 1.99 wasip1**；
  TinyGo 预览；C/C++ 仅 vendor；其他语言暂缓。
- ABI：`RS<len>\n<JSON>` 帧（长度前缀 + 单行 JSON），同一套 **golden frame conformance** 校验两种 SDK；
  宿主函数封闭：`db.exec/db.query/log/asset.read`（无 AI/网络/文件/env）。
- 节奏：**M4 先用 Go 打通**（参考实现生态一致），**GA 前加 Rust**；TinyGo 只在需要小体积时评估。

**理由**：p1 扁平 ABI 最容易做到语言无关；p2 各语言工具链差异大；Go 生态最容易被员工/AI 生成，
Rust 体积/启动最优（研究用 CLI 构建：37.6 KB / 15.9 ms；本原型带 serde_json/WASI 的 Rust guest 107 KB、Go guest 4.49 MB，口径不同，见 `experiments/wasm-poc/README.md`）。
**原型证据**：`experiments/wasm-poc` 已用 wasmtime 50 同时跑通 Go/Rust guest 的帧协议、宿主 DB、SQLite 持久与 fuel 限额。

**后果/风险**：Go 每请求实例成本（内存/启动）需配额、模块缓存、池化；两套 SDK 有 ABI 漂移风险，
由 golden frame conformance + 版本化 host function 兜底。

### ADR-036 · 内容可见性与月发放模式 `accepted`

**决策**：技能/应用可见性 = **全员 / 指定用户**（无部门、无私有模式）；月发放默认**全员**，支持两种模式：
`accumulate`（余额累加，默认）与 `overwrite`（月初余额置为发放额）；单人覆盖暂不做。
**理由**：需求方确认；去掉部门后"指定用户"覆盖例外场景；累加/覆盖覆盖两种企业福利策略。
**后果**：`content_grants` 支持 `grantee_type=user`；`wallet_grant_rules` 增加 `mode` 字段；月作业幂等键不变。

### ADR-037 · macOS 公证：CI 先就绪，证书后配；当前本地测试（TODO）`accepted`

**决策**：macOS 安装包要求公证 + staple；CI 流水线先实现并默认跳过，待配置 Apple 证书/API key 后启用。
Windows 仍不签名（ADR-033）。
**理由**：需求方确认"需要公证，但证书以后再配置，CI 先做好"；**当前状态：TODO——先在本地测试公证流程，证书/密钥就绪后再接入 CI**。
**后果**：发布流程需检测证书缺失并给出明确提示；无证书时产物带"未公证"标记，不阻塞内部构建。

### ADR-038 · 门户边界：只做员工客户端下载，不做 OEM 自助门户 `accepted`

**决策**：服务端提供员工客户端下载门户（三平台安装包、版本、SHA256、安装说明，随渠道品牌）；
**不提供** OEM 客户自助改品牌/看版本/自助构建的门户。
**理由**：需求方明确"不要 OEM 自助门户，但服务端需要门户提供客户端下载"。
**后果**：门户只读；OEM 变更走我方 CI 流程与人工沟通。

### ADR-039 · 最严格静态检查 + 边界运行时校验 `accepted`

**决策**：TypeScript 开启 `strict` 及全部严格附加项（`noUncheckedIndexedAccess`、`exactOptionalPropertyTypes` 等），
禁用 `any`/`as`；ESLint 用 `strictTypeChecked` + `--max-warnings 0`；Rust 用 workspace lints
（`unsafe_code=forbid`、`clippy::all/pedantic/nursery/cargo=deny`、`unwrap_used/expect_used/panic/
indexing_slicing/arithmetic_side_effects/as_conversions=deny`）；所有外部输入用 Rust 生成的 JSON Schema
在运行时校验；CI 全量门禁 0 容忍。细节见 `18-strictness.md`。
**理由**：需求方要求"防止技术性 bug"；把类型/边界/错误处理变成编译期或 CI 失败，而不是靠 review。
**后果**：初期开发摩擦增大（尤其 `exactOptionalPropertyTypes` 与 `isolatedDeclarations`）；
所有例外必须行内注明理由；第三方 d.ts 允许 `skipLibCheck`（唯一妥协）。

### ADR-040 · Rust 手写代码零 `unsafe`（产品 crate forbid）`accepted`

**决策**：本仓库**手写** Rust 代码零 `unsafe`：产品 crate（`packages/`、`apps/`、`tools/`、`crates/`）
一律 `#![forbid(unsafe_code)]`；禁止手写 `unsafe` 与 `#[allow(unsafe_code)]`。FFI/WASM 导入只允许来自
①自动生成绑定（`@generated`）或 ②独立、最小、审计并登记 ADR 的 `*_sys` crate；第三方依赖内的 unsafe
通过**锁定版本 + `cargo-geiger` + `cargo-deny`** 管理。`experiments/` 原型允许标注例外。
**理由**：需求方明确"Rust 不允许使用 unsafe"；把内存安全从"review 靠人"变成"编译期不可能"。
**边界**：完全依赖树（含 wasmtime/Tauri）零 unsafe 不可行——那等于放弃 WASM 运行时与桌面壳；
本政策约束**我们写的代码**，依赖按供应链审计管理，并对外说明。
**后果**：CI 增加 `cargo geiger` 报告与源码 `unsafe` 扫描；任何 `allow(unsafe_code)` 直接红。

### ADR-041 · 更新通道：个人版 GitHub Releases / 企业版自建更新面 + 手动升级 / 客户端 1:1 `accepted`

**决策**：
- 个人版：只从 **GitHub Releases** 检查与下载（stable/beta，`latest.json` + `SHA256SUMS` + 安装包），不做自建更新面。
- 企业版服务端：从**自建更新服务器**（静态 HTTPS，`<channel>/latest.json` + 版本镜像 `tar.zst` + SHA256SUMS）检查；
  **必须手动升级**（备份 → 下载 → 校验 → `docker load` → `docker compose up -d` → `/readyz`），
  不做自动升级/自更新守护进程；内网可离线拷贝镜像包。
- 企业版客户端：**只从登录的服务端**取更新清单与安装包；清单 `client_version` 必须等于服务端镜像内
  `CLIENT-RELEASE.json` 版本（1:1）；渠道/edition 不匹配拒绝升级；服务端可强制更新。
- 发布单元：服务端镜像与三平台客户端同批构建、同版本号；镜像缺客户端资产或版本不一致 → 构建失败。
**理由**：需求方明确要求；参考 ZeoKeel-Harness 的"客户端只从登录服务端更新 + 服务端手动升级 + 安装包随镜像"模型。
**后果**：企业版升级是人工运维动作（有备份/回滚步骤）；个人版依赖 GitHub 可用性；客户端不能跨渠道洗包。

### ADR-042 · 渠道可内置服务端地址，首配/登录页按渠道决定是否免填 `accepted`

**决策**：`channel.json` 的 `defaults.server_url` **可选**：
- 非空（OEM 典型）：构建器注入客户端常量，登录页**不显示服务端地址输入框**，员工安装后直接账号/密码；地址不可编辑，同渠道跨版本不得变化。
- 为空（官方企业版默认）：首配页显示地址输入，客户首次填写并保存到本地；设置中可改（需重新登录）。
个人版必须为空且无登录。**构建期默认值**唯一真源 = `channel.json`（`oem.toml` 不再重复）；空时由首配页本地覆盖。
**理由**：需求方明确"OEM 包里包含指定域名，装完直接输账号密码"；参考 `zeokeel/channels` 的 `defaults.server_url`。
**后果**：渠道校验新增 server_url 规则（可选/合法/不可变）；首配/登录页按地址是否为空分两种形态；非空时客户端不得提供改地址入口，空时只允许在设置中改（重新登录）。

### ADR-043 · 个人版优先；企业版 = 个人版 + 企业管控包 `accepted`

**决策**：
- `editions/personal.toml` 是唯一客户端基线（壳/UI/agent/工具/审批/会话/本地 provider/GitHub 更新）。
- `editions/enterprise.toml` 设 `base_edition = "personal"`，**排除**个人版专属包（本地 provider 设置 UI、
  GitHub 更新器、个人引导），**追加**企业管控包（登录/JWT、网关 provider、技能/连接器/应用客户端、服务端域、管理台），
  并**覆盖**少数角色（provider、更新源、客户端认证）。公式：`(personal − personal-only) + 企业包 + 覆盖`。
- 不复制客户端基线；企业版闭包必须能表示为 `(personal − personal-only) + 追加/覆盖`，CI 用 `zk plan` 断言
  "除被覆盖/排除项外，个人版包集合 ⊆ 企业版闭包"，且企业版闭包不含 personal-only 包。
- 开发顺序：个人版先交付，企业版在其上叠加；WASM 应用中心再后置。
**理由**：需求方明确"优先个人版，企业版只是增加了企业管控包"；避免两条产品线/双客户端基线。
**后果**：任何企业版功能都必须能表达为"追加包/覆盖角色"；不能在企业版里私改个人版行为。
个人版安装包最小；企业版安装包=个人版能力+管控客户端。

### ADR-044 · 渠道与版本正交：官方企业版也是渠道 `accepted`

**决策**：所有产物 = `edition × channel`：
- `personal × official`：个人版客户端（无登录、无 server_url）。
- `enterprise × official`：官方企业版；`server_url` 默认空 → 首配页由客户配置服务器地址。
- `enterprise × oem-<customer>`：OEM 版；`server_url` 可预置（免填）也可留空（客户配置）。
官方渠道与 OEM 渠道走**同一条**构建/校验/测试链路（品牌、server_url 语义、渠道身份校验完全一致）；
`demo-oem-acme` 等 demo 渠道用于验证 OEM 能力，不是特殊代码路径。
**理由**：需求方要求"官方企业版也作为一个渠道，地址可指定或不指定"，以获得灵活性与 OEM 可测试性。
**后果**：渠道实现不能有"官方特判"；`official` 只是内置的一个 channel 目录；EDITION/CHANNEL 两个常量共同决定运行形态。

### ADR-045 · 组合优先级与 exclude 语义冻结 `accepted`

**决策**：层序 `base_edition → edition → profile → channel`；同层重复 singleton = error；跨层后覆盖前；
`exclude` 只作用于继承 base，值为包 id，未命中=error；multi 默认替换，追加/删除用 `[packages.add]`/`[packages.remove]`；
每次 compose 输出 `resolve-report.json` 进 T0。详见 `24` §1。

### ADR-046 · 生成物一路径一 writer `accepted`

**决策**：每个生成物只有一个写入者（zk / ts-rs / schemars / 生成块）；`--check` 只生成到临时目录逐字节比较、不写源文件；
混合文件用 `@generated:begin/end` 块；禁止两个命令写同一路径。详见 `24` §3。

### ADR-047 · 路由与 IPC 命令采用静态真源 `accepted`

**决策**：`[[routes]]`/`[[commands]]` 在 manifest 静态声明，`zk api` 不编译即可生成 catalog/permissions；
运行期 `Register()` 只做一致性校验，多/少/冲突/顺序不符 → 拒启。详见 `24` §4。

### ADR-048 · 钱包：账本只增 + hold 预授权 + 幂等 `accepted`

**决策**：余额只经账本变更，禁止直写；`overwrite` 必须写 `adjust`（delta=新−旧），欠费不得被发放抹平；
账本 `idempotency_key` 唯一索引；准入原子 hold（`wallet_holds`）并限制负余额/并发/单请求上限；结算状态 pending/settled/voided。
详见 `22` §2–§5。

### ADR-049 · 审计哈希链必选 + WORM 锚定 `accepted`

**决策**：审计只追加，哈希链必选；每日锚定到只写介质；应用账号无 UPDATE/DELETE 权限；
钱操作审计矩阵（配额入账/调账/改价/时段/发放/清空/销户）缺项 CI 红。详见 `22` §8。

### ADR-050 · 客户端窗口 capability 隔离与本地载体防 rebinding `accepted`

**决策**：主窗口/登录窗口/WASM 应用窗口各自最小 capability；应用窗口只允许 `app_request`，禁止 `ipc_call`/shell/fs；
`serve` 绑 127.0.0.1，校验 Host/Origin/token/自定义头，防 DNS rebinding。详见 `19` §7、`21` §6。

### ADR-051 · 备份 PITR + 加密分离 `accepted`

**决策**：PG WAL 归档/PITR + `/data` 快照；备份加密且密钥与备份分离；每月自动恢复校验、每季度人工演练；无备份不升级。
**不定义 RPO/RTO 承诺**（D64：暂不考虑），只记录实际备份/恢复耗时。详见 `20` §3。

### ADR-052 · 升级 expand/contract、digest 固定、客户端可降级 `accepted`

**决策**：迁移分 expand/contract，保证 N-1 读兼容；`SERVER_IMAGE` 用 tag@digest，保留前 2 版；
升级前备份；服务端回滚保留上一版客户端清单/降级包，禁止客户端砖化；镜像与清单独立验签。详见 `20` §4。

### ADR-053 · 客户端会话存储只选 SQLite WAL `accepted`

**决策**：SQLite WAL 为唯一运行时存储（schema v1 + 迁移）；turn 原子提交；interrupted 可恢复；单实例锁；
JSONL 仅导入/导出。详见 `24` §7。

### ADR-054 · 更新签名密钥与 `master.key` 生命周期 `accepted`

**决策**：更新签名私钥离线/HSM 保管，客户端内置双公钥轮换；`master.key` 首启强制生成/0600/离线备份/版本化轮换；
JWT 双 key 重叠；MCP 改为公共客户端 + PKCE。详见 `21` §3。

### ADR-055 · 余额 = 企业配额（非预付资金）`accepted`

**决策**：钱包余额是**企业内部用量配额**，不是客户预付资金；不可提现、转让、退款。
不提供收款、开票、税务、坏账、资金存管功能（D60）。`充值/调账` 仅指管理员分配配额。详见 `22` §1。

### ADR-056 · OEM 定制代码与 "Powered by" `accepted`

**决策**：OEM 定制代码由**我方开发、源码纳入我方仓库**；不引入第三方闭源预构建包（D61），因此无闭源许可/赔偿例外。
"Powered by ZeoKeel"：官方渠道默认显示；OEM 渠道默认隐藏（可配置显示），允许隐藏（D62）。详见 `23` §7。

## B. 风险登记

等级：L=可能性 H/M/L，I=影响 H/M/L。触发条件写进 CI 或监控，避免"感觉还好"。

| # | 风险 | L | I | 缓解 | 触发/守卫 |
| --- | --- | --- | --- | --- | --- |
| R1 | 需求方期待"运行时换包"，构建期方案不满足 | M | H | 规划阶段讲清代价（ADR-001）；保留 WASM 扩展点；profile 切换足够快（增量构建 < 2 min） | 需求评审时明确"换包是否要求免重建" |
| R2 | 契约过宽（什么都进契约）导致改一处全生态红；或过窄导致实现细节外泄 | H | H | 契约只放"替换边界必需"；`cargo public-api` diff 审查；契约变更需符合性套件全绿 | 契约 PR 必须附 public-api diff |
| R3 | `zk` 组合器变成第二个产品，复杂度失控 | M | H | 先做最小命令集；包索引读 `zk.toml`，`cargo metadata` 只做交叉校验；golden 测试锁行为 | `tools/zk` 代码量预算（< 8k 行）与月度复审 |
| R4 | 编译时间/磁盘爆炸（70+ crate、双端依赖树、多 edition/OEM 渠道） | H | M | 单 workspace + 增量构建、sccache、nextest、PR 只跑受影响项、nightly 跑全矩阵；`CARGO_TARGET_DIR` 共享 | PR < 20 min，nightly < 90 min（S0 标定） |
| R5 | WebView 跨平台差异（Linux WebKitGTK / WebView2 / WKWebView） | H | M | Transport 抽象 + HTTP 副载体 e2e；Playwright webkit project；browserslist + `@supports` 兜底；关键 CSS/特性降级 | 每平台冒烟 job；发布前人工三平台验收 |
| R6 | 供应链：npm 依赖数量与 cargo 依赖审计 | M | H | `cargo-deny` / `cargo-audit` / `pnpm audit`；lockfile 入库；`cargo vendor` 离线；SBOM；新依赖需说明 | CI `supply-chain` 阶段失败即阻断 |
| R7 | cordis 内核限制（async 支持、panic 容器、生命周期语义） | M | H | 研究简报 `research/kernel.md` 已梳理边界；宿主侧补服务监督（`supervision` feature）、超时、取消；关键语义写测试 | 内核升级必须跑全量换包矩阵（先例教训） |
| R8 | rig 高频 breaking（1–3 周/次，0.46-rc 已发布）+ MSRV 1.97.1/edition 2024 + lock 体积（facade 663 vs core+agent 244 包） | H | M | ADR-007 双层隔离；`=0.45.0` 精确钉；只依赖 rig-core+rig-agent；升级仅改两个适配包并跑契约对拍/录制回放；`zk-llm-openai`/`zk-llm-mock` 保证契约可被非 rig 实现 | 升级 PR 必附 conformance + 冷构建时长/lock 体积变化 |
| R9 | 前端包 id 覆盖率不足、React 双实例、bundle 膨胀 | M | M | composer 校验 `required ⊆ provided`；vite 单实例配置；bundle 体积预算；包依赖去重 | 构建期失败 + 体积预算 job |
| R10 | 团队认知负担：包化"四件套"（清单/契约/符合性/README）拖慢迭代 | H | M | `zk init` 脚手架；包模板；`zk doctor`；文档只有一份入口；新包 review checklist | 新建包的平均落地时间作为指标 |
| R11 | 服务端迁移事故（历史迁移被改写/无修复通道） | M | H | ADR-022：不可变 + 内容哈希 + 只前向；备份/恢复演练；升级 e2e | 改历史迁移必须红；恢复演练 M6 退出条件 |
| R12 | 性能预算（启动、流式首字、二进制体积） | M | M | 启动阶段计时、首 token 计时、体积预算；profile 裁剪（minimal 证明可裁） | 性能 job 对比基线，超阈值告警 |
| R13 | 契约发布/单实例方案不可行 → 第三方接入落空 | M | H | S0-P1 先做 2 天 spike；退路：固定 git rev 或 vendor-only，并在文档降级承诺 | spike 结论必须落 ADR；未通过则 M3 退出条件改口径 |
| R14 | 并发/取消/审批挂起语义缺失，M2 才炸 | M | H | 05 §4.1 先定状态机与规则，M1 就有契约测试（审批中取消、双 turn、断流重订阅） | 这些测试进 M1 退出条件 |
| R15 | 跨平台（Windows CRLF、WebView2、路径）拖到 M4 | H | M | `.gitattributes` 强制 LF；M0 起 3-OS `cargo check`；每平台冒烟 | 3-OS job 恒开 |
| R16 | `zk` 与 cargo 的耦合出错（metadata 失败、路径/版本解析） | M | M | 包索引读 `zk.toml`；metadata 只做交叉校验且 fail closed；POC 式 golden 测试 + 故意做坏的 fixtures | `zk` 的单测与负向 fixtures |
| R17 | 排期低估（初版 M1 31 包/3 周） | H | M | 按包数 × 0.5–1 人日重排（M1 14 包 5–6 周）；先做 S0 基线测量 | 每里程碑复盘实际人日 vs 估算 |
| R18 | 范围过大（双端 + 7 域 + OEM + 双版本）导致长期无法交付 | H | H | 严格 M1 个人版 → M2 企业版递进；每阶段可演示；控制 M1 包数 | 每阶段退出条件未达不进入下一阶段 |
| R19 | 服务端安全（认证绕过/网关 SSRF/密钥泄漏/WASM 逃逸） | M | H | 单租户；密钥不出服务端；**内部安全评审**门禁；WASM 无网络/文件/限额；自查清单 | 内部评审无高危；安全用例进 CI |
| R20 | 计费漏洞（并发扣费/未定价模型/流式透支） | M | H | 结算与 usage 同事务；未定价拒绝；预授权 + 兜底；并发扣费测试 | 计费一致性测试与对账 job |
| R21 | OEM 私有仓/权限/产物隔离 | M | H | deploy key + commit pin；最小权限；产物隔离与品牌残留门禁 | OEM CI 负向测试 |
| R22 | Windows 未签名的安装警告/分发摩擦 | H | L | 已接受不签名（ADR-033）；文档/门户提示 SmartScreen；提供 SHA256；所有渠道一致，不做 OEM 签名例外 | 安装文档检查；客户验收清单 |
| R23 | 个人版/企业版代码分叉 | M | M | 同库 edition；CI 同时构建两版；共享契约 | 两版构建与单测都进 PR |
| R24 | WASM 应用资源耗尽/生命周期错乱 | M | M | 限额单一真源；App/Release/Deployment 状态机；退役调度；并发配额 | 资源/超时/上下架测试 |
| R25 | 弱管控下员工行为导致的数据/费用/合规风险 | M | M | 服务端不兜底；文档/销售明确边界；客户终端与网络管理承担责任；审计只记录可观测部分 | 销售材料评审；安全声明随发布 |
| R26 | 只支持 Responses API 导致部分模型/客户端兼容性下降 | M | M | 上游 provider 仍为 OpenAI 兼容；必要时在上游做转换；客户端只做 Responses 适配 | 兼容性测试矩阵；用户反馈 |

## C. 决策状态

`15-open-questions.md` 的 Q1–Q20 **均已决**（D1–D65 / ADR-001…056）；域内补充规范见 `19–24`。没有阻塞 S0/M0 的开放架构问题；业务口径 D60–D65 已拍板。
剩余为细节配置，不阻塞开发：

| 细节 | 默认 |
| --- | --- |
| 峰谷时段具体值 | 默认 9–18 点（Asia/Shanghai），可按模型覆盖 |
| 技能/应用"指定用户"选取交互 | 管理台多选用户；接口按 D47 |
| MCP 客户端 OAuth 令牌存放 | 客户端本地（D53） |
| macOS 公证证书 | 本地先测，证书后配（ADR-037） |
