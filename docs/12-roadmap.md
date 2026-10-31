# 12 · 路线图（S0 + M0–M6）

> 排期按 1 名熟悉 Rust + 前端的工程师全职估算；2–3 人并行可压缩约 30–40%。
> M0–M6 的退出条件是**可执行断言**，不是"感觉做完了"；S0 是 spike，退出物 = ADR 结论 + 可重跑命令 + 证据路径。
> 范围：个人版 → 企业版核心 → 内容平台 → 应用中心 → OEM/交付 → 加固。

> **状态（2026-10-31 最终审计）**：本文件是**历史计划与退出条件清单**，不是当前施工状态。
> M0–M4 已完成（证据见 `specs/changes/2026-10-30-*/acceptance.md` 与 `docs/reviews/`）；
> M5/M6 仅软件/本地可执行部分完成，Docker 镜像、多平台安装包、公证、真实私有 OEM 索引等
> 明确标 **UNVERIFIED**；下文 M2/M5/M6 未勾选条目保留为历史计划，不得当作当前 false-green。
> 当前可执行门禁与 backog 状态以 `AGENTS.md` §6、`docs/24` §3/§4、`docs/reviews/2026-10-31-round12-verification.md` 为准。

## 总览

| 阶段 | 主题 | 人周 | 关键产物 | 阻塞 |
| --- | --- | --- | --- | --- |
| **S0** | 6 个前置 spike | 2 | 6 条 ADR + 命令/证据；基线在 M0 实测 | — |
| **M0** | 骨架与机制 | 3 | `zk` 双端组合、两端可启动、CI 骨架 | S0 |
| **M1** | 个人版 MVP | 6 | 纯客户端：对话/工具/审批/本地存储/自配 provider/三平台安装包 | M0 |
| **M2** | 企业版 = 个人版 + 企业管控包 | 6 | 在 M1 客户端上追加：登录/JWT、网关 provider、商店客户端、服务端（认证/钱包/网关/商店/审计）、管理台核心、docker | M1 |
| **M3** | 内容平台 | 5 | 技能中心 + 连接器 + 审计 + 客户端安装/更新 | M2 |
| **M4** | 应用中心 | 5 | WASM 运行时/每应用 DB/生命周期/客户端开窗/管理台运维 | M3 |
| **M5** | OEM 与交付 | 4 | 渠道/品牌、OEM submodule、双版本+OEM 产物、更新、门户（**Windows 不签名**） | M4 |
| **M6** | 加固与发布 | 4 | 安全评审、备份恢复、i18n/无障碍、性能、可复现、发布 | M5 |

合计约 35 人周；**关键路径** S0 → M0 → M1（个人版客户端）→ M2。**客户端优先**：M1 是第一个可交付产品，
服务端只做认证/网关/钱包配套；WASM 应用中心（M4）后置，不阻塞客户端。

**S0 进度**：P2（WASM 运行时/帧 ABI/宿主 DB/限额）原型已完成（证据未归档，M0 前补跑并存 `target/evidence/`），见 `experiments/wasm-poc`；
P1（双 app 组合）、P3（认证+JWT）、P4（OEM submodule CI）、P5（迁移不可变）、P6（管理台包化）待做。

---

## S0 · 六个前置 spike（2 人周）

每个 ≤2 天，结论必须落 ADR；失败要有退路。

| # | spike | 验什么 | 退路 |
| --- | --- | --- | --- |
| P1 | 双 app 组合 | 同一 workspace 下 client/server 两个 app 各自组合、共享契约、单实例 | 退回两个 workspace + 契约发布 |
| P2 | WASM 运行时 + 语言集 + 每应用 DB | **✅ 原型完成（证据待归档）**（`experiments/wasm-poc`）：wasmtime 50 + Go/Rust wasip1、`RS<len>\n<JSON>` 帧、`db.exec/db.query` 宿主函数、SQLite 持久、fuel 限额、golden conformance；剩余：墙钟超时/内存上限/SQL 闸门/并发配额（M4 补） | 已通过；M4 先 Go 打通，GA 前加 Rust |
| P3 | 认证 + JWT | 本地/OIDC/LDAP 三实现同一契约；JWT access/refresh/吊销 | 退回不透明 token（参考实现） |
| P4 | OEM submodule CI | 私有索引仓 + submodule pin + deploy key + 渠道构建 | 退回纯引用列表（zk 拉取） |
| P5 | 迁移不可变 | 内容哈希 + 启动校验 + 前向修复流程 | 退回只读校验（无自动） |
| P6 | 管理台包化 | admin UI 包 + 内嵌二进制 + 权限矩阵生成 | 退回单一 SPA（先不分包） |

每个 spike 的退出物 = **ADR 结论 + 可重跑命令 + 证据路径**（`11` §5）；S0 只做机制验证，T0–T5 基线在 M0 骨架实测（`11` §1）。

---

## M0 · 骨架与机制（3 人周）

**交付物**

1. 单一 workspace + 工具链（1.99.1、clippy、nextest、pnpm）、`.gitattributes`、根 `zk.toml`（`24` §2.1）。
2. `zk` v0：`validate` / `compose` / `compose --check` / `plan` / `api` / `docs` / `db lock`（含 `--check`）；支持 edition/profile/channel 叠加。
3. 契约最小集：package / kernel（Registrar/Config/Tasks/Clock）/ tool（`02` §5；`api` 契约在 M1 加入，M0 的 api-catalog 由 manifest `[[routes]]` 生成）。
4. 两端骨架：`apps/desktop`（Tauri 空壳 + UI registry）+ `apps/server`（axum + /healthz + 管理台空壳）。
5. 生成物：两端 glue、api-catalog、迁移清单；负向测试 7 条。
6. CI 骨架：三平台 `cargo check`、validate/drift、单包测试。
7. 复用/去重门禁：`zk capability` 能力索引 + 禁止目录/import/克隆检测 + `duplication-baseline.json`（`25` §4）。

**退出条件**

- [ ] `zk compose --edition personal --profile default --channel official` 产出可编译的 desktop glue；`--edition enterprise`（同参数）产出 desktop+server+admin glue。
- [ ] 客户端可启动并渲染一个来自包注册表的组件；服务端可启动并返回 `/healthz`。
- [ ] 7 条负向测试全红（唯一清单见 `03` §6.4）。
- [ ] 路由/命令静态表与运行期注册不一致时拒启；迁移重复/改历史/顺序漂移均红（`24` §9 M0 清单）。
- [ ] 完整 M0 验收另见 `24` §9（M0 部分）。
- [ ] `zk capability --check` 绿；新增克隆块 = 0（相对 baseline）；kit 外新增 utils/helpers = 0。
- [ ] CI 热缓存下 T0+T1 < 10 min（实测记录）。

---

## M1 · 个人版 MVP（6 人周）

**目标**：个人用户下载安装包，自配 key，能完成真实任务。

**交付物（包清单由 `zk plan` 派生，不在文档手写数量）**

- 客户端壳（Tauri）、UI shell/chat/composer/settings、本地化与主题；
- 本地 agent-loop（自研状态机，rig-core 只做模型调用）、工具 fs/shell/todo、**审批占位（M1 inline；完整 takeover/策略 post-M1）**、会话存储（SQLite WAL 唯一真源；JSONL 仅导入导出，`24` §7）；
- provider：DeepSeek / OpenAI 兼容 / Ollama（自配 key，系统密钥库）；
- **工作区（Workspace）**：会话必须归属工作区；首次使用创建/选择默认工作区（`docs/26`）；侧栏按工作区组织会话；
- 本地技能目录、本地 MCP（可选，若时间允许）；
- 三平台安装包（**Windows 不签名**，ADR-033）、**GitHub Releases** 更新源。

**退出条件**

- [ ] 真模型一轮：流式 + 工具调用 + 会话落库 + 重开恢复；审批为 inline 占位（完整审批 post-M1）。
- [ ] 无工作区不能建会话；两个工作区的会话/消息隔离；重启后 workspace_id 归属不丢（`26` §7）。
- [ ] `chat.send` 无 `workspace_id/conversation_id` 时失败（删除 auto-create fallback）；agent cwd/工具根 = workspace。
- [ ] 离线 e2e（mock provider）全绿；无服务端依赖。
- [ ] 三平台安装/卸载/启动冒烟通过。
- [ ] 冷启动 < 2s、首 token < 2s（本地 mock 基线）。

---

## M2 · 企业版 = 个人版 + 企业管控包（6 人周）

**目标**：在 M1 个人版客户端之上叠加企业管控包：一套 docker 部署起来，员工登录即用，AI 走网关并扣费。
**不重做客户端基线**：`(personal − 个人专属包) + 企业包 + 覆盖 provider/更新源角色`（`exclude` 本地 provider 设置 UI、GitHub 更新器）。

**交付物（包清单由 `zk plan` 派生）**

- 服务端：认证（local/OIDC/LDAP 单一主源 + 应急超管）、JWT+refresh、用户（无部门）、钱包（余额/月发放/充值）、
  网关（**仅 `/v1/responses`** + 流式 + 工具调用；上游 OpenAI 兼容）、计量结算、审计基础、系统配置；
  **目录下发**（模型目录、商店目录、更新通道）——只下发内容，不下发工具/网络/审批策略；
- 客户端：登录/刷新、bootstrap 配置下发、网关 provider（不可自配）、未登录态处理；
- 管理台：登录、用户/角色、provider 与峰谷价、用量、系统配置；
- 部署：多阶段 Dockerfile + compose + Caddy + 门户 + 客户端安装包内嵌；`.env`（键见 `06` §5）；备份/恢复脚本。

**退出条件**

- [ ] 新环境：`docker load` + `compose up` + 管理员初始化 + 员工登录 + 完成一轮对话并扣费。
- [ ] 超额阻断：余额不足时网关拒绝，客户端提示明确。
- [ ] 三种认证源各自可用（切换配置）；JWT 刷新/吊销正确。
- [ ] 企业版客户端**没有** provider 设置入口（构建期裁剪验证）；设置页快照不含被禁字段。
- [ ] 目录下发：管理员改模型/商店目录后，客户端选项随之变化；员工不能本地伪造服务端目录/绕过网关。
- [ ] 管理台改配置即时生效或明确提示重启。
- [ ] 升级/回滚演练通过（旧数据 → 新版本 → 回滚）。

---

## M3 · 内容平台（5 人周）

**目标**：技能与连接器可管理、可分发、可强制更新。

**交付物（包清单由 `zk plan` 派生）**

- 技能中心：zip + SKILL.md 校验（16MB、允许资源、暂不扫描）、上传、审批、归属、转交、可见范围（全员/指定用户）、版本、
  强制更新（sha256 整树）、回滚、配额；
- 客户端技能：安装/卸载/更新/冲突处理/来源展示；
- 连接器中心：管理员下发 MCP（公共客户端 + PKCE、OAuth 令牌客户端本地）、员工启停、强制更新；个人版本地 MCP；
- 审计：管理操作、审批、下发、登录事件；
- 管理台对应页面。

**退出条件**

- [ ] 员工上传 → 管理员审批 → 全员/指定可见并安装 → 新版本强制覆盖 → 删除后客户端自动移除。
- [ ] 非归属人无法更新；管理员转交后权限正确。
- [ ] 连接器密钥不出现在客户端日志/存储；令牌过期后自动刷新。
- [ ] 审计可查完整链路（谁上传/谁审批/谁安装）。

---

## M4 · 应用中心（5 人周）

**目标**：员工可上传 WASM 小应用（管理员审核/上架），客户端内独立窗口使用。**M4 先用 Go(wasip1) 打通，GA 前加 Rust**（TinyGo 预览）。

**交付物（包清单由 `zk plan` 派生）**

- 服务端：Wasmtime LTS embedding + wasip1、`RS<len>\n<JSON>` 帧 ABI、golden frame conformance、每应用 SQLite（限额）、
  App/Release/Deployment 生命周期、
  上架/下架、退役调度、资源限额与并发配额、审计；
- 客户端：自定义 origin per app、窗口闸门、深链、请求信封、错误页；
- 管理台：应用列表、版本、运行诊断、上下架；
- 示例应用（表单/数据处理各一）。

**退出条件**

- [x] 示例应用发布 → 员工在客户端独立窗口打开 → 数据写入服务端每应用 DB → 重启后仍在（M4 R8；Tauri 窗口 ACL 为 P2 manual exception）。
- [x] 上架/下架语义正确：下架后新打开 403、已打开 request 410。
- [x] 应用无法调用 AI、无法访问其他应用 DB、无法发起未授权网络请求（宿主函数封闭/跨 app DB e2e）。
- [x] 资源耗尽（内存/CPU/时间/连接）被限额与超时挡住（单测/限额 e2e；HTTP 逐项矩阵 M4.1）。

---

## M5 · OEM 与交付（4 人周）

**目标**：新增一个白标客户只需建目录/仓 + 跑流水线。

**交付物（包清单由 `zk plan` 派生）**

- 渠道/品牌体系：channel id、brand 资源与文案、构建期注入、跨渠道唯一性校验、品牌残留门禁；
  **官方企业版也是渠道**（`official`，server_url 默认空由客户配置），OEM 渠道可预置地址；demo 渠道验证 OEM；
- OEM 索引仓模型：私有索引仓 + submodule + pin；基本 OEM（品牌/字符串/配置）与深度 OEM（代码包）；
- 双版本 + OEM 产物：企业版/个人版/OEM 镜像与安装包；门户按渠道；
- 签名：**Windows 不签名**（安装时提示 SmartScreen）、**macOS 公证 + staple**（本地先测 → CI 就绪 → 证书后启用）、Linux AppImage 校验和；
- 更新：**个人版走 GitHub Releases**；**企业版服务端用自建更新服务器 + 手动导入镜像升级**；企业版客户端只从登录服务端按 1:1 更新；强制/灰度策略；
- 交付文档：部署、升级、回滚、OEM 接入。

**退出条件**

- [ ] 用 demo 客户仓跑通（实现 R6 APPROVED；真实镜像/三平台包 UNVERIFIED）"新增客户 → 产出其镜像 + 三平台安装包"；按 `brand.powered_by` 断言品牌残留（hide=零厂商串，show=仅固定文案）。
- [ ] 深度 OEM 代码包通过契约 conformance + 换包矩阵。
- [ ] macOS 公证 CI 就绪（拿到证书后一键启用）；Windows 安装包附 SHA256 并在文档提示未签名。
- [ ] 企业版员工从门户下载客户端，且只从登录服务端 1:1 更新；个人版从 GitHub Releases 更新。
- [ ] 服务端升级演练：自建更新服务器下载 → 校验 → `docker load` → 备份 → compose up → `/readyz`；失败可回滚。

---

## M6 · 加固与发布（4 人周）

- **内部安全评审**（认证/网关/WASM/文件/供应链）+ 自查清单修复；**弱管控风险声明**（工具/网络不限制）写入文档；
- 备份/恢复演练、故障注入、限流与超时、日志脱敏；
- i18n/无障碍/键盘导航的**全面审计**（中英，中文优先；页面级从 M1 起持续，见 `19` §6）；
- 性能预算（启动、首 token、网关延迟、镜像体积、管理台 bundle）；
- 可复现构建（同版本两次产物哈希）、SBOM、许可证清单；
- 文档：部署/运维/API/贡献/故障排查；发布 1.0。

**退出条件**

- [ ] 安全评审无高危未修复项。
- [ ] 备份恢复演练成功；故障注入不导致数据损坏。
- [ ] 性能预算达标；可复现校验通过。
- [ ] 三平台 + 镜像 + 门户 + 文档齐备。

---

## 并行拆分

| 工作流 | 范围 | 依赖 |
| --- | --- | --- |
| A 内核/组合 | `zk`、契约、runtime、CI | S0 |
| B 客户端 | agent/工具/UI/登录/技能/连接器/WASM 宿主 | A 契约 |
| C 服务端 | auth/users/wallet/gateway/skills/connectors/apps/audit | A 契约 |
| D 管理台 | admin UI 包 + 页面 | C 的 API |
| E 交付/OEM | docker、安装包（Windows 不签名）、渠道、OEM 流水线 | A–D 产物 |

原则：**契约先冻结**（M0 末）；C 与 D 以 API catalog 为接口并行；E 从 M1 起并行做安装包骨架。

## 待决策（影响排期）

**无阻塞性决策**：关键项已全部落定（见 `15-open-questions.md`、`13`）：
Responses-only、月发放全员/累加或覆盖、峰谷默认 9–18、WASM 语言集 Go+Rust、可见性全员/指定用户。

剩余实现细节统一索引 `15` §C（峰谷时段已定默认 9–18 点可按模型覆盖，见 D53；MCP 令牌已定客户端本地）。
