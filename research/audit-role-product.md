# 产品/交付视角审计（只读）

> 审计对象：`/data/ZeoKeel` Plan v3.2（当前只有文档/研究/实验，无产品代码）。时间：2026-10-30。
> 立场：产品负责人/交付负责人；只判断“设计是否完整、能否交付、能否验收、每个细节是否想清楚”，不重复找错别字与已修冲突。
> 范围：`AGENTS.md`、`README.md`、`docs/PLAN.md`、`docs/00/01/02/09/12/15/17`；按需对照 `03/04/05/07/08/11/13`。
> 结论：**P0 3 项、P1 13 项、P2 14 项**。版本模型（个人版基线、企业版追加、official/OEM 渠道、更新通道、WASM 后置）方向清楚；主要缺口在“开通/首配的可执行步骤、OEM 交付责任与验收、更新与运维的异常路径、量化的验收标准”。
> P0 判定口径：没有该流程/文档，承诺的角色或交付旅程就无法完成或无法验收；不是代码 bug 级 P0，也不等同于前序审计里的文档冲突。

## 结论先说

- **个人版方向可交付**，但“首次安装→配 provider→第一次对话”没有产品级流程与验收，只有包名和零散能力描述。
- **企业版能装起来，不等于员工能用起来**：从 `docker load` 到“第一轮对话并扣费”之间缺一段开通链路（provider/模型/价格/用户/自检）。
- **OEM 是收入交付物，但缺客户可执行流程**：只有我方内部构建流水线，没有客户接入清单、责任矩阵、UAT/签收与升级责任。
- 最大风险不是架构，而是**多处“已定”只到方向，未到默认值、失败提示、空态、配额、权限与验收脚本**；这会让排期与“完成”都不可验证。

## 审计维度覆盖

| 维度 | 对应条目 |
| --- | --- |
| 1 用户旅程 | P0-1/2/3；P1-1/2/4/9/10/11；P2-3/5/12 |
| 2 版本与范围 | P1-5/6/7；P2-7/8/9 |
| 3 OEM/渠道 | P0-3；P1-1/3/5/6/7；P2-9/10/11 |
| 4 更新与运维 | P1-2/3/13；P2-3/6 |
| 5 完整性（验收/错误/空态/配额/权限/保留/提示） | P0-1/2；P1-8/9/10/11；P2-1/2/3/4/5/12/13/14 |
| 6 竞品/参考实现 | 见“竞品对照与取舍” |
| 7 排期/人周/包数 | P1-12；P2-14 |

## P0（阻断承诺的旅程或验收）

| # | 位置（文件:行） | 问题/缺口 | 影响 | 建议 |
| --- | --- | --- | --- | --- |
| P0-1 | `docs/07-server-domains.md:81-82`；`docs/06-server-runtime.md:82`；`docs/08-admin-console.md:111`；`docs/00-design-principles.md:51`；`docs/04-contracts-and-api.md:159` | 管理台登录只写“仅本地超管”，但审计员被定义为可登录管理台只读；审计员用哪种身份（本地/OIDC/LDAP）、谁创建、如何映射角色、首登改密均未定义。 | 审计员旅程走不通；M2“管理台核心”和 M3“审计可查”无法验收；合规角色形同虚设。 | 定义管理台认证源与角色映射：至少支持“本地 auditor 账号”；OIDC/LDAP 组→角色作为可选；补登录/首登/会话/审计用例，并同步 04/06/07/08 的权限矩阵。 |
| P0-2 | `docs/09-delivery-and-oem.md:225-241`；`docs/06-server-runtime.md:90`；`docs/07-server-domains.md:144-148`；`docs/12-roadmap.md:102`；`docs/01-vision-and-editions.md:50` | 从 `docker load` + `compose up` 到“员工完成一轮对话并扣费”之间没有开通顺序与验收：上游 provider/key、模型目录、默认模型、价格/峰谷、月发放、首个管理员、用户创建、连通性自检均无步骤/默认值。`.env` 只有 6 个键，provider 配置在运行期。 | 新环境无法按文档完成首配；M2 退出条件不可执行/不可验收；客户现场依赖口头支持，排障无基线。 | 增加“Day-0 开通”章节或首启向导：必填项、默认模型与价格、连通性测试、失败提示、幂等初始化、验收脚本；把 M2 退出条件拆成可执行步骤。 |
| P0-3 | `docs/09-delivery-and-oem.md:171-179`；`docs/09-delivery-and-oem.md:196`；`docs/09-delivery-and-oem.md:243-244`；`docs/09-delivery-and-oem.md:335`；`docs/12-roadmap.md:168` | OEM 只有内部构建流水线（建目录→submodule→构建→发布）；没有客户接入清单（域名/DNS/TLS/IdP 元数据/法务主体/logo/支持联系人）、RACI（谁提供、谁审批、谁部署、谁升级）、UAT/验收标准、签收与交接包清单。 | OEM 交付不可复制；责任边界与延期争议；客户不知道自己要做什么；M5“新增客户只需建目录/跑流水线”只在数据已齐时成立。 | 补一页“客户接入→验收”流程：intake 表单、责任矩阵、验收 checklist（品牌/登录/计费/更新/回滚）、交接包（CHANNEL/VERSION/SBOM/SHA256/凭据/回滚包）与升级/支持责任条款。 |

## P1（严重缺口，影响交付质量/节奏/商业边界）

| # | 位置（文件:行） | 问题/缺口 | 影响 | 建议 |
| --- | --- | --- | --- | --- |
| P1-1 | `docs/09-delivery-and-oem.md:127`；`docs/09-delivery-and-oem.md:159`；`docs/13-risks-and-adr.md:361`；`docs/01-vision-and-editions.md:113-114` | OEM 的 `server_url` 非空即锁定、地址不可编辑，且“同渠道跨版本不得变”是构建红线；但没有域名迁移、服务端重建、证书轮换后的恢复通道，客户端只认旧地址。 | 客户换域名/迁移机房后全员无法登录且无法自更新，只能换包装机；OEM 运维事故。 | 定义签名“服务器迁移”机制（旧地址返回迁移指令/一次性重定向）或受审计的支持重置入口；把“不可变”改为“默认不可变 + 受控迁移流程”。 |
| P1-2 | `docs/09-delivery-and-oem.md:313`；`docs/04-contracts-and-api.md:255`；`docs/04-contracts-and-api.md:155`；`docs/09-delivery-and-oem.md:314`；`docs/09-delivery-and-oem.md:325` | 强制更新“宽限待确认”；`deadline/forced` 与离线宽限、未保存工作、安装权限、磁盘不足、启动失败回滚、车队回退均无策略；04 写“按用户比例灰度”，09 又写 1:1 单版本。 | 强制更新可能阻断工作或产生砖机；灰度不可实现；无法写验收用例。 | 定义一条更新状态机（提示/截止/强制/离线宽限）+ 每平台安装前置条件 + 失败回滚/启动保护 + 车队回退窗口 + 统一灰度口径（渠道或用户组）。 |
| P1-3 | `docs/09-delivery-and-oem.md:297`；`docs/09-delivery-and-oem.md:356`；`docs/09-delivery-and-oem.md:298`；`docs/09-delivery-and-oem.md:300` | 一处说服务端更新面“客户自建”，一处说“我方托管私有面”；没有凭据/访问/上传/保留/SLA/谁批准升级的说明。 | 客户拿不到镜像包；OEM 升级断链；托管与支持责任不清。 | 按渠道定义托管模式（我方/OEM/客户），给出获取/校验/上传/保留流程与凭据生命周期；OEM 合同写明升级责任与支持窗口。 |
| P1-4 | `docs/09-delivery-and-oem.md:205`；`docs/13-risks-and-adr.md:207`；`docs/11-testing-and-evidence.md:47`；`docs/07-server-domains.md:144`；`docs/09-delivery-and-oem.md:26` | 文档声称“客户侧零外网依赖/内网离线”，但企业 AI 必须由服务端出网到上游；没有出网/代理/自定义 CA/私有 OpenAI 兼容端点的前置条件。 | 隔离网客户首次对话即失败；验收争议；排障没有网络基线。 | 发布网络前置矩阵（服务端 egress、proxy、CA、DNS、端口），支持内网兼容端点；把“零外网依赖”改为“安装包/运行时数据零外网，AI 上游需出网或内网端点”。 |
| P1-5 | `docs/04-contracts-and-api.md:181`；`docs/09-delivery-and-oem.md:44`；`docs/09-delivery-and-oem.md:134`；`docs/13-risks-and-adr.md:200` | 04 说品牌/渠道“服务端优先，登录后服务端可覆盖”；09/ADR-024 说构建期固定、线上不可改。两套真源。 | OEM 白标可被服务端运行时改写，破坏渠道身份与“品牌不可线上篡改”承诺；安全/审计边界模糊。 | 明确 channel 身份（slug/app_id/scheme/server_url/品牌）构建期不可变；服务端只可下发同渠道内容/主题变量；删除“可覆盖品牌”表述或限定为缓存。 |
| P1-6 | `docs/01-vision-and-editions.md:43`；`docs/01-vision-and-editions.md:101`；`docs/09-delivery-and-oem.md:18`；`docs/09-delivery-and-oem.md:340`；`docs/01-vision-and-editions.md:104-105`；`docs/09-delivery-and-oem.md:351` | OEM 许可一处“客户合约”、一处“MIT（白标服务另计）”；“Powered by”/商标措辞待法务；深度 OEM 闭源赔偿另约。 | 无法报价/签约；MIT 义务与闭源产物边界不清，存在商业风险。 | 明确 OEM 商业模式（MIT 核心 + 品牌/服务合约，或商业许可）、商标/Powered by 文本、闭源代码的许可与赔偿条款。 |
| P1-7 | `docs/09-delivery-and-oem.md:99`；`docs/09-delivery-and-oem.md:351`；`docs/16-ui-packages.md:19` | 深度 OEM 代码包要求“声明契约 + conformance + 许可证 + 来源”，但又接受“闭源预构建产物”；无源码时 conformance/换包矩阵/许可证审计如何执行未定义。 | 门禁自相矛盾；供应链与责任不可控；交付延期。 | 二选一：NDA 源码 + 同 CI 门禁；或定义二进制准入（SBOM、来源证明、独立 conformance harness、escrow、责任条款）。 |
| P1-8 | `docs/04-contracts-and-api.md:5`；`docs/04-contracts-and-api.md:60`；`docs/04-contracts-and-api.md:111`；`docs/04-contracts-and-api.md:129`；`docs/04-contracts-and-api.md:143`；`docs/07-server-domains.md:148`；`docs/08-admin-console.md:46` | 管理端 API 反复写“另文”，实际只有 provider/model/usage/credit 等零散端点；users/skills/connectors/apps/settings/audit 的路径、权限、幂等、审计、错误未定义。 | 管理台无法按契约实现/测试；前后端联调靠猜；工作量与验收不可估。 | 发布 admin API catalog（或由同一路由表生成并纳入 `http-routes.json`），补全 CRUD/审批/设置/审计导出与权限矩阵。 |
| P1-9 | `docs/07-server-domains.md:95`；`docs/07-server-domains.md:102`；`docs/04-contracts-and-api.md:52`；`docs/06-server-runtime.md:82` | 本地源用户如何创建/邀请/初始密码/改密/忘记密码/禁用/删除/内容转交未定义；无 SMTP；`password_must_change` 只有字段。 | 本地认证客户无法自助入职/找回密码；离职内容归属不清；支持成本高。 | 定义 create/invite/reset/disable/delete + 密码策略 + 内容转交/下架 + 审计与通知；或明确只支持 OIDC/LDAP 并删除本地用户管理承诺。 |
| P1-10 | `docs/01-vision-and-editions.md:20`；`docs/03-composition-and-ci.md:64`；`docs/10-repo-and-build.md:86`；`docs/09-delivery-and-oem.md:265`；`docs/12-roadmap.md:73`；`docs/12-roadmap.md:77-82` | “个人引导”只作为被排除包名出现，无角色/页面/步骤；首次启动如何选 provider、填 key/base_url、拉模型、测试连接、错误提示/空态均未定义；M1 退出条件不覆盖。 | 个人版首次旅程断点；用户卡在配置；支持成本；M1 验收不完整。 | 定义 onboarding 流程与状态（无 provider/校验中/成功/失败/无 keychain），provider 测试连接、模型发现、错误文案；加入 M1 可执行验收。 |
| P1-11 | `docs/07-server-domains.md:155`；`docs/07-server-domains.md:173`；`docs/07-server-domains.md:174`；`docs/07-server-domains.md:266`；`docs/12-roadmap.md:150` | 技能配额 50/5GB 仍“推荐待确认”；不扫描的风险声明缺失；应用下架时已打开窗口行为写“符合定义”但无定义；技能超限（16MB/数量）的用户提示未定义。 | 商店/应用的错误路径和验收缺失；企业内容治理承诺模糊。 | 定稿配额与超限错误/提示；写下架宽限/关闭策略；发布“不扫描、客户自担内容风险”声明；补技能超限 UX。 |
| P1-12 | `docs/12-roadmap.md:11`；`docs/12-roadmap.md:23-24`；`docs/12-roadmap.md:30`；`docs/12-roadmap.md:20`；`docs/12-roadmap.md:91`；`docs/PLAN.md:95`；`docs/PLAN.md:121`；`docs/12-roadmap.md:65/86/112/134/156/180` | S0 2 人周=10 天；即便 S0 的 WASM spike 已完成，剩余 5 个 spike 各≤2 天=10 天，还没算 T0–T5 标定（按全 6 个算则最多 12 天）；35 人周对应 64–74 个包 + 双端 + 三平台 + OEM + 更新，无 QA/设计/DevOps/法务/文档角色与缓冲；M1 6 周/14 包、M2 6 周/20–30 包明显偏乐观。 | 排期不可信；团队按此承诺会延期；质量/安全/文档被挤压。 | 重做 WBS + 角色与并行度，单列 QA/设计/DevOps/法务；S0 加到 ≥3 周或砍 spike；给 30–50% 缓冲；把 M1/M2 拆成“可演示 MVP”与 GA 两个验收点。 |
| P1-13 | `docs/06-server-runtime.md:32`；`docs/07-server-domains.md:39`；`docs/09-delivery-and-oem.md:256`；`docs/12-roadmap.md:182-187` | 明确单实例无 HA，但没有对外 SLA/支持等级/响应时间/维护窗口；备份只写“N 份”，无 RPO/RTO、告警、日志轮转、恢复演练脚本；无监控告警产品化。 | 企业采购/运维验收无法通过；故障时无承诺；磁盘/备份故障不可见。 | 发布支持矩阵与 SLA（或明确“无 SLA/社区支持”）；定义 RPO/RTO/保留份数、监控告警、日志轮转、恢复 runbook 与演练证据。 |

## P2（重要但不阻断，需明确取舍/后补）

| # | 位置（文件:行） | 问题/缺口 | 影响 | 建议 |
| --- | --- | --- | --- | --- |
| P2-1 | `docs/00-design-principles.md:51`；`docs/06-server-runtime.md:82`；`docs/04-contracts-and-api.md:159`；`docs/08-admin-console.md:92` | 管理台只有超管写 + 审计员读，无 billing/user/settings 分权；MFA 可选，未要求超管强制。 | 所有 IT 共用一个全能账号；内部人风险与合规问题。 | 增加角色/权限拆分或明确接受；超管强制 MFA，保留 break-glass 并审计。 |
| P2-2 | `docs/07-server-domains.md:95`；`docs/09-delivery-and-oem.md:133`；`docs/08-admin-console.md:99`；`docs/04-contracts-and-api.md:215` | 无 SCIM/组→角色映射；无 SMTP（只有“邮件文案”）；审计导出仅有“导出也审计”一句，无格式/范围/SIEM 推送；通知只有站内事件。 | 常见企业采购项缺失；用户管理手工；告警不可运营。 | 写明“故意不做/后补”；至少 SMTP、审计 CSV/JSON/Syslog、组映射路线图。 |
| P2-3 | `docs/09-delivery-and-oem.md:243-244`；`docs/08-admin-console.md:105`；`docs/09-delivery-and-oem.md:314`；`docs/11-testing-and-evidence.md:46` | 门户只说列安装包 + SHA + 安装说明；无 OS 矩阵、SmartScreen/Gatekeeper 步骤、按用户/按机器与 UAC、AppImage 权限、校验和验证方法。 | 首次安装失败与支持量；Windows 静默更新可能因权限失败。 | 门户增加安装指南/系统要求/每平台前置条件/截图命令；纳入安装验收。 |
| P2-4 | `docs/07-server-domains.md:221`；`docs/07-server-domains.md:102`；`docs/09-delivery-and-oem.md:256`；`docs/09-delivery-and-oem.md:339` | 服务端审计/用量保留已定义，但客户端本地数据保留/导出/删除、离职/注销、DPA/数据地图、用户数据处理告知缺失。 | 合规/离职交接/隐私评审缺口。 | 做数据地图与保留矩阵；用户导出/删除；DPA 模板；隐私告知与设置入口。 |
| P2-5 | `docs/04-contracts-and-api.md:215`；`docs/07-server-domains.md:119`；`docs/07-server-domains.md:126`；`docs/07-server-domains.md:132` | 有 `balance.low` 事件但无阈值配置/站内横幅/邮件；无用量导出/chargeback；月发放“金额”默认值未定义。 | 用户被硬阻断时才知道；财务无法对账。 | 低余额阈值 + 横幅/邮件；用量 CSV；月发放默认额；管理员告警。 |
| P2-6 | `docs/09-delivery-and-oem.md:266`；`docs/09-delivery-and-oem.md:321`；`docs/09-delivery-and-oem.md:290`；`docs/09-delivery-and-oem.md:288` | 个人版清单 ed25519 签名但无密钥管理/轮换/吊销；无降级/最低版本策略；beta 切换行为未定义。 | 密钥泄露或坏版本恢复不明确。 | 定义密钥轮换/吊销、签名撤销、最低版本、手动回滚说明。 |
| P2-7 | `docs/01-vision-and-editions.md:38-40`；`docs/12-roadmap.md:21`；`docs/PLAN.md:105`；`docs/01-vision-and-editions.md:42` | 企业版产品矩阵含 WASM 应用中心，但 M2/M3 先交付无应用中心；没有 SKU/功能边界与“无应用中心企业版”的验收。 | 销售/客户预期错位；验收范围争议。 | 定义 “Enterprise Core” 与 “App Center add-on”；按里程碑给功能矩阵；只卖已交付范围。 |
| P2-8 | `docs/01-vision-and-editions.md:77`；`docs/12-roadmap.md:74`；`docs/12-roadmap.md:77-82` | 个人版本地技能/MCP 写在功能清单，但又写“可选，若时间允许”；M1 退出条件不含。 | 范围蔓延或验收缺口。 | 明确保留或砍掉；保留则加验收与本地技能的来源/审批/安全规则。 |
| P2-9 | `docs/09-delivery-and-oem.md:85`；`docs/09-delivery-and-oem.md:196`；`docs/09-delivery-and-oem.md:141`；`docs/09-delivery-and-oem.md:287` | 平台只列 win-x64/mac-arm64/linux-x64，服务端 linux-amd64；语言只 zh/en；无“不支持矩阵/系统要求”。 | 客户平台不匹配；销售过度承诺。 | 发布支持矩阵 + 系统要求 + i18n 范围；明确 non-goals。 |
| P2-10 | `docs/09-delivery-and-oem.md:122`；`docs/09-delivery-and-oem.md:97`；`docs/09-delivery-and-oem.md:127` | 产品身份/数据根按 channel 区分；OEM 换品牌 = 新 channel = 新数据根，无迁移/改名路径。 | 换品牌导致本地数据丢失或强制重装。 | 定义产品身份迁移，或明确“换品牌 = 新产品，不做数据迁移”。 |
| P2-11 | `docs/09-delivery-and-oem.md:18`；`docs/09-delivery-and-oem.md:346`；`docs/13-risks-and-adr.md:309-314` | OEM 行写“macOS 我方证书/客户自带另议”，Q1 已不再覆盖证书主体；OEM 公证责任/SLA 未定。 | OEM macOS 交付成本/责任不清。 | 按 OEM 合同明确证书主体、公证执行方与失败处理；写入 intake。 |
| P2-12 | `docs/07-server-domains.md:209`；`docs/04-contracts-and-api.md:232`；`docs/12-roadmap.md:143`；`docs/12-roadmap.md:151` | 每应用 DB 100MB/16 表、请求/guest 超时、并发限额触达时的用户提示、只读/清理/归档策略、管理台可见性未定义；应用错误页只有“要有”，无状态/文案/重试。 | 应用数据触顶后死角或静默失败；支持无证据。 | 定义限额告警/只读/清理策略、统一错误页与可重试语义、管理台诊断字段与审计。 |
| P2-13 | `docs/07-server-domains.md:110`；`docs/07-server-domains.md:117`；`docs/07-server-domains.md:136`；`docs/04-contracts-and-api.md:82` | 价格快照/峰谷默认有，但币种/舍入/展示、请求中途改价、OEM 时区、负余额展示与结算未定义。 | 账单争议/舍入 bug。 | 定义金额单位/舍入/展示、生效时间语义、时区、负余额策略。 |
| P2-14 | `docs/12-roadmap.md:82`；`docs/12-roadmap.md:185`；`docs/12-roadmap.md:191`；`docs/09-delivery-and-oem.md:201` | 退出条件有不可测项：“冷启动 <2s”无硬件、“性能预算”无数字、“无高危”无分级标准、可复现“容忍签名段”无阈值。 | “完成”争议；门禁无法自动判定。 | 定义硬件/测试数据、p50/p95、阈值、安全分级、可复现容差与证据路径。 |

## 竞品/参考实现对照与取舍

- **已覆盖的企业刚需**：SSO（local/OIDC/LDAP）、余额/峰谷/月发放、内容商店 + 审批 + 可见性、审计、门户、离线镜像、1:1 更新；在“弱管控”定位下这是合理的最小集。
- **明确不做且必须写进销售材料**：SCIM/部门/组策略、DLP/网络/工具限制、HA/多租户、端点管理、Windows 签名。不写会被客户当成缺陷，而不是取舍。
- **仍缺的常见项**：SIEM/审计导出、SMTP/通知、细粒度管理员角色（billing/user/settings）、强制 MFA、代理/自定义 CA、用量导出/chargeback、SCIM/组→角色映射、SLA/支持等级。
- **建议**：把“故意不做”和“后补”分成两张表；每个“不做”给客户替代方案（IdP、网络边界、终端管理、备份系统）。

## 必须补齐的产品决策/文档（≤10）

1. **管理台认证**：审计员与 OIDC/LDAP 管理员如何登录、角色如何映射；本地 auditor 是否可建（P0-1）。
2. **企业版 Day-0 开通 runbook/向导**：provider、模型、价格、月发放、首管理员、用户、连通性自检、验收脚本（P0-2）。
3. **OEM 客户接入与验收**：intake 清单、RACI、UAT/签收、交接包、升级/支持责任（P0-3）。
4. **OEM 地址迁移/恢复**：域名变更、证书轮换、服务端重建的受控迁移机制（P1-1）。
5. **统一更新策略**：强制/截止/离线宽限/每平台安装前置/失败回滚/车队回退/灰度口径（P1-2）。
6. **更新面托管与升级责任**：我方/OEM/客户三种模式、凭据、保留、SLA（P1-3）。
7. **网络前置与内网模型**：egress、proxy、自定义 CA、私有 OpenAI 兼容端点；修正“零外网依赖”表述（P1-4）。
8. **品牌与渠道唯一真源**：构建期身份不可变，服务端只发同渠道内容/主题（P1-5）。
9. **许可与深度 OEM 边界**：OEM 许可模式、Powered by/商标、闭源二进制准入与赔偿（P1-6/P1-7）。
10. **排期与验收重估**：WBS/角色/缓冲；把个人首配、管理员 API、用户生命周期纳入最小验收（P1-8/P1-9/P1-10/P1-12）。

## 审计边界

- 未重复检查错别字、链接、ADR 编号、已修复的跨文档冲突（见 `research/audit-consolidated.md`）。
- 本报告只写本文件，未修改任何其他文件；引用行号基于 2026-10-30 当前快照。
