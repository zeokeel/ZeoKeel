# 二轮交叉互审 A：19/24 与既有规范的冲突与重复

> 范围：`docs/19`、`24`（新增/重写）对照 `01/02/03/04/05/06/09/10/11/16/18/21`、`13`（ADR）、`AGENTS`。
> 行号 `NN:12-34` = `docs/NN-*.md` 第 12–34 行；`AG:237` = `AGENTS.md` 237 行。
> 结论先行：`24` 的决策与 ADR-045/046/047/053 一致，**主要问题是旧文未回写**；`19` 的问题不是缺内容，而是**越界复述 05/16/21/09 并引入两个不可实现项**（每应用 capability、关闭窗口=托盘）。

## 0. 权威裁定（本轮建议冻结）

| 域 | 唯一权威 | 必须回写/删除的旧文 |
| --- | --- | --- |
| 组合层、exclude、manifest v1、生成物 writer、路由静态表、迁移命名、shutdown、metadata | `24`（+ADR-045/046/047/053） | `02` 只留包模型/契约；`03` 只留示例；`10` 只留布局；`06` 的路由/迁移段重写 |
| 客户端运行时、包注册、IPC 分发、生命周期、审批门语义 | `05` | `19` 只写 UX，机制一律链接 `05` |
| 页面/槽位/组件/registry | `16` §3（`SLOT_DEFS`） | `19` §2 不得另立 slot 命名；`02` 删重复 `[[ui.contributions]]` 示例 |
| 测试层、证据、通用门禁、六态、axe | `11`（+`18` 静态检查） | `19` §10、`24` §9 只保留域内断言并链接 |
| 本地载体安全、capability、密钥、审计完整性 | `21` | `05:297-299`、`16:230-231`、`19:131-136` 全部压成链接 |
| 渠道/server_url/i18n/更新源/交付 | `09`（+ADR-041/042/044） | `19` 的地址、更新、i18n 叙述改为链接 |
| 客户端会话存储 | `24` §7（+ADR-053） | `03:99` 必须改；`05:61`、`19:146`、`21:85` 只链接 |
| 服务端进程/PG/调度 | `06` | 但 `06` 的路由、迁移 schema、关闭 FSM 三节须按 `24` 重写 |

## 1. P0（照任一份实现都会错）

| # | 文件:行 | 冲突/重复 | 影响 | 建议（保留/删除/链接） |
| --- | --- | --- | --- | --- |
| P0-1 | `02:28-72` vs `24:65-116` | 两套 `zk.toml` v1：`[package]/[provides]/[requires]{contract}` vs 顶层 `schema/id`+`[role]`+`[requires]=cardinality`；`24` 未知键=error，`02` 的 `[config]/[permissions]/[compat]/[conformance]/stability/summary` 在 `24` 无位置 | 按 `02` 写的包被 `24` 校验拒绝；生成器无法定 schema；C3 四件套无法校验 | 以 `24` §2 为唯一 v1，但先把 `02` 的 8 个字段并入；删除 `02:28-72` 代码块，`02` §2 只留“为什么+四件套”并链接 `24` §2；`AG:243` 同步 |
| P0-2 | `24:133-138`+`13:407`(ADR-047) vs `04:16,159-166`、`06:55-58`、`03:254` | 路由真源三说：manifest `[[routes]]` 静态真源 / `Register()` 唯一真源 / `routes()` 启动汇总 | 同一路由手写两遍；T0 无法生成 catalog；启动比对必失败或形同虚设；04/06 与已 accepted 的 ADR-047 矛盾 | 保留 `24` §4+ADR-047；`ServerPackage::routes()` 删除或改为读生成表；`04` §5 改“静态表+Register 校验”；`06` §2 规则 1 删除；`03:254` 改链接；`02:180` 契约表同步 |
| P0-3 | `24:90-93,138` vs `05:280,287-288` | IPC 命令真源：manifest `[[commands]]`+手写 `args_schema` vs 契约 crate schemars 单源且“不再手写 `[[commands]]`” | schema 两套；Ajv/TS 类型漂移；违反 C10；`05` 自身 280 与 287 也互相矛盾 | 保留 `24` 的声明（name/permission），`args_schema` 必须是生成路径；改 `05:287-288` 为“类型由契约生成，manifest 只声明身份/权限”，删“不再手写 `[[commands]]`”绝对句；`05:280` 链接 `24` §4 |
| P0-4 | `03:99` vs `24:167`、`13:436`(ADR-053)、`19:146`、`21:85` | profile 仍选 `store-session-jsonl`，而新规范冻结 SQLite WAL 唯一真源、JSONL 仅导入导出 | 照抄 profile 即违反 ADR-053；会话迁移/schema/崩溃恢复全部按错存储写 | 保留 `24` §7+ADR-053；`03:99` 改 `zeokeel/store-session-sqlite`；`05:61` H13 加链接；`19:146`/`21:85` 只链接 |
| P0-5 | `03:66`、`10:86`、`09:87` vs `24:20-21`+`13:395` | `exclude` 值写成角色名（`client.provider-settings`），`24` 要求包 id `<vendor>/<name>` 且未命中=error；`09:87` 深度 OEM 又用 `[packages]` 覆盖 | 企业版 compose 直接报错或排除不到；personal-only 断言失效；OEM 包绕过 channel 层校验 | 保留 `24` §1；`03`/`10` 全部改包 id；`09:87` 映射为 channel 层 `remove/add`；`editions/personal-only.toml` 为唯一清单，删 `03:77-79` 的第二套 `zk plan` 断言（改链接） |
| P0-6 | `24:142-146` vs `06:67-72`、`10:59,110`、`13:184-185` | 迁移版本：全局单调 `0001_`+`schema_migrations(package,version,checksum,applied_at)` vs 每包 `0001_init.sql`+`schema_migrations(version,name,checksum,applied_at,applied_by)` | 跨包重号；启动“重复版本”拒启；`06` 表缺 package 列，`24` 表缺 name/applied_by；`10` 示例与两者都不一致 | 保留 `24` §5（全局号+复合键）；`06` §3 改表结构并补 `name/applied_by`；`10:59` 示例改全局号；ADR-022 只留决策+链接 |
| P0-7 | `03:175-177` vs `03:168-169`、`24:175-177`、`02:156` | metadata 失败策略：03 说“可用则用，不可用则降级”，同文件 168 与 02/24 说 fail-closed | 契约双实例可漏检；POC 的“同名不同 path/git rev”仍能混过 | 03 拆两句：包索引只读 `zk.toml` 不需 metadata；契约单实例检查必须 metadata 且失败=非零；删“降级”；`24` §8 唯一 |
| P0-8 | `24:27-29` vs `03:84-85`、`09:16-17`、`16:237` | 入库基线：24 冻结 `personal×default×official`；03 的 `profiles/default.toml` 却写 `edition="enterprise"`；09 企业版也用该 profile；16 又用 `--edition personal` | `--check`/Cargo.lock 的“基线”不存在或随文档不同；无法判定提交哪份 | 保留 `24` §1.2；profile 不写 edition（由 CLI 决定）；或把基线改为 `enterprise×default×official` 并回写 24；二者只能选一 |
| P0-9 | `19:125-129` vs `05:67`、`21:25`、`AG:33`(C2) | `app-<app_id>` 每应用 capability：app id 是商店运行期数据，Tauri capability 是构建期静态文件 | 运行期安装任意应用无法生成对应 capability；只能退化成“所有 app 共享”或构建期白名单，ADR-050 的隔离承诺落空 | 改为单一 `app` capability+窗口 label/origin 白名单；删 `app-<app_id>`；`19` §7.1 表与 `21` §6 同步，ADR-050 补“按窗口类不按 app id” |
| P0-10 | `19:141` vs `05:54,172-174` | “关闭窗口默认最小化到托盘” vs 05 把“窗口关闭”当退出并拒绝审批、取消 turn | 审批挂起时窗口隐藏，用户以为已拒绝；关闭语义两义；契约测试无法写 | 保留 05 的退出语义；19 区分“关闭按钮=托盘”与“退出=quit”；`05:172/174` 改“退出应用”；补“窗口隐藏时审批仍通知+超时”测试 |
| P0-11 | `24:123-128` vs `10:105-107`、`03:171,254`、`13:398`(ADR-046) | `api-catalog/http-routes/permissions` writer：24 说 `zk api`，10 说 `zk compose`，03 说 `zk-srv-http` 汇总；`shared/ts` 10 说“ts-rs + zk compose”两个 writer | 一路径两 writer，`--check` 无法定义；生成物互相覆盖，违反 ADR-046 | 保留 `24` §3；`10` §4 整表改“见 24 §3”；`03:254` 改为运行期 Router 不写 catalog；`shared/ts` 只 ts-rs |

## 2. P1（口径漂移，实现前必须定）

| # | 文件:行 | 冲突/重复 | 影响 | 建议（保留/删除/链接） |
| --- | --- | --- | --- | --- |
| P1-1 | `05:297-299`、`16:230-231`、`19:131-136`、`21:84` | 本地 `serve` 安全同一机制四处规范；19 还新增“必需自定义头”“随机端口”，05 没有 | Host/Origin/token 校验口径漂移；安全测试按哪份写不确定 | 保留 `21` §6；`19` §7.2 删成一句“见 21 §6”，只留 UX 可见错误；05/16 只留“存在 serve 载体” |
| P1-2 | `19:135` vs `04:46,153` | 同名头 `X-ZeoKeel-Client`：04 用于服务端识别客户端版本/渠道；19 用于本地 serve 必需头 | 代理/网关按同一头做白名单会互斥；CORS 预检与日志字段含义混淆 | 本地载体改用 `X-ZeoKeel-Bridge` 或 `X-ZeoKeel-Local-Token`；`04` 保留原头，`19` 改新名并定义值语法 |
| P1-3 | `19:9-18` vs `05:93-100` | 19 的 Boot 只有“读常量→页面/门禁”，没有 `register_all`、逐包 `required` 判定；且“任何一步失败→ErrorScreen”与 `required=false` 记 degraded 继续启动冲突 | 包失败时 19 要求整页退出，05 要求降级 fallback；启动排障无统一入口 | 19 §1 增加“包注册/激活”阶段并链接 `05` §3.1/§3.4；删除“任何一步失败→ErrorScreen”的绝对句，改为按 `required` 分流 |
| P1-4 | `19:30-53` vs `16:83-88,132-137`、`02:131`、`01:75-80` | 19 用 `ui.chat`/`ui.shell`/`ui.apps` 当“槽位 id”，16 的 slot 是 `chat.main`/`settings.section`，02 的角色族是 `ui.slot.<位置>`；19 标题“必须冻结”但表头写“归属包（建议）”；19 个人版仍列 `ui.apps` | shell 按 19 的 slotId 找不到 `SLOT_DEFS`；个人版引用不存在的应用页；冻结/建议自相矛盾 | 保留 `16` §3 `SLOT_DEFS` 为 slot id 唯一源；19 表改为“页面/componentId/slotId/包 id/edition”五列，全部用真实 id；个人版只留本地技能/MCP，应用页标 enterprise |
| P1-5 | `24:41-63` vs `02:37-45` | 根 `[roles]` 手写 contract/cardinality/editions/protected，与每包 `[provides]` 的 contract/cardinality 重复 | 两处手写同一事实；换包时只改一处，validate 才发现；违反 SSOT | 保留 `24` 的角色注册表，但只放全局策略（`protected/editions`）；contract/cardinality 由包声明并生成 `roles.gen.json` 交叉校验；或删 `[provides]`，二者只能一个手写 |
| P1-6 | `24:22` vs `03:117-124`、`02:142`、`09:87` | 24 的 `[packages.add]/[packages.remove]` 与 03“没有第二组合通道，换包就是改 `[packages]`”、02“multi 不做单点覆盖”、09 的 oem `[packages]` 覆盖三套语义 | multi 角色增删无统一写法；OEM 深度包可能绕过 channel 层；组合器解析歧义 | 保留 `24` §1；03 §3 规则 3 重写为“add/remove 是唯一增删语义”；02 §4 同步；09 的 oem `[packages]` 明确映射到 channel 层 add/remove |
| P1-7 | `24:21` vs `03:77-79` | `editions/personal-only.toml` 清单 vs 03 用 `zk plan` 断言企业闭包不含 personal-only | 两套 CI 机制；新增个人专属包只改一处会漏 | 保留 `24` 的 `personal-only.toml`；03 删除断言句并链接；CI 只读该文件 |
| P1-8 | `24:153` vs `09:302`、`20:64-68`、`06:34` | 24 要求 SIGTERM 等 30s 结算/审计，compose 默认 stop grace 10s 即 SIGKILL；24 又要求启动取 PG advisory lock，而 `compose up -d` 可能旧实例未退完 | 升级时在途结算/审计 flush 被截断；新实例拿不到 lock 拒启或双实例 | `09`/`20` 的 compose 增加 `stop_grace_period: 45s`；advisory lock 改为有界等待+明确错误；升级流程写“先停旧再起新”并加演练 |
| P1-9 | `19:74-76` vs `01:114`、`04:62-63`、`09:127`、`13:365` | 19 在登录页放“修改地址”入口；01/04/09/ADR-044 都说地址只在设置中改、且需重登 | 用户可在登录页改地址绕过“需重登”；两处编辑路径漂移 | 保留 `09`/ADR-044；19 登录页只读展示+“去设置修改”，删除登录页编辑入口 |
| P1-10 | `19:62,103-108` vs `09:291-293` | 19 对个人版也写“下载/校验/安装/重启+回滚”；09 规定个人版只提示、用户手动下载安装包 | 个人版是否内置安装器未定；实现会多出一条静默升级路径 | 保留 `09`；19 §5 按 edition 拆：个人版=检查+提示+打开下载页，企业版=完整 FSM |
| P1-11 | `19:99,113` vs `11:146`、`18:56-88` | 19 的 axe 门槛是“无 serious”，11 是“无 serious/critical”；19 新增 `eslint-plugin-jsx-a11y`，18 的 ESLint 配置里没有；WCAG 2.2 AA 只在 19 出现 | 按 19 实现会放过 critical；jsx-a11y 不进门禁；a11y 标准无单一出处 | 保留 `11` 的 serious/critical 门禁；`18` §2 加 jsx-a11y；19 只留“WCAG 2.2 AA+组件要求”并链接 11/18 |
| P1-12 | `19:32-46,163`、`19:158-165`、`24:179-186` vs `11:56-89,144,155`、`AG:205` | 19 的每页状态不是统一六态；19/24 各自写验收清单，与 11 的通用门禁重复 | 页面可只测“正常/离线”就过；三份退出条件会漂移 | 保留 `11` 的六态与门禁；19 表加“六态映射”列；19/24 只留域内断言（审批卡、capability、首配）并链接 11 |
| P1-13 | `19:89,92` vs `02:63-65`、`01:117`、`05:262` | 19 新增 risk level（工具声明+规则计算），02 只有粗粒度 `[permissions].declares`；19 又禁止“全局永久允许”，01/05 说本地审批可配 | risk 无契约来源，审批卡无法计算；本地审批可配置的边界与 19 冲突 | risk 进 `zk-contract-tool` 的 `ToolSpec`；在 05 定义默认策略“无全局永久允许，仅会话/工具/路径”，并回写 01 的“本地审批可配”含义 |
| P1-14 | `19:125-127` vs `05:281`、`02:233` | 19 需要 `ipc_call` 之外的 `app_request`、`auth_*`；05 说 Tauri 侧“只有一个 `#[tauri::command] ipc_call`”；02 反模式禁止包里写 `#[tauri::command]` | 平台命令面未定义；包可能各自加 command；capability 白名单无法生成 | 保留 05 的“业务命令单入口”；在 05 §6 明确固定平台命令白名单（`ipc_call/app_request/auth_*`），02 反模式改为“包不得新增平台命令” |
| P1-15 | `24:29` vs `03:146`、`16:237` | 24 要求 `zk compose/api/db` 的 `--channel` 必填；03 的 CLI 是可选；16/AGENTS 示例都不写 | 按示例执行直接失败；CI 命令面不一致 | `compose/api` 必填 `--channel`，`db` 不需要（迁移与渠道无关）；03 的 CLI 表与 16 示例统一 |
| P1-16 | `24:142-146` vs `24:167` | 24 的全局 `zk db lock` 迁移编号没有说明是否覆盖客户端 SQLite 的 schema 迁移 | 客户端 store 迁移可能被误纳入 PG 全局号，或完全无编号规则 | 在 24 §5/§7 明确：PG 迁移走全局号；客户端 SQLite 由 store 包自带 `schema_version` 与顺序迁移，不使用 `zk db lock` |
| P1-17 | `19:114-115` vs `09:140` | 19 说缺词回退 `zh-CN`；09 说品牌键禁止回落、缺即失败，功能键才回退 | OEM 品牌缺键可能被 19 允许回落官方身份 | 保留 `09` §3；19 只写“功能键回退 `zh-CN`，`brand.*` 缺键构建失败”，并链接 09 |

## 3. P2（命名/摘要噪音，统一即可）

| # | 文件:行 | 冲突/重复 | 影响 | 建议 |
| --- | --- | --- | --- | --- |
| P2-1 | `19:11` vs `09:34` | 构建期常量名 `SERVER_URL_DEFAULT` vs `DEFAULT_SERVER_URL`，19 还少 `LOGIN_REQUIRED/PROVIDER_MODE/UPDATE_SOURCE` | 生成器常量表对不上 | 以 09 §1 的 6 个常量为准，19 只引用 |
| P2-2 | `19:52` vs `16:125-126`、`11:149` | “每个 `ui.gate.*` 必须 fallback”与 16 的 slot fallback、11 的 error boundary 覆盖率重复 | 三处各写一遍，验收口径分散 | 16/11 保留，19 只写“门禁页不得白屏”并链接 |
| P2-3 | `19:138-142` vs `21:86`、`09:121` | 深链白名单、托盘、通知在 19/21/09 三处 | 渠道 scheme 与安全规则漂移 | 21 管安全、09 管渠道字段，19 只写 UX 行为 |
| P2-4 | `19:152-156` vs `05:314-320` | 诊断页字段、崩溃报告、日志脱敏与 05 §8 重复 | 诊断面板两份字段表 | 05 §8 唯一，19 只写入口与用户动作 |
| P2-5 | `24:127` vs `18:104-129` | 24 说 `[workspace.lints]` 由 `zk compose` 在生成块内写；18 把它当手写政策配置 | 严格检查政策可能被生成器覆盖 | lints 保持手写（18/configs），从 24 §3 writer 表删除该行 |
| P2-6 | `24:176` vs `24:118-128` | `metadata.snapshot.json` 要求入库，但 §3 writer 表没有它的 writer | 违反“一路径一 writer”，无人生成/校验 | 在 §3 增加 writer=`zk lock --metadata-snapshot`，或改为 CI 缓存不入库 |
| P2-7 | `24:33` vs `16:168`、`13:396` | `resolve-report.json` vs `compose-report.json`，ADR-045 用前者 | CI 比对不存在的文件 | 统一为 `compose-report.json`，改 24/ADR-045 |
| P2-8 | `19:136` vs `16:252` | 19 要求启动随机端口；Playwright 用固定 `portFor(p)` | e2e 无法发现端口 | serve 写 `serve.json`（port/token），Playwright 读取；或 CI 固定端口 |
| P2-9 | `24:104-108` vs `06:45-48`、`10:59` | manifest 里的 `[jobs]`、`[migrations].dir` 与 `ServerPackage::jobs()/migrations()`、目录约定重复 | 同一作业/迁移两处声明 | manifest 只留元数据，`jobs/migrations` 由 trait 或生成表提供；删 `[jobs]` 占位示例 |

## 4. 必须重构的 10 条（按顺序）

1. **冻结 manifest v1**：`24` §2 补入 `02` 的 `config/permissions/compat/conformance/stability/ui.entry/assets`，删除 `02:28-72`，`02` 只留包模型与契约。
2. **冻结路由/命令静态真源**：以 `24` §4+ADR-047 为准，删 `06` 的 `routes()` 真源、`04` §5 的 `Register()` 真源、`02:180` 的 `routes()` 契约；`Register()` 只做比对。
3. **重写生成物 writer 表**：`24` §3 唯一；`10:105-107`、`03:171,254` 全部改链接；`shared/ts` 只归 ts-rs。
4. **统一 exclude 与 personal-only**：`03:66`、`10:86`、`09:87` 改包 id；`editions/personal-only.toml` 唯一；`09` 的 OEM `[packages]` 映射到 channel 层。
5. **切换会话存储**：`03:99` 改 `zeokeel/store-session-sqlite`；`05:61`、`19:146`、`21:85` 只链接 `24` §7+ADR-053。
6. **统一迁移命名**：`24` §5 全局号+复合键；`06` §3 补 `package/name/applied_by`；`10:59` 示例改全局号；ADR-022 只留决策。
7. **修 metadata 降级**：`03:175-177` 拆成“包索引不需 metadata”与“契约单实例必须 fail-closed”；删“不可用则降级”。
8. **解决基线/频道命令**：profile 不写 `edition`；`compose/api` 必填 `--channel`、`db` 不必填；回写 `03/09/16` 与 `24` §1.2。
9. **重写 19 §2**：以 `16` 的 `SLOT_DEFS` 为 slot id 唯一源，表改五列（页面/componentId/slotId/包 id/edition）；删 `ui.chat`/`ui.apps` 等自造 id，个人版去掉应用页。
10. **收紧 19 的安全/生命周期**：单一 `app` capability（不按 app id）；区分“关闭=托盘”与“退出=quit”；本地头改名；`serve`/地址/更新/i18n 全部改链接 `21/09/05`，不在 19 重写机制。

> 统计：P0 11 条、P1 17 条、P2 9 条。`24` 本身与 ADR-045/046/047/053 一致，P0-2/3/4/5/6/7/11 都是“新决策已下、旧文未回写”；`19` 的 P0-9/P0-10 是新增规范自身的设计错误，必须先改 19 再实现。
