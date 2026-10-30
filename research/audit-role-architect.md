# 架构可实现性审计（Architect / Senior Rust）

> 范围：AGENTS.md、README.md、docs/00–18、PLAN；重点 02/03/04/05/06/10/13/18，按需 07/09/11/12/16。只读审计，未改任何文档。
> 口径：**能否按文档无歧义地写出正确实现，并通过文档自己定义的验收**（不是"文本互相矛盾"）。`research/audit-consolidated.md:9-10` 的"P0/P1=0"是文本冲突口径，本审计结论不同。
> 行号：`NN:行` = `docs/NN-*.md`；`AGENTS:n` / `README:n` 为根文件。分组即级别；每条给出证据、缺口、后果、建议。

## 结论

**不能无歧义实现。** 五个系统级定义缺失：组合输入语义、`zk.toml` 真源 schema、路由真源时序、客户端数据层、钱/关闭语义；其余为域级缺口。现有验收门禁中至少 4 条按文档写法**不可执行**。

## P0（不补就无法写出正确代码 / 无法验收）

| # | 位置 | 歧义 / 缺口 | 后果 | 建议 |
| --- | --- | --- | --- | --- |
| P0-1 | `03:43-51,62-70,85-106,115-122`；`10:84-95`；`09:87` | 三层只写"叠加"；profile 重写 edition 已写的 `llm.provider.default`；"同 singleton 两次=error"与示例冲突；`minimal` 要删包（`03:131`）但 profile 只能覆盖 singleton（`02:142`）；channel 对 multi 是并集还是替换未定；`oem.toml [packages]` 无优先级 | 同一组输入可解析出多种包集合；企业版可能留本地 provider 或漏服务端包，C1/ADR-043 失效 | 冻结 precedence：层内重复=error；跨层只允许覆盖 singleton，multi 用显式 add/remove/replace；`exclude` 只作用 base；输出 resolve-report 进 T0 |
| P0-2 | `03:64`；`10:86`；`02:99`；`01:25` | `exclude` 值如 `client.provider-settings` 既非包 id（`<vendor>/<name>`）也非 02 §4 角色族；无"未命中即错"规则 | 静默不排除 → 企业版含本地 provider 设置/GitHub 更新器，违反"物理不存在"；ADR-043 闭包无法证明 | 只接受包 id；必须命中 base 闭包；personal-only 清单独立成文件；产物级断言（`cargo tree` + 字符串/符号扫描） |
| P0-3 | `02:28-72`；`03:343`；`10:65`；`13:173`；`ADR-010:86`；`05:280,287-288`；`08:53`；`03:197`；`05:96-97` | 真源示例缺 `required`、`[[commands]]`、`[jobs]`、`[editions]`、`swap_required`、`migrations`、`ui.contributions[].permission`；`05:287-288` 又禁止手写命令表，与 `05:280`/ADR-010 冲突 | `zk` 无法生成 glue；required/degraded 无输入；管理台权限无法校验；迁移清单无来源 | 冻结 manifest v1 JSON Schema，未知键=error；命令声明只保留"契约 schema 单通道" |
| P0-4 | `02:4,37-44,121-143`；`03:68,92`；`09:33`；`04:23` | 同一客户端认证角色出现 `auth.client` 与 `auth.provider`；基数由各提供者自行声明可冲突；无角色注册表/受保护角色表；`server-connector` 仍是"建议名" | typo/未知角色不可判定；swap 矩阵与 OEM 可覆盖范围不可枚举；"深度 OEM 禁碰契约/内核"无门禁 | 根 `zk.toml [roles]`：角色→cardinality/contract/editions/protected；提供者声明必须一致；未知角色=error；M0 冻结命名 |
| P0-5 | `06:44,53`；`04:164`；`03:168-169,359`；`10:105-106`；`11:10` | 路由在 Rust `routes()`/`Register()` 运行期产生，但 catalog/permissions 由 compose 构建前生成；T0 又要求"不编译、<10s"产出权限矩阵 | 生成物时序不成立；权限矩阵门禁无法前移；T0 目标假 | 路由声明进 manifest（`[[routes]]`）或 `route!` 宏导出静态元数据；运行期只做一致性校验；定义 path+method 冲突=拒启与中间件固定顺序 |
| P0-6 | `02:155-158`；`03:165-167 vs 173-175`；`13:413` | 一边"拿不到 `cargo metadata` 直接报错"，一边"metadata 只做交叉校验、不可用则降级" | 工作区损坏时恰好跳过契约单实例检查 → 两份契约副本进构建（延迟到 E0599 或 trait 不匹配） | 唯一规则：凡声称契约唯一性的命令必须有 metadata，失败=非零；离线用 metadata 快照；`--no-verify` 仅本地且 CI 禁用 |
| P0-7 | `10:3,101-113`；`03:280`；`18:188`；`03:147-148 vs 168`；`10:106`；`03:399`；`18:104-129`；`10:108` | 同一路径多写者：ts-rs/schemars 由 `cargo test` 写；`zk api` 与 `zk compose` 都写 api-catalog/openapi/permissions；根 `Cargo.toml [patch]` 由 zk 维护但 lints 手写；Dockerfile 有"生成段" | 非确定性 diff；手写段被覆盖或 `--check` 永远红；"只有 zk 能写"不成立 | 一路径一 writer；混合文件用 `@generated:begin/end` 块；`--check`=生成到临时目录逐字节比较、永不写；ts-rs 唯一命令=`cargo test export_bindings` + `git diff` |
| P0-8 | `03:297-303,322`；`09:34`；`03:144`；`AGENTS:177-178` | 只提交 `default` profile，但 personal 与 enterprise 都用 profile `default` 且产物不同；drift job 不带 edition/channel；`--channel` 可选却总是生成 `CHANNEL_ID`/`DEFAULT_SERVER_URL` 常量 | 两 edition 的 `--check` 必有一个红；渠道常量来源不确定 | 固定入库基线元组 personal×official×default；`--channel` 在 compose 必填；头部写 `@edition/@profile/@channel` |
| P0-9 | `06:65-70`；`03:249`；`10:110` | 每包 `0001_init.sql`，但 `schema_migrations(version,...)` 以 version 为键；聚合"按包顺序"随包集合变化 | 版本碰撞/历史不可重现；第二个 0001 被当作已应用 | 全局单调版本，或 `(package,version)` 复合键 + 固定拓扑序；`zk db lock` 唯一分配；启动检测重复与顺序漂移 |
| P0-10 | `07:114-126`；`13:276-282`；`06:74` | 写"预授权估算"，但表只有 balance+ledger，无 hold/reservation；准入 `balance>0` 与结算分离 | N 个并发请求全部通过 → 余额无界为负；"硬阻断"失效 | 定义原子条件 UPDATE 或 hold 表；结算 diff/释放；流式中断对账作业；并发测试进 T3 |
| P0-11 | `06:7-26,32,59,115` | 只有启动序；无 SIGTERM 状态机（停 accept→drain SSE→结算→flush audit→停 job→关池）、无超时、无 PG advisory lock、无 pending 结算补偿 | 流中丢结算或双扣；两个实例并发跑月发放/退役 | 冻结 shutdown FSM 与时限；pending 结算表+启动补偿；启动取 advisory lock，拿不到=拒启 |
| P0-12 | `05:61,173`；`12:72`；`ADR-014:115-118`；`05:29` | "SQLite/JSONL"未二选一；无 schema/事务/原子写/多窗口锁/崩溃恢复；只说"部分输出落库并标记 interrupted" | 崩溃/取消致会话损坏或半条消息；无法做恢复测试；个人版数据丢失 | 定 SQLite WAL 为唯一会话存储 + schema v1 + 迁移；JSONL 仅导入导出；定义 turn 原子提交、`interrupted` 恢复、checkpoint 周期、单实例锁 |

## P1（会做出错误实现或必然返工）

| # | 位置 | 歧义 / 缺口 | 后果 | 建议 |
| --- | --- | --- | --- | --- |
| P1-1 | `04:229`；`07:209`；`15:69`；`ADR-035:292` | `db.exec` 只许 SELECT/INSERT/UPDATE/DELETE 且禁 DDL，`db.define/tx_*` 明确不存在；但要求每应用 16 表/100MB | 应用无法建表 → 每应用 DB 不可用；或应用偷 DDL → 沙箱洞 | 应用 release 内 `schema.sql` 由宿主迁移，或 ABI v2 `db.define`；明确 100MB/16 表/5s 的执行点；补 schema 升级与备份 |
| P1-2 | `04:38,189,79`；`07:150` | "错误码封闭集合"无 enum 真源/注册表；`errors.json` 生成者未指定；`/v1/*` 用统一信封还是 OpenAI 错误形状未定；SSE error 帧无 schema | SDK 兼容性与客户端分支都不确定；新错误码可绕过封闭集 | 单一 enum crate 生成 errors.json；明确 `/v1/*` 错误形状；定义 SSE `error` 帧字段 |
| P1-3 | `02:165-190`；`04:17`；`07:229`；`08:75` | 契约清单不一致：02 有 `admin-ui` 但无 `zk-contract-events`/`content`/`admin-api`，08 同时用 `admin-api` 与 `admin-ui` | `[contracts]` 双向校验无法建立；缺 crate 的契约 id 会在 profile 中悬空 | 以 02 为唯一清单，补齐 events/content/admin-api 并删除别名 |
| P1-4 | `02:17,70`；`03:326`；`05:243`；`18:151` | conformance：suite id→crate 无映射；UI facet/OIDC/LDAP/gateway 的 fixture 未定义；要求跨语言"字节一致"（JSON 键序/浮点/转义不可保证） | "每个实现跑同一套"不可执行；字节门禁必然误报 | suite registry + 每 facet harness；跨语言改语义等价；OIDC/LDAP 用可编程 fixture |
| P1-5 | `03:150,75-77`；`09:35`；`ADR-043:374` | `zk plan` 无输出 schema/退出码/断言语义；CI 闭包断言依赖它，"被覆盖角色"集合未定义 | C1 的机械证明无法实现 | 定义 `--json` schema 与 `--assert` 子命令；显式输出 removed/overridden/added 三类 |
| P1-6 | `03:108-110`；`09:79`；`06:90-93` | `[settings]`/`config/defaults.toml`/`system_settings`/`channel.json.defaults` 四套配置真源与优先级未定，类型与 hot/restart 边界不清 | 同一配置多处写、互相覆盖；OEM 默认值不确定 | 一张 key→owner→scope→hot 表；由契约 schema 生成消费端 |
| P1-7 | `02:169`；`03:194-205`；`05:276` | `Package::install() -> Installed` 无 ctx，`reg.install(plugin,cfg)` 又出现；"包不拿 Context"却要 `ctx.command`/注册 service/effect | 包安装与资源归属 API 写不出来；effect 回收无法保证 | 冻结 `Installed`/`Registrar`/effect API 签名；明确 ctx 只在 apply 内可见 |
| P1-8 | `05:97`；`03:209`；`06:60`；`ADR-009:78-82` | "诊断页"无归属包，且要求在 0 包激活时可用；bootstrap 又被禁止含业务 | UI 全失败时无人渲染诊断页 | 独立最小诊断产物（静态页），不参与组合、不依赖契约 |
| P1-9 | `03:222-225`；`05:98` | `required/degraded` 只定义启动策略；未定义降级对 UI/角色/健康上报的可见性、是否允许重试 | 用户看到"不可用"但无原因/无恢复入口 | 定义 StartupReport→UI 映射、diagnostics 字段、重试入口 |
| P1-10 | `05:166-177` | turn/审批：排队 turn 无状态；排队取消、审批中崩溃、关窗拒绝后的恢复未定义；工具批"已开始/未开始"语义模糊；10min 超时无配置键 | M1 核心交互会出现不可测试的边界态 | 转换表 + 持久化 + 每事件点取消/重启测试 |
| P1-11 | `05:29,297-299`；`04:222`；`09:121` | 多窗口/单实例/深链/每应用窗口生命周期未定义；二次启动、同会话双窗、窗口崩溃重开未定；serve token 生命周期只散文 | 数据竞争与重复窗口；深链落到错误会话 | 窗口 FSM + 单实例锁 + 深链路由表；token 一次性+过期规格 |
| P1-12 | `04:164` vs `06:54` | `RouteContribution` 字段两套：06 缺 audit/rate-limit/step-up；路由冲突与中间件顺序未定义 | 权限矩阵/审计门禁漏项；鉴权与限流先后不定 | 单一 RouteContribution schema；固定中间件顺序；冲突=拒启 |
| P1-13 | `06:59,99-106`；`07:132` | 作业无 lease/retry/跳过已成功/漏月补发语义；`job_runs(job,key)` 唯一性未定；关停时在跑作业未定 | 月度发放漏发或重发；退役作业重复删 | 作业状态机 + 补跑规则（按月逐 key）+ 关停取消 |
| P1-14 | `06:73`；`07:220` | `usage`/`audit_log` 按月分区，但作业表无分区维护；审计在业务事务内 fail-closed | 缺下月分区时所有写操作失败（审计写失败=业务失败） | 预建分区作业 + 启动检查；缺分区=启动告警而非运行时爆炸 |
| P1-15 | `06:115`；`09:253` | 旧二进制遇到新 `schema_migrations` 的行为未定义；"回滚只回镜像"与"跨迁移边界回 DB"的判定条件未定 | 回滚后可能以旧代码跑新 schema | 启动断言 schema 版本==期望，否则拒启；给出回滚矩阵 |
| P1-16 | `06:114`；`09:250-257`；`07:199` | PG + `/data`（含 app SQLite）+ `master.key` 的时点一致性、在线备份方式、恢复顺序未定义 | 恢复后 app DB 与元数据不一致；密钥丢失即不可解密 | 定义在线备份顺序（PG dump → quiesce/快照 → app DB）、校验清单、恢复演练脚本 |
| P1-17 | `05:297-299`；`ADR-010:91` | serve 载体 token/Origin/CSRF 只散文；"平台包可保留少量原生命令"白名单未列出 | 无法写安全测试；单入口承诺有未知例外 | 列白名单与校验矩阵；补 CSRF/Origin 用例 |
| P1-18 | `12:105`；`11:69` | "物理不存在"与"不能绕过网关"只有文字，无二进制/网络级断言 | 企业版裁剪与网关边界不可证 | `cargo tree`/符号/字符串断言 + 网络拦截 e2e |
| P1-19 | `README:41`；`03:343`；`11:14` | "8 个 swap 角色"从未列出；`swap_required` 不在 manifest | T3/T4 矩阵不可枚举 | 在 `[roles]` 标注 8 个受测角色并生成矩阵 |
| P1-20 | `04:154`；`09:313` | 426 只豁免稳定面/登录/刷新；`/v1/*` 是否属于"业务面"未定义；与"本地 agent 可短暂保留"不一致 | 升级前 AI 请求全 426，宽限承诺落空 | 明确豁免面（含 `/v1/*`）与宽限截止规则 |
| P1-21 | `02:87,153-154,220`；`18:154,185`；`12:51` | 契约 semver/public-api baseline 存放/审批/双 major 迁移窗口未定；`kind=contract` 要求 M0 即发布又与 M0 最小集冲突 | 契约破坏性变更无流程；M0 范围膨胀 | baseline 入库；semver 决策表；M0 用 workspace 单实例、发布推迟到 S0-P1 结论 |

## P2（债 / 成本 / 需明确取舍）

| # | 位置 | 问题 | 建议 |
| --- | --- | --- | --- |
| P2-1 | `05:41-42`；`02:235` | 进程级 `BlockingExecutor` "先装者赢"与"禁止全局 OnceCell 单例"反模式冲突；`cargo test` 单进程下顺序依赖 | 改为显式注入 + 重复安装=error；测试用 nextest 进程隔离写进规范 |
| P2-2 | `03:399-401`；`ADR-016:134-139` | "无论 crates.io/git/vendor 都链接同一实例"技术上不成立：`[patch]` 只对 registry 源生效，git 需同 URL+rev 的 patch | 降级表述：M0 只承诺 vendor+registry；git 源写 `[patch."<url>"]` 并验证 |
| P2-3 | `10:113`；`09:201` | 生成确定性只写"按 id 排序、无时间戳"；JSON 键序/换行/相对路径/生成器版本未定；无两次生成逐字节比对 | 固定格式化器与路径相对化；生成器版本进头；加 determinism 测试 |
| P2-4 | `03:139-156`；`13:400`；`12:45-63`；`README:34` | `zk` 已有 17+ 命令（channel/brand/docs/sbom 等），预算 <8k 行，但 M0 后无 owner/排期 | 砍到 M0 必需 5 命令；其余延后并给预算 |
| P2-5 | `12:28-39,86-108`；`13:414` | 剩余 5 个 S0 spike ×2d 恰好占满 2 人周、无集成缓冲；M2 6 周要 20–30 包+3 认证源+计费+管理台，与 R17 教训冲突 | 给出 spike 并行与缓冲；M2 拆成 2a 认证/钱包/网关、2b 商店/管理台 |
| P2-6 | `18:154,185` | `cargo public-api diff` 与 `cargo-semver-checks` 二选一未定，baseline 未定 | 固定 semver-checks + 入库 baseline |
| P2-7 | `02:220` | 0.x 包"minor 可破坏"与 `zk.lock` 可复现、第三方 vendor 兼容承诺冲突 | 明确 0.x 锁定策略与升级窗口 |
| P2-8 | `13:107-111`；`03:452` | ADR-013（`host.wasm` 工具包）与 ADR-035（应用 WASM）共用 wasm 命名空间，边界只靠一句别名 | 在 ADR-013 加"仅客户端工具 wasm，与应用中心无关" |
| P2-9 | `08:118-122` | 管理台主题是否复用客户端、OEM 是否自建前端仍未定 | 定"共享 token 包 + 我方代构建"为默认 |
| P2-10 | `06:71`；`10:119` | `docs/generated/schema.md` 的生成输入未定义；sqlx 编译期查询需要活 PG/`.sqlx` 缓存，构建文档未提 | 用 migrate+离线元数据生成；CI 加 `sqlx prepare --check` |
| P2-11 | `18:98` | 金额 2^53 边界只写"应改字符串传输"，未定何时改 | 从现在起用量/余额对外用字符串或 int64 十进制 |

## 过度设计 / 欠设计 / 排期

- **过度**：M0/M1 就引入 8 种 kind、22 个契约、3 层组合 + OEM 索引仓 + channel + brand 派生 + i18n catalog + per-profile 锁重解析 + 17 命令 `zk`；这些机制成本大于首个可交付客户端的收益。T3 swap 矩阵在产品代码为 0 时无法标定。
- **欠设计**：manifest schema、路由声明、客户端持久化、钱包并发、关闭/作业、迁移命名空间、WASM 应用 DB 建表、审批持久化、网关错误契约、conformance harness——全是"实现第一天就会撞上"的部分。
- **排期**：35 人周覆盖全平台明显偏乐观；`zk` 在 M0 之后持续长胖却无工时；S0 无缓冲；M2 的 6 周同时包含三认证源、计费正确性、管理台与 Docker，风险最高。计划里没有"补本审计 P0"的工时。

## 可测试性（T0–T5）

| 层 | 文档断言 | 可执行性 | 缺口 |
| --- | --- | --- | --- |
| T0 | 组合/基数/闭包/生成物 diff/负向（`03:352-359`） | 组合与负向可执行；路由/权限矩阵**不可**（P0-5） | 路由真源时序；`swap_required` 无字段 |
| T1 | `cargo check -p` + 本包 conformance（`03:355`） | 部分可执行 | conformance suite 无 registry（P1-4） |
| T2 | 构建 + 启动自检 + 冒烟（`03:356`） | 可构建；"required 服务就位"无清单 | 降级语义未定义（P1-9） |
| T3 | 真 PG + 真 IPC（`11:13`） | 主流程可；钱/流式/关闭**无规格**（P0-10/11） | 并发、关停、对账用例缺失 |
| T4 | 8 swap 角色 × 替代实现（`11:14`） | **不可枚举**（P1-19）；T0 前提不成立 | OEM 渠道 fixture 规格缺失 |
| T5 | 安装包/镜像/升级回滚（`11:15`） | 依赖未就绪证书（ADR-037）；回滚判定未定义（P1-15） | 只能 skip，不能算证据 |

判定：6 层中 T0/T4 的关键断言不可执行，T3 的钱与关闭无验收；按 `11:52` 的"完成判据"，当前文档不足以证明平台可交付。

## 必须补齐的 10 条设计细节（按依赖顺序）

1. **冻结 `zk.toml` / edition / profile / channel / oem 的 v1 JSON Schema 与角色注册表**；未知键、未知角色、未命中的 `exclude` 一律 error；给出唯一 precedence 表与删除/追加/覆盖三种操作。
2. **把路由与权限声明前移为静态输入**（manifest `[[routes]]` 或宏导出），`zk compose` 可离线生成 catalog；定义冲突、中间件顺序、启动一致性校验。
3. **出生成物 ownership matrix**：每路径一个 writer，混合文件用生成块，`--check` 只读比较；固定入库元组 personal×official×default 并让 `--channel` 必填。
4. **定客户端持久化**：SQLite WAL + schema v1 + 迁移 + turn 原子提交 + `interrupted` 恢复 + 单实例锁；JSONL 仅导出。
5. **定钱包准入状态机**：hold/reservation 表或原子条件更新、结算 diff、流式中断对账、负余额上限，配 T3 并发测试。
6. **定服务端生命周期**：SIGTERM 关闭 FSM 与时限、SSE drain、结算/审计 flush、advisory lock、pending 结算启动补偿。
7. **定迁移命名空间与分区运维**：全局单调版本或复合键、固定聚合序、启动漂移检测、预建分区作业。
8. **定 WASM 应用 DB 供给**：release 内 `schema.sql` 由宿主迁移（或 ABI v2 `db.define`），并明确 100MB/16 表/5s 的执行点与备份归属。
9. **定 turn/审批/窗口状态机**：转换表、排队与排队取消、审批跨重启、工具批部分语义、多窗口/深链路由。
10. **定契约治理**：补 events/content/admin-api 契约、conformance suite registry 与每 facet harness、错误码 enum 与 `/v1/*`/SSE 错误形状、public-api baseline 与 semver 决策表。
