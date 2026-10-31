# 09 · 版本与交付：企业版 / 个人版 / OEM 白标

> 范围：**同一代码库产出哪几种产品、构建期怎么裁剪、OEM 数据放哪、CI 怎么出包、客户怎么部署、客户端怎么更新。**
> 不涉及：服务端内部架构、安装器 UI 细节。状态：Plan v3.2（§9 为默认值；开放项索引见 [`15` §C](15-open-questions.md#c-待定清单)）。关联：`02-package-model.md`（角色）、
> `03-composition-and-ci.md`（profile/生成物/门禁）、`10-repo-and-build.md`（目录/锁/可复现）、`research/harness-client-channel.md`（渠道/品牌一手事实）。

## 1. 版本矩阵

**两个正交维度**：`edition` 决定**包含哪些包与能力**（个人版基线 vs 企业版 + 管控包）；`channel` 决定**品牌、身份、默认值（含服务端地址）**。
**官方企业版 = edition `enterprise` × channel `official`**；OEM = `enterprise` × 客户渠道；个人版 = `personal` × `official`。
官方不是特例，只是 `server_url` 默认空、登录时由客户配置地址的渠道。
产物 = edition × channel，在**构建期**合成；运行期没有任何"切换版本/换品牌"的代码路径。

| 产物 | edition | channel | 构建输入 | 输出 | 许可 | 分发 / 更新源 | 签名/校验 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 企业版 | `enterprise` | `official` | `editions/enterprise.toml` + `profiles/default.toml` + `brands/official/` | 服务端二进制 + webadmin + 门户 + 三平台安装包 + compose | MIT 免费 | docker 镜像 tar 交付；客户端从**登录的服务端**取更新 | Windows 不签名（SHA256）；macOS 我方证书 |
| 个人版 | `personal` | `official` | `editions/personal.toml` + `profiles/default.toml` + `brands/official/` | 纯客户端三平台安装包（无服务端产物） | MIT 免费 | **GitHub Releases**（stable/beta）；客户端查 `latest.json` + `SHA256SUMS` | Windows 不签名（SHA256）；macOS 我方证书 |
| OEM 客户版 | `enterprise` | `oem-<customer>` | `editions/enterprise.toml` + 私有索引仓该客户子仓（品牌/文案/配置/可选代码包） | 同企业版，服务端与客户端均为客户品牌 | MIT（白标服务另计） | 客户端从登录服务端；服务端走客户自建更新面；**绝不进公开 release** | Windows 不签；macOS 我方证书/客户自带另议（ADR-037） |

合法组合：企业版 = `official` + 任意 `oem-*`；个人版 = 仅 `official`（个人版不做 OEM，见 §9 09-Q3）。

| 维度 | 企业版 | 个人版 |
| --- | --- | --- |
| 服务端 | 有：`server`+`webadmin`+`portal` 编进镜像 | 无（二进制里物理不含服务端客户端包与网关调用） |
| 登录 | 必须；服务端签发会话，登录前只显示登录页 | 无登录、无账号 |
| AI provider | **服务端网关**；员工不可自配、客户端不持有上游 key | 用户自配（deepseek/openai/ollama 等），key 存本地 |
| 下发内容 | 模型目录、默认模型、技能/内容清单（**不含工具/网络/审批策略**） | 全部本地设置 |
| 其余 | 渠道 = official+oem-*；更新源 = 登录的服务端（1:1）；三平台 | 渠道 = official；更新源 = GitHub Releases；三平台 |

裁剪机制（**edition → profile → channel 三层，构建期固定**）：

- `editions/enterprise.toml` 继承 `personal`，`exclude` 个人专属包，`[packages]` 追加/覆盖企业角色；`profiles/*` 只做环境覆盖；`channel` 注入品牌/默认值（含 `server_url`）。
- 角色统一为 `llm.provider.default`（本地 provider ↔ `zeokeel/llm-gateway`）、`client.update`（GitHub ↔ 服务端）、`auth.client`；`targets` 统一为 `server,desktop,admin`（门户/webadmin 作为 server/admin 内嵌产物）。
- edition 事实由 `zk compose` 固化成常量 `edition.gen.{rs,ts}`（`EDITION`/`LOGIN_REQUIRED`/`PROVIDER_MODE`/`UPDATE_SOURCE`/`CHANNEL_ID`/`DEFAULT_SERVER_URL`），`--check` 字节比对；**禁止 Cargo feature 或运行期 env 切 edition**（理由同 `03-composition-and-ci.md` §2）。
- 企业版能力**在服务端**：客户端二进制不含"解锁代码"；`zk plan --edition personal --profile default --channel official` 可证明服务端包/企业包不在闭包里。
- 新增版本 = 新 edition + profile + channel；不改构建器、不加开关。

### 1.5 构建期锁定

原则见 `00` P1/P8；常量与 `--check` 见 §1.4；产物/签名见 §4–§7。本节不重复。

## 2. OEM 仓库模型

**demo 仓库建在 `zeokeel` 组织**：我们的索引仓**独立创建**（demo 用 [`zeokeel/demo-oem-index`](https://github.com/zeokeel/demo-oem-index)，含 submodule）；
示例客户仓 [`zeokeel/demo-oem-acme`](https://github.com/zeokeel/demo-oem-acme)；示例应用仓 [`zeokeel/demo-wasm-app`](https://github.com/zeokeel/demo-wasm-app)（均私有）。
参考仓库 `zeokeel/channels` **只用于对照 `channel.json` 字段设计，不作为我们的索引仓/依赖**。

**我们的 OEM 索引仓（独立私有仓，demo 为 `zeokeel/demo-oem-index`）只装数据**（不装证书、不装脚本、不装产物），每个客户 = 一个 git submodule = 一个渠道包；
`zeokeel/channels` 仅作字段设计参考。
内部文件与主仓 `brands/official/` 同构（客户子仓根 = 渠道包根），`zk brand`（未实现，backlog）对两者一视同仁。

```text
zeokeel/demo-oem-index/           # demo 索引仓（CI 只读；示例客户渠道以 submodule 追加；official/beta 品牌在主仓 brands/）
├─ oem/index.toml                 # 客户列表 = 渠道清单（唯一真源）
├─ oem/customers/<channel>/       # git submodule（每客户一仓）
├─ channels.manifest.json         # 期望渠道集（防"少发一个渠道没人发现"）
└─ .gitmodules
# oem/index.toml
schema = 1
[channels.oem-acme]               # 键 = channel id（oem-<customer>，见 §3），必须等于目录名
submodule = "oem/customers/oem-acme"
pin       = "9f2c…"               # 冗余断言值，必须 == submodule gitlink
edition   = "enterprise"
depth     = "basic"               # basic | deep
owner     = "acme-it"             # 我方对接人（不进公开仓）
```

```text
<customer>/                       # 客户子仓 = 渠道包根（channel.json 字段与参考仓同构）
├─ oem.toml                       # 构建/交付元数据（不含品牌内容）
├─ channel.json                   # 渠道 + 品牌 + 默认值唯一真源（schema 见 §3）
├─ logo.svg / logo-dark.svg / app-icon.png / assistedMessages.yml
├─ strings/{zh-CN,en}.json        # i18n 覆盖（只许 brand.* 前缀）
├─ config/defaults.toml           # 服务端默认配置（品牌/文案/默认 provider/管理台主题）
└─ packages/                      # 可选：深度 OEM 自定义代码包（Rust/UI，走同一契约）
# <customer>/oem.toml
channel = "acme"
edition_base = "enterprise"
locales = ["zh-CN", "en"]
platforms = ["windows-x64", "macos-arm64", "linux-x64"]   # 交付面收窄：缺的平台不出现，不是灰卡片
depth = "basic"
[packages]                        # 仅深度 OEM：可覆盖实现类角色，禁碰契约/内核
# "tool.fs" = "acme/tool-fs-vault"
```

> **服务端地址的构建期默认值只有一个真源**：`channel.json` 的 `defaults.server_url`（字段形状参考 `zeokeel/channels`，不依赖该仓）。
> 非空 = 锁定免填；空 = 首配页输入并本地保存（设置可改，需重登）。`oem.toml` 不再重复写 `server_url`；构建器从 `channel.json` 读取并生成客户端默认常量。

| 规则 | 内容 | 门禁 |
| --- | --- | --- |
| pin 一致性 | `.gitmodules` + submodule gitlink + `index.toml.pin` 三处必须一致；pin 是**发布 tag 上的硬要求**。`zk channel verify` 对 **git 索引仓强制要求** submodule gitlink（mode 160000）并与 pin 比对，缺失/不匹配直接失败；只有纯目录夹具/导出包（非 git）回退目录哈希，错误信息标明 `fixture fallback` | 不一致 → 红（防"以为更新了 submodule"） |
| 版本 | 客户仓不独立定版本；产物版本 = 主仓 git tag（OEM 热修复走新 tag） | tag == 产物 VERSION == 镜像 tag |
| 公开仓零客户信息 | 主仓只留 `oem-demo`（假客户夹具）+ `brands/official/` | 域名/渠道名白名单扫描 |
| 深度 OEM | 代码包必须声明契约 + conformance + 许可证 + 来源；生成物 diff 最小性同官方 | 同一 CI 门禁，不另建流程 |

| 维度 | 基础 OEM（数据） | 深度 OEM（数据 + 代码） |
| --- | --- | --- |
| 可改 | 品牌、文案、默认配置、管理台主题、交付平台 | 以上 + 通过 profile 覆盖实现类角色（新增/替换包） |
| 不可改 | 服务端核心逻辑、安全默认值、更新校验、签名与渠道身份规则 | 同上 + 契约与内核、运行时动态加载 |
| 门禁 | schema/形状/素材/i18n/跨渠道唯一/产物级白标 | 基础门禁 + 契约单实例 + conformance + T2 换包 + 许可证审计 + 生成物 diff 最小性 |
| 交付 | 同一条流水线（§4） | 同一条流水线（多一步代码包 vendored + `zk.lock` 记录） |

## 3. 渠道（channel）与品牌（brand）规范

**channel id**：`^[a-z0-9][a-z0-9-]{0,31}$`；保留字 `official`/`beta`/`personal`/`demo`/`test`。一个客户 = 一个 channel id，
它同时是子仓目录名、`oem.toml.channel`、镜像 tag 后缀、产物文件名段、更新面目录名。渠道在构建期注入，
服务端**不参与选择**（只读镜像内渠道内容）；运行期"我是谁"= 二进制常量 + 登录服务端的一致性校验。

**渠道/品牌资源清单**（`channel.json` schema 1；字段设计参考 `zeokeel/channels`，实现与索引仓独立）

| 键 | 形态 | 校验 | 用途 |
| --- | --- | --- | --- |
| `identity.slug` / `product_name` | ASCII `[a-z0-9-]{1,32}` / 显示名 | 包名合法；后者路径安全且跨渠道唯一 | 安装包名、窗口/安装目录/userData |
| `identity.app_id` | 反向域名 | 跨渠道唯一且 ≠ 官方 | Tauri `identifier` |
| `identity.company` / `copyright` / `linux{maintainer,synopsis,category}` | 文本 | 非空 | 安装器与 deb/AppImage 元数据 |
| `identity.deep_link_scheme` / `app_origin_scheme` | `[a-z][a-z0-9+.-]*` | 两者互不相等、各自跨渠道唯一、非保留字 | 深链 `acme://auth`；WebView 自定义协议 |
| `identity.data_dir_name` / `product_home_dir` | 目录名 | 路径安全；**同一 edition+channel 升级不得变**；不同产品身份必须不同 | 数据根 |
| `brand.logo.light` / `brand.logo.dark` | 渠道目录内相对 SVG 路径 | 非空、相对路径、canonicalize 后必须位于渠道目录内、禁 `..`/绝对路径/符号链接逃逸、大小上限（256KB）、`<script>`/外链资源拒绝 | 构建期注入 `public/brand/`；派生托盘/favicon（`scripts/gen-icons.sh`） |
| `brand.icon_source`（可选） | 渠道目录内相对 PNG/SVG 路径 | 同 logo 路径安全；缺省 = `brand.logo.light` 派生 | 安装包/窗口图标；由 `scripts/gen-icons.sh` 生成，不手改 |
| `assets.accent` | `#RRGGBB` | 与前景色对比度阈值；仅派生资源/门户，不覆盖客户端 live accent（`27`） | 托盘、门户、管理台强调色 |
| `copy.*` | 文本，支持 `{product}`/`{version}` | 占位符集合与官方一致、关键键非空 | 安装器/登录页/窗口标题/门户 |
| `brand.powered_by` | `show` / `hide` | **必填枚举**；官方默认 `show`；OEM 默认 `hide`；未知值构建红；`zk compose` 校验 | 是否显示“Powered by ZeoKeel” |
| `defaults.server_url` | 绝对 https URL（回环例外）或空 | **任意渠道都可空**：为空 → 首配页显示地址输入（客户自己配置，官方企业版默认如此）；非空 → 客户端内置并锁定地址（OEM 免填）；设置了就必须合法；个人版必须为空且无登录；同渠道跨版本不得变 | **渠道决定登录形态**：非空 = 只显示账号/密码、地址不可编辑；空 = 首次输入并本地保存，设置中可改（需重新登录） |
| `strings/<locale>.json` | 词条覆盖 | 键在 i18n catalog 内且只允许 `brand.*` 前缀 | UI 品牌文案 |
| `config/defaults.toml` | 服务端默认值 | 键在服务端部署配置注册表白名单内 | 服务端默认配置/管理台主题 |

**品牌是数据 + 渲染包两层**：渠道数据（`brand.json`+图形+文案）构建期派生进产物；`brand.mark`/`theme.tokens` 是包，
由生成常量驱动的默认品牌包实现。基础 OEM 只动数据；深度 OEM 要改品牌布局/行为才替换品牌包（同一条 profile 通道）。
**服务端 OEM 面** = **员工客户端下载门户**（只读下载页，不是 OEM 自助门户）、管理台主题（accent/logo/CSS 变量）、登录与邮件文案、默认配置、客户端清单，
全部**构建期注入、运行期只读**（不提供上传 logo 换品牌）。产品功能文案只能由包提供，渠道不得改（防升级后术语漂移）。

**i18n 词条覆盖**

- 真源：各包 `locales/<locale>.json` → `zk compose` 汇总成 `shared/contracts/i18n-catalog.json` + `i18n.gen.ts`，`--check` 防漂移。
- 渠道只可覆盖 `brand.*`（登录页/安装器/门户/品牌错误提示）；出现功能键 → 红。
- 回落链：品牌键 → **禁止回落**（缺即失败，绝不继承官方身份）；功能键 → 官方同 locale → 官方 `zh-CN`。
- 占位符必须与 catalog 完全一致；`oem.toml.locales` 声明的每种语言都要覆盖 `brand.required` 全集，未声明语言不允许存在文件（防半翻译上线）。

**构建期派生（`build/`，不入库）与校验**

| 派生 | 输入 | 输出 | 门禁 |
| --- | --- | --- | --- |
| 托盘图标 6 位深 | logo + accent | `build/tray/*.png` | 尺寸/alpha/位深齐全 |
| favicon / 门户 / 管理台 logo、管理台主题 | logo + accent | `build/web-brand/*`、`build/webadmin/theme.css` | 同源派生；对比度 |
| mac 应用图标 | app_icon | `build/app-icon-mac.png`（824 安全区） | 1024 画布 |
| 安装器引导文案 | `assistedMessages.yml` | `build/assistedMessages.yml` | 声明的 locale 键齐 |
| 渠道包 | 以上全部 | `build/channel.json`（随包分发，登录前就要品牌） | 打包后重派生逐字节比对 |

| 闸门 | 抓什么 | 失败后果 |
| --- | --- | --- |
| 必填/未知键 | 缺键、类型错、未知键、`"false"` 字符串 | 构建红（不是警告） |
| 形状/素材 | slug/app_id/scheme/目录名/URL 正则；尺寸、格式、透明通道、平坦色 | 构建红 |
| 跨渠道唯一 | app_id / 深链 scheme / app origin scheme / 数据根 / product_name 撞车 | 构建红（全部渠道**同一 job** 校验） |
| 禁止回落 | 品牌渠道缺 `brand.required` 键却取到官方值 | 构建红 |
| 服务端地址 | 非空时非法（非 https/带路径/尾斜杠）或跨版本变化 → 红；**空是合法值**（空时由首配页本地填写）；个人版非空 → 红 |
| 产物级 | 打包后重派生逐字节 + 拆包断言 `channel_id`；按 `brand.powered_by` 分支：`hide` 不得出现厂商品牌，`show` 允许且文案固定 | 构建红 |
| 单一读取点 | 除组装期生成物外任何代码读渠道文件（源码扫描） | 构建红 |

**从参考实现借的 5 条**：渠道 = 一个目录 + 一份 JSON 且**随包分发**（→ §2 子仓 + 镜像/安装包内 `channel.json`）；
单一真源 + 生成器 + 逐字节 `--check`；**跨渠道唯一性在构建期做**（运行期只剩能/不能启动）；**产物级白标校验**优于相信构建参数；
凭证纪律（deploy key 优先、渠道 id 打码、错误只报字段名）。

**要避的 3 个坑**：①少写一个键就静默继承厂商身份 → 禁止回落 + 官方模板逐键必填 + 产物重派生三重门禁；
②参考实现 Windows 安装包完全没签名 → **v3.1 明确不做 Windows 代码签名**（§7）；③多入口 = 多份真相（派生了但没人调用）→ 每个"必须发生"的派生/校验
都返回布尔证据进构建日志，且只有组装期能读渠道内容。

> 客户接入/RACI/UAT/支持见 `23`；运维/备份/升级见 `20`。

## 4. 构建流水线（新增客户 → 镜像 + 三平台安装包）

| # | 步骤 | 动作 | 门禁 | 产物 |
| --- | --- | --- | --- | --- |
| 1 | 建客户 | 私有仓建 `<channel>/`（`oem.toml`+`channel.json`+`logo*/app-icon/assistedMessages`+`strings/`+`config/`）；索引仓加 submodule 并在 `index.toml` 登记；**不做 OEM 自助门户** | schema + 评审 checklist（品牌点清单逐项标必改/可选/保留） | 一次纯数据 PR |
| 2 | 取包 + 解析 pin | CI 用只读凭据拉索引仓 → `git submodule update --init --recursive --depth 1` → 校验 gitlink == pin | pin 一致 + 渠道集 == manifest | `channels.list`（渠道 id 全程 add-mask） |
| 3 | 全渠道校验 | `zk channel validate --index <dir> --all`（形状/素材/i18n/跨渠道唯一/深度包门禁） | 全绿 | 校验报告（进交付记录/SBOM） |
| 4 | 逐渠道构建 | `zk compose --edition enterprise --profile default --channel <id>` → cargo build → 三平台 `tauri build` → `zk channel verify --channel <id> --dist` | 产物级白标 + 启动冒烟 + mock 登录会话 | 安装包 + `CLIENT-RELEASE.json` |
| 5 | 镜像与发布 | docker build（`clientassets` + `channelassets` 上下文）→ 镜像内断言 → `docker save` → tar.zst → 上传私有更新面 | 镜像内容逐项断言 + `SHA256SUMS` 复算 | 镜像 tar + 三平台安装包 + 校验和 |

**CI 取私有仓**

| 方式 | 适用 | 说明/风险 |
| --- | --- | --- |
| SSH deploy key（只读、单仓、不过期） | 索引仓 | 每个 submodule 仓需各自授权；渠道多时管理成本高 |
| 机器账号（robot）+ 只读 team | 索引仓 + N 个客户仓（同 host） | **推荐**；权限可审计、一把钥匙 |
| fine-grained PAT | 自建/临时 git | 会过期 → 发布构建随机失败；禁止用于打 tag 的 job |
| 本地目录 `OEM_INDEX_DIR` | 开发/夹具 | 渠道数量下限断言自动跳过（夹具规模不同） |

submodule 取法：`GIT_SSH_COMMAND` 注入 key + `[url] insteadOf` 重写；`--depth 1` + 按 pin `fetch`；**gate job 只解析一次 pin**，
后续 job 消费同一份 `channels.list`（"镜像内客户端"与"镜像内渠道内容"同源、可复现）。OEM 构建在**交付工作区**叠加：
公开仓 + 私有 overlay，生成物写未跟踪的 `.zk-delivery/`，**不回写主仓**（客户名不得进开源仓）。

**产物命名**：Linux `zeokeel-enterprise-<ver>-<channel>-linux-x64.{AppImage,deb}`；macOS `…-macos-<arm64|x64>.dmg`；
Windows `…-windows-x64-setup.exe`；个人版 `zeokeel-personal-<ver>-<os>-<arch>.<ext>`；镜像 tag `zeokeel/server:<ver>-<channel>`，
导出 `zeokeel-enterprise-<ver>-<channel>-linux-amd64.tar.zst`；交付目录 `dist/<ver>/<channel>/{clients,images,SHA256SUMS,SBOM/,CLIENT-RELEASE.json,CHANNEL.json}`。

**可复现与校验和**：版本唯一真源 = git tag `vX.Y.Z`（`zk version check`，未实现，backlog，断言 tag == 清单 == 产物 `VERSION`）；
`cargo --locked` + `--offline`（vendor）、`pnpm --frozen-lockfile`、固定 `SOURCE_DATE_EPOCH`、生成物无时间戳；
基础镜像按 digest 钉（builder/caddy/postgres），CI 镜像预烤 NSIS/WiX/linuxdeploy/webkit；每个交付 tar 带 `SHA256SUMS`+SBOM+
脱敏 `CHANNEL.json`；nightly 同 channel 两次冷构建比对哈希（容忍签名段），不一致即红。

## 5. 企业版交付物

**镜像内容（自包含，客户侧零外网依赖）**

| 路径 | 内容 | 镜像内断言 |
| --- | --- | --- |
| `/opt/zeokeel/bin/server` | 服务端二进制（webadmin 内嵌） | `--version` 含版本 + channel |
| `/opt/zeokeel/channel/` | `channel.json`（构建期注入，只读） | `channel_id` == 镜像 tag |
| `/opt/zeokeel/client/` | 三平台安装包 + `CLIENT-RELEASE.json` | 逐平台键存在、sha256 与文件一致 |
| `/opt/zeokeel/deploy/` | compose + 三份 Caddyfile + `.env.example` | `docker compose config` 通过 |
| `/opt/zeokeel/{VERSION,CHANNEL}`、`/data`（VOLUME） | 版本/渠道；运行期数据 | 与 tag 一致；0700、非 root（uid 10001） |

**单机 compose 拓扑（唯一默认拓扑）**

| 服务 | 镜像 | 端口 | 卷 | 职责 |
| --- | --- | --- | --- | --- |
| `caddy` | `caddy:2-alpine`@digest | 80/443，唯一对外入口 | `caddy_data`/`caddy_config` | TLS 终结 + 反代（`TLS_MODE=auto/manual/internal` 选 Caddyfile） |
| `server` | `${SERVER_IMAGE}` | 仅 `expose 8787` | `/data` | 认证、AI 网关、门户、安装包、管理台；`stop_grace_period: 45s`（见 `24` §6） |
| `postgres` | `postgres:18-alpine`@digest | 不出宿主 | `pgdata` | 业务库 |

导出 tar **必须含三张镜像**（含 caddy/postgres），否则"导入后 `compose up` 即用"在内网不成立；体积预算见 §9 09-Q2。客户侧：

```bash
docker load -i images.tar          # 或 zstd -dc images.tar.zst | docker load
cd deploy && cp .env.example .env  # 至少改 4 个；SERVER_IMAGE/MASTER_KEY 按部署/升级需要
docker compose up -d
curl -fsS https://$DOMAIN/healthz  # 门户下载页即落地页
```

**`.env` 最小配置**

| 键 | 必填 | 说明 |
| --- | --- | --- |
| `DOMAIN` | 是 | 门户/登录/更新清单的绝对 https 基址（前面还有反代时可用 `ZEOKEEL_PUBLIC_BASE_URL` 覆盖对外基址；该覆盖是可选变量，不计入最小 6 键） |
| `TLS_MODE` | 是 | `auto`/`manual`/`internal`，决定挂哪份 Caddyfile |
| `ADMIN_PASSWORD` | 首启 | 首启强制改密；只在客户环境，不进镜像 |
| `PG_PASSWORD` | 是 | 与 server 注入一致；随机生成并写进客户交付记录 |
| `SERVER_IMAGE` | 是 | `<ver>-<channel>`：同机多栈隔离 + 回滚切换点 |
| `MASTER_KEY` | 否（推荐是） | 上游 key 加密密钥；不提供则首启生成；**单独离线保管，不放进备份包**（见 `20` §3） |

**门户下载页**：`/` 与 `/portal` 为零脚本 HTML（CSP `default-src 'none'`）；员工从这里下载客户端安装包，下载卡片读镜像内
`CLIENT-RELEASE.json` 并按渠道 `platforms` 收窄（未声明平台整块不渲染，不出现灰卡片）；管理入口只在页脚；品牌/主题构建期注入。

**升级 / 回滚 / 备份**：完整流程、门禁、备份/PITR、runbook 见 [`20` §3–§4](20-operations.md)；本节只定义交付物（镜像/安装包/门户）与 `.env`。

## 6. 个人版交付

| 项 | 内容 |
| --- | --- |
| 产物 | 三平台安装包 + `SHA256SUMS` + SBOM；无镜像、无 compose、无服务端 |
| 安装 | 用户自助安装；数据根为独立目录（与企业版/任何渠道都不同，可并存） |
| 能力 | 本地对话、工具、审批、会话存储；AI 需用户自配 provider 与 key（Windows DPAPI / macOS Keychain / Linux secret-service；无 keychain 时加密文件 + 明确警告） |
| 更新源 | **GitHub Releases**（stable/beta；`latest.json` + `SHA256SUMS` + 安装包），签名校验 |
| 能力边界 | 无账号/组织管理/额度/内容中心/门户；无服务端下发模型目录与默认配置；遥测默认关 |
| 不做 | 不做"填个 license 变企业版"；不承诺与企业版数据互通（09-Q13） |

## 7. 更新设计（v3.2）

> 参考 `zeokeel/ZeoKeel-Harness` 的更新模型：**服务端镜像与客户端安装包同批发布、客户端只从登录的服务端更新、
> 服务端升级必须手动导入镜像**。我们沿用这个结构，并把个人版改为 GitHub Releases。

### 7.0 发布单元：一个版本 = 服务端镜像 + 三平台客户端

- CI 对每个版本同时产出：server 镜像包（含 compose/Caddy/门户/客户端安装包）+ 三平台安装包 + `CLIENT-RELEASE.json`。
- 镜像内固定路径 `/opt/zeokeel/client/`，服务端从同一份 `CLIENT-RELEASE.json` 生成下载门户与客户端更新清单。
- **1:1 可用**：`server.version == client.version` 由发布流水线强制；镜像缺客户端资产或版本不一致 → 构建失败，
  不允许"先发服务端、后补客户端"。

### 7.1 个人版：GitHub Releases

| 项 | 规则 |
| --- | --- |
| 来源 | **仅 GitHub Releases**（`github.com/zeokeel/ZeoKeel/releases`），不依赖任何自建更新面 |
| 渠道 | `stable`（latest release）/ `beta`（prerelease）；客户端设置里可切换，默认 stable |
| 资产 | 三平台安装包 + `latest.json` + `SHA256SUMS`；`latest.json` 含版本/资产/sha256/最低支持版本 |
| 检查 | 启动后异步检查（可关闭自动检查）；只做提示，不静默安装 |
| 校验 | HTTPS + `latest.json` 签名 + 安装包 sha256；Windows 未签名（提示 SmartScreen），macOS 公证后校验签名 |
| 离线 | 用户可手动下载安装包覆盖安装；无服务端依赖 |

### 7.2 企业版服务端：自建更新服务器 + 必须手动升级

| 项 | 规则 |
| --- | --- |
| 更新源 | **自建更新服务器**（静态 HTTPS：nginx/对象存储均可），地址在管理台「系统配置 → 更新清单地址」配置；默认指向客户自建面，不指向官方 |
| 清单布局 | `<channel>/latest.json`（no-cache）+ `<channel>/releases/<ver>/zeokeel-server-<ver>-<channel>-linux-amd64.tar.zst` + `SHA256SUMS` + release notes；版本目录不可变 |
| 检查 | 服务端定期/手动检查 `latest.json`（可比对版本与 sha256）；管理台「服务器信息」显示"有新版本"与升级步骤；**不自动下载、不自动升级** |
| 升级流程（人工，必须） | ① 备份（`pg_dump` + `zeokeel-data/`）→ ② 下载镜像包 → ③ 校验 `SHA256SUMS` → ④ `docker load` → ⑤ `docker compose up -d` → ⑥ 启动自动迁移 → ⑦ `/readyz` + 门户/客户端清单校验 → ⑧ 失败则回滚旧镜像 + 备份 |
| 离线/内网 | 更新服务器可完全离线：管理员把镜像包拷入内网，重复 ③–⑦；可关闭更新检查 |
| 禁止 | 不做服务端自动更新/自更新守护进程；不接受"在线热升级" |

### 7.3 企业版客户端：只从登录的服务端更新（强制 1:1）

| 项 | 规则 |
| --- | --- |
| 登录地址 | 来自构建期 `channel.json` 的 `defaults.server_url`；**非空时**登录页只有账号/密码、地址不可编辑；**为空时**（官方企业版默认）首配页显示地址输入，首次填写并本地保存（设置中可改，需重新登录） |
| 唯一来源 | `GET {server}/api/updates/manifest` + `GET {server}/api/updates/client/<file>`；客户端**没有任何指向 GitHub/官方更新面的代码路径** |
| 清单 | `{schema, channel_id, edition, server_version, client_version, min_supported, forced, rollout?, deadline?, previous?, assets{<platform>:{url,sha256,size}}, signature}`；`url` 必须同源绝对 https |
| 1:1 | 清单里的 `client_version` 必须等于当前服务端镜像内 `CLIENT-RELEASE.json` 的版本；客户端安装该版本后与登录服务端配对可用；服务端版本变化 → 清单变化 → 客户端强制跟进 |
| 防串渠道 | `channel_id`/`edition` 不匹配 → 拒绝升级并报错（不提示"手动下载"，否则等于跨渠道洗包） |
| 强制更新 | 服务端可标记 `forced`/`min_supported`；低于最低版本的业务 API 返回 `426 CLIENT_TOO_OLD`，客户端显示强制升级页；本地 agent 离线能力可短暂保留（宽限待定，见 [`15` §C](15-open-questions.md#c-待定清单)） |
| 流程 | 检查 → 断点续传（`.partial`+Range）→ sha256 + 清单签名校验 → 安装 → 启动标记；失败回滚上一版；Win 静默 NSIS `/S`，mac 原地替换 + 30s 启动标记回滚，Linux AppImage 原子替换 |
| 门户 | 服务端根路径门户展示三平台安装包（读 `CLIENT-RELEASE.json`），按渠道品牌；员工首次安装也从这里下载 |

### 7.4 校验、签名与回滚

| 平台 | 要求 |
| --- | --- |
| 清单 | 服务端/个人版清单均签名（ed25519）；客户端先验清单再下载 |
| Windows | **不签名**（v3.1 已定）；依赖 HTTPS + 清单签名 + sha256；安装提示 SmartScreen |
| macOS | Developer ID + hardened runtime + **notarize + staple**；**当前 TODO：本地测试，证书后配** |
| Linux | SHA256SUMS + minisign/ed25519；AppImage 原子替换 |
| 回滚 | 客户端保留上一版直至新版成功启动；服务端回滚 = 旧镜像 + 备份；跨大版本回滚需检查迁移（迁移只前向，故回滚需备份恢复） |

> 参考实现对照见 `research/harness-client-channel.md` §7：客户端只从登录服务端取清单、安装包随镜像、
> 更新服务器 `<channel>/latest.json`、服务端手动升级、门户读 `CLIENT-RELEASE.json`。个人版我们改为 GitHub Releases。

## 8. 安全与合规

| 面 | 规则 |
| --- | --- |
| 产物隔离 | 每渠道独立 artifact 目录/文件名带 channel；OEM 产物**不进公开 release**，走私有更新面；跨渠道中转路径不含渠道名（HMAC 派生临时前缀，取回即销毁） |
| 私有仓权限 | 索引仓/客户仓对 CI 只读；写权限仅限两名维护者；客户无本仓写权（提交经我方 review）；分支保护 + 必须 PR |
| 密钥注入 | 仓库/子仓零密钥；签名与上传凭据只进 CI secret（OIDC/短期优先）；镜像与 tar 不含密钥；渠道 id 与 token 打码；错误只报字段名 |
| SBOM | 每产物 CycloneDX（cargo+pnpm），进交付目录与 tar；深度 OEM 代码包必须出现；`zk sbom --edition E --profile X --channel C`（**命令已实现；进 tar/镜像打包 UNVERIFIED**，见下方限制） |
| 许可证 | 本产品 MIT（两版均免费开源）；`cargo-deny` + pnpm 许可证白名单；生成 `THIRD-PARTY.md`（含 caddy/postgres 镜像与内置字体/图标素材） |
| 供应链/数据 | lockfile + vendor + 基础镜像 digest + 依赖审计 + 发布前签名；企业数据留客户服务器、个人数据本地；遥测默认关且不含 PII |
| 白标合规 | MIT 允许商用改名，但 ZeoKeel 商标/官方标识不得用于暗示背书；**官方默认显示 Powered by，OEM 默认隐藏、可按渠道配置**（D62） |

> **SBOM 实现状态（round 11，2026-10-31）**：`zk sbom --edition E --profile X --channel C [--output FILE] [--check]`
> 已实现 CycloneDX 1.5（离线、无时间戳、`serialNumber` 由内容 SHA-256 派生、`--check` 逐字节校验）：
> 覆盖 **Cargo.lock 全部 crate**（name/version/checksum + workspace package license，来自
> `cargo metadata --no-deps --offline`）与 **pnpm workspace 直接包**（`pnpm-workspace.yaml` globs 下的
> package.json name/version/license，purl 为 `pkg:npm/...`）。CI docker job 产出
> `target/evidence/sbom/zeokeel-enterprise.cdx.json` 并上传 artifact。**限制**：components 来自全 workspace Cargo.lock + pnpm direct 定义，personal/enterprise 输出可能相同（edition 仅校验组合，不做过滤）；pnpm transitive 依赖
> （需 pnpm-lock 解析）尚未纳入；Docker 镜像内 SBOM 与「SBOM 打包进交付 tar」仍由 release/build-dist
> 流程处理，当前 **UNVERIFIED**；`--check` 只证明生成确定性，不证明镜像内容。

## 9. 交付细节默认值（编号 09-Q* 保留；收敛点：M5 spec）

> Q1/Q4/Q6/Q13 为**已定**（见对应 D/ADR），其余为默认值；实现前在 M5 spec 确认（C16），未决项统一索引见 [`15` §C](15-open-questions.md#c-待定清单) T12。

| # | 问题 | 默认值 |
| --- | --- | --- |
| 09-Q1 | ~~Windows 签名主体~~ → **Windows 不签名**（所有渠道）；macOS **需要公证**（ADR-037） | 已定；证书待配 |
| 09-Q2 | 交付 tar 是否必须含 caddy/postgres 镜像（离线导入）？体积预算多少？ | 必须含；体积预算按 M5 实测确定 |
| 09-Q3 | 个人版是否也允许 OEM？ | 否，仅 `official` |
| 09-Q4 | ~~Powered by~~ → **官方默认显示；OEM 默认不出现，可按渠道配置**（D62） | 已定 |
| 09-Q5 | 企业版免费 + MIT 后是否需要 entitlement 校验？ | 不做；商业边界 = 服务端能力 + 私域更新面 |
| 09-Q6 | ~~OEM 闭源包~~ → **OEM 定制代码由我方开发并入库；不接受第三方闭源预构建包**（D61） | 已定 |
| 09-Q7 | 服务端是否要"钉住对外提供的客户端版本"？兼容窗口几个小版本？ | **精确钉住**：客户端版本必须等于镜像内 `CLIENT-RELEASE.json`（1:1）；不承诺跨小版本兼容窗口 |
| 09-Q8 | 是否需要 beta/预发渠道做灰度？渠道 id 是否保留 `beta`？ | 保留 `beta` 但 M4 不发；灰度用服务端钉版本 |
| 09-Q9 | macOS 公证方式（ASC API key vs Apple ID）；Linux 是否 minisign | ASC API key 可自动化 |
| 09-Q10 | 增量更新是否列入 M4？ | 不列入；整包 + 断点续传已够 |
| 09-Q11 | 更新面/签名资产托管在哪？OEM 私有更新面归属？镜像是否进 registry？ | 我方托管私有面 + tar 内自带安装包兜底；不进公共 registry（可 `docker load` 或客户自建 registry） |
| 09-Q12 | 是否允许生产环境用外部 PG / 已有反代（不用 compose 内 caddy/postgres）？ | 允许，作为"高级部署"文档化；默认仍单机 compose |
| 09-Q13 | ~~数据根~~ → **已定：企业版/个人版分开数据根，可并存，不做数据互通** | 已定 |
