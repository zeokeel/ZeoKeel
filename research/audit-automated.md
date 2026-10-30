# 自动化文档一致性审计（机械扫描）

- 扫描时间：2026-10-30
- 文件数：40（docs 26）

## 1. 失效链接
| 文件 | 链接文字 | 目标 |
| --- | --- | --- |
| research/wasm-languages.md | "_start" | store |

## 2. 重复标题与代码块闭合
无。

## 3. 已废弃/强管控类关键词残留（需人工确认是否是否定表述）
### 3.1 强管控（作为现行）
| 文件:行 | 命中词 | 行内容 |
| --- | --- | --- |
| docs/00-design-principles.md:3 | 强管控 | > **v3.1 总基调**：企业版**不是**"强管控终端"。服务端只管**认证、钱包/网关、商店内容、审计、管理台**； |

### 3.2 工具白名单（作为现行）
| 文件:行 | 命中词 | 行内容 |
| --- | --- | --- |
| AGENTS.md:249 | 工具白名单 | | 加回强管控（工具白名单、网络限制、企业审批、部门/用户组） | 已决策弱管控（ADR-030/032） | |
| docs/07-server-domains.md:17 | 工具白名单 | | 认证与会话、用户、钱包与账本、AI 网关与计费、技能/连接器/应用的分发与审批、审计、管理 API、门户与客户端分发 | agent 循环/工具执行、会话存储、工具白名单、网络策略、企业审批、部门/用户组、多租户、部门预算、MCP 流量代理、技能内容执行、应用内 AI、运行时插件加载、LLM 响应缓存 | |
| docs/07-server-domains.md:59 | 工具白名单 | | 认证、JWT 立即吊销、角色（超管/审计员/员工） | 下发工具白名单、限制工具种类 | |
| docs/07-server-domains.md:256 | 工具白名单 | - 服务端**不**下发：工具白名单、审批规则、网络策略、员工操作策略。 |
| docs/17-v3.1-decisions.md:10 | 工具白名单 | | 服务端职责 | （v2 旧）认证、组织、钱包、网关、技能、连接器、应用、审计、**策略下发/工具白名单** | **（v3.1 现行）只管认证相关 + 商店功能 + 钱包/网关 + 审计 + 管理台** | |

### 3.3 网络限制（作为现行）
| 文件:行 | 命中词 | 行内容 |
| --- | --- | --- |
| AGENTS.md:249 | 网络限制 | | 加回强管控（工具白名单、网络限制、企业审批、部门/用户组） | 已决策弱管控（ADR-030/032） | |
| docs/04-contracts-and-api.md:70 | 网络策略 | | `GET /config/bootstrap` | 登录后全量配置 | Bearer `config:read` | resp: schema_version、identity、gateway（模型目录/默认模型）、skills、connectors、apps、updates、brand、config_etag；* |
| docs/05-client-runtime.md:264 | 网络策略 | 网络策略、审批策略；客户端也不提供"被服务端远程限制"的入口。 |
| docs/07-server-domains.md:17 | 网络策略 | | 认证与会话、用户、钱包与账本、AI 网关与计费、技能/连接器/应用的分发与审批、审计、管理 API、门户与客户端分发 | agent 循环/工具执行、会话存储、工具白名单、网络策略、企业审批、部门/用户组、多租户、部门预算、MCP 流量代理、技能内容执行、应用内 AI、运行时插件加载、LLM 响应缓存 | |
| docs/07-server-domains.md:256 | 网络策略 | - 服务端**不**下发：工具白名单、审批规则、网络策略、员工操作策略。 |

### 3.4 企业审批（作为现行）
| 文件:行 | 命中词 | 行内容 |
| --- | --- | --- |
| AGENTS.md:249 | 企业审批 | | 加回强管控（工具白名单、网络限制、企业审批、部门/用户组） | 已决策弱管控（ADR-030/032） | |
| docs/07-server-domains.md:17 | 企业审批 | | 认证与会话、用户、钱包与账本、AI 网关与计费、技能/连接器/应用的分发与审批、审计、管理 API、门户与客户端分发 | agent 循环/工具执行、会话存储、工具白名单、网络策略、企业审批、部门/用户组、多租户、部门预算、MCP 流量代理、技能内容执行、应用内 AI、运行时插件加载、LLM 响应缓存 | |

### 3.5 部门/用户组（作为现行）
| 文件:行 | 命中词 | 行内容 |
| --- | --- | --- |
| AGENTS.md:249 | 部门 | | 加回强管控（工具白名单、网络限制、企业审批、部门/用户组） | 已决策弱管控（ADR-030/032） | |
| AGENTS.md:249 | 用户组 | | 加回强管控（工具白名单、网络限制、企业审批、部门/用户组） | 已决策弱管控（ADR-030/032） | |
| AGENTS.md:253 | 部门 | | 服务端执行 agent、执行技能、代理 MCP 流量、多租户、部门预算 | 非目标（`docs/07` §0.1） | |
| docs/00-design-principles.md:38 | 部门 | | 部门/用户组 | 已删除该功能（企业实践不再需要） | |
| docs/00-design-principles.md:38 | 用户组 | | 部门/用户组 | 已删除该功能（企业实践不再需要） | |
| docs/07-server-domains.md:17 | 部门 | | 认证与会话、用户、钱包与账本、AI 网关与计费、技能/连接器/应用的分发与审批、审计、管理 API、门户与客户端分发 | agent 循环/工具执行、会话存储、工具白名单、网络策略、企业审批、部门/用户组、多租户、部门预算、MCP 流量代理、技能内容执行、应用内 AI、运行时插件加载、LLM 响应缓存 | |
| docs/07-server-domains.md:17 | 用户组 | | 认证与会话、用户、钱包与账本、AI 网关与计费、技能/连接器/应用的分发与审批、审计、管理 API、门户与客户端分发 | agent 循环/工具执行、会话存储、工具白名单、网络策略、企业审批、部门/用户组、多租户、部门预算、MCP 流量代理、技能内容执行、应用内 AI、运行时插件加载、LLM 响应缓存 | |
| docs/13-risks-and-adr.md:263 | 部门 | **决策**：删除组织树、部门归属、部门可见范围、部门管理员；技能/应用可见性 = **全员 / 指定用户** |
| docs/13-risks-and-adr.md:306 | 部门 | **理由**：需求方确认；去掉部门后"指定用户"覆盖例外场景；累加/覆盖覆盖两种企业福利策略。 |
| docs/17-v3.1-decisions.md:66 | 部门 | | `07-server-domains.md` | 已更新：删组织/部门、钱包峰谷价/月发放/透支、JWT 立即失效、技能删除同步、MCP stdio + 客户端 OAuth、员工上传应用、审计员 | |
| research/harness-server.md:16 | 部门 | | 部门 | `groups` 自引用树 + `user_groups`；有效组 = 归属+祖先+主管子树+「全员」 | `server/internal/serverstore/effective.go:112-160` | |
| research/harness-server.md:17 | 部门 | | 额度 | **唯一闸门 = 员工账户余额**（元/微元）；token 配额、金额配额、部门预算**已下线但列仍在** | `server/internal/serverstore/balance.go:15-35`；`llmgateway/balance_gate.go:1-45` | |
| research/harness-server.md:44 | 部门 | | 认证/管理 | `serverauth` | local/LDAP/OIDC、token、admin session、MFA、RBAC、部门/用户/审计 handler | |
| research/harness-server.md:72 | 部门 | | 内容 | `apps`、`app_releases`、`app_grants` | 长期身份 / 不可变版本（`archive BYTEA`）/ 用户或部门授权 | |
| research/harness-server.md:125 | 部门 | | 组映射 | `groups` claim → 部门（**指针区分"未下发"与"空数组"**，未下发时不清空） | `oidc.go:249,264-274`；`handler.go:783-789` | |
| research/harness-server.md:166 | 部门 | ## 3. 部门管理 |
| research/harness-server.md:171 | 部门 | | 层级 | 任意深度自引用树；`parent_id=0` 为顶层；删除要求无成员/无子部门/无授权引用 | `server/docs/03-api-reference.md:96-99` | |
| research/harness-server.md:172 | 部门 | | 多部门 | 用户可属多个部门（`PUT /users/:id/department {group_ids:[]}`，兼容旧 `{group_id}`） | `router.go:342`；`server/docs/03-api-reference.md:93-95` | |
| research/harness-server.md:173 | 部门 | | 有效权限组 | 归属部门 + **祖先链** + 主管（`leader_id=该用户`）的**子树** + 隐式「全员」组 | `effective.go:112-160` | |
| research/harness-server.md:174 | 部门 | | 授权继承 | 授权对象是组名：授权给父部门 ⇒ 子树成员可见；主管自动获得所辖子树授权 | 同上 | |
| research/harness-server.md:184 | 部门 | | 组落地 | `SyncUserGroups` **按名字 GetOrCreate**：LDAP/OIDC 的组名自动建成**平铺**部门（`INSERT INTO groups(name)`，无 parent），不做 DN→树映射 | `groups.go:30-55,111-135` | |
| research/harness-server.md:188 | 部门 | **部门管理员权限：不存在。** RBAC 只有三个全局角色，权限点是 `<域>:<动作>` 常量表，不落库（`rbac.go:19-106`）。部门 `leader_id` 只影响**内容授权继承**（`effective.go:121-127`），不带来任何管理端权限；管理面写操作一律要求 `dept:write` |
| research/harness-server.md:429 | 部门 | | 2 | **多套并行额度**：token 月配额 + 金额月配额 + 部门预算 + 账户余额四套闸门同时存在，互相打架（"充了钱仍被软配额拦住"），下线后**列与请求字段长期保留**，客户端/管理端/文档各自保留一套说法 | `server/docs/01-architecture.md:38-52`（"别再把它们 |
| research/harness-server.md:443 | 部门 | | "部门预算约束网关" | 已下线，`groups.budget_money` 只读不写 | `server/docs/06-database.md` groups 一节开头 vs `server/docs/01-architecture.md:38-52` | |

### 3.6 Chat Completions（作为现行）
| 文件:行 | 命中词 | 行内容 |
| --- | --- | --- |
| docs/13-risks-and-adr.md:259 | Chat Completions | **后果**：客户端网关适配只做 Responses；第三方只兼容 Chat Completions 的模型需要上游网关转换（或后续再加）。 |
| research/harness-server.md:249 | chat/completions | | 对外协议 | OpenAI 兼容（`/v1/chat/completions`、`/completions`、`/responses`、`/embeddings`、`/models`）+ Anthropic Messages（`/v1/messages`）+ DeepSeek Files API | `router |
| research/harness-server.md:377 | chat/completions | | LLM 网关 | `/v1/*`（另有无前缀别名） | Bearer | chat/completions、completions、responses、embeddings、messages、models、files | |
| research/rig.md:538 | Chat Completions | .completion("my-model");           // 或 .chat("my-model") 强制 Chat Completions |

### 3.7 Windows 签名（作为要求）
| 文件:行 | 命中词 | 行内容 |
| --- | --- | --- |
| research/frontend.md:291 | Authenticode | | 签名/公证 | **现成**：macOS notarytool + Windows Authenticode + updater minisign 配置化 | 全自建 | 基本不需要 | 全自建 | |

## 4. 参考仓库相关表述（zeokeel/channels、Harness、Next）
| 文件:行 | 行内容 |
| --- | --- |
| AGENTS.md:47 | | C15 | **参考仓库只作参考**：`zeokeel/channels`、`ZeoKeel-Harness`、`ZeoKeel-Next` 等只可作事实/设计参考，**不得**作为我们的依赖、构建输入、索引仓或数据源；我们的 OEM 索引仓必须自有命名（demo 为 `zeokeel/demo-oem-index`） | 拒绝合并 | |
| README.md:7 | > [ZeoKeel-Harness](https://github.com/zeokeel/ZeoKeel-Harness)。 |
| README.md:110 | [ZeoKeel-Harness](https://github.com/zeokeel/ZeoKeel-Harness) 的一手研究（见 |
| docs/00-design-principles.md:65 | - **参考仓库边界**：`zeokeel/channels`、`ZeoKeel-Harness`、`ZeoKeel-Next` 只作事实/设计参考， |
| docs/09-delivery-and-oem.md:51 | 参考仓库 `zeokeel/channels` **只用于对照 `channel.json` 字段设计，不作为我们的索引仓/依赖**。 |
| docs/09-delivery-and-oem.md:54 | `zeokeel/channels` 仅作字段设计参考。 |
| docs/09-delivery-and-oem.md:91 | > **服务端地址的构建期默认值只有一个真源**：`channel.json` 的 `defaults.server_url`（字段形状参考 `zeokeel/channels`，不依赖该仓）。 |
| docs/09-delivery-and-oem.md:114 | **渠道/品牌资源清单**（`channel.json` schema 1；字段设计参考 `zeokeel/channels`，实现与索引仓独立） |
| docs/09-delivery-and-oem.md:274 | > 参考 `zeokeel/ZeoKeel-Harness` 的更新模型：**服务端镜像与客户端安装包同批发布、客户端只从登录的服务端更新、 |
| docs/13-risks-and-adr.md:355 | **理由**：需求方明确要求；参考 ZeoKeel-Harness 的"客户端只从登录服务端更新 + 服务端手动升级 + 安装包随镜像"模型。 |
| docs/13-risks-and-adr.md:364 | **理由**：需求方明确"OEM 包里包含指定域名，装完直接输账号密码"；参考 `zeokeel/channels` 的 `defaults.server_url`。 |
| docs/15-open-questions.md:12 | | D1 | 不沿用 ZeoKeel-Next 的任何先例/数据 | |
| docs/15-open-questions.md:58 | | D42 | demo 仓库建在 **`zeokeel` 组织**；我们的索引仓自建（demo 为 `zeokeel/demo-oem-index`）；`zeokeel/channels` 仅作字段参考 | |
| docs/PLAN.md:5 | > 模型层 `rig`；组织方式参考 DeepSeek Harness 与 ZeoKeel-Harness。 |
| docs/PLAN.md:40 | | 参考 ZeoKeel-Harness | 一手研究见 `research/harness-server.md`、`research/harness-client-channel.md` | 已研究 | |
| docs/PLAN.md:123 | ## 6. 与参考实现（ZeoKeel-Harness）的关系 |
| research/frontend.md:4 | > 证据来源：本机 DSH 安装（305 个 `@deepseek-ai/*` 包，版本 `0.2.1-rc.2`）、`/data/ZeoKeel-Next`（只读先例）、crates.io 与官方文档（2026-10-30 经 `proxychains -q` 查询）。 |
| research/frontend.md:509 | ## 附录 A：与 DSH / ZeoKeel-Next 的取舍一览 |
| research/frontend.md:519 | | ZeoKeel-Next `UiContribution`(Rust) + TS registry | ✅ 采用 | 加上"提供者包"清单 + profile 角色 | |
| research/harness-client-channel.md:13 | | 2 | 渠道怎么定 | 渠道 = 私有仓 `zeokeel/channels` 的 `channels/<id>/channel.json` 目录；**构建期**用 `DSH_BUILD_CHANNEL=<id>` 选，服务端**不参与选择**。运行期"我是哪个渠道"由**登录的那台服务端**在结构上决定（客户端不直连分发面）| |
| research/harness-client-channel.md:86 | | 定义位置 | 私有仓 `zeokeel/channels` 的 `channels/<id>/channel.json`（**不在本仓**；本仓只有 `brands/official/`） | `.github/workflows/ci.yml` "Fetch channel packages (private repo)"；`AGENTS.md` 铁律 |
| research/harness-client-channel.md:277 | 1. **私有仓建目录**：`zeokeel/channels` → `channels/<id>/`（`channel.json` + 素材）。在 `channels.manifest.json` 里登记（否则"少发一个渠道"无人发现）。 |
| research/harness-client-channel.md:292 | | `.gitmodules` | **只有一条**：`deepseek-harness` → `https://github.com/deepseek-ai/deepseek-harness.git`（上游镜像 submodule；本地检出里该目录为空）。**渠道仓不是 submodule** | |
| research/harness-client-channel.md:293 | | 渠道仓获取 | CI 运行时克隆（`git clone --depth 1` 或按 pin `fetch --depth 1 <sha>`），凭据三选一：`CI_CHANNELS_URL`（自建/本地）> SSH deploy key > PAT | |
| research/harness-client-channel.md:335 | 1. 本仓只有 `brands/official/`；私有仓 `zeokeel/channels` 的内容（`official`/`beta` 的实际 `channel.json`）不可见，"官方模板 28 键全部显式"只能从 `CHANNEL_TEMPLATE_KEYS` 与 CI 判据推断。 |
| research/prior-art.md:1 | # 先例研究：ZeoKeel-Next → ZeoKeel「一切皆包」 |
| research/prior-art.md:3 | > **研究对象**：`/data/ZeoKeel-Next`（只读，未做任何修改）。 |
| research/prior-art.md:10 | > **一句话判断**：ZeoKeel-Next 已经把「插件 = 构建期静态包」做到了 **80%**，但它验证的是 **"运行时禁用"**，不是 **"构建期替换"**；两字之差就是新项目全部工程量所在。 |
| research/prior-art.md:633 | ## 附录 B：只读核验命令（在 `/data/ZeoKeel-Next` 下） |
| research/rig.md:47 | 0.45.0      2026-10-19   <- ZeoKeel-Next 钉的版本 |
| research/rig.md:82 | ### 1.3 ZeoKeel-Next 的钉版核实 |
| research/rig.md:243 | ### 2.4 Probe B：`rig = "=0.45.0"` facade 根路径（验证 ZeoKeel-Next 的钉法） |

## 5. ADR 与 D 编号完整性
- ADR 定义数：56，范围 1–56
- 重复 ADR：无
- 缺口 ADR：无
- 被引用但未定义的 ADR：无
- D 定义数：59，范围 1–59
- 重复 D：无
- 缺口 D：无

## 6. docs 编号与 README 映射
| 文件 | 标题 |
| --- | --- |
| 00-design-principles.md | # 00 · 设计原则（v3.1：服务端弱管控） |
| 01-vision-and-editions.md | # 01 · 产品形态、双版本与 OEM |
| 02-package-model.md | # 02 · 包模型：什么算一个"包" |
| 03-composition-and-ci.md | # 03 · 组合与 CI 替换机制（核心） |
| 04-contracts-and-api.md | # 04 · 契约与 API：客户端 ↔ 服务端 |
| 05-client-runtime.md | # 05 · 客户端运行时：内核宿主、两种版本、Agent 循环、WASM 宿主 |
| 06-server-runtime.md | # 06 · 服务端运行时：进程、包、数据库、迁移、调度 |
| 07-server-domains.md | # 07 · 服务端领域设计（v3.1：弱管控） |
| 08-admin-console.md | # 08 · 管理后台：也是一个包集合 |
| 09-delivery-and-oem.md | # 09 · 版本与交付：企业版 / 个人版 / OEM 白标 |
| 10-repo-and-build.md | # 10 · 仓库布局与构建 |
| 11-testing-and-evidence.md | # 11 · 测试分层与证据政策 |
| 12-roadmap.md | # 12 · 路线图（S0 + M0–M6） |
| 13-risks-and-adr.md | # 13 · 架构决策记录（ADR）与风险登记 |
| 14-review-v1v2.md | # 14 · 对抗性评审与 v2 修订记录（历史） |
| 15-open-questions.md | # 15 · 已定事项与问题清单（v3.1 + v3.2 增量） |
| 16-ui-packages.md | # 16 · 前端也是包：注册表、槽位、类型契约、构建期换包 |
| 17-v3.1-decisions.md | # 17 · 决策记录（v3.1 弱管控 + v3.2 增量） |
| 18-strictness.md | # 18 · 严格类型与静态检查（防止技术性 bug） |
| 19-client-ux-flows.md | # 19 · 客户端 UX 流程与页面契约（补充规范） |
| 20-operations.md | # 20 · 运维、部署与 Day-0 开通（补充规范） |
| 21-security-privacy.md | # 21 · 安全、隐私与信任边界（补充规范） |
| 22-finance-billing.md | # 22 · 钱包、计费与对账（补充规范） |
| 23-support-oem.md | # 23 · OEM 交付、客户接入与支持（补充规范） |
| 24-composition-and-runtime-spec.md | # 24 · 组合、Manifest 与运行时契约（补充规范） |
| PLAN.md | # PLAN · ZeoKeel 平台总规划（v3.2） |

- README 未列出的 docs 文件：无

## 7. 完成/验证类声明（需人工核对证据）
| 文件:行 | 行内容 |
| --- | --- |
| AGENTS.md:261 | | 把"未验证"说成"已验证"；用模型自述当证据 | 证据政策 | |
| AGENTS.md:266 | - S0：P2（WASM）原型已完成；P1 双 app 组合、P3 认证+JWT、P4 OEM submodule CI、P5 迁移不可变、P6 管理台包化待做。 |
| README.md:105 | ## 已验证 / 未验证 |
| configs/README.md:14 | - TS 唯一妥协是 `skipLibCheck: true`（第三方 d.ts）；自有包必须全绿。 |
| docs/03-composition-and-ci.md:6 | > 证据：`experiments/composition-poc/` 已实测三种实现换同一角色，含 **workspace 外**实现（形状等同外部包；真正仓外 git 源见 §6.5）。 |
| docs/07-server-domains.md:208 | （原型验证见 `experiments/wasm-poc`：Go/Rust 双 guest + 帧 ABI + SQLite + fuel 已跑通） |
| docs/09-delivery-and-oem.md:179 | | 3 | 全渠道校验 | `zk channel validate --index <dir> --all`（形状/素材/i18n/跨渠道唯一/深度包门禁） | 全绿 | 校验报告（进交付记录/SBOM） | |
| docs/11-testing-and-evidence.md:109 | | 基础原语（弹窗/菜单/滚动/焦点） | Radix UI（无样式、可访问性已验证） | |
| docs/12-roadmap.md:23 | **S0 进度**：P2（WASM 运行时/帧 ABI/宿主 DB/限额）原型已完成，见 `experiments/wasm-poc`； |
| docs/12-roadmap.md:35 | | P2 | WASM 运行时 + 语言集 + 每应用 DB | **✅ 原型已完成**（`experiments/wasm-poc`）：wasmtime 50 + Go/Rust wasip1、`RS<len>\n<JSON>` 帧、`db.exec/db.query` 宿主函数、SQLite 持久、fuel 限额、golden conformance；剩 |
| docs/12-roadmap.md:80 | - [ ] 离线 e2e（mock provider）全绿；无服务端依赖。 |
| docs/13-risks-and-adr.md:60 | **理由**（已实测，`research/rig.md`）：rig 0.45.0 三件套真实可用，但处于每 1–3 周一次 breaking 的 |
| docs/13-risks-and-adr.md:215 | **理由**：参考实现已验证形态；"不给自由度只给铺好的路"降低攻击面；AI 能力由客户端 agent 提供。 |
| docs/13-risks-and-adr.md:461 | | R2 | 契约过宽（什么都进契约）导致改一处全生态红；或过窄导致实现细节外泄 | H | H | 契约只放"替换边界必需"；`cargo public-api` diff 审查；契约变更需符合性套件全绿 | 契约 PR 必须附 public-api diff | |
| docs/18-strictness.md:42 | "skipLibCheck": true                 // 唯一妥协：第三方 d.ts；自有包必须全绿 |
| docs/21-security-privacy.md:130 | - [ ] 认证绕过/JWT 吊销/CSRF/SSRF/zip 穿越/WASM 逃逸/命令注入/rebinding 用例全绿。 |
| docs/21-security-privacy.md:134 | - [ ] 隐私口径与 DPA 模板评审；销售材料不得声称"服务端不读取会话"除非网关不落库已验证。 |
| docs/22-finance-billing.md:142 | - [ ] 钱审计矩阵全绿；哈希链/WORM 验证通过。 |
| docs/23-support-oem.md:120 | - [ ] OEM 渠道白标/地址/更新/审计验收全绿。 |
| docs/PLAN.md:77 | | ★13 | WASM 应用：服务端执行 + 客户端独立 origin 开窗 + 每应用 SQLite；不可调 AI | 参考实现已验证形态；缩小能力面 | 026 | |
| docs/PLAN.md:150 | 2. S0 六个 spike，各 ≤2 天，结论落 ADR；**P2（WASM）已完成**，其余待做。 |
| experiments/wasm-poc/README.md:9 | ## 已验证 |
| research/harness-server.md:403 | | 发布通道 | `push tag v*` → 四路（server/linux/win/mac）全绿 → 按渠道构建镜像 → `docker save|zip` → GitHub Release（仅 beta/official）+ 上传 `<channel>/releases/<v>/`（同目录 SHA256SUMS，保留最近 3 版）；**不经任何镜像仓 |
| research/kernel.md:69 | ⇒ **一个进程里可以跑多个互不干扰的 root**（探针已验证，见 §8.1），这是宿主做「每窗口/每会话/每 agent 一个容器」的基础。 |
| research/prior-art.md:236 | **Root 即贡献**（`[工作区]` 已实现，这是关键模式）： |
| research/review-plan.md:232 | POC 的边界（必须写进 README 的"已验证"清单）：无 cordis、无 `ctx`、无异步/取消/审批、无角色基数、 |
| research/wasm-languages.md:25 | - 另外已验证 `-W timeout=100ms`、`-W fuel=...` 能在死循环上触发 trap；`-W max-memory-size` 让 `memory.grow` 返回 -1。服务端应使用 embedding 的 epoch/fuel/StoreLimits，而不是依赖 CLI。 |
| research/wasm-languages.md:112 | 以下命令均在 `/tmp/wasm-research` 下执行；海外下载加 `proxychains -q`。已实测通过。 |

## 8. 实验命令与文件存在性
- experiments/composition-poc/resolve.py: 存在
- experiments/wasm-poc/build.sh: 存在
- experiments/wasm-poc/golden/run.py: 存在
- experiments/wasm-poc/host/src/main.rs: 存在
- configs/tsconfig.base.json: 存在
- configs/eslint.config.js: 存在
- configs/rust-workspace-lints.toml: 存在

## 9. 数字/规模类声明（需互相核对）
| 文件:行 | 行内容 |
| --- | --- |
| 03-composition-and-ci.md:418 | 列表做 **id 定向 patch / insert** 合成。本机合成结果是 200 个顶层 entry / 194 个包 / 33 个 disabled。 |
| 03-composition-and-ci.md:437 | 2. **一个包里可以带 patch**（bundle 即"一层能力 + 它的默认 entry"），这是它能把 305 个包组织成 |
| 05-client-runtime.md:226 | ### 5.3 必须显式处理的 0.45 行为（都是实测坑；标 🤖 的仅在使用 `agent-rig` 时相关） |
| 12-roadmap.md:11 | | **S0** | 6 个前置 spike | 2 | 6 条 ADR + T0–T5 基线 | — | |
| 12-roadmap.md:20 | 合计约 35 人周；**关键路径** S0 → M0 → M1（个人版客户端）→ M2。**客户端优先**：M1 是第一个可交付产品， |
| 12-roadmap.md:28 | ## S0 · 六个前置 spike（2 人周） |
| 12-roadmap.md:45 | ## M0 · 骨架与机制（3 人周） |
| 12-roadmap.md:65 | ## M1 · 个人版 MVP（6 人周） |
| 12-roadmap.md:86 | ## M2 · 企业版 = 个人版 + 企业管控包（6 人周） |
| 12-roadmap.md:112 | ## M3 · 内容平台（5 人周） |
| 12-roadmap.md:134 | ## M4 · 应用中心（5 人周） |
| 12-roadmap.md:156 | ## M5 · OEM 与交付（4 人周） |
| 12-roadmap.md:180 | ## M6 · 加固与发布（4 人周） |
| 14-review-v1v2.md:8 | > [`research/review-plan.md`](../research/review-plan.md)（246 行，含可重跑实验）。 |
| 14-review-v1v2.md:31 | | S8 | M1 约 31 包排 3 人周不现实；包数口径三套；命名不一致 | M1 砍到约 14 包/5–6 周；M2 再 +15（约 30）；M3 只要求 8 个角色各 ≥2 实现；统一包名（06） | ✅ 已修 | |
| 15-open-questions.md:17 | | D6 | M1 砍到约 14 个包（功能正常即可） | |
| 19-client-ux-flows.md:94 | ### 4.2 行为 |
| PLAN.md:121 | 单人约 35 人周；2–3 人并行约 4–5 个月。**关键路径**：S0 → M0 → M1（个人版客户端）→ M2（企业管控包，认证/网关是最大块）。 |
