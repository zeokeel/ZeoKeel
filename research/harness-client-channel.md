# ZeoKeel Harness：客户端 / 渠道(OEM) / 版本与分发体系

> 素材：`/tmp/zk-harness`（HEAD `9252f15`，v2.10.0-beta.4）· 只读调研 · 面向"新 Rust/Tauri 桌面客户端 + 白标体系"设计
> 约定：正文里反引号包裹的路径一律是**仓库相对路径**（可直接在 `/tmp/zk-harness` 下打开）；未在本仓验证的推断显式标「推断」。

---

## 0. 结论速查

| # | 问题 | 一句话结论 |
|---|---|---|
| 1 | 客户端形态 | Electron **只做引导器**：拿单实例锁 → 组装固定 `desktop` profile → 在**主进程**内启动 DSH Host（Cordis）→ 渲染层加载该 Host 绑在 `127.0.0.1:<随机端口>` 的**同源 Web 页面**。没有 Electron 自有的插件表、没有 preload 桥（只有一个沙箱化的错误上报 preload）|
| 2 | 渠道怎么定 | 渠道 = 私有仓 `zeokeel/channels` 的 `channels/<id>/channel.json` 目录；**构建期**用 `DSH_BUILD_CHANNEL=<id>` 选，服务端**不参与选择**。运行期"我是哪个渠道"由**登录的那台服务端**在结构上决定（客户端不直连分发面）|
| 3 | 品牌 | `brands/official/`（`brand.json` schema 1 + 三个图形 + 安装器文案）是唯一事实源；`brand-prepare.mjs` 派生出 `build/**` 供 electron-builder 用；渠道用 `channels/<id>/` 逐文件覆盖，品牌渠道**禁止回落** |
| 4 | 功能模块 | 真源 `module-contract.json`（3 模块）→ `gen-module-contract.mjs` 生成 6 文件 11 个生成区（Go 常量/`-tags`、webadmin、客户端缺省表、Makefile、Dockerfile、ci.yml），`--check` 逐字节对拍；渠道用 `features.{skills,apps,connectors}` 显式关 |
| 5 | 连服务端 | 登录拿 bearer token → 存 `$DSH_HOME/session.json`(0600)；**AI 全走服务端网关** `/v1/messages`（客户端不持上游 key）；配置由 `GET /api/client/v2/config/bootstrap` 下发并写回 settings；客户端**默认禁一切代理** |
| 6 | WASM 应用 | 每个应用一个 `<app_origin_scheme>://<app_id>` **独立 origin + 独立窗口**，协议 handler 把请求转成 JSON 信封 POST 给服务端；服务端才是信任边界。应用 DB（SQLite）、准入、执行都在服务端，客户端无 DB 桥 |
| 7 | 更新与发布 | 客户端**只从登录的服务端**取更新清单 `GET /api/client/v2/updates/manifest`；安装包打进服务端镜像 `/opt/zeokeel/client/` 随镜像分发；R2 上 `<channel>/latest.json` 仅供服务端自检 |
| 8 | OEM 交付 | 新客户 = 私有仓建 `channels/<id>/` → CI 用只读 deploy key 拉取 → 三平台逐渠道打包 → 镜像逐渠道 `docker build`（含该渠道客户端）→ R2 上传 + `latest.json` → 客户 `docker load` + `.env` |
| 9 | 最值得抄的 | 单一真源 + 生成器 + `--check`；"渠道包随包分发"（登录前就要品牌）；跨渠道唯一性闸门；凭证形态/掩码纪律；产物级白标校验。**最大的坑**：Windows 安装包根本没签名 |

**channel / brand / module 三者关系（一句话）**：渠道 id 是**索引**，指向同一份 `channel.json`；`channel.json` 同时承载**品牌**（`identity`/`copy`/`assets`）、**构建期身份**（`desktop.slug|app_id|scheme|home_dir|platforms`）与**功能裁剪**（`features.*`），而 `features` 的合法键集合与缺省值来自**仓库根 `module-contract.json` 的生成物**——三者共用一个 id、一份文件、一条 CI 校验链。

---

## 1. 客户端形态（Electron 壳 + 官方 DSH Host）

### 1.1 组合方式

```
Electron 主进程（最小引导）
 ├─ 单实例锁 → 读随包 build/channel.json（可选）→ 解析 PRODUCT_NAME / DEEP_LINK_SCHEME / APP_ORIGIN_SCHEME
 ├─ 模块作用域、whenReady 之前：no-proxy-server、registerSchemesAsPrivileged(<app_origin_scheme>)、调试开关闸门
 ├─ prepareDesktopProfile()：装载持久 profile `desktop`（$DSH_HOME/profiles/desktop）
 └─ boot()：在**主进程**内启动 Host Cordis 根
      └─ dsh-web-app 把 HTTP/WS 绑到 127.0.0.1:<临时端口>
           └─ BrowserWindow 以沙箱渲染进程加载同一 origin 的页面（这就是"渲染层"）
```

- 证据：`packages/host/desktop/README.md`（"The Electron executable is minimal bootstrap code… boots the Host Cordis root in the Electron main process"）
- 证据：`packages/host/desktop/src/main.ts:1`（模块头）、`:120-215`（渠道包读一次 → 三个常量）
- 窗口/托盘/更新/通知都是 **Host 插件**（`desktop-shell`、`desktop-diagnostics`、`desktop-updates`、`desktop-loop-notify`），通过 Cordis effect 注册托盘条目（三组 `tools`/`profiles`/`status`）；证据：`src/electron-runtime.ts:1093`（`rebuildTrayMenu`）、`src/runtime.ts:44`（条目分组）

### 1.2 profile 与补丁层（这是最值得复用的结构）

层序（自下而上），证据：`src/profile.ts:80-133`、`:1048-1200`：

| 层 | 内容 | 谁可改 |
|---|---|---|
| 上游 bundle | `dsh-base` + `@deepseek-ai/dsh-web-app` + 一批官方实验 bundle | 上游 pin（`upstream.json`）|
| 安装面自有 bundle（10 个）| `dsh-plugin-desktop` 自身 `cordis.patch.yml` + enterprise / capability-center / account-card / wasm-apps / foot-menu / wasm-apps-host / connectors / browser / memory-evolve | 仅本仓 |
| profile 自有补丁 | `$DSH_HOME/profiles/desktop/cordis.patch.yml`（用户/设置写入的落点）| 用户设置 |
| home 补丁 | `$DSH_HOME/cordis.patch.yml` | 用户手工 |
| **launcher overlays** | 启动器最后 push 的 pin 层：settings / webserver / ui-layout / agent-presets / desktop-shell / **渠道注入** | 本仓 |

- 关键不变量：这 10 层必须作为**真 bundle 层**（`dsh.bundle.patch`）进入组合，不能塞进 `context.overlays`，否则 `config-editor.edit()` 的复算会丢行（证据：`src/profile.ts:67-79`）。
- 必需层被跳过 = fail-loud（`skippedRequiredBundles` → 抛错），第三方 bundle 坏掉则只跳过并上报（证据：`src/profile.ts:1155-1175`）。
- 渠道注入本身是**纯函数** `channelProfilePatches(profile, rows, home)`，只对**已存在的行**打 patch：`zeokeel-auth-gate`（默认服务端地址 + brand）、`zeokeel-channel-sync`（brand 兜底）、`zeokeel-connectors`（OAuth 客户端名）、`zeokeel-session`（深链 scheme）、`speech-to-text-sensevoice`（模型目录/镜像）。证据：`src/profile.ts:298-380`
- 铁律（踩过）：**跨包的渠道内容一律在组装期注入，插件不得自己去读随包 `channel.json`**（插件被打进自己的 lib 后相对路径失效，会静默回落官方 scheme）。证据：`src/profile.ts:340-350`

### 1.3 窗口 / 托盘 / 更新 / 渲染层

| 面 | 做法 | 证据 |
|---|---|---|
| 窗口 | 三平台统一 `advanced` 壳：mac `hiddenInset`+`vibrancy:sidebar`+透明；win `titleBarOverlay`+Mica+圆角；linux 系统框。`contextIsolation:true / nodeIntegration:false / sandbox:true / devTools:!packaged` | `src/window-options.ts:43-90` |
| 导航闸门 | 同一闭包挂 `will-navigate` / `will-frame-navigate` / `will-redirect`，只允许停留本 shell origin | `src/electron-runtime.ts:150-170, 1306` |
| CSP | `default-src 'self' data: blob: ws:`；`img-src` 必须放行 `http: https:`（渠道 logo 来自客户自己的服务器） | `src/electron-runtime.ts:128-140` |
| 托盘 | 原生 `Tray` + 有序条目注册表；更新徽标、诊断、退出都在插件里贡献 | `src/electron-runtime.ts:1093-1110` |
| 主进程↔渲染层 | **不用 IPC/contextBridge**：同源 HTTP 路由（`/api/zeokeel/desktop/update/*`、`/_dsh/desktop/renderer-boot`、`pick-directory`、`loop-notify`、`voice-mic`、`/favicon.svg`、`/manifest.webmanifest`），写面必须过 `connection.requestRejection()` 的 BrowserAuth **持有性证明**（fence 缺席 ⇒ 503 fail-closed） | `src/write-proof.ts:1-30`、`src/desktop-update-route.ts:30-60`、`src/brand-web-route.ts` |
| 唯一的 preload | 沙箱 CJS preload，只把渲染进程未捕获错误经 DOM CustomEvent 桥 + IPC 转给主进程；**不向页面暴露任何 API** | `src/preload/renderer-error.ts:1-40` |

**第三方 UI 如何承载（两层，别混淆）**：
1. **第三方 Web 客户端插件**：就是 DSH client Cordis 插件，跑在同一个渲染层里，通过 slots（`sidebar` / keyed `main` / `rightbar` / `shell.overlay`）落座；`@zeokeel/dsh-panel-surface` 提供"中列面板"协议（`html[data-dsh-panel-active]` + 互斥广播事件）。证据：`packages/client/panel-surface/src/index.ts:1-40`、`packages/host/desktop/README.md`（"Advanced mode details"）
2. **第三方 WASM 应用**：完全隔离的独立窗口 + 独立自定义 origin（见 §6），与宿主 UI 只通过"应用中心面板 / 底部菜单 / 深链"发生关系。

> 对 Rust/Tauri 的直接含义：**把"壳"和"UI 运行时"彻底解耦**——壳只负责单实例、数据根、协议注册、窗口/托盘/更新；UI 永远是本机回环上的一个 Web 应用（同源），Tauri 的 webview 加载 `http://127.0.0.1:<port>`，自定义 scheme 用 Tauri 的 custom protocol 复刻 `zeokeel-app://` 语义。

---

## 2. 渠道（channel）体系

### 2.1 唯一定义与选择

| 项 | 事实 | 证据 |
|---|---|---|
| 定义位置 | 私有仓 `zeokeel/channels` 的 `channels/<id>/channel.json`（**不在本仓**；本仓只有 `brands/official/`） | `.github/workflows/ci.yml` "Fetch channel packages (private repo)"；`AGENTS.md` 铁律 0 |
| id 形状 | `^[a-z0-9][a-z0-9-]{0,31}$`（客户端 / 服务端 / CI 三处同源） | `scripts/ci-channels.sh`（`ID_PATTERN`）、`packages/host/desktop/src/desktop-channel.ts`（`CHANNEL_ID_PATTERN`）、`server/internal/channel` |
| 一致性 | `channel.json` 的 `channel_id` **必须等于目录名**，否则构建期抛错 | `packages/host/desktop/scripts/channel-build.ts:579-610` |
| 选择方式 | **构建参数**：环境变量 `DSH_BUILD_CHANNEL=<id>`（缺省 `official`），由 `ci-channels.sh` 从私有仓解析后逐渠道注入 | `channel-build.ts:311`、`scripts/ci-package-clients.sh` |
| 渠道集 | 由 ref 形态决定（唯一真源在脚本里）：正式 `vX.Y.Z` → 全部渠道；预发 `vX.Y.Z-beta.N` → 只 `beta`；非 tag → 只 `official` | `scripts/ci-release-policy.sh:1-40`、`ci-channels.sh`（消费 `channel_set`） |
| 服务端是否参与 | **不参与选择**。服务端只读镜像内 `/opt/zeokeel/channel/`（`ZEOKEEL_CHANNEL_DIR`）并对外下发；客户端不直连分发面，故"渠道身份"由它登录的服务端结构决定 | `server/internal/channel/channel.go:1-40`、`packages/host/desktop/src/desktop-release.ts:1-25` |

### 2.2 三类渠道的差异

| 维度 | `official` | `beta` | 品牌渠道（客户/OEM）|
|---|---|---|---|
| 品牌内容 | 官方（`brands/official`）| **复用官方**品牌（只换 channel_id/深链等）| 客户自有 logo/文案 |
| 数据根 `home_dir` | `.zeokeel-harness`（编译期常量）| **必须等于官方**（`.zeokeel-harness`）——预发版要延续正式版会话/登录态 | 必填且**不得**等于官方（`.acme-harness`）|
| `userData` 目录 | 产品名 | 产品名（与官方一致）| `<产品名> (<渠道id>)` |
| 更新面 | GitHub Release + R2 | GitHub Release(Pre-release) + R2 | **只有 R2** `<channel>/`；绝不进公开 Release |
| 产物中转 | 公开 artifact | 公开 artifact | 经 R2 临时前缀（HMAC 派生、无渠道名），取回即销毁 |
| mac 签名 | 签名+公证+staple | 只签名（不公证）| 签名+公证+staple（逐渠道 3 次重试）|
| 必填字段 | 模板可省（官方兜底）| 同官方（品牌字段须与 official 一致）| `identity.*`、`desktop.slug/app_id/deep_link_scheme/home_dir/app_origin_scheme`（+ 素材）|
| 门槛 | — | — | `MIN_EXPECTED_CHANNELS=2` 下限棘轮 + 可选 `channels.manifest.json` 双向对拍 |

证据：`docs/planning/2026-09-10-channel-package-reference.md` §2/§4.4/§5.0/§7、`scripts/ci-channels.sh`（`MIN_EXPECTED_CHANNELS`、`channels.manifest.json`）、`scripts/ci-channel-transfer.sh`、`docs/decisions/2026-09-11-channel-scoped-data-roots.md`

### 2.3 渠道影响面（一张表）

| 受影响项 | 键 | 生效时机 | 证据 |
|---|---|---|---|
| 品牌 | `identity.*` / `copy.*` / `assets.*` | 构建期（随包）+ 运行期（服务端下发，服务端胜出）| `server/internal/channel/channel.go:39-60` |
| 功能模块 | `features.{skills,apps,connectors}` | 构建期（Go `-tags` / 客户端 bundle 裁剪）| §4 |
| 数据根 | `desktop.home_dir`（+ `product_name` 派生 userData）| 运行期（主进程写 `DSH_HOME`）| `src/desktop-home.ts`、`src/desktop-user-data.ts` |
| 深链 | `desktop.deep_link_scheme`（默认 `zeokeel`）| 构建期（electron-builder `protocols`）+ 运行期注入 `zeokeel-session` | `channel-build.ts`、`src/deep-link.ts:1-45` |
| 应用 origin | `desktop.app_origin_scheme`（official/beta=`zeokeel-app`，**全部渠道必填、跨渠道唯一**）| 构建期（`registerSchemesAsPrivileged`，模块作用域）+ 服务端校验 | `channel-constants.ts:100-130`、`check-channel-json.mjs`（末尾跨渠道扫描）|
| 交付平台 | `desktop.platforms`（`mac`/`win`/`linux`，缺省三端）| 打包 + 镜像装配 + 门户下载区 + 更新清单一起收窄 | `channel-constants.ts:53-66`、`ci-build-channel-images.sh` |
| locale | **没有渠道级语言字段**（`desktop.locale` 不存在）。界面语言 = 用户偏好 → 系统/应用语言；安装器文案 `assistedMessages.yml` 三语（en/zh_CN/zh_TW）| — | `docs/planning/2026-09-16-channel-locale-copy.md`（结论是"建议"，代码零命中）、`brands/official/assistedMessages.yml` |

---

## 3. 品牌（brand）体系

### 3.1 `brands/official/` 的完整字段（`brand.json` schema 1）

| 分组 | 键 | 说明 |
|---|---|---|
| `identity` | `display_name` / `short_name` / `slug` / `app_id` / `company` / `copyright` / `description` | slug=安装包名 ASCII、app_id=反向域名、company/copyright 上安装包元数据 |
| | `deep_link{name,scheme}` | `protocols[0]` |
| | `linux{maintainer,synopsis,category}` | deb/Linux 元数据 |
| | `data_dir_name` / `product_home_dir` / `telemetry_release` | 数据根名、遥测发布名 |
| `assets` | `logo_primary` / `logo_dark` / `app_icon` / `accent` | 单段文件名（非路径）|
| | `tray{source,variants[]}` | 6 个派生位图（mac 模板 @1x/@2x + win/linux 彩色 @1/1.25/1.5/2）|
| | `mac_app_icon{source,canvas,artwork}` | 1024 画布 / 824 安全区 |
| `copy` | `window_title` / `product_name` / `login_title` / `login_fallback_*` / `session_title_zh` / `session_title_en` / `hero_headline` / `default_brand{}` | 支持 `{product}`/`{version}` 模板 |
| | `tray_update.{available,downloading}`（en/zh 各一份）、`installer_installing{en,zh_CN,zh_TW}`、`update_version_label`、`errors{…}`、`diagnostics_help` | **只有少数键是双语/三语**，其余单值 |
| `defaults` | `server_url` / `telemetry_channel` | 默认服务端地址（空=不预填）|
| `updates` | `mode:"github"` / `repository` / `asset_template` / `assets.{mac,win,linux}.{pattern,ext}` / `sha256` | 官方渠道的 GitHub 资产命名（**渠道不用**，见 §7）|
| `sources` | 每个 copy/assets 键 → **当前源码位置**（`file:line`）| 让"归一化改造"可跟踪，也是品牌点清单 |

证据：`brands/official/brand.json`、`brands/official/README.md`（派生规则 + 日夜双色铁律 + 官网边界）

### 3.2 派生流程（构建期）

```
brands/official/                      channels/<id>/  （公共渠道逐文件回落 official；品牌渠道不回落）
 ├─ logo.svg ──┬─→ generate-tray-icons ─→ build/tray-icon*.png（mac 模板取去方块后的 alpha；win/linux 方块换成 accent 色）
 │             └─→ build/web-brand/favicon.svg（本次构建品牌）
 ├─ app-icon.png ─┬─→ build/app-icon.png（win/linux）
 │                └─→ generate-mac-app-icon ─→ build/app-icon-mac.png（824/1024）
 ├─ logo-dark.svg ─→ 暗色面（不栅格化）
 ├─ assistedMessages.yml ─→ build/assistedMessages.yml（NSIS 引导文案）
 └─ brand.json ──→ 校验 + 托盘强调色（assets.accent）
                     + 另落 build/web-brand/official.svg（官方几何兜底，因为 asar 里没有 brands/**）
```

- 入口：`packages/host/desktop/scripts/brand-prepare.mjs`（`prepareBrandAssets`），已挂进 `yarn workspace dsh-plugin-desktop build`，并被**五个打包入口**显式调用（`prepareChannelPackaging()`）——历史 P0 是"这条链没人调用 ⇒ 渠道包带官方图标出厂"。证据：`docs/planning/2026-09-10-channel-package-reference.md` §5
- `build/` 全部是派生产物，**不入库**（`.gitignore` 显式列出每个文件）。证据：`.gitignore:18-35`

### 3.3 可换 / 不可换与"不遗漏"的保证

| 类别 | 内容 |
|---|---|
| 可换（字符串）| `identity.*`、`copy.*`、`desktop.product_name/window_title/shortcut_name/maintainer/synopsis`、`assets.accent`、安装器文案（三语 yml）|
| 可换（资源）| `logo.svg` / `logo-dark.svg` / `app-icon.png(1024² RGBA16+ICC)` / `favicon.*` |
| 不可换（技术）| `hardenedRuntime`、entitlements、asar 规则、electronFuses、更新清单 schema、`app_origin_scheme` 的语法、数据根目录名的正则形状 |
| 没有的东西 | **没有通用 i18n 词条表**：客户端 UI 文案在各插件的 `locales.ts`（zh/en 字典 + `ctx.locale` 订阅）；宿主侧文案走 `@zeokeel/host-locale`（探测 `desktopRuntime.locale` → `Accept-Language` → `zh`）。渠道品牌文案是**单语言**（双语化只有提案，未实现）|

校验链（"怎么保证不遗漏"）：

| 闸门 | 抓什么 | 证据 |
|---|---|---|
| `ci-channels.sh --require-template` | **官方渠道必须逐键显式**（含 `features.<m>` 叶键）；缺键点名 | `channel-constants.ts:87-130` |
| `scripts/lib/check-channel-json.mjs` | 逐字段：必填、形状（slug/app_id/scheme/home_dir/product_name 路径安全）、`server_url` 必须 https（回环例外）、素材存在 + app-icon 1024² RGBA16+ICC、`app_origin_scheme` ≠ `deep_link_scheme` 且非保留字；**报错只报字段名不回显取值** | 同文件 |
| 跨渠道 pass | `app_origin_scheme` 跨渠道唯一（品牌渠道之间 / 与公共渠道都不得撞）| 同文件末尾 |
| `verify-channel-package.ts` | **打包后**用同一 `prepareBrandAssets()` 重新派生并逐字节比对 + 断言 `channel_id` + 拆 `app.asar` 确认随包生效（官方包则断言"无渠道残留"）| `scripts/verify-channel-package.ts`、`ci-package-clients.sh --verify-app-dir` |
| `verification-brand-agnostic.spec.ts` | 验证脚本里出现品牌字面量标题断言、或从 `package.json` 取产品名 ⇒ 红 | `packages/host/desktop/tests/` |
| e2e 白标断言 | 渠道构建下断言页面标题/body 不出现厂商品牌（`E2E_FORCE_BRAND_LEAK_CHECK=1` 可在官方构建自证）| `docs/planning/2026-09-10-channel-package-reference.md` §4.1 |
| `check-no-real-domains.mjs` | 白名单式前向守卫：未登记 host/域名/渠道名一律失败（含提交信息区间）| `scripts/check-no-real-domains.mjs`、`AGENTS.md` 铁律 0 |
| 品牌点清单 | `docs/planning/2026-09-04-channel-brand-inventory.md`：逐项标注 🟥必改/🟨可选/🟩保留 | 同文件 |

---

## 4. 功能模块裁剪（module-contract → Go tags / bundle 裁剪 / webadmin 变体）

| 环节 | 事实 | 证据 |
|---|---|---|
| 单一真源 | 仓库根 `module-contract.json`：`version:1`，3 个模块 `skills/apps/connectors`，各带 `goConst`/`title`/`defaultInChannel`；**顺序=契约** | `module-contract.json` |
| 生成器 | `scripts/gen-module-contract.mjs` 生成 **6 文件 / 11 生成区**：`server/internal/features/features.go`（常量+`allModules`）、`server/webadmin/features.mjs`、`channel-constants.ts`（`FEATURE_MODULES` / `FEATURE_MODULE_DEFAULTS` / `'features.<m>'` 模板键）、`server/Makefile`（`GO_TAGS ?=`）、`server/Dockerfile`（3 个 ARG）、`.github/workflows/ci.yml`（`-tags=` 两处）| `gen-module-contract.mjs:1-60` |
| 门禁 | `--check` 在内存重算与磁盘**逐字节**比对；手改生成区 / 改了真源没重跑 ⇒ 红。文件读不到 = 红（不是 skip）| 同文件 |
| 棘轮 | `MAX_MODULES_DEFAULT_ON = 3`（缺省开的模块数上限）+ 纪律"**新增模块一律先 `defaultInChannel:false`**" | `gen-module-contract.mjs:92` |
| 渠道声明 | `channel.json` 顶层 `features.{skills,apps,connectors}`：缺键 = 按契约缺省；未知键/非布尔（含 `"false"` 字符串）⇒ **构建期抛错**；没有"全关"简写 | `channel-constants.ts:210-265` |
| 变体标识 | `channelFeaturesVariant()` → 逗号串（`skills,apps,connectors` / `apps` / `none`），消费键名 `FEATURES`，同一串同时喂 Go `-tags`、docker `--build-arg`、客户端清单 | `channel-constants.ts:282-300` |
| 服务端 | `go build -tags=…` 决定模块；`features` 包用 8 个穷尽 build-tag 文件（`features_skills_apps.go` 等）+ `router/optional_<m>_on.go`\|`_off.go` 成对。**关闭态不是"传 nil handler"**：`_off` 侧把 `WasmAPI` 变成空类型，引用其方法的注册代码**编不过**，模块的包只经 `_on.go` 的 import 进二进制 | `server/internal/features/*.go`、`server/internal/router/optional_{apps,skills,connectors}_{on,off}.go`、`docs/decisions/2026-10-14-server-build-tags.md` |
| 客户端 | 每个模块对应若干 bundle 层（`FEATURE_MODULE_BUNDLES`）；`requiredDesktopBundles()` 从活动清单里剪掉关闭的层；"安装面自有集合"**恒含**全部模块层（否则跨版本 `dsh.profile.bundles` 会把关闭的模块当第三方加回来）| `src/profile.ts:137-215`、`src/channel-features.ts` |
| 哨兵 | `zeokeel.feature.<module>.v1` 写在**真正被使用**的路径上；判据必须双侧（关闭变体无、开启变体有）| `channel-constants.ts:302-333` |

**edition/OEM 差异化怎么表达（现状）**：只有"渠道 × 三个布尔"两维，没有 edition 概念。企业版/个人版/客户版的差别事实上靠三件事组合：① `features` 三开关；② `desktop.platforms` 交付面收窄；③ 渠道品牌/数据根/服务端地址。**没有**按功能收费、许可校验或"版本内灰度"的机制。

---

## 5. 客户端如何连服务端

| 环节 | 机制 | 证据 |
|---|---|---|
| 登录入口 | 未登录时主窗口第一页是**客户端自有**的登录页（`zeokeel-auth-gate` 在本机回环 origin 上渲染 HTML，不是服务端页面）：填服务端地址 + 账号/密码，或走浏览器 SSO；登录经客户端**本机 API** 调网关完成，成功后同窗口 reload 进 DSH Web | `packages/host/enterprise/src/auth-gate.ts:1-40`、`packages/host/enterprise/src/loopback.ts` |
| 接口 | `POST /api/client/v2/auth/login`、`/auth/logout`、`/auth/me`、`/auth/usage`、`/auth/password`、`/auth/methods`；OIDC：`GET /api/client/v2/auth/<name>/login` → `…/callback` → 深链 `<scheme>://auth?token=…` | `server/internal/router/router.go:206-226` |
| 令牌落地 | `$DSH_HOME/session.json`（0600，`DSH_HOME` 指向系统关键目录时拒绝）；同时写入 `credentials` 引用 `ZEOKEEL_GATEWAY_TOKEN` | `packages/host/enterprise/src/session-service.ts:8-40`、`gateway-contract.ts` |
| AI 请求 | **全部经网关**：本地 `llm-deepseek` 适配器把 baseURL 指向 `<serverURL>/v1`，`Authorization: Bearer <会话令牌>`，端点 `/v1/messages`；客户端**不持有上游 API key**。web_search 也走同一网关与同一凭据 | `packages/host/enterprise/src/gateway-llm.ts:1-30`、`server/internal/router/router.go:280-311` |
| 配置下发 | `GET /api/client/v2/config/bootstrap`（Bearer）→ `{models[], default_model, skills[], mcp[], web{default_thinking_level, error_reporting_*}}`，客户端把它投影成 settings（`llm-deepseek` 模型表、`agent-default-model`、`web-search-deepseek`）| `server-connector/bootstrap.ts:1-60`、`enterprise/src/bootstrap.ts:60-158` |
| 其它下发 | `GET /api/client/v2/channel`(+`/channel/logo`、`/channel/logo-dark`、`/channel/favicon`)、`/api/client/v2/skills/builtin`(+`/:name/archive`)、`/api/client/v2/apps/wasm/catalog` | `router.go:233-247`、`server/Dockerfile:233-234` |
| 品牌优先级 | 服务端 `/api/client/v2/channel` > 随包 `build/channel.json` > 内置官方；逐字段 `mergeChannel`（服务端有值即胜出），且服务端载荷必须至少含一个非空品牌字段才采纳 | `docs/planning/2026-09-10-channel-package-reference.md` §4.1 |
| 网络 | **默认禁止一切代理**（`no-proxy-server`，模块作用域 append）：默认 session / 内置浏览器分区 / 应用窗口全覆盖；`ZEOKEEL_ALLOW_SYSTEM_PROXY` > 渠道 `desktop.allow_system_proxy` > 禁 | `src/main.ts:167-183`、`docs/decisions/2026-09-22-client-system-proxy-ban.md` |
| 离线 | UI 可离线启动（会话从 `session.json` 恢复）；bootstrap 失败 → `EMPTY` 配置（**分两类**：`empty` vs `default_model_substituted`，前者才丢 DSN）；401/过期 ⇒ 清会话回到登录页；品牌回落随包；但**AI、WASM 应用、更新都硬依赖服务端**；WASM 未登录页给可读 HTML 而不是白屏 | `server-connector/bootstrap.ts:5-40`、`wasm-apps-host/README.md`（"错误呈现分两档"）|

---

## 6. WASM 应用在客户端如何打开

```
应用中心面板（宿主 client 插件） ──POST /api/zeokeel/wasm-apps/open──▶ 宿主任 hosted handler
   体 {app_id, path?, window?}                                          （写面必须过持有性证明）
        └─▶ 新建 BrowserWindow（partition: persist:agent-browser-<user>@<server-hash>）
              load <app_origin_scheme>://<app_id>/<path>
                   │ Chromium 交给已注册的协议 handler
                   ▼
              handler：① app_id 只取 URL host 段  ② 取会话令牌+serverURL
                       ③ 补 Origin: <scheme>://<app_id>
                       ④ POST <serverURL>/api/client/v2/apps/wasm/<app_id>/request
                       ⑤ {status,headers,body(base64)} → Response（丢 Set-Cookie 与逐跳头）
```

| 要点 | 事实 | 证据 |
|---|---|---|
| 自定义 origin | `protocol.registerSchemesAsPrivileged([{scheme, privileges:{standard,secure,supportFetchAPI,corsEnabled:false,stream,codeCache}}])`，必须在 `app.whenReady()` 之前（模块作用域）| `packages/host/wasm-apps-host/src/app-protocol.ts:1-40`、`wasm-apps-host/README.md`「装配」 |
| 窗口闸门 | 每个应用独立窗口（非内置浏览器标签）+ 独立 partition（后缀 = 服务端地址 sha256 前 32 hex，切服务端不跨租户）；`window:{ratio,width,height}` 形状与目录行逐字相同，缺省回查一次 catalog | `wasm-apps-host/README.md`、`src/wasm-apps-windows.ts` |
| 深链 | `<channel scheme>://app/<app_id>`；桌面壳**白名单** action（`auth` / `app`），未知 action/scheme/host 一律丢弃，**不回落**；长度上限 4096 | `packages/host/desktop/src/deep-link.ts:20-50` |
| 数据库桥 | **客户端没有 DB 桥**。应用的 SQLite 与 wasm 实例都在服务端（`appserver.serveApp`），客户端只转发信封。客户端侧唯一的本地面是内容缓存（键 = session-scope + app_id + version + path，仅静态子资源直出，登出清空）| `wasm-apps-host/README.md`、`docs/decisions/2026-09-19-wasm-client-internal-origin.md:1-45` |
| 与宿主 AI | `POST /__zeokeel/ai/chat` 由协议 handler **本地**处理（不进信封、不出站）：首次授权闸门 → 用隐藏会话 `app:<app_id>#<账号作用域>` 跑一轮仅对话的 AI → SSE 回给应用页。其余 `__zeokeel/*` 一律 404 | `wasm-apps-host/src/ai-chat.ts:1-25`、`src/handler.ts:273-280` |
| 与宿主 UI 的关系 | 三种入口：应用中心面板（`panel-surface` 中列协议）、底部菜单条目（`foot-menu`）、深链；窗口建好后注册进 `browserSurface` 注册表（给 CDP/浏览器能力用），关窗/登出注销 | `packages/client/panel-surface/src/index.ts`、`packages/client/foot-menu/README.md`、`wasm-apps-host/README.md` |
| 安全边界 | handler **fail-closed**：畸形 URL / 超限（请求 1 MiB、响应 8 MiB）/ 非信封响应 → 可读错误页。服务端才是准入/执行/计量的权威 | `wasm-apps-host/README.md` |

---

## 7. 更新与发布

| 环节 | 事实 | 证据 |
|---|---|---|
| 客户端更新源 | **只从登录的那台服务端**：`GET {serverURL}/api/client/v2/updates/manifest`。客户端没有任何指向 `release.zeokeel.com` 的路径（这就是"渠道不可能错乱"的结构保证）| `src/desktop-release.ts:1-25`、`src/updates.ts` |
| 清单 schema | `{schema:1, channel_id, client:{version, assets:{mac-universal,win-x64,linux-x64:{url,sha256,size}}}}`；`url` 必须**绝对 https**、`sha256` 必须 64 hex、`channel_id` 必须等于期望渠道（否则拒绝 = 不允许跨渠道"洗包"）；读不到 `client` 段时给 `client_unavailable` 原因（与"已是最新"区分）| `src/desktop-release.ts:60-200` |
| 下载与安装 | 断点续传（`.partial` + `.partial.json`，Range）+ SHA-256 校验 + 完成后复用；win：静默启动 NSIS `/S --updated --force-run`；mac：**原地替换**（helper `/bin/sh` detached，ditto 到同卷暂存 → `codesign --verify --strict` → 换包 → 等新版写启动标记，30s 无标记则回滚）| `src/update-download.ts:1-25`、`src/electron-runtime.ts:1043-1068`、`src/macos-update-install.ts:1-45` |
| 安装包如何随镜像 | CI 三平台 job 产出 → `client-assets/<channel>/` → release job 打成 `client-assets/client/` + `CLIENT-RELEASE.json` → `docker build --build-context clientassets=` → 镜像内 `/opt/zeokeel/client/`（`ZEOKEEL_CLIENT_RELEASE_DIR`）→ 服务端 `GET /updates/client/<file>` + 清单里拼绝对 url（来源 `RequestOrigin`/`ZEOKEEL_PUBLIC_BASE_URL`）| `scripts/ci-build-channel-images.sh:160-215`、`server/Dockerfile:203-227`、`server/internal/clientrelease/clientrelease.go:1-120` |
| 门户页下载 | 服务端根路径 `/`（`/portal`）是**零脚本 HTML**，下载卡片读同一份 `CLIENT-RELEASE.json`，并按 `desktop.platforms` 收窄（未声明平台整块不渲染，而不是灰卡片）| `server/internal/portal/portal.go:1-30`、`channel.go:79` |
| R2 布局（服务端自检用）| `<channel>/releases/<ver>/zeokeel-server-<ver>-amd64.zip` + `SHA256SUMS`（immutable）、`<channel>/latest.json`（no-cache）；上传前后逐字节校验（size + ChecksumSHA256），不过就不写 `latest.json`；保留策略按版本目录删 | `scripts/ci-publish-update-server.sh:1-50` |
| 版本号 | **git tag 是唯一真源**（`vX.Y.Z`），`scripts/version.mjs check` 强制 tag == 两处 `package.json`；镜像打三个 tag（`vX.Y.Z`、`X.Y.Z`、`<channel>-X.Y.Z`，后者用于同机多栈隔离）| `scripts/version.mjs:1-30`、`ci-build-channel-images.sh:235-260` |
| 发布面 | GitHub Release **只有公开渠道**（official/beta）的 server zip + SHA256SUMS；发布说明必须策展（`docs/releases/<tag>.md`），正式与预发都要求；品牌渠道只进 R2 | `.github/workflows/ci.yml:1872-2070` |
| 代码签名 | mac：Developer ID + hardened runtime + `notarize:true` + staple；凭据来自 `MAC_CERT_P12_BASE64`/`MACOS_SIGN_IDENTITY`/`CSC_KEY_PASSWORD` + Apple 三选一（keychain profile / Apple ID trio / ASC API key），`release-preflight.ts` fail-loud。**Windows：不签名**（`--config.win.signExecutable=false` + `CSC_IDENTITY_AUTO_DISCOVERY=false`）| `release-preflight.ts:1-30`、`packages/host/desktop/package.json build.mac`、`scripts/package-win.ts:194` |

---

## 8. OEM 交付流水线

### 8.1 输入清单（一个新客户要准备什么）

| 输入 | 形态/约束 | 谁提供 |
|---|---|---|
| 渠道 id | `^[a-z0-9][a-z0-9-]{0,31}$`，与目录名/`channel_id`/R2 目录/镜像 tag 全一致 | 我方定 |
| `channel.json` | 官方模板**逐键显式**（`CHANNEL_TEMPLATE_KEYS` 共 30 键 = 27 个身份/文案/桌面键 + 3 个 `features.<m>` 叶键）| 我方（技术）+ 客户（品牌）|
| `logo.svg` / `app-icon.png` | logo 必须**平坦十六进制色方块 + 方块外有 mark**（托盘模板取 alpha）；app-icon 1024² RGBA16+ICC | 客户 |
| `assistedMessages.yml`（可选）| NSIS 引导文案三语 | 我方 |
| 签名凭据 | mac 证书 + 公证凭据（渠道与官方**同一张厂商证书**，2026-09-10 定案）| 我方 |
| R2 凭据 | 上传 + HMAC 中转前缀 | 我方 |
| 私有仓凭据 | `CHANNELS_REPO_SSH_KEY`（**推荐**：只读 deploy key，不过期、只对一个仓）或 `CHANNELS_REPO_TOKEN`（细粒度 PAT，会过期）| 我方 |

### 8.2 步骤（从建目录到"镜像 + 三平台安装包"）

1. **私有仓建目录**：`zeokeel/channels` → `channels/<id>/`（`channel.json` + 素材）。在 `channels.manifest.json` 里登记（否则"少发一个渠道"无人发现）。
2. **CI 取包（gate 一次 pin）**：`scripts/ci-channels.sh --require-template` → 克隆/按 pin 检出 → 逐渠道校验（模板完整性、必填、形状、素材、跨渠道 `app_origin_scheme` 唯一）→ 写 `channels.list`；渠道 id 全程 `::add-mask::`，日志只报数量。
3. **三平台打包**：每个平台 job 里 `scripts/ci-package-clients.sh --official-last --patterns … -- <dist 命令>`，逐渠道 `DSH_BUILD_CHANNEL=<id>`，产物归 `client-assets/<channel>/`，每渠道跑 `verify-channel-package.ts`（重派生逐字节 + 拆 asar）。
4. **品牌渠道产物中转**：品牌渠道安装包走 R2 `_transfer/<run-id>-<HMAC>/ch-<index>/`（路径里没有渠道 id），官方/beta 仍走公开 artifact。
5. **逐渠道建镜像**：release job 取回中转 → `ci-build-channel-images.sh` 生成 `CLIENT-RELEASE.json`（逐平台必需，缺则红）→ `channels-context/<id>/` → `docker buildx --build-arg CHANNEL=<id> --build-context clientassets= --build-context channelassets=` → 镜像内断言 `/opt/zeokeel/client/CLIENT-RELEASE.json`、`/opt/zeokeel/channel/channel.json`、逐平台键存在 → 导出 `release-bundle/<channel>/zeokeel-server-<ver>-amd64.zip` + `SHA256SUMS`。
6. **上传更新服务器**：`ci-publish-update-server.sh --list channels.list` → 逐渠道 `releases/<ver>/…`（immutable）+ `latest.json`（no-cache，含 `version`/`image_tag`/`image_asset`），上传后完整性校验不过就不写指针。
7. **GitHub Release**（仅公开渠道）+ 销毁中转前缀（`if: always()`）。
8. **客户侧部署**：`unzip -p … image.tar | docker load` → `.env` 四键（`SERVER_IMAGE=<channel>-<ver>` 做同机多栈隔离）→ `docker compose up -d`。

证据：`.github/workflows/ci.yml`（desktop-linux/windows/macos、release job）、`scripts/ci-{channels,package-clients,channel-transfer,build-channel-images,publish-update-server,release-policy}.sh`、`server/Dockerfile:195-300`、`docs/deploy/AI-DEPLOY.md:280-305`

### 8.3 私有代码怎么进 CI / 有没有 submodule

| 项 | 事实 |
|---|---|
| `.gitmodules` | **只有一条**：`deepseek-harness` → `https://github.com/deepseek-ai/deepseek-harness.git`（上游镜像 submodule；本地检出里该目录为空）。**渠道仓不是 submodule** |
| 渠道仓获取 | CI 运行时克隆（`git clone --depth 1` 或按 pin `fetch --depth 1 <sha>`），凭据三选一：`CI_CHANNELS_URL`（自建/本地）> SSH deploy key > PAT |
| 同源性 | 发布 tag 上 **pin 必填**：gate 用 `--resolve-only` 解析一次 `channels_rev`，四处调用点共用 ⇒ 同一次 tag 的"包内客户端"与"镜像内渠道内容"必然同源、可复现 |
| 本地开发 | `CI_CHANNELS_SOURCE` 指向本地目录（跳过克隆），此时"渠道目录数下限"判据自动跳过（夹具规模不同）|

---

## 9. 对我们（Rust/Tauri + 自有 zk 组合器 + OEM 私有索引仓）的 5 条经验 + 3 个坑

### 值得抄的 5 条

1. **"渠道 = 一个目录 + 一份 JSON"是正确的最小单元**，而且必须**随包分发**（登录前就要品牌：服务端地址是鸡生蛋问题）。我们的 OEM 私有索引仓应只装数据（`channels/<id>/`），不装代码/脚本/证书，让"新客户"是一次纯数据 diff + 可评审。
2. **单一真源 + 代码生成 + 逐字节 `--check`**，比"多处登记 + 交叉对拍"强一个量级。`module-contract.json → 6 文件 11 区`这套结构应直接搬：一个 `contract.json` 生成 Go 常量/`-tags`、Rust `cfg` feature、TS 常量、Dockerfile ARG、CI 矩阵，任何手改都红。
3. **跨渠道唯一性必须在构建期做**（`app_origin_scheme` 跨渠道唯一、数据根不得撞、`product_name` 路径安全），因为运行期只有"能用/不能用"，且故障现象与配置毫无关系。Tauri 侧对应物：自定义 protocol scheme、`identifier`、数据根目录名，全部要跨渠道唯一 + 形状校验。
4. **产物级白标校验**（打包后重新派生并逐字节比对、拆包断言 `channel_id`、e2e 断言页面不出现厂商名）比"构建参数传对了"可信得多。移植建议：安装包出包后跑一个 `verify-channel` 二级程序，输入 = 渠道目录，输出 = 逐项 PASS/SKIP（并**如实标注跳过**，别让门禁看起来在跑）。
5. **凭证/身份纪律必须写进脚本层**：deploy key 优先于 PAT；渠道 id 一读到就 `::add-mask::`；错误信息只报字段名不回显取值；品牌渠道产物不进公开 artifact，走 HMAC 派生的临时前缀。Rust 侧同样适用（CI 日志、构建产物名、崩溃报告）。

### 3 个坑（我们一定会遇到）

1. **"配置里少写一个键"会静默继承厂商身份**。他们用三层兜底（官方模板逐键必填 + 品牌渠道禁止回落 + 打包后重派生）才堵住；且**beta 与 official 必须共用数据根**这条反直觉规则来自一次真实事故（预发用户升级后"所有对话都没了"）。→ 我们的数据根/namespace 规则要**显式成对**写清"哪种渠道共用、哪种必须分开"，并各有一条 CI 用例。
2. **Windows 安装包当前完全没签名**（`signExecutable=false`），mac 才有 Developer ID + 公证 + staple。这是交付面最大的现实缺口；我们若做白标，签名主体与证书链必须一开始就决定（用谁的证书、能不能给客户自己的），否则"客户验收"这一关过不去。
3. **多入口 = 多份真相**。他们的历史 P0 全是这一类：`brand-prepare` 没被打包入口调用（渠道包带官方图标出厂）；`protocols` 数组不能用 CLI 点号覆盖；`--verify-app-dir` 相对路径解析失败被静默丢弃（asar 断言"从未执行过"）；插件自己读随包 `channel.json`（打包后路径失效 → 静默回落官方 scheme）。→ 我们的原则：**每一个"必须发生的派生/校验"都要有一个断言它在本次构建里真的跑过**（布尔证据进日志），并且**只有一处**能读到渠道内容（组装期注入，插件无权自读）。

---

## 附录 A · 关键路径索引

| 主题 | 路径 |
|---|---|
| 模块契约 | `module-contract.json`、`scripts/gen-module-contract.mjs`、`scripts/channel-modules.mjs`、`packages/host/desktop/scripts/channel-constants.ts` |
| 渠道构建/校验 | `scripts/ci-channels.sh`、`scripts/lib/check-channel-json.mjs`、`packages/host/desktop/scripts/channel-build.ts`、`channel-prepare.ts`、`channel-variant.ts`、`verify-channel-package.ts` |
| 渠道运行时 | `packages/host/desktop/src/desktop-channel.ts`、`desktop-home.ts`、`desktop-user-data.ts`、`channel-features.ts` |
| 品牌 | `brands/official/{brand.json,logo.svg,logo-dark.svg,app-icon.png,assistedMessages.yml}`、`packages/host/desktop/scripts/brand-prepare.mjs`、`generate-tray-icons.mjs`、`generate-mac-app-icon.mjs` |
| profile/补丁层 | `packages/host/desktop/src/profile.ts`、`cordis.patch.yml`、`*/*/cordis.patch.yml` |
| 壳/窗口/托盘/IPC | `src/main.ts`、`src/electron-runtime.ts`、`src/window-options.ts`、`src/runtime.ts`、`src/write-proof.ts`、`src/preload/renderer-error.ts` |
| 更新 | `src/desktop-release.ts`、`update-checker.ts`、`update-download.ts`、`updates.ts`、`macos-update-install.ts`、`src/client/desktop-update.tsx` |
| WASM | `packages/host/wasm-apps-host/README.md`、`src/{handler,app-protocol,ai-chat,electron-adapter}.ts`、`packages/client/wasm-apps/`、`docs/decisions/2026-09-19-wasm-client-internal-origin.md` |
| 服务端 | `server/internal/{channel,clientrelease,portal,features,updatecheck}/`、`server/internal/router/router.go`、`server/Dockerfile`、`docs/deploy/AI-DEPLOY.md` |
| 发布 | `.github/workflows/ci.yml`、`scripts/ci-{release-policy,package-clients,channel-transfer,build-channel-images,publish-update-server}.sh`、`scripts/version.mjs` |
| 设计/决策 | `docs/planning/2026-09-10-channel-package-reference.md`、`docs/decisions/2026-10-13-channel-feature-modules.md`、`2026-09-11-channel-scoped-data-roots.md`、`AGENTS.md`（铁律 0）|

## 附录 B · 未验证 / 需注意

1. 本仓只有 `brands/official/`；私有仓 `zeokeel/channels` 的内容（`official`/`beta` 的实际 `channel.json`）不可见，"官方模板 28 键全部显式"只能从 `CHANNEL_TEMPLATE_KEYS` 与 CI 判据推断。
2. 渠道文案多语言（`copy.login_tagline` 对象形态、`defaults.locale`）只有**提案**（`docs/planning/2026-09-16-channel-locale-copy.md`），代码零命中——不要当作既有能力。
3. `server/docker/` 只有 `entrypoint.sh`；镜像装配逻辑在 `server/Dockerfile` + `ci-build-channel-images.sh`（与任务书里"server/docker/"的预期略有差异）。
4. `module-contract` 的三模块（skills/apps/connectors）之外没有 edition 维度；若我们要做"企业版/个人版"，需要新增**第二个维度**并在生成器里显式建模，而不是复用 `features`。
5. 客户端 `desktop.platforms` 只影响交付面，不改变代码；mac 交付只有 arm64（`build.mac.arch:["arm64"]`）。
