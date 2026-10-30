
## 个人版/企业版结构
docs/03-composition-and-ci.md:59: # editions/enterprise.toml —— 个人版 + 企业管控包
docs/03-composition-and-ci.md:67: # 追加企业管控包（示例）：登录、网关、商店客户端、服务端域
docs/15-open-questions.md:116: | D58 | **个人版优先；企业版 = 个人版 + 企业管控包**（登录/网关/商店客户端/服务端/管理台），不另造客户端基线 |
docs/02-package-model.md:99: `base_edition = "personal"` 后**追加企业管控包 + 覆盖角色**（企业版 = 个人版 + 企业管控包）；`oem/<customer>/`
docs/13-risks-and-adr.md:365: ### ADR-043 · 个人版优先；企业版 = 个人版 + 企业管控包 `accepted`
docs/13-risks-and-adr.md:370: GitHub 更新器、个人引导），**追加**企业管控包（登录/JWT、网关 provider、技能/连接器/应用客户端、服务端域、管理台），
docs/13-risks-and-adr.md:375: **理由**：需求方明确"优先个人版，企业版只是增加了企业管控包"；避免两条产品线/双客户端基线。
docs/17-v3.1-decisions.md:54: | 32 | 个人版优先；企业版 = 个人版 + 企业管控包 | 见 ADR-043 |
docs/07-server-domains.md:4: > 它是**企业版追加的管控包集合**（企业版 = 个人版 + 企业管控包）；个人版不部署服务端。
docs/05-client-runtime.md:32: - **企业版 = 个人版 + 企业管控包**（ADR-043）：客户端基线只有一份（个人版）；企业版在其上追加登录/网关/商店客户端/服务端，
docs/PLAN.md:27: | 两个版本 | **个人版为基线；企业版 = 个人版 + 企业管控包**（`01` §1.5、ADR-043） | 已拍板 |
docs/PLAN.md:71: | ★7 | **企业版 = (个人版 − 个人专属包) + 企业管控包**（继承、排除、追加、覆盖角色） | 个人版先交付；企业版不另造客户端基线 | 020/043 |
docs/PLAN.md:103: | 3 | **企业版 = 个人版 + 企业管控包**：登录/JWT、网关 provider、技能/连接器/应用客户端、服务端（认证/钱包/网关/商店/审计）、管理台 | 企业版不另做客户端基线，只在个人版之上叠加管控包并覆盖少数角色 |
docs/PLAN.md:115: | M2 | 企业版 = 个人版 + 企业管控包 | 6 | 认证三源（单一主源）、JWT、用户（无部门）、钱包、网关计费、客户端登录、管理台核心、docker 部署 |
docs/PLAN.md:121: 单人约 35 人周；2–3 人并行约 4–5 个月。**关键路径**：S0 → M0 → M1（个人版客户端）→ M2（企业管控包，认证/网关是最大块）。
docs/01-vision-and-editions.md:15: ## 1.5 版本模型：企业版 = 个人版 + 企业管控包
docs/01-vision-and-editions.md:17: - **个人版是基线**：客户端壳、UI、本地 agent、工具、审批、会话、自配 provider、本地技能/MCP、GitHub Releases 更新。
docs/01-vision-and-editions.md:18: - **企业版不另造客户端**：在个人版包集合之上**追加企业管控包**（登录/JWT、网关 provider、技能中心客户端、连接器客户端、WASM 应用客户端、服务端、管理台），
docs/01-vision-and-editions.md:21: 公式：`enterprise = (personal − personal-only) + 企业管控包 + 角色覆盖`。
docs/12-roadmap.md:14: | **M2** | 企业版 = 个人版 + 企业管控包 | 6 | 在 M1 客户端上追加：登录/JWT、网关 provider、商店客户端、服务端（认证/钱包/网关/商店/审计）、管理台核心、docker | M1 |
docs/12-roadmap.md:86: ## M2 · 企业版 = 个人版 + 企业管控包（6 人周）
docs/12-roadmap.md:88: **目标**：在 M1 个人版客户端之上叠加企业管控包：一套 docker 部署起来，员工登录即用，AI 走网关并扣费。
AGENTS.md:15: - **双版本**：`personal`（纯客户端、自配 provider、无应用中心）是基线；`enterprise` = (个人版 − 个人专属包) + 企业管控包（服务端、登录、网关、商店客户端、管理台），通过 `base_edition = "personal"` + `exclude` 追加/覆盖，不复制客户端基线。
AGENTS.md:32: | C1 | **个人版优先**：先做个人版客户端（壳/UI/agent/工具/本地 provider）；**企业版 = 个人版 + 企业管控包**（登录/网关/商店/服务端/管理台），并排除个人版专属包（本地 provider 设置 UI、GitHub 更新器），不另造客户端基线；WASM 应用中心后置 | 拒绝合并 |
README.md:12: > **企业版 = 个人版 + 企业管控包**（登录、网关、商店、钱包、审计、服务端），不是独立产品线；

## 个人版更新
docs/03-composition-and-ci.md:70: "client.update"         = "zeokeel/client-update-server"   # 覆盖 GitHub Releases
docs/15-open-questions.md:112: | D54 | **个人版**更新只走 **GitHub Releases**（stable/beta，`latest.json` + `SHA256SUMS`） |
docs/04-contracts-and-api.md:118: 规则：企业版客户端**只从登录的这台服务端**取更新（二进制里没有指向 GitHub/公共更新面的路径），`channel_id`/`edition` 必须匹配，`client_version` 必须等于服务端镜像内 `CLIENT-RELEASE.json` 的版本（1:1）；低于 `min_supported` 时业务面 426，稳定面始终可用于升级。**个人版不走此接口**，直接查 GitH
docs/09-delivery-and-oem.md:17: | 个人版 | `personal` | `official` | `editions/personal.toml` + `profiles/default.toml` + `brands/official/` | 纯客户端三平台安装包（无服务端产物） | MIT 免费 | **GitHub Releases**（stable/beta）；客户端查 `latest.json` + `SHA256S
docs/09-delivery-and-oem.md:18: | OEM 客户版 | `enterprise` | `oem-<customer>` | `editions/enterprise.toml` + 私有索引仓该客户子仓（品牌/文案/配置/可选代码包） | 同企业版，服务端与客户端均为客户品牌 | MIT（白标服务另计） | 只走私有更新面，**绝不进公开 release** | 待定（Q1） |
docs/09-delivery-and-oem.md:28: | 其余 | 渠道 = official+oem-*；更新源 = 登录的服务端（1:1）；三平台 | 渠道 = official；更新源 = GitHub Releases；三平台 |
docs/09-delivery-and-oem.md:266: | 更新源 | **GitHub Releases**（stable/beta；`latest.json` + `SHA256SUMS` + 安装包），签名校验 |
docs/09-delivery-and-oem.md:273: > 服务端升级必须手动导入镜像**。我们沿用这个结构，并把个人版改为 GitHub Releases。
docs/09-delivery-and-oem.md:282: ### 7.1 个人版：GitHub Releases
docs/09-delivery-and-oem.md:286: | 来源 | **仅 GitHub Releases**（`github.com/zeokeel/ZeoKeel/releases`），不依赖任何自建更新面 |
docs/09-delivery-and-oem.md:288: | 资产 | 三平台安装包 + `latest.json` + `SHA256SUMS`；`latest.json` 含版本/资产/sha256/最低支持版本 |
docs/09-delivery-and-oem.md:290: | 校验 | HTTPS + `latest.json` 签名 + 安装包 sha256；Windows 未签名（提示 SmartScreen），macOS 公证后校验签名 |
docs/09-delivery-and-oem.md:298: | 清单布局 | `<channel>/latest.json`（no-cache）+ `<channel>/releases/<ver>/zeokeel-server-<ver>-amd64.zip` + `SHA256SUMS` + release notes；版本目录不可变 |
docs/09-delivery-and-oem.md:299: | 检查 | 服务端定期/手动检查 `latest.json`（可比对版本与 sha256）；管理台「服务器信息」显示"有新版本"与升级步骤；**不自动下载、不自动升级** |
docs/09-delivery-and-oem.md:328: > 更新服务器 `<channel>/latest.json`、服务端手动升级、门户读 `CLIENT-RELEASE.json`。个人版我们改为 GitHub Releases。
docs/09-delivery-and-oem.md:334: | 产物隔离 | 每渠道独立 artifact 目录/文件名带 channel；OEM 产物**不进公开 release**，走私有更新面；跨渠道中转路径不含渠道名（HMAC 派生临时前缀，取回即销毁） |
docs/13-risks-and-adr.md:343: ### ADR-041 · 更新通道：个人版 GitHub Releases / 企业版自建更新面 + 手动升级 / 客户端 1:1 `accepted`
docs/13-risks-and-adr.md:346: - 个人版：只从 **GitHub Releases** 检查与下载（stable/beta，`latest.json` + `SHA256SUMS` + 安装包），不做自建更新面。
docs/13-risks-and-adr.md:347: - 企业版服务端：从**自建更新服务器**（静态 HTTPS，`<channel>/latest.json` + 版本镜像 zip + SHA256SUMS）检查；
docs/17-v3.1-decisions.md:52: | 30 | 个人版更新 = GitHub Releases；企业版服务端 = 自建更新服务器 + 手动导入镜像；企业版客户端 = 只从登录服务端 1:1 更新 | 见 ADR-041、`09` §7 |
docs/05-client-runtime.md:68: | ★H20 | 自动更新 | `client-update` 包 | 企业版只从登录服务端取清单（1:1）；个人版只从 GitHub Releases |
docs/PLAN.md:102: | 2 | **个人版客户端完整闭环**：本地 agent、工具、审批、会话、自配 provider、GitHub Releases 更新、安装包 | 个人版不需要服务端，最快形成可交付产品；它是企业版的基础 |
docs/01-vision-and-editions.md:17: - **个人版是基线**：客户端壳、UI、本地 agent、工具、审批、会话、自配 provider、本地技能/MCP、GitHub Releases 更新。
docs/01-vision-and-editions.md:78: - 自动更新（公开 release 渠道）。
docs/12-roadmap.md:75: - 三平台安装包（**Windows 不签名**，v3.1 决策）、**GitHub Releases** 更新源。
docs/12-roadmap.md:167: - 更新：**个人版走 GitHub Releases**；**企业版服务端用自建更新服务器 + 手动导入镜像升级**；企业版客户端只从登录服务端按 1:1 更新；强制/灰度策略；
docs/12-roadmap.md:175: - [ ] 企业版员工从门户下载客户端，且只从登录服务端 1:1 更新；个人版从 GitHub Releases 更新。
AGENTS.md:45: | C13 | **更新通道固定**：个人版只走 GitHub Releases；企业版客户端只从登录服务端按 1:1 更新；企业版服务端必须手动导入镜像升级；禁止客户端直连公共更新面/跨渠道 | 见 ADR-041 |
AGENTS.md:227: - 更新：个人版只从 GitHub Releases；企业版客户端只从登录服务端（1:1）更新；服务端更新必须人工备份 + `docker load` + compose；禁止自动升级。

## 企业版更新
docs/15-open-questions.md:113: | D55 | **企业版服务端**从**自建更新服务器**取镜像包，**必须手动升级**（备份→校验→`docker load`→compose up→readyz），不自动升级 |
docs/15-open-questions.md:114: | D56 | **企业版客户端只从登录的服务端更新**，版本与服务端镜像内客户端 1:1 配对；服务端版本变则客户端强制跟进 |
docs/04-contracts-and-api.md:118: 规则：企业版客户端**只从登录的这台服务端**取更新（二进制里没有指向 GitHub/公共更新面的路径），`channel_id`/`edition` 必须匹配，`client_version` 必须等于服务端镜像内 `CLIENT-RELEASE.json` 的版本（1:1）；低于 `min_supported` 时业务面 426，稳定面始终可用于升级。**个人版不走此接口**，直接查 GitH
docs/09-delivery-and-oem.md:28: | 其余 | 渠道 = official+oem-*；更新源 = 登录的服务端（1:1）；三平台 | 渠道 = official；更新源 = GitHub Releases；三平台 |
docs/09-delivery-and-oem.md:226: docker load -i images.tar          # 或 zstd -dc images.tar.zst | docker load
docs/09-delivery-and-oem.md:273: > 服务端升级必须手动导入镜像**。我们沿用这个结构，并把个人版改为 GitHub Releases。
docs/09-delivery-and-oem.md:279: - **1:1 可用**：`server.version == client.version` 由发布流水线强制；镜像缺客户端资产或版本不一致 → 构建失败，
docs/09-delivery-and-oem.md:293: ### 7.2 企业版服务端：自建更新服务器 + 必须手动升级
docs/09-delivery-and-oem.md:297: | 更新源 | **自建更新服务器**（静态 HTTPS：nginx/对象存储均可），地址在管理台「系统配置 → 更新清单地址」配置；默认指向客户自建面，不指向官方 |
docs/09-delivery-and-oem.md:300: | 升级流程（人工，必须） | ① 备份（`pg_dump` + `zeokeel-data/`）→ ② 下载镜像包 → ③ 校验 `SHA256SUMS` → ④ `docker load` → ⑤ `docker compose up -d` → ⑥ 启动自动迁移 → ⑦ `/readyz` + 门户/客户端清单校验 → ⑧ 失败则回滚旧镜像 + 备份 |
docs/09-delivery-and-oem.md:304: ### 7.3 企业版客户端：只从登录的服务端更新（强制 1:1）
docs/09-delivery-and-oem.md:311: | 1:1 | 清单里的 `client_version` 必须等于当前服务端镜像内 `CLIENT-RELEASE.json` 的版本；客户端安装该版本后与登录服务端配对可用；服务端版本变化 → 清单变化 → 客户端强制跟进 |
docs/09-delivery-and-oem.md:328: > 更新服务器 `<channel>/latest.json`、服务端手动升级、门户读 `CLIENT-RELEASE.json`。个人版我们改为 GitHub Releases。
docs/09-delivery-and-oem.md:356: | Q11 | 更新面/签名资产托管在哪？OEM 私有更新面归属？镜像是否进 registry？ | 我方托管私有面 + tar 内自带安装包兜底；不进公共 registry（可 `docker load` 或客户自建 registry） |
docs/11-testing-and-evidence.md:43: - 镜像：`docker load` → compose up → `/readyz` → 门户可访问 → 客户端安装包校验和。
docs/13-risks-and-adr.md:343: ### ADR-041 · 更新通道：个人版 GitHub Releases / 企业版自建更新面 + 手动升级 / 客户端 1:1 `accepted`
docs/13-risks-and-adr.md:347: - 企业版服务端：从**自建更新服务器**（静态 HTTPS，`<channel>/latest.json` + 版本镜像 zip + SHA256SUMS）检查；
docs/13-risks-and-adr.md:348: **必须手动升级**（备份 → 下载 → 校验 → `docker load` → `docker compose up -d` → `/readyz`），
docs/13-risks-and-adr.md:351: `CLIENT-RELEASE.json` 版本（1:1）；渠道/edition 不匹配拒绝升级；服务端可强制更新。
docs/13-risks-and-adr.md:353: **理由**：需求方明确要求；参考 ZeoKeel-Harness 的"客户端只从登录服务端更新 + 服务端手动升级 + 安装包随镜像"模型。
docs/17-v3.1-decisions.md:52: | 30 | 个人版更新 = GitHub Releases；企业版服务端 = 自建更新服务器 + 手动导入镜像；企业版客户端 = 只从登录服务端 1:1 更新 | 见 ADR-041、`09` §7 |
docs/05-client-runtime.md:68: | ★H20 | 自动更新 | `client-update` 包 | 企业版只从登录服务端取清单（1:1）；个人版只从 GitHub Releases |
docs/PLAN.md:30: | 企业版 Docker 交付 | 单镜像自带 compose/Caddy/门户/客户端安装包，`docker load` 即用（`09`） | 已拍板 |
docs/12-roadmap.md:102: - [ ] 新环境：`docker load` + `compose up` + 管理员初始化 + 员工登录 + 完成一轮对话并扣费。
docs/12-roadmap.md:167: - 更新：**个人版走 GitHub Releases**；**企业版服务端用自建更新服务器 + 手动导入镜像升级**；企业版客户端只从登录服务端按 1:1 更新；强制/灰度策略；
docs/12-roadmap.md:175: - [ ] 企业版员工从门户下载客户端，且只从登录服务端 1:1 更新；个人版从 GitHub Releases 更新。
docs/12-roadmap.md:176: - [ ] 服务端升级演练：自建更新服务器下载 → 校验 → `docker load` → 备份 → compose up → `/readyz`；失败可回滚。
AGENTS.md:45: | C13 | **更新通道固定**：个人版只走 GitHub Releases；企业版客户端只从登录服务端按 1:1 更新；企业版服务端必须手动导入镜像升级；禁止客户端直连公共更新面/跨渠道 | 见 ADR-041 |
AGENTS.md:227: - 更新：个人版只从 GitHub Releases；企业版客户端只从登录服务端（1:1）更新；服务端更新必须人工备份 + `docker load` + compose；禁止自动升级。

## 官方渠道
docs/15-open-questions.md:115: | D57 | **OEM 渠道内置服务端地址**（`channel.json.defaults.server_url`）：登录页只输账号/密码，地址不可编辑；官方渠道可为空、首次输入一次 |
docs/15-open-questions.md:117: | D59 | **官方企业版也是渠道**（`official`）：`server_url` 默认空、客户登录时自配；任意渠道都可指定或留空；官方/OEM 同一构建链路 |
docs/09-delivery-and-oem.md:10: **官方企业版 = edition `enterprise` × channel `official`**；OEM = `enterprise` × 客户渠道；个人版 = `personal` × `official`。
docs/09-delivery-and-oem.md:11: 官方不是特例，只是 `server_url` 默认空、登录时由客户配置地址的渠道。
docs/09-delivery-and-oem.md:16: | 企业版 | `enterprise` | `official` | `editions/enterprise.toml` + `profiles/default.toml` + `brands/official/` | 服务端二进制 + webadmin + 门户 + 三平台安装包 + compose | MIT 免费 | docker 镜像 tar 交付；客户端从**登录的服务端**取更新 
docs/09-delivery-and-oem.md:17: | 个人版 | `personal` | `official` | `editions/personal.toml` + `profiles/default.toml` + `brands/official/` | 纯客户端三平台安装包（无服务端产物） | MIT 免费 | **GitHub Releases**（stable/beta）；客户端查 `latest.json` + `SHA256S
docs/09-delivery-and-oem.md:20: 合法组合：企业版 = `official` + 任意 `oem-*`；个人版 = 仅 `official`（个人版不做 OEM，见 §11 Q3）。
docs/09-delivery-and-oem.md:91: > **服务端地址只有一个真源**：`channel.json` 的 `defaults.server_url`（字段形状参考 `zeokeel/channels`，不依赖该仓）。
docs/09-delivery-and-oem.md:92: > `oem.toml` 不再重复写 `server_url`；构建器从 `channel.json` 读取并生成客户端常量。
docs/09-delivery-and-oem.md:110: **channel id**：`^[a-z0-9][a-z0-9-]{0,31}$`；保留字 `official`/`beta`/`personal`/`demo`/`test`。一个客户 = 一个 channel id，
docs/09-delivery-and-oem.md:127: | `defaults.server_url` | 绝对 https URL（回环例外）或空 | **任意渠道都可空**：为空 → 登录页显示地址输入（客户自己配置，官方企业版默认如此）；非空 → 客户端内置并锁定地址（OEM 免填）；设置了就必须合法；个人版必须为空且无登录；同渠道跨版本不得变 | **渠道决定登录形态**：非空 = 只显示账号/密码、地址不可编辑；空 = 首次输入地址并保存；地
docs/09-delivery-and-oem.md:308: | 登录地址 | 来自构建期 `channel.json` 的 `defaults.server_url`；**非空时**登录页只有账号/密码、地址不可编辑；**为空时**（官方企业版默认）登录页显示地址输入，首次填写并保存 |
docs/09-delivery-and-oem.md:348: | Q3 | 个人版是否也允许 OEM？ | 否，仅 `official` |
docs/13-risks-and-adr.md:358: **决策**：`channel.json` 的 `defaults.server_url` **可选**：
docs/13-risks-and-adr.md:360: - 为空（官方企业版默认）：登录页显示地址输入，客户首次填写并保存；地址仍来自渠道默认值（空）。
docs/13-risks-and-adr.md:362: **理由**：需求方明确"OEM 包里包含指定域名，装完直接输账号密码"；参考 `zeokeel/channels` 的 `defaults.server_url`。
docs/13-risks-and-adr.md:363: **后果**：渠道校验新增 server_url 规则（必填/合法/不可变）；登录页 UI 按地址是否为空分两种形态；客户端不得提供改地址入口。
docs/13-risks-and-adr.md:379: ### ADR-044 · 渠道与版本正交：官方企业版也是渠道 `accepted`
docs/13-risks-and-adr.md:382: - `personal × official`：个人版客户端（无登录、无 server_url）。
docs/13-risks-and-adr.md:383: - `enterprise × official`：官方企业版；`server_url` 默认空 → 登录页由客户配置服务器地址。
docs/13-risks-and-adr.md:384: - `enterprise × oem-<customer>`：OEM 版；`server_url` 可预置（免填）也可留空（客户配置）。
docs/13-risks-and-adr.md:385: 官方渠道与 OEM 渠道走**同一条**构建/校验/测试链路（品牌、server_url 语义、渠道身份校验完全一致）；
docs/13-risks-and-adr.md:387: **理由**：需求方要求"官方企业版也作为一个渠道，地址可指定或不指定"，以获得灵活性与 OEM 可测试性。
docs/13-risks-and-adr.md:388: **后果**：渠道实现不能有"官方特判"；`official` 只是内置的一个 channel 目录；EDITION/CHANNEL 两个常量共同决定运行形态。
docs/17-v3.1-decisions.md:53: | 31 | OEM 渠道内置服务端域名（`channel.json.defaults.server_url`），登录页免填地址 | 见 ADR-042、`09` §2–§3 |
docs/17-v3.1-decisions.md:55: | 33 | 渠道与版本正交：官方企业版也是渠道（server_url 可空/可指定），OEM 同一链路 | 见 ADR-044 |
docs/07-server-domains.md:35: | 客户端地址 | 企业版客户端从构建期 `channel.json` 的 `defaults.server_url` 取服务端地址；OEM 渠道非空 → 登录页不显示地址输入；官方渠道为空 → 允许首次输入一次 |
docs/05-client-runtime.md:30: - **服务端地址是构建期常量**：企业版 OEM 渠道由 `channel.json` 的 `defaults.server_url` 注入；非空时登录页**只有账号/密码**，
docs/05-client-runtime.md:31: 地址不可编辑；**渠道 `server_url` 为空时（官方企业版默认）登录页显示地址输入**，首次填写并保存到本地（不写回渠道）。
docs/01-vision-and-editions.md:23: - **官方企业版也是一个渠道**（`official`）：默认 `server_url` 为空，客户登录时自己填服务器地址；
docs/01-vision-and-editions.md:114: 官方企业版渠道地址为空时才允许首次输入一次。未登录只显示登录页，无本地功能。
docs/12-roadmap.md:163: **官方企业版也是渠道**（`official`，server_url 默认空由客户配置），OEM 渠道可预置地址；demo 渠道验证 OEM；
AGENTS.md:46: | C14 | **渠道决定登录地址形态**：官方企业版也是渠道（`official`），`server_url` 默认空 → 登录页由客户填写地址；OEM 渠道可设非空 → 登录页**不显示地址输入**（只账号/密码），地址不可编辑；个人版必须空且无登录 | 见 ADR-042/044 |
AGENTS.md:57: - 未登录（企业版）只显示登录页；**登录地址来自构建期 `channel.json`**：非空（渠道预置，OEM 常见）→ 只显示账号/密码；为空（官方企业版默认）→ 显示地址输入，客户自己配置。

## 部门
docs/15-open-questions.md:23: | D12 | 组织数据：可配置多源（**v3.1 起删除部门/用户组**） |
docs/15-open-questions.md:25: | D14 | 角色：**超管 + 审计员（只读）+ 普通员工**；无部门 |
docs/15-open-questions.md:45: | D29 | **删除部门/用户组**（企业实践不再需要）；技能/应用可见性：全员/私有 |
docs/15-open-questions.md:63: | D47 | 技能/应用可见范围：**全员 + 指定用户**（无部门） |
docs/04-contracts-and-api.md:165: | 角色 | 超管 / 审计员（只读）/ 员工；**无部门/用户组**；内容可见性：**全员 / 指定用户** |
docs/04-contracts-and-api.md:259: | Q10 | ~~组织层级（部门）~~ → **已删除部门/用户组**；可见性只保留全员/私有 | — | 已决（v3.1） |
docs/00-design-principles.md:38: | 部门/用户组 | 已删除该功能（企业实践不再需要） |
docs/02-package-model.md:133: | ★`srv.auth` / `srv.users` / `srv.wallet` | 例：`srv.auth.source`、`srv.users`、`srv.wallet` | singleton | 服务端域，一域一实现；OEM 可整域替换（无部门/用户组） |
docs/02-package-model.md:183: | `zk-contract-user` | 用户/角色仓储与查询（**无部门**） | srv-users、admin-ui |
docs/13-risks-and-adr.md:190: **决策**：只有个人钱包余额（不做部门预算，不做 token 配额/金额配额并行）；月发放是"给钱包充值"的一种（v3.1：未折天，全额）；
docs/13-risks-and-adr.md:260: ### ADR-032 · 删除部门/用户组；角色为超管/审计员/员工 `accepted`
docs/13-risks-and-adr.md:262: **决策**：删除组织树、部门归属、部门可见范围、部门管理员；技能/应用可见性 = **全员 / 指定用户**。
docs/13-risks-and-adr.md:302: **决策**：技能/应用可见性 = **全员 / 指定用户**（无部门/私有）；月发放默认**全员**，支持两种模式：
docs/13-risks-and-adr.md:304: **理由**：需求方确认；去掉部门后"指定用户"覆盖例外场景；累加/覆盖覆盖两种企业福利策略。
docs/17-v3.1-decisions.md:30: | 8 | **去掉用户组/部门** | 删除组织树、部门可见范围、部门管理员概念；技能可见范围为全员/私有 |
docs/17-v3.1-decisions.md:46: | 24 | 可见范围 = **全员 / 指定用户**（无部门/私有） | 技能与应用一致；`content_grants` 支持 user |
docs/17-v3.1-decisions.md:63: | `01-vision-and-editions.md` | 已更新：企业版全能力、个人版无应用中心、无部门 |
docs/17-v3.1-decisions.md:66: | `07-server-domains.md` | 已更新：删组织/部门、钱包峰谷价/月发放/透支、JWT 立即失效、技能删除同步、MCP stdio + 客户端 OAuth、员工上传应用、审计员 |
docs/17-v3.1-decisions.md:76: - 技能/应用可见范围：已定「全员 / 指定用户」（D47），无部门。
docs/07-server-domains.md:17: | 认证与会话、用户、钱包与账本、AI 网关与计费、技能/连接器/应用的分发与审批、审计、管理 API、门户与客户端分发 | agent 循环/工具执行、会话存储、工具白名单、网络策略、企业审批、部门/用户组、多租户、部门预算、MCP 流量代理、技能内容执行、应用内 AI、运行时插件加载、LLM 响应缓存 |
docs/07-server-domains.md:95: **口径**：✅ 角色只有**超管 / 审计员 / 员工**；✅ **无部门/用户组**（已删除）；✅ 用户来源可配置（主源同步或管理员手工）。
docs/07-server-domains.md:104: - 不做部门、不做用户组、不做组织树；技能/应用可见性：**全员 / 指定用户**（无私有/部门）。
docs/07-server-domains.md:156: ✅ 16MB 上限；✅ 员工上传需管理员审批；✅ owner=首发布者，管理员可转交；✅ 可见范围**全员/指定用户**（无部门）；
docs/PLAN.md:33: | 无部门/用户组 | 已删除组织树、部门归属、部门管理员；技能/应用可见性 = 全员/指定用户（`07` §2） | 已拍板 |
docs/PLAN.md:81: | ★17 | 删除部门/用户组；角色超管/审计员/员工 | 需求方定 | 032 |
docs/PLAN.md:115: | M2 | 企业版 = 个人版 + 企业管控包 | 6 | 认证三源（单一主源）、JWT、用户（无部门）、钱包、网关计费、客户端登录、管理台核心、docker 部署 |
docs/01-vision-and-editions.md:59: 2. **用户**：超管 / 审计员（只读）/ 员工；**无部门/用户组**；用户来源可配置。
docs/01-vision-and-editions.md:85: - **不做部门/用户组**：已删除该功能；可见性为全员/指定用户。
docs/01-vision-and-editions.md:86: - **不做部门预算**：只有个人钱包（已拍板）。
docs/12-roadmap.md:93: - 服务端：认证（local/OIDC/LDAP 单一主源 + 应急超管）、JWT+refresh、用户（无部门）、钱包（余额/月发放/充值）、
AGENTS.md:39: | C8 | **无部门/用户组**；角色只有超管/审计员/员工；内容可见性 = 全员/指定用户 | 见 ADR-032 |
AGENTS.md:246: | 加回强管控（工具白名单、网络限制、企业审批、部门/用户组） | 已决策弱管控（ADR-030/032） |
AGENTS.md:250: | 服务端执行 agent、执行技能、代理 MCP 流量、多租户、部门预算 | 非目标（`docs/07` §0.1） |

## 工具/网络/审批
docs/15-open-questions.md:42: | D26 | **不做工具白名单**，默认全部允许 |
docs/15-open-questions.md:44: | D28 | **不做企业审批**；员工拥有客户端全部能力（本地审批仍可作为客户端功能） |
docs/15-open-questions.md:60: | D44 | 企业版客户端与管理台都按弱管控设计：目录/模型由服务端下发，但不下发工具/网络/审批策略 |
docs/04-contracts-and-api.md:30: 网络不限制、不做企业审批。契约层只保留三类硬限制与通用工程约束：
docs/04-contracts-and-api.md:34: | C1 | 服务端不控制工具/网络/审批 | 不下发工具白名单、网络策略、审批策略；客户端本地审批由用户决定；API 中不存在 `POLICY_DENIED` 这类"服务端禁止操作"错误 |
docs/09-delivery-and-oem.md:27: | 下发内容 | 模型目录、默认模型、技能/内容清单（**不含工具/网络/审批策略**） | 全部本地设置 |
docs/00-design-principles.md:4: > 客户端能力**完整下发**；工具默认全开、网络默认全开、不做企业审批。
docs/00-design-principles.md:19: | P2 | **客户端能力完整** | 企业版员工拥有客户端全部能力：工具全开、网络全开、本地审批可配；服务端不下发工具白名单/网络策略/审批策略 | 客户端包集合（01/05） |
docs/00-design-principles.md:34: | 工具白名单 / 按工具限制 | 不做；默认全部允许 |
docs/00-design-principles.md:36: | 企业审批（服务端下发审批策略） | 不做"企业审批那套"；员工有全部能力 |
docs/02-package-model.md:64: declares = ["fs.read", "fs.write"]    # 粗粒度能力声明；用于统一审批策略，不是安全边界
docs/13-risks-and-adr.md:102: 不承诺插件沙箱；`[permissions].declares` 只驱动统一审批策略，不是安全边界。
docs/13-risks-and-adr.md:237: 钱包/网关、商店内容、审计、构建期版本；工具默认全开、网络不限制、不做企业审批。本 ADR 仅作历史记录。
docs/13-risks-and-adr.md:246: **不下发**工具白名单/网络策略/审批策略；工具默认全部允许；网络默认开启、允许所有地址；企业不做审批流。
docs/17-v3.1-decisions.md:10: | 服务端职责 | （v2 旧）认证、组织、钱包、网关、技能、连接器、应用、审计、**策略下发/工具白名单** | **（v3.1 现行）只管认证相关 + 商店功能 + 钱包/网关 + 审计 + 管理台** |
docs/17-v3.1-decisions.md:13: | 企业审批 | 服务端下发审批策略，员工不可改 | **不做企业审批那套；员工拥有客户端全部能力** |
docs/17-v3.1-decisions.md:23: | 1 | 服务端弱管控；能力/工具全部下发 | 删除工具白名单、企业审批、网络限制、策略下发中的对应部分 |
docs/17-v3.1-decisions.md:41: | 19 | 企业版员工拥有客户端**全部能力**，无企业审批 | 删除服务端审批策略；客户端审批仍可作为本地功能 |
docs/17-v3.1-decisions.md:42: | 20 | 工具白名单不做，默认全部允许 | 删除策略中的工具白名单与 `POLICY_DENIED` 场景 |
docs/07-server-domains.md:6: > 不做企业审批（`00-design-principles.md`、`17-v3.1-decisions.md`）。
docs/07-server-domains.md:17: | 认证与会话、用户、钱包与账本、AI 网关与计费、技能/连接器/应用的分发与审批、审计、管理 API、门户与客户端分发 | agent 循环/工具执行、会话存储、工具白名单、网络策略、企业审批、部门/用户组、多租户、部门预算、MCP 流量代理、技能内容执行、应用内 AI、运行时插件加载、LLM 响应缓存 |
docs/07-server-domains.md:59: | 认证、JWT 立即吊销、角色（超管/审计员/员工） | 下发工具白名单、限制工具种类 |
docs/07-server-domains.md:61: | 商店：技能/连接器/应用的审核、版本、可见性、强制更新、删除同步 | 企业审批流（危险操作由客户端本地审批，用户自己决定） |
docs/07-server-domains.md:253: - 服务端**不**下发：工具白名单、审批规则、网络策略、员工操作策略。
docs/05-client-runtime.md:264: 以下能力**不受服务端控制**：工具选择、网络访问（默认全开、所有地址）、本地审批策略、会话/文件操作、
docs/05-client-runtime.md:265: 快捷键/主题/语言、本地 MCP（企业版仍可用管理员下发的连接器，个人版自配）。服务端不下发工具白名单、
docs/05-client-runtime.md:266: 网络策略、审批策略；客户端也不提供"被服务端远程限制"的入口。
docs/PLAN.md:19: | 企业版弱管控（服务端只管认证/钱包/商店/审计） | `00-design-principles.md`（v3.1）、ADR-030；工具全开、网络不限、无企业审批 | 已拍板 |
docs/01-vision-and-editions.md:87: - **不做企业审批**：员工拥有客户端全部能力；危险操作由客户端本地审批（用户自己决定）。
docs/12-roadmap.md:95: **目录下发**（模型目录、商店目录、更新通道）——只下发内容，不下发工具/网络/审批策略；
AGENTS.md:37: | C6 | **企业版弱管控**：不做工具白名单、不做网络限制、不做企业审批；服务端只控制身份/钱/商店/构建期 | 见 §11 禁止项 |
AGENTS.md:246: | 加回强管控（工具白名单、网络限制、企业审批、部门/用户组） | 已决策弱管控（ADR-030/032） |
README.md:25: 网络不限制、不做企业审批（见 `docs/00`、`docs/17`）。

## Responses
docs/15-open-questions.md:28: | D17 | AI 网关：**只支持 OpenAI Responses API** |
docs/04-contracts-and-api.md:76: | `POST /v1/responses` | **唯一网关协议**：OpenAI Responses API | Bearer `gateway:chat` | input/instructions/tools；返回 response 对象或 SSE 流；工具调用子树透传 | 否；仅在首字节前可换 provider | WALLET_EXHAUSTED、MODEL_NOT_PRICED、RAT
docs/04-contracts-and-api.md:250: | Q1 | ~~网关协议~~ → **已定：只支持 Responses API** | — | 已决（v3.1） |
docs/13-risks-and-adr.md:253: ### ADR-031 · AI 网关只支持 OpenAI Responses API `accepted`
docs/13-risks-and-adr.md:255: **决策**：企业版网关只实现 `POST /v1/responses`（流式 SSE + 工具调用透传）；不做 Chat Completions。
docs/13-risks-and-adr.md:257: **理由**：需求方明确"企业版只支持 Responses"；减少协议面与测试矩阵。
docs/13-risks-and-adr.md:258: **后果**：客户端网关适配只做 Responses；第三方只兼容 Chat Completions 的模型需要上游网关转换（或后续再加）。
docs/13-risks-and-adr.md:422: | R26 | 只支持 Responses API 导致部分模型/客户端兼容性下降 | M | M | 上游 provider 仍为 OpenAI 兼容；必要时在上游做转换；客户端只做 Responses 适配 | 兼容性测试矩阵；用户反馈 |
docs/17-v3.1-decisions.md:25: | 3 | 企业版网关只支持 **Responses API** | 删除 `/v1/chat/completions`；只保留 `/v1/responses`（流式/工具调用） |
docs/17-v3.1-decisions.md:64: | `04-contracts-and-api.md` | 已更新：Responses-only、审计员角色、无策略白名单、事件补充 `skill.deleted` |
docs/07-server-domains.md:140: **口径**：✅ 唯一 AI 出口；✅ 只支持 **OpenAI Responses API**（`POST /v1/responses`，流式 SSE + 工具调用透传）；
docs/07-server-domains.md:142: ✅ 流式超额允许透支；⛔ 不做 Chat Completions、不做响应缓存。
docs/PLAN.md:35: | AI 网关 + 计费 | 服务端转发、密钥不出服务端、token 计量与价格表；**只支持 Responses API**（`07` §4） | 已拍板 |
docs/PLAN.md:80: | ★16 | 网关只支持 Responses API | 需求方定；减少协议面 | 031 |
docs/PLAN.md:138: | 网关只支持 Responses 导致兼容性下降 | 上游仍可兼容 OpenAI；客户端只做 Responses；后续按需扩展（ADR-031） |
docs/12-roadmap.md:213: Responses-only、月发放全员/累加或覆盖、峰谷默认 9–18、WASM 语言集 Go+Rust、可见性全员/指定用户。
AGENTS.md:38: | C7 | **企业版 AI 只走网关**：只支持 OpenAI **Responses API**；客户端不得持有上游 key、不得填 base_url | 见 ADR-031 |
AGENTS.md:247: | 支持 Chat Completions（企业网关） | 只支持 Responses（ADR-031） |
README.md:92: | v3.1 决策记录（弱管控/Responses/无签名等） | [`docs/17-v3.1-decisions.md`](docs/17-v3.1-decisions.md) |

## Windows/Mac
docs/15-open-questions.md:52: | D36 | **不做 Windows 代码签名**；macOS 公证待定 |
docs/15-open-questions.md:65: | D49 | macOS **需要公证**；证书以后再配；CI 先做好；**当前 TODO：本地测试公证流程** |
docs/15-open-questions.md:69: | D53 | 其余按推荐：峰时默认 9–18 点（可配/可按模型覆盖）；MCP OAuth 令牌存客户端；每应用 SQLite 100MB/16 表；审计员只读范围=审计/用量/登录/内容变更；Windows 未签名提示 SmartScreen；深度 OEM 只实现契约；OEM 构建统一由我方 CI；允许导出会话/文件；允许多设备；管理台 MFA 可选；个人版本地 MCP 后补；更新通道 sta
docs/15-open-questions.md:89: | Q8 | ~~macOS 公证~~ → **已定：需要公证；证书以后配置，CI 先做好**（D49） | 已决 |
docs/15-open-questions.md:90: | Q9 | ~~Windows 提示~~ → 按推荐：门户显式提示 + 文档步骤（D53） | 已决 |
docs/10-repo-and-build.md:146: - `.gitattributes`：`* text=auto eol=lf`（Windows checkout 不影响 `--check` 字节比对）。
docs/10-repo-and-build.md:149: - CI 从 M0 起三平台 `cargo check`；M1 起三平台安装包冒烟；M5 起签名/公证。
docs/10-repo-and-build.md:150: - 交叉编译：Windows/Linux 用对应 runner 原生构建，macOS 用 macOS runner；不在 Linux 交叉编 mac。
docs/09-delivery-and-oem.md:168: ②参考实现 Windows 安装包完全没签名 → **v3.1 明确不做 Windows 代码签名**（§7）；③多入口 = 多份真相（派生了但没人调用）→ 每个"必须发生"的派生/校验
docs/09-delivery-and-oem.md:195: Windows `…-windows-x64-setup.exe`；个人版 `zeokeel-personal-<ver>-<os>-<arch>.<ext>`；镜像 tag `zeokeel/server:<ver>-<channel>`，
docs/09-delivery-and-oem.md:265: | 能力 | 本地对话、工具、审批、会话存储；AI 需用户自配 provider 与 key（Windows DPAPI / macOS Keychain / Linux secret-service；无 keychain 时加密文件 + 明确警告） |
docs/09-delivery-and-oem.md:290: | 校验 | HTTPS + `latest.json` 签名 + 安装包 sha256；Windows 未签名（提示 SmartScreen），macOS 公证后校验签名 |
docs/09-delivery-and-oem.md:322: | Windows | **不签名**（v3.1 已定）；依赖 HTTPS + 清单签名 + sha256；安装提示 SmartScreen |
docs/09-delivery-and-oem.md:346: | Q1 | ~~Windows 签名主体~~ → **Windows 不签名**（所有渠道）；macOS 公证是否保留 | 待确认 macOS 证书 |
docs/09-delivery-and-oem.md:354: | Q9 | macOS 公证方式（ASC API key vs Apple ID）；Linux 是否 minisign | ASC API key 可自动化 |
docs/11-testing-and-evidence.md:46: - 安装包：三平台安装/卸载/自动更新；签名校验（Windows/macOS）。
docs/11-testing-and-evidence.md:66: | 未签名 | Windows/macOS 安装包未签名（发布 job） |
docs/11-testing-and-evidence.md:117: | 平台差异 | Linux WebKitGTK / Windows WebView2 / macOS WKWebView 冒烟 |
docs/13-risks-and-adr.md:267: ### ADR-033 · 不做 Windows 代码签名 `accepted`
docs/13-risks-and-adr.md:269: **决策**：Windows 安装包不签名；保留 SHA256 校验和与更新清单签名；文档提示 SmartScreen 风险。
docs/13-risks-and-adr.md:270: macOS 公证是否保留待定（需 Apple 证书）。
docs/13-risks-and-adr.md:271: **理由**：需求方明确"Windows 代码签名证书不做"。
docs/13-risks-and-adr.md:272: **后果**：Windows 首次安装有 SmartScreen 警告；企业分发需在文档中说明。
docs/13-risks-and-adr.md:307: ### ADR-037 · macOS 公证：CI 先就绪，证书后配；当前本地测试（TODO）`accepted`
docs/13-risks-and-adr.md:309: **决策**：macOS 安装包要求公证 + staple；CI 流水线先实现并默认跳过，待配置 Apple 证书/API key 后启用。
docs/13-risks-and-adr.md:310: Windows 仍不签名（ADR-033）。
docs/13-risks-and-adr.md:311: **理由**：需求方确认"需要公证，但证书以后再配置，CI 先做好"；**当前状态：TODO——先在本地测试公证流程，证书/密钥就绪后再接入 CI**。
docs/13-risks-and-adr.md:312: **后果**：发布流程需检测证书缺失并给出明确提示；无证书时产物带"未公证"标记，不阻塞内部构建。
docs/13-risks-and-adr.md:410: | R15 | 跨平台（Windows CRLF、WebView2、路径）拖到 M4 | H | M | `.gitattributes` 强制 LF；M1 起 3-OS `cargo check`；每平台冒烟 | 3-OS job 恒开 |
docs/13-risks-and-adr.md:418: | R22 | Windows 签名缺失（参考实现的硬缺口） | M | H | M5 前采购证书；CI 签名 job；未签名产物不发布 | 发布门禁校验签名 |
docs/13-risks-and-adr.md:423: | R27 | 不做 Windows 签名导致安装警告与分发摩擦 | H | L | 文档/门户提示 SmartScreen；提供 SHA256；OEM 如需签名自行提供证书 | 安装文档检查；客户验收清单 |
docs/13-risks-and-adr.md:435: | macOS 公证证书 | 本地先测，证书后配（ADR-037） |
docs/17-v3.1-decisions.md:39: | 17 | **不做 Windows 代码签名** | 交付文档与路线图移除；安装时提示 SmartScreen 风险 |
docs/17-v3.1-decisions.md:48: | 26 | macOS **需要公证**；证书后配，CI 先就绪 | ADR-037；Windows 仍不签 |
docs/17-v3.1-decisions.md:68: | `09-delivery-and-oem.md` | 已更新：不做 Windows 签名；索引仓自建（参考仓仅作字段设计） |
docs/07-server-domains.md:274: | Q11 | macOS 公证 | 需要；**证书待配置，CI 先做好**（外部条件） |
docs/16-ui-packages.md:200: | 签名/公证/更新 | ✅ 成熟 | 自己造 | 自己造 | 中 |
docs/PLAN.md:82: | ★18 | 不做 Windows 代码签名 | 无证书预算 | 033 |
docs/PLAN.md:118: | M5 | OEM 与交付 | 4 | 渠道/品牌、OEM 索引仓+submodule、个人版/企业版+OEM 产物、更新、门户（Windows 不签名） |
docs/PLAN.md:130: WASM 应用曾内置 AI 能力（已删除，我们不提供）；Windows 安装包不签名（我们必须签）。
docs/PLAN.md:144: | Windows 签名缺失 | 采购证书；CI 签名 job 为 M5 退出条件 |
docs/12-roadmap.md:17: | **M5** | OEM 与交付 | 4 | 渠道/品牌、OEM submodule、双版本+OEM 产物、更新、门户（**Windows 不签名**） | M4 |
docs/12-roadmap.md:75: - 三平台安装包（**Windows 不签名**，v3.1 决策）、**GitHub Releases** 更新源。
docs/12-roadmap.md:166: - 签名：**Windows 不签名**（安装时提示 SmartScreen）、**macOS 公证 + staple**（CI 先做好，证书后配）、Linux AppImage 校验和；
docs/12-roadmap.md:174: - [ ] macOS 公证 CI 就绪（拿到证书后一键启用）；Windows 安装包附 SHA256 并在文档提示未签名。
docs/12-roadmap.md:206: | E 交付/OEM | docker、安装包（Windows 不签名）、渠道、OEM 流水线 | A–D 产物 |
AGENTS.md:230: - Windows 安装包**不签名**（已定）；macOS 公证 CI 先就绪、证书后配（见 §12 TODO）。
AGENTS.md:248: | 做 Windows 代码签名 | 已决策不做（ADR-033） |
AGENTS.md:261: - macOS 公证：**本地先测**，证书后配，CI 先就绪（ADR-037）。

## unsafe
docs/03-composition-and-ci.md:14: | C. 动态库 | `dylib` / `abi_stable` / 插件 `.so` | 否 | ❌（要带若干 .so） | 与"无 ABI、无 unsafe、静态链接"直接冲突 | 否 |
docs/10-repo-and-build.md:154: - Rust：`clippy -D warnings`；**所有 crate `#![forbid(unsafe_code)]`（零例外，含 wasmtime 集成层）**；
docs/10-repo-and-build.md:155: 第三方依赖的 unsafe 用 `cargo-geiger`/`cargo-deny` 审计，不自己写 unsafe。
docs/13-risks-and-adr.md:11: 静态链接、`#![forbid(unsafe_code)]`。
docs/13-risks-and-adr.md:325: （`unsafe_code=forbid`、`clippy::all/pedantic/nursery/cargo=deny`、`unwrap_used/expect_used/panic/
docs/13-risks-and-adr.md:332: ### ADR-040 · Rust 手写代码零 `unsafe`（产品 crate forbid）`accepted`
docs/13-risks-and-adr.md:334: **决策**：本仓库**手写** Rust 代码零 `unsafe`：产品 crate（`packages/`、`apps/`、`tools/`、`crates/`）
docs/13-risks-and-adr.md:335: 一律 `#![forbid(unsafe_code)]`；禁止手写 `unsafe` 与 `#[allow(unsafe_code)]`。FFI/WASM 导入只允许来自
docs/13-risks-and-adr.md:336: ①自动生成绑定（`@generated`）或 ②独立、最小、审计并登记 ADR 的 `*_sys` crate；第三方依赖内的 unsafe
docs/13-risks-and-adr.md:338: **理由**：需求方明确"Rust 不允许使用 unsafe"；把内存安全从"review 靠人"变成"编译期不可能"。
docs/13-risks-and-adr.md:339: **边界**：完全依赖树（含 wasmtime/Tauri）零 unsafe 不可行——那等于放弃 WASM 运行时与桌面壳；
docs/13-risks-and-adr.md:341: **后果**：CI 增加 `cargo geiger` 报告与源码 `unsafe` 扫描；任何 `allow(unsafe_code)` 直接红。
docs/18-strictness.md:107: unsafe_code = "forbid"
docs/18-strictness.md:132: - **Rust 手写代码零 `unsafe`**：产品 crate（`packages/`、`apps/`、`tools/`、`crates/`）每个
docs/18-strictness.md:133: `#![forbid(unsafe_code)]`，禁止手写 `unsafe` 块/函数/trait impl，禁止手写 `#[allow(unsafe_code)]`。
docs/18-strictness.md:135: ② 独立 `*_sys` crate（必须最小、锁定、审计并在 ADR 登记）。业务/UI/服务端代码不允许出现 unsafe。
docs/18-strictness.md:136: - 第三方依赖内的 unsafe 通过锁定版本 + `cargo-geiger` + `cargo-deny` 管理；**不接受为了性能/方便的例外**。
docs/18-strictness.md:137: - 若要求"连生成绑定/依赖也零 unsafe"，则 Rust WASM guest 无法调用宿主函数（Rust FFI 语义要求 unsafe），
docs/18-strictness.md:183: cargo geiger --all-features # 依赖树 unsafe 统计（报告变化；新增高危需说明）
docs/18-strictness.md:197: - `unsafe_code` 的 `forbid` **不允许任何 allow**；若第三方 API 必须 unsafe 包装，改用提供安全封装的依赖，
docs/18-strictness.md:198: 或把该能力隔离到独立进程/服务（不在本仓写 unsafe）。
AGENTS.md:41: | C9b | **Rust 手写代码零 `unsafe`**：所有产品 crate（packages/apps/tools/crates）`#![forbid(unsafe_code)]`，禁止手写 `unsafe`/`#[allow(unsafe_code)]`；FFI/WASM 导入只能用**自动生成绑定**（`@generated`，单独审计）或独立 sys crate；第三方与生成代码内
AGENTS.md:188: - workspace lints 全开（`docs/18` §4）：`unsafe_code=forbid`、`clippy all/pedantic/nursery/cargo`,
AGENTS.md:192: - 库用 `thiserror`；边界用 `anyhow`；**每个 crate `#![forbid(unsafe_code)]`**（零例外，禁止 `#[allow(unsafe_code)]`）。
AGENTS.md:193: - 需要底层能力（WASM 运行时、系统 API）时，使用经过审计的依赖（wasmtime/tauri 等），**不自己写 unsafe**；
AGENTS.md:194: 依赖内的 unsafe 由锁定版本 + `cargo-geiger`/`cargo-deny` 审计管理（见 `docs/18` §4）。
AGENTS.md:252: | 写 `unsafe` / `#[allow(unsafe_code)]` / 为性能放宽 forbid | 零 unsafe 政策（C9b） |

## 严格检查
docs/10-repo-and-build.md:154: - Rust：`clippy -D warnings`；**所有 crate `#![forbid(unsafe_code)]`（零例外，含 wasmtime 集成层）**；
docs/13-risks-and-adr.md:11: 静态链接、`#![forbid(unsafe_code)]`。
docs/13-risks-and-adr.md:323: **决策**：TypeScript 开启 `strict` 及全部严格附加项（`noUncheckedIndexedAccess`、`exactOptionalPropertyTypes` 等），
docs/13-risks-and-adr.md:324: 禁用 `any`/`as`；ESLint 用 `strictTypeChecked` + `--max-warnings 0`；Rust 用 workspace lints
docs/13-risks-and-adr.md:325: （`unsafe_code=forbid`、`clippy::all/pedantic/nursery/cargo=deny`、`unwrap_used/expect_used/panic/
docs/13-risks-and-adr.md:332: ### ADR-040 · Rust 手写代码零 `unsafe`（产品 crate forbid）`accepted`
docs/13-risks-and-adr.md:335: 一律 `#![forbid(unsafe_code)]`；禁止手写 `unsafe` 与 `#[allow(unsafe_code)]`。FFI/WASM 导入只允许来自
docs/18-strictness.md:30: "noUncheckedIndexedAccess": true,    // arr[i] 可能是 undefined
docs/18-strictness.md:62: ...tseslint.configs.strictTypeChecked,
docs/18-strictness.md:107: unsafe_code = "forbid"
docs/18-strictness.md:133: `#![forbid(unsafe_code)]`，禁止手写 `unsafe` 块/函数/trait impl，禁止手写 `#[allow(unsafe_code)]`。
docs/18-strictness.md:197: - `unsafe_code` 的 `forbid` **不允许任何 allow**；若第三方 API 必须 unsafe 包装，改用提供安全封装的依赖，
AGENTS.md:41: | C9b | **Rust 手写代码零 `unsafe`**：所有产品 crate（packages/apps/tools/crates）`#![forbid(unsafe_code)]`，禁止手写 `unsafe`/`#[allow(unsafe_code)]`；FFI/WASM 导入只能用**自动生成绑定**（`@generated`，单独审计）或独立 sys crate；第三方与生成代码内
AGENTS.md:188: - workspace lints 全开（`docs/18` §4）：`unsafe_code=forbid`、`clippy all/pedantic/nursery/cargo`,
AGENTS.md:192: - 库用 `thiserror`；边界用 `anyhow`；**每个 crate `#![forbid(unsafe_code)]`**（零例外，禁止 `#[allow(unsafe_code)]`）。
AGENTS.md:252: | 写 `unsafe` / `#[allow(unsafe_code)]` / 为性能放宽 forbid | 零 unsafe 政策（C9b） |

## bootstrap
docs/03-composition-and-ci.md:222: `required=true` 的包失败 → 由 bootstrap `dispose_all()` 后进入诊断页并退出；`required=false` 的包失败
docs/03-composition-and-ci.md:231: `platform.shell` 包），`main.rs` 固定为 `fn main() { zk_bootstrap::run() }`，不含任何具体平台 crate 名。
docs/04-contracts-and-api.md:35: | C2 | 能力完整下发 | bootstrap **只下发**模型目录、商店目录（技能/连接器/应用）、更新清单、渠道/品牌信息；不下发"能力开关" |
docs/04-contracts-and-api.md:62: ### 2.2 bootstrap/config — 配置下发
docs/04-contracts-and-api.md:67: | `GET /config/bootstrap` | 登录后全量配置 | Bearer `config:read` | resp: schema_version、identity、gateway（模型目录/默认模型）、skills、connectors、apps、updates、brand、config_etag；**不含工具/审批/网络策略**；`If-None-Match`→304 | 是 
docs/04-contracts-and-api.md:79: 规则：`base_url` 来自 bootstrap，客户端**不持有上游 key**；上游选择、故障转移、计量与扣费全在服务端（准入在转发前、usage 与账本同事务）；错误不回显上游细节；流式中断以 SSE error 帧显式结束。
docs/04-contracts-and-api.md:140: | 离线 | 本地 agent 可用；服务端依赖功能（AI/技能安装/连接器/应用/更新）以类型化错误降级；缓存 bootstrap（ETag/到期）与已装技能；禁止离线续期 |
docs/04-contracts-and-api.md:169: 登录后 `GET /config/bootstrap` 原子下发；客户端以 `config_etag` 做条件请求，失败保留 last-known-good，不进入半初始化状态。
docs/04-contracts-and-api.md:210: | 重放 | 断线后带 `from_seq` 续订；缓冲已过期 ⇒ 服务端回 `resync_required{topic, latest_seq}`，客户端整量重取（bootstrap/目录）后从 latest 继续 |
docs/10-repo-and-build.md:35: ├─ scripts/                    # bootstrap/dev/test/ci/release/evidence
docs/10-repo-and-build.md:134: bash scripts/bootstrap.sh                 # 系统库 + pnpm + cargo check + 生成
docs/06-server-runtime.md:11: ├─ bootstrap（唯一非包代码）
docs/06-server-runtime.md:92: - 配置读取经 `zk-contract-config` 的 `ConfigService`；包不得直接读环境变量（除 bootstrap 注入的少数）。
docs/13-risks-and-adr.md:77: **决策**：以下不是包：`crates/zk-bootstrap`、`apps/desktop/src/main.rs`、web 的 `main.tsx`、
docs/13-risks-and-adr.md:81: **后果**：bootstrap 的代码审查按"平台代码"标准，任何业务逻辑进入即 review 拒绝。
docs/13-risks-and-adr.md:156: `zk_bootstrap::run()`。
docs/07-server-domains.md:251: - 客户端→服务端：登录/刷新、bootstrap（**只下发模型目录、商店目录、更新清单、渠道信息**；不下发工具/审批/网络策略）、
docs/05-client-runtime.md:15: │ │ apps/desktop  ── bootstrap（唯一非包代码）                            │ │
docs/05-client-runtime.md:41: - 运行时只允许一个 `BlockingExecutor`（进程级 `OnceLock`，先装者赢）——所以它由 bootstrap 装，
docs/05-client-runtime.md:50: | H1 | 组合根 / 包清单 | `crates/zk-composed`（生成）+ bootstrap | profile → 注册表，编译期固定 |
docs/05-client-runtime.md:54: | H5 | 关闭编排 | bootstrap/runtime | 固定顺序：`when_idle` → `dispose_all`（重试 `LifecycleBusy`）→ `drain_orphan_async_cleanups` → 关 tokio → 关窗口 |
docs/05-client-runtime.md:64: | ★H16 | 服务端配置同步 | `client-bootstrap` 包 | 登录后拉 provider/模型目录/技能源/连接器源/应用源/更新源，写本地配置 |
docs/05-client-runtime.md:91: bootstrap `dispose_all()` 重建/退出，而不是指望"撤销已成功的包"。
docs/16-ui-packages.md:212: `fn main() { zk_bootstrap::run() }`——**换壳只改 profile，不改应用代码**（评审 S7）。
docs/PLAN.md:23: | 两端都用 zeokeel-cordis | 同一套包模型/组合器/契约；客户端与服务端各一个 bootstrap（`02`、`03`） | 方案定 |
docs/12-roadmap.md:96: - 客户端：登录/刷新、bootstrap 配置下发、网关 provider（不可自配）、未登录态处理；
AGENTS.md:134: bash scripts/bootstrap.sh          # 系统库 + pnpm + cargo check + 生成

## WASM 语言
docs/15-open-questions.md:61: | D45 | WASM v1 语言集：**Go(wasip1) + Rust(wasip1)** Tier-1；TinyGo 预览；C/C++ 仅 vendor；wasip2 与其他暂缓（M4 先 Go，GA 前加 Rust） |
docs/15-open-questions.md:121: - 按编号回答即可（"Q1 Go+Rust；Q3 9–18 点峰时…"）。
docs/13-risks-and-adr.md:295: **原型证据**：`experiments/wasm-poc` 已用 wasmtime 50 同时跑通 Go/Rust guest 的帧协议、宿主 DB、SQLite 持久与 fuel 限额。
docs/17-v3.1-decisions.md:45: | 23 | WASM 语言与 ABI 已调研（`research/wasm-languages.md`） | v1 Tier-1：Go(wasip1)+Rust(wasip1)；帧 `RS<len>\n<JSON>` + `db.*` 宿主函数；M4 先 Go，GA 前加 Rust；Wasmtime 锁 LTS |
docs/07-server-domains.md:196: **语言（已调研，`research/wasm-languages.md`）**：v1 Tier-1 = **Go(wasip1) + Rust(wasip1)**，共用 p1 core module；
docs/07-server-domains.md:269: | Q6 | WASM 语言 | Go+Rust（ADR-035） |
docs/18-strictness.md:138: 只能改用纯 WASI（文件/管道）通信或放弃 Rust guest——需要产品决策。
docs/12-roadmap.md:136: **目标**：员工可上传 WASM 小应用（管理员审核/上架），客户端内独立窗口使用。**M4 先用 Go(wasip1) 打通，GA 前加 Rust**（TinyGo 预览）。
docs/12-roadmap.md:213: Responses-only、月发放全员/累加或覆盖、峰谷默认 9–18、WASM 语言集 Go+Rust、可见性全员/指定用户。
AGENTS.md:150: # WASM 应用 POC：host + Go/Rust guest + 帧 ABI + SQLite + fuel

## 审计保留
docs/15-open-questions.md:68: | D52 | 审计保留 ≥180 天、用量 ≥12 个月；**可配置但不得低于下限** |
docs/15-open-questions.md:103: | Q17 | ~~保留期~~ → **已定：审计 ≥180 天、用量 ≥12 个月；可配置但不得低于下限**（D52） | 已决 |
docs/11-testing-and-evidence.md:65: | 保留期 | 审计 <180 天或用量 <12 个月（配置项低于下限） |
docs/17-v3.1-decisions.md:51: | 29 | 审计 ≥180 天、用量 ≥12 个月；可配置但不得低于下限 | `system_settings.min_value` |
docs/07-server-domains.md:221: - 保留期：审计 ≥180 天、用量 ≥12 个月；**可配置但不得低于下限**（`system_settings.min_value` 校验）。
AGENTS.md:66: - 审计 ≥180 天、用量 ≥12 个月，可配置但不得低于下限。
AGENTS.md:229: - 审计写入失败时业务 fail-closed；审计/用量保留期不得低于 180 天/12 个月。

## 应用限额
docs/15-open-questions.md:69: | D53 | 其余按推荐：峰时默认 9–18 点（可配/可按模型覆盖）；MCP OAuth 令牌存客户端；每应用 SQLite 100MB/16 表；审计员只读范围=审计/用量/登录/内容变更；Windows 未签名提示 SmartScreen；深度 OEM 只实现契约；OEM 构建统一由我方 CI；允许导出会话/文件；允许多设备；管理台 MFA 可选；个人版本地 MCP 后补；更新通道 sta
docs/15-open-questions.md:82: | Q6 | ~~SQLite 限额~~ → **已定：100MB / 16 表**（D53） | 已决 |
docs/07-server-domains.md:209: 每应用 SQLite 上限 **100MB / 16 表**；退役作业：软删 → 保留期 → 真删（含 DB 文件）。
docs/07-server-domains.md:270: | Q7 | 每应用 SQLite | 100MB / 16 表 |

## JWT
docs/15-open-questions.md:46: | D30 | 员工**禁用后 JWT 立即失效**（`token_epoch` 每请求校验） |
docs/04-contracts-and-api.md:36: | C3 | 三类硬限制 | ① 企业版 AI 只能走服务端网关（客户端不持上游 key、不填 base_url）；② 企业内容只能从企业商店安装；③ 禁用用户令牌立即失效 |
docs/04-contracts-and-api.md:134: | access JWT | 短效 15 min；签名 EdDSA，`kid` 轮换、算法白名单；claims: iss/aud/sub/uid/role/epoch/device_id/channel/jti/iat/exp；服务端用 `users.token_epoch` 实现全局失效（缓存窗口建议 5 s，管理面写操作强制查库） |
docs/04-contracts-and-api.md:138: | 吊销 | logout、改密、降权、禁用、管理员强制下线、设备移除；refresh 立即失效，access 最迟一个 TTL 内失效；WS 推 `session.revoked`（§8） |
docs/00-design-principles.md:43: 3. 员工被禁用后**立即失效**——认证硬约束。
docs/06-server-runtime.md:85: - 员工禁用/降权 → `token_epoch++`，每请求校验（写操作强制查库），**立即失效**，不等 access 过期（见 07 §1）。
docs/13-risks-and-adr.md:221: **v3.1 追加**：员工被禁用/降权后**立即失效**——每请求校验 `token_epoch`（写操作强制查库），不等 access 过期。
docs/17-v3.1-decisions.md:31: | 9 | 员工禁用后 JWT **立即失效** | 每请求校验 `token_epoch`（或等价版本），不再等 access 过期 |
docs/17-v3.1-decisions.md:66: | `07-server-domains.md` | 已更新：删组织/部门、钱包峰谷价/月发放/透支、JWT 立即失效、技能删除同步、MCP stdio + 客户端 OAuth、员工上传应用、审计员 |
docs/07-server-domains.md:21: 1. **身份**：谁能登录、角色是什么、禁用后立即失效；
docs/07-server-domains.md:59: | 认证、JWT 立即吊销、角色（超管/审计员/员工） | 下发工具白名单、限制工具种类 |
docs/07-server-domains.md:71: ✅ 禁用/降权后**立即失效**；✅ 钉钉/企微/飞书后续为独立包（形态按各家官方，优先级低）。
docs/07-server-domains.md:75: | `users` | id, username(唯一), display_name, role(`super_admin`\|`auditor`\|`employee`), source, status, password_hash(argon2id, 仅 local), `token_epoch`, last_login_at |
docs/07-server-domains.md:87: 2. **立即失效**：每请求校验 `token_epoch`（5s 缓存，写操作强制查库）；禁用/改密/降权 → `token_epoch++`
docs/07-server-domains.md:102: - 删除用户：软删 + 令牌立即失效 + 保留审计；其上传内容按规则转交或下架（见 §5/§7）。
AGENTS.md:62: - 认证：单一主源（local/OIDC/LDAP）+ 本地应急超管；JWT access + refresh；禁用后**每请求校验 `token_epoch` 立即失效**。
AGENTS.md:226: - 企业版客户端不得持有上游 key；AI 只经网关；禁用用户令牌立即失效。

## 角色
docs/08-admin-console.md:90: - **角色**：超管（全部写权限）、**审计员（只读）**、员工（无管理台权限）。审计员仅可访问审计、用量、
docs/08-admin-console.md:109: | 会话 | Cookie（HttpOnly/Secure/SameSite）+ CSRF token；空闲超时；超管/审计员可登录（审计员只读） |
docs/08-admin-console.md:119: - 审计员可见页面范围（建议：审计/用量/登录/内容变更，只读）。
docs/15-open-questions.md:25: | D14 | 角色：**超管 + 审计员（只读）+ 普通员工**；无部门 |
docs/15-open-questions.md:26: | D15 | 认证：单一主源 + 本地应急超管；钉钉/企微/飞书后续为包（优先级低，按官方形态） |
docs/15-open-questions.md:57: | D41 | 应急超管 + 单一主源；审计员只读 |
docs/15-open-questions.md:59: | D43 | 管理台需要**只读审计员**角色 |
docs/15-open-questions.md:69: | D53 | 其余按推荐：峰时默认 9–18 点（可配/可按模型覆盖）；MCP OAuth 令牌存客户端；每应用 SQLite 100MB/16 表；审计员只读范围=审计/用量/登录/内容变更；Windows 未签名提示 SmartScreen；深度 OEM 只实现契约；OEM 构建统一由我方 CI；允许导出会话/文件；允许多设备；管理台 MFA 可选；个人版本地 MCP 后补；更新通道 sta
docs/15-open-questions.md:83: | Q7 | ~~审计员范围~~ → **已定：审计/用量/登录/内容变更，只读**（D53） | 已决 |
docs/04-contracts-and-api.md:133: | 认证主源 | 服务端只启用**一种主源**：local / OIDC / LDAP（三选一）＋一个本地应急超管账号；应急账号仅 break-glass，不参与日常登录，登录必审计并告警 |
docs/04-contracts-and-api.md:157: **角色**：`super_admin`（超管，可登录管理端；是否可登录客户端另定）与 `employee`（普通员工，仅客户端）。权限点形如 `<域>:<动作>[:<范围>]`；员工默认持有 `auth:self`、`config:read`、`gateway:chat`、`skills:read`、`skills:publish`、`connectors:use`、`apps:use`、`te
docs/04-contracts-and-api.md:165: | 角色 | 超管 / 审计员（只读）/ 员工；**无部门/用户组**；内容可见性：**全员 / 指定用户** |
docs/00-design-principles.md:50: | 超管 | 同员工 + 管理台全部写权限 | 全部 |
docs/00-design-principles.md:51: | 审计员 | 同员工 | **只读**：审计、用量、登录事件、内容变更记录；不能改任何配置 |
docs/06-server-runtime.md:56: 2. **权限点同处申报**：权限常量是枚举（超管/员工），路由注册时一并声明，CI 生成"全量端点权限矩阵"。
docs/06-server-runtime.md:82: | `/api/admin/*` + `/admin/*` | Cookie session + CSRF | 管理台 | 超管（读写）/审计员（只读）；TOTP MFA（待确认） |
docs/13-risks-and-adr.md:217: ### ADR-027 · 认证单一主源 + 本地应急超管；JWT + refresh `accepted`
docs/13-risks-and-adr.md:219: **决策**：企业同一时间只启用一个主认证源（local/OIDC/LDAP），本地应急超管始终可用；员工客户端用
docs/13-risks-and-adr.md:260: ### ADR-032 · 删除部门/用户组；角色为超管/审计员/员工 `accepted`
docs/13-risks-and-adr.md:263: 管理台新增**只读审计员**角色（审计/用量/登录/内容变更，只读）。
docs/13-risks-and-adr.md:264: **理由**：需求方"去掉用户组，现在企业都不怎么做这种东西"；审计员满足合规查看需求。
docs/17-v3.1-decisions.md:44: | 22 | 管理台增加**只读审计员**角色 | 角色：超管、审计员（只读）、员工；审计员仅能看审计/用量等只读页 |
docs/17-v3.1-decisions.md:64: | `04-contracts-and-api.md` | 已更新：Responses-only、审计员角色、无策略白名单、事件补充 `skill.deleted` |
docs/17-v3.1-decisions.md:66: | `07-server-domains.md` | 已更新：删组织/部门、钱包峰谷价/月发放/透支、JWT 立即失效、技能删除同步、MCP stdio + 客户端 OAuth、员工上传应用、审计员 |
docs/17-v3.1-decisions.md:67: | `08-admin-console.md` | 已更新：审计员角色、弱管控说明 |
docs/17-v3.1-decisions.md:80: - 审计员的具体只读页面范围
docs/07-server-domains.md:59: | 认证、JWT 立即吊销、角色（超管/审计员/员工） | 下发工具白名单、限制工具种类 |
docs/07-server-domains.md:70: **口径**：✅ 单一主源（`local` / `OIDC` / `LDAP` 三选一，换源=换包）+ 本地应急超管；✅ 短效 JWT + refresh；
docs/07-server-domains.md:75: | `users` | id, username(唯一), display_name, role(`super_admin`\|`auditor`\|`employee`), source, status, password_hash(argon2id, 仅 local), `token_epoch`, last_login_at |
docs/07-server-domains.md:82: （仅本地超管，Cookie+CSRF）。
docs/07-server-domains.md:90: 4. 应急超管：仅当库中无超管时由部署期创建，审计打 `emergency=true`；🟡 是否限制来源网段（Q1）。
docs/07-server-domains.md:91: 5. 审计员：只读角色，仅能访问审计/用量/登录事件/内容变更记录页，**不能**改任何配置（`08` §6）。
docs/07-server-domains.md:95: **口径**：✅ 角色只有**超管 / 审计员 / 员工**；✅ **无部门/用户组**（已删除）；✅ 用户来源可配置（主源同步或管理员手工）。
docs/07-server-domains.md:103: - 用户列表/搜索/启停/改角色（仅超管）；审计员只读。
docs/07-server-domains.md:220: - 审计写入在业务事务内（写失败即业务失败）；审计员只读查询。
docs/07-server-domains.md:247: `admin-apps`、`admin-connectors`、`admin-audit`（审计员可读）、`admin-settings`、`admin-ui-kit`。
docs/07-server-domains.md:264: | Q1 | 应急超管来源限制 | 仅本机/内网（推荐，未反对） |
docs/07-server-domains.md:271: | Q8 | 审计员范围 | 审计/用量/登录/内容变更，只读 |
docs/PLAN.md:32: | 认证：本地/OIDC/LDAP | 单一主源 + 本地应急超管；钉钉/企微/飞书后续为包（`07` §1） | 已拍板 |
docs/PLAN.md:78: | ★14 | 认证单一主源 + 本地应急超管；JWT+refresh | 避免多源冲突；桌面端需要无 cookie 会话 | 027 |
docs/PLAN.md:81: | ★17 | 删除部门/用户组；角色超管/审计员/员工 | 需求方定 | 032 |
docs/01-vision-and-editions.md:34: | 登录 | 必须（本地/OIDC/LDAP 单一主源 + 应急超管） | 不需要 | 继承企业版（客户可换登录源品牌） |
docs/01-vision-and-editions.md:58: 1. **认证与会话**：本地/OIDC/LDAP（单一主源）+ 本地应急超管；JWT access + refresh；后续钉钉/企微/飞书为包。
docs/01-vision-and-editions.md:59: 2. **用户**：超管 / 审计员（只读）/ 员工；**无部门/用户组**；用户来源可配置。
docs/12-roadmap.md:93: - 服务端：认证（local/OIDC/LDAP 单一主源 + 应急超管）、JWT+refresh、用户（无部门）、钱包（余额/月发放/充值）、
AGENTS.md:39: | C8 | **无部门/用户组**；角色只有超管/审计员/员工；内容可见性 = 全员/指定用户 | 见 ADR-032 |
AGENTS.md:62: - 认证：单一主源（local/OIDC/LDAP）+ 本地应急超管；JWT access + refresh；禁用后**每请求校验 `token_epoch` 立即失效**。

## MCP
docs/15-open-questions.md:49: | D33 | 企业版 MCP **允许本地 stdio**（管理员配置即可用） |
docs/15-open-questions.md:81: | Q5 | ~~MCP 令牌~~ → **已定：客户端本地**（与服务端只存配置一致）（D53） | 已决 |
docs/04-contracts-and-api.md:34: | C1 | 服务端不控制工具/网络/审批 | 不下发工具白名单、网络策略、审批策略；客户端本地审批由用户决定；API 中不存在 `POLICY_DENIED` 这类"服务端禁止操作"错误 |
docs/04-contracts-and-api.md:99: 规则：企业密钥**不出服务端**（加密存储、只在服务端使用）；OAuth 授权码流程在服务端回调完成（`/connectors/oauth/callback`），refresh token 存服务端；客户端只拿短期令牌，MCP 流量客户端直连、服务端不代理；定义唯一真源在服务端，客户端只读；企业版默认禁 `stdio`（Q17），个人版本地自配（D19）。
docs/04-contracts-and-api.md:208: | 信封 | `{v, event_id, type, topic, seq, ts, payload, schema_version}`；`seq` 按 topic 单调递增，与客户端本地 IPC 的 stream seq 无关（[05-client-runtime.md](05-client-runtime.md) §6）；payload 最小化，业务数据一律回源 HTTP 拉取 |
docs/13-risks-and-adr.md:434: | MCP 客户端 OAuth 令牌存放 | 客户端本地（D53） |
docs/17-v3.1-decisions.md:35: | 13 | 企业版 MCP **允许本地 stdio**，管理员配置即可用 | 连接器支持 stdio/http/sse；stdio 命令由管理员配置 |
docs/17-v3.1-decisions.md:66: | `07-server-domains.md` | 已更新：删组织/部门、钱包峰谷价/月发放/透支、JWT 立即失效、技能删除同步、MCP stdio + 客户端 OAuth、员工上传应用、审计员 |
docs/17-v3.1-decisions.md:77: - MCP 客户端 OAuth 的令牌存放（客户端本地 vs 服务端代理）
docs/07-server-domains.md:61: | 商店：技能/连接器/应用的审核、版本、可见性、强制更新、删除同步 | 企业审批流（危险操作由客户端本地审批，用户自己决定） |
docs/07-server-domains.md:178: **口径**：✅ 企业连接器由管理员配置下发；✅ 支持 **stdio（本地进程）与 http/sse**；✅ 员工可启用/停用；
docs/07-server-domains.md:179: ✅ 更新强制覆盖；✅ **OAuth 回调由客户端承担**（🟡 令牌存放位置 Q5）；个人版允许本地自配（无服务端）。
docs/07-server-domains.md:183: | `connectors` | id, name, transport(stdio/http/sse), config(json, 密钥加密), enabled, updated_at |
docs/07-server-domains.md:187: - 管理员配置 stdio 命令/参数/环境变量，或 http URL；员工只看到"启用/停用"与状态。
docs/07-server-domains.md:188: - 员工启用后，客户端执行本地 stdio 或连接远程；令牌由客户端 OAuth 获取并本地保存（🟡 与"密钥不出服务端"的边界见 Q5）。
docs/07-server-domains.md:268: | Q5 | MCP OAuth 令牌 | 客户端本地 |
docs/PLAN.md:37: | 连接器（MCP） | 企业下发配置、服务端存 secret、允许 stdio；OAuth 回调在客户端、令牌默认客户端本地；个人版本地自配（`07` §6） | 已拍板 |
docs/01-vision-and-editions.md:87: - **不做企业审批**：员工拥有客户端全部能力；危险操作由客户端本地审批（用户自己决定）。
docs/12-roadmap.md:216: MCP 客户端 OAuth 令牌存放（默认客户端本地）。
AGENTS.md:65: - 商店：技能（DSH zip，16MB，客户端执行，不扫描）、连接器（MCP，允许 stdio，OAuth 回调在客户端）、WASM 应用（服务端 wasmtime 执行、每应用 SQLite、不可调 AI）。
AGENTS.md:263: - 待用户确认：峰谷时段具体配置、技能/应用"指定用户"细节、MCP 客户端 OAuth 令牌存放（默认客户端本地）。