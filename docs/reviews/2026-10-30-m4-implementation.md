# 2026-10-30 · M4 WASM 应用中心实现对抗性验证

- **评审对象**：`crates/zk-kit-wasm`、`packages/contracts/app`、`packages/server/{apps,app-runtime,app-db}`、`packages/admin/admin-apps`、`packages/ui/ui-apps`、`apps/desktop/src/ipc.rs` `app_request`、`apps/desktop/web`、`scripts/apps-e2e.sh`、`e2e/m4-*`、docs/04/07/24、ADR-062
- **评审者**：audit-arch（独立只读；仅写本记录）
- **结论**：CHANGES-REQUESTED
- **关联 spec**：`specs/changes/2026-10-30-wasm-app-center`
- 证据日期：2026-10-30；本记录为唯一新增文件，未改产品代码/spec。

## 1. 结论

**CHANGES REQUIRED（CHANGES-REQUESTED）。**

服务端 happy path 已能跑通：员工/管理员上传 API（实际 e2e 用 admin token）、审批、grants、open、request、每 app SQLite、重启持久、跨 app DB、下架 403/410；`apps-e2e.sh`、`admin-e2e.sh`、`cargo test`、`compose` 均能绿。但 M4 的实现与 spec 有**关键安全/退出条件缺口**：**没有独立 Tauri origin 窗口**（实际是主 WebView 内 iframe）、**`can_open` 授权被忽略**、**rollback/delete/purge/retire 生命周期未实现**、**资源/配额大部分未强制执行**，且安全/限额/UI 测试基本缺位。不能 APPROVED。

## 2. 证据命令与结果

| 命令 | 结果 |
| --- | --- |
| `bash scripts/apps-e2e.sh` | **PASS**（EXIT 0）：`apps e2e ok: upload/approve/open/request/restart/delist`；仅 happy path，无 limits/UI/employee/Tauri |
| `bash scripts/admin-e2e.sh` | **PASS**（EXIT 0）：9 passed（M2 6 + M3 2 + M4 admin 1） |
| `bash scripts/login-e2e.sh` | **PASS**（EXIT 0）：3 passed；**不含 M4 client/app 用例** |
| `cargo test -p zk-kit-wasm -p zk-srv-apps -p zk-srv-app-db --offline` | **PASS**（EXIT 0）；注意 `go_and_rust_guests_share_frame_semantics` **1 ignored** |
| `cargo test --workspace --offline` | **PASS**（EXIT 0） |
| `zk compose --edition enterprise --profile default --channel official --check` | **PASS**：`compose: up to date (16 files)` |
| `zk validate` | **PASS**：69 packages scanned；enterprise 44 packages/46 roles |
| `bash scripts/schema-check.sh` | **PASS** |
| `zk db lock --check` | **PASS**：12 migrations |
| `apps/desktop/tauri.conf.json` | `withGlobalTauri: true`、`security.csp: null`；**没有** `apps/desktop/capabilities/` |
| `apps/desktop/web/src/App.tsx:245,374`、`ui-apps/web/src/index.tsx:53` | app 在主 WebView 内以 **iframe `srcDoc`** 打开；没有 `WebviewWindow`/per-app scheme/partition |
| `apps/desktop/src/ipc.rs:254-278` | `app_request` 仅校验 app_id 字符；主 WebView 可直接调用，无 capability |
| `apps/server/apps/src/lib.rs:178-196` | request 只按 app 维度 semaphore；没有 per-user/global/queue、1MiB/8MiB、日志/配额落库 |
| `apps/server/apps/src/lib.rs:199-215` | asset 固定 `text/html`，不校验 ui manifest；UI 静态资源只会 index.html |
| `apps/server/apps/src/lib.rs:233-248` | rollback 只改 status；delete 不写 deleted_at；`purge` 未处理 |
| `apps/server/apps/migrations/0012_apps.sql` | 有 `app_databases`/`app_calls`/`app_runtime_limits`；无 `app_runtime_logs`；`app_databases` 从未写入 |

## 3. P0（阻塞）

### P0-1 · 没有独立 Tauri origin 窗口/partition/capability；主 WebView 内 iframe 不满足 M4 隔离

- `App.tsx` 的 `AppSurface` 来自 `@zeokeel/ui-apps`，实际是主 WebView 内的 `<iframe sandbox="allow-scripts" srcDoc=...>`（`ui-apps/web/src/index.tsx:53-57`）；没有 `Tauri WebviewWindow`、每 app origin `zk-app://<id>/`、partition、导航闸门。
- `apps/desktop/tauri.conf.json`：`withGlobalTauri: true`、`csp: null`；且 **没有 `apps/desktop/capabilities/`**。主窗口 WebView 拥有全局 Tauri IPC，任何 app 页面/脚本可尝试调用主窗口命令；`app_request` 也没有被限制为 app 窗口唯一命令。
- `apps/desktop/src/ipc.rs:254-278` 的 `app_request` 是全局 IPC 命令；没有 capability/窗口校验。
- 这直接违反 docs/04:222/224、docs/21:82、ADR-062 的“独立窗口 + 独立 partition + 只允许 app_request”。

**修法**：新建 app 窗口 capability（只允许 `app_request`），主窗口不暴露；`withGlobalTauri` 对 app WebView 关闭/隔离；实现 Tauri `WebviewWindow`/自定义 scheme `zk-app://<app_id>/`、channel 唯一 scheme、per-app partition、导航闸门、窗口关闭清理；`AppSurface` iframe 仅 serve e2e 载体；补跨窗口 IPC 403/导航逃逸测试。

### P0-2 · `can_open` 被忽略：被 grant 但 `can_open=false` 的用户仍可 open/request

- `admin_grants` 将 `can_open` 写入 `content_grants.can_install`（`apps/src/lib.rs:257-268`）。
- `catalog`（`apps/src/lib.rs:116-134`）、`open`（`:136-159`）、`request`（`:161-197`）、`asset`（`:199-215`）都只检查 visibility/grant 存在，**从不检查 `can_install`/`can_open`**。
- 结果是：管理员可把用户设为“可见但不可打开”，实际仍可 open/request；M4 权限/grants 语义失效。

**修法**：查询统一加 `can_install = true`（或新增显式 `can_open` 列，避免与 M3 `can_install` 复用混淆）；contract/API/管理台 grants 明确字段；补 can_open=false 的 open/request/asset 403/404 测试。

### P0-3 · `rollback`/`delete`/`purge`/`app-retire` 生命周期未实现

- `admin_action`（`apps/src/lib.rs:233-248`）：
  - `rollback` 仅 `UPDATE contents SET status='approved'`，**没有切换 current_release**，不是回滚。
  - `delete` 仅 `status='delisted'`，没有 `deleted_at` 软删、没有 deployment/审计语义。
  - `purge` 直接落到 `_ => 400`；docs/04:114 明确有 purge。
- 没有 `app-retire` job（docs/06:99、spec:32）；没有保留 30 天/真删 wasm/UI/DB/logs/calls；`app_calls` 也没有清理。
- 退役/删除是 M4 退出条件与数据保留/审计的一部分，当前不可执行。

**修法**：实现 rollback 切换 `current_release`（且校验 schema_version 兼容）、delete 写 `deleted_at` + deployment、purge 仅 superadmin、app-retire job（每小时、幂等、审计、保留期配置）；补 rollback/delete/purge/retire 的 e2e 和失败回滚测试。

### P0-4 · 资源/配额大部分未强制执行，M4 “资源耗尽被挡住”不成立

- `request` 只有 per-app `Semaphore(4)`（`apps/src/lib.rs:182`、`:29-36`）；**没有 per-user 8/global 32/queue 64**，没有端到端 60s 排队，只有 runtime 30s。
- 没有请求 1MiB/响应 8MiB 的 HTTP 层强制；`DefaultBodyLimit::max(49MB)`（`apps/src/lib.rs:49`）反而放大了上传/请求体积。
- `app_databases` 表从未插入/更新；`app_runtime_limits` 表未使用；`app_runtime_logs` 表不存在；没有每 app DB 20GB 总配额、日志 10 万行、调用清理、全局 DB 限额。
- `enforce_quota` 只在 guest 启动前查 DB 大小；guest 一次 `INSERT` 仍可把 DB 写超 100MB；没有写前/写后配额检查。
- `app_calls` 插入错误被 `let _ =` 吞掉；审计/诊断 fail-open。
- 因此 fuel/memory 有单测，但 M4 退出“资源耗尽（内存/CPU/时间/连接）被限额与超时挡住”整体不成立。

**修法**：在 server 层实现 per-user/global 并发 + queue + 60s e2e timeout；HTTP body 1MiB/响应 8MiB；app_databases 行/size/updated 落库；app_runtime_limits 读取；DB 写前/写后配额（含 20GB 总配额）；日志表/全局 cap/保留；app_calls 写入失败 fail-closed 或明确降级；补每项限额 e2e/单测。

### P0-5 · 静态 UI 资产路径/入口不可用，且缺 CSP（XSS/外网风险）

- `fetchAppHtml` 固定请求 `/assets/index.html`（`apps/desktop/web/src/store.ts`/App.tsx），忽略 `open` 返回的 `ui_manifest`/入口；`asset` 对所有文件固定 `content-type: text/html`（`apps/src/lib.rs:213-214`），JS/CSS/图片类型错误，UI 静态资源无法正常加载。
- `AppSurface` 直接把服务端返回的 HTML 放进 `srcDoc`（`ui-apps/web/src/index.tsx:57`），没有 `CSP`/`sandbox` 的网络限制；`tauri.conf` `csp:null`。app HTML 可 `fetch` 外部网络、内联脚本可跑，违背 M4“静态 UI 资产/auth/UI XSS”安全目标。
- `asset` 只用 `path.contains("..")/":"` 粗检，未按 release `ui/` manifest 白名单；content-type 统一 HTML。

**修法**：open 返回明确的 UI entry/manifest；前端按 manifest 加载入口、相对资源；asset 按后缀设置 MIME 且只服务 manifest 白名单；iframe `sandbox` 去网络能力（CSP `connect-src 'none'`）；Tauri app 窗口独立 capability + CSP；补 XSS/外链/路径穿越/MIME 测试。

## 4. P1（应修）

1. **SQL 5s/DB 大小写时限制未实现**：`zk-kit-wasm` 只有 guest epoch 30s，没有单条 SQL 5s；`validate_sql` 是词法黑名单，不是 spec/design 的 AST 单语句白名单；`db_query` 无 row/响应上限；`app_db` 迁移无“≤32 条/≤64KB total/≤16 表”的 manifest 校验（`ensure_schema` 只在执行后查表数）。
2. **迁移/版本记录不完整**：`ensure_schema` 不校验 migration 连续/总数/总大小；`app_databases.schema_version` 不落库；`ensure_schema` 对缺失版本会返回 target_version 而未真正应用；没有 migration rollback 测试；`db_migrations` 上传时未预校验。
3. **`app_request`/C10**：`apps` 使用本地 `FrameBody`，没有复用 `AppRequest`/`AppResponse`；response 未用 schema 校验；`app_request` JSON 无大小/字段白名单；前端无 Ajv。C4/C10 未闭合。
4. **员工上传/owner/transfer 权限**：`upload` 只 `current_user`，未校验 `apps:publish`；没有 owner 转移 API；admin 只有 approve/reject/delist，没有 transfer/versions 的完整管理路由；`admin-e2e` 只用 admin token，未验证员工 pending/非 owner 403。
5. **`asset.read`/guest assets 不可用**：`app-runtime::execute` 传空 `HashMap`；`asset.read` 永远 -3；spec 的 asset.read 数据面未实现。
6. **`app_request`/UI 测试缺失**：`AppSurface` 无 Playwright；`login-e2e` 无 M4 client；`m4-admin-apps` 只 approve/grant/delist seeded app，不测上传/打开/请求/错误页/axe；`apps-e2e` 无 limits/AI/network/file/SQL/UI/Tauri。
7. **测试假覆盖**：`go_and_rust_guests_share_frame_semantics` 被 `#[ignore]`；memory 测试接受 `Memory|Fuel|Timeout` 任一，不证明内存错误；没有 SQL 5s、can_open、rollback、purge、配额、日志、CSP 测试。
8. **审计 fail-open**：`app_calls` insert 用 `let _ =`；request/limit 没有 `audit()`；`open` 有 audit，但 limit/请求失败不写；违反“审计失败 fail-closed”。
9. **C3 命名/四件套**：`packages/admin/admin-apps` crate 名/`zk-admin-admin-apps` 的 `<page>` 语义需按 C3 检查；`ui-apps`/`admin-apps` conformance 目前仍可能只查 manifest 存在（需补真实契约/conformance）。
10. **`app_request` 无窗口/来源校验**：主 WebView 任意调用；见 P0-1。
11. **UI 资源版本竞态**：open 返回 version 但 `fetchAppHtml` 不按 version 取 asset；rollback/新 release 期间可能 UI/guest 版本不一致。
12. **`app_databases`/`app_runtime_limits` 表是“死表”**：无写入、无读取；diagnostics 无法给出真实 DB/call 数据。

## 5. P2（建议）

- `asset` 无 ETag/Cache-Control；docs/04 要求缓存键/ETag。
- `AppSurface` `onLoad` 里 `frame.addEventListener("load", ...)` 在 load 后注册，监听器清理时序不可靠；close 后 window listener 可能泄漏。
- `request` 的 `body_b64` 解析失败静默 `Value::Null`，应 `APP_BAD_REQUEST`。
- `app_calls` 保留 30 天/`app_runtime_logs` 1000 行/全局 10 万行需后台清理 job；当前无 job。
- `app_request` 方法/路径未做白名单；guest 可能收到异常 method/path，需统一错误。
- `apps-e2e` 使用 admin JWT 上传，没有 employee 用户；应补 employee/owner/transfer 用例。
- `app_db` 文件 0600 未在 server 侧显式设置（仅 DB 文件由 SQLite 创建，umask 决定）；补权限断言。
- `ui-apps`/`admin-apps` 的 UI 包 `zk.toml`/README/conformance 需按 C3 四件套复核。

## 6. 退出条件对照

| M4 退出条件 | 结果 |
| --- | --- |
| 员工上传→审批→独立窗口打开→写 DB→重启仍在 | **PARTIAL/FAIL**：server e2e 用 admin token；重启 DB 通过；独立 Tauri 窗口未实现（P0-1）；员工权限未验证 |
| 上下架：新 open 拒绝，已打开 request 410 | **PASS（basic）**：apps-e2e 验证；rollback/delete/purge 不完整（P0-3） |
| 不能 AI/其他 DB/网络 | **PARTIAL**：跨 app DB e2e 通过；AI/network/file 无测试；UI iframe 可外网（P0-5） |
| 资源耗尽被限额/超时挡住 | **FAIL**：只有 fuel/memory 单测；per-user/global/queue/DB quota/logs 未实现（P0-4） |
| 审计链路 | **PARTIAL**：open/upload/admin audit；request/limit 无 audit，app_calls fail-open |
| 独立 origin/partition/capability | **FAIL**：主 WebView iframe + global Tauri（P0-1） |
| 静态 UI 资产/UI XSS | **FAIL**：固定 index、text/html、无 CSP（P0-5） |
| 测试证据诚实 | **PARTIAL/FAIL**：apps/admin/login 绿，但 M4 安全/限额/UI 用例缺失；golden ignored |

## 7. 结论

- **P0：5**：独立窗口/Tauri capability、`can_open` 授权、rollback/delete/purge/retire、资源/配额、静态 UI/XSS。
- **P1：12**：SQL 5s/迁移限额、迁移记录、C10 DTO、员工权限/owner transfer、asset.read、M4 UI/security e2e、测试假覆盖、审计 fail-open、C3 四件套、窗口来源、UI 版本竞态、死表。
- **P2**：见 §5。
- **M4 实现不能 APPROVED**：先修 P0/P1，补独立窗口/权限/生命周期/限额/UI 安全与真实负向证据，再由同一评审者复评。

- 关联 spec：`specs/changes/2026-10-30-wasm-app-center`。
- 复评结论：**CHANGES-REQUESTED**


## 6. 修复进展（Lead，2026-10-30）

- P0-2 can_open：catalog/open/request/asset 查询加入 `content_grants.can_install`；待补 can_open=false e2e。
- P0-3 生命周期：rollback 切换 current_release + deployment；delete 写 deleted_at + deployment；purge 删除内容与 DB 文件；app-retire job 未实现。
- P0-4 限额：per-user 8/global 32、60s 端到端超时、request 1MiB body、response 8MiB、DB 后置配额、app_calls 写失败 fail-closed、app_databases upsert、runtime_limits fuel 读取；20GB/日志表/队列仍缺。
- P0-5 静态 UI：asset 按 manifest 白名单与扩展名 content-type；AppSurface 注入 CSP（default-src none/connect-src none）。
- P1：SQL busy_timeout 5s；host migration 32 条/64KB 校验；上传拒绝 auditor；新增 admin transfer。
- 仍未修：P0-1 Tauri 独立窗口/capability/partition；P1 剩余（AST SQL、app_runtime_logs、asset.read、M4 client e2e、golden ignored 等）。


## 7. R1 修复（Lead，2026-10-30）

- P0-1：新增 Tauri `open_app_window`（`zk-app://localhost/<app_id>/index.html`、per-app `data_directory`、incognito、导航只允许本 app）、`register_uri_scheme_protocol("zk-app")`（服务端带 Bearer 拉静态资源、注入 CSP/`zkAppRequest`）；`ipc_call`/`open_app_window` 拒绝 `zk-app-*` 窗口，`app_request` 拒绝非 app 窗口（窗口标签门禁）。
- P0-2/P0-3/P0-4/P0-5 见上一节。
- P1：request 改用 `zk-contract-app::AppRequest` DTO；`asset.read` 注入 UI 资源；SQL busy_timeout 5s；host migration 32/64KB；上传拒绝 auditor；transfer。
- 仍未做：Tauri capability 文件（权限 ACL）、真实 Tauri 窗口 e2e（无显示环境）、app-retire job、app_runtime_logs、并发/队列全量、golden 未去 ignored。

---

## 8. 第二轮复评（R2，2026-10-30）

> 对象：R1 修复后的 M4 实现。只读复跑；未改产品代码/spec（仅本记录）。
> 仅列剩余 P0/P1。

### 8.1 结论

**CHANGES REQUIRED（CHANGES-REQUESTED）**

- **P0：1**（Tauri capability 未落，`withGlobalTauri`/无 capability 文件）。
- **P1：5 组**（退役/20GB/日志、SQL/限额、UI 入口/测试、Tauri partition、权限/生命周期边界）。
- **M4 实现仍不可 APPROVED**。

### 8.2 已关闭（R1 P0/P1）

- **R1 P0-1 独立窗口**：`apps/desktop/src/main.rs:39-58` 已实现 `open_app_window`、`zk-app://localhost/<app_id>/index.html`、`register_uri_scheme_protocol("zk-app")`、`incognito`/per-app data_directory、navigation gate；`App.tsx:253-256` Tauri 路径调用 `open_app_window`，serve 才用 iframe；`ipc_call`/`open_app_window` 拒绝 `zk-app-*`，`app_request` 只接受 `zk-app-*`。**但 capability 文件仍未落：`apps/desktop/capabilities/` 不存在，`tauri.conf` 仍 `withGlobalTauri:true`/`csp:null`（见 P0-1）。**
- **R1 P0-2 can_open**：`apps/src/lib.rs:139,159,183,249` 查询已加 `g.can_install`；`admin_grants` 写 `can_open`；catalog/open/request/asset 统一拒绝 `can_open=false`。
- **R1 P0-3 rollback/delete/purge**：`admin_action` 已处理 rollback（切 current_release + deployment）、delete（deleted_at + deployment）、purge（删 DB/contents + audit）；transfer 路由存在。**retire job 仍缺失（P1-1）。**
- **R1 P0-4 限额**：per-user/global semaphore、60s timeout、request 1MiB、response 8MiB、DB post-run quota、`app_calls` fail-closed、`app_databases` upsert、runtime_limits fuel 已落。**20GB/log 表/queue 仍缺失（P1-1）。**
- **R1 P0-5 静态 UI**：`apps/src/lib.rs:244-264` 按 `ui_manifest` 白名单 + MIME；`ui-apps` `srcDoc` 注入 CSP；Tauri HTML 注入 CSP/inline app_request。XSS/外网基本收口。
- **R1 P1**：AppRequest DTO、asset.read assets、SQL busy_timeout、migration count/size、role check、transfer 已落。

### 8.3 剩余 P0

#### P0-1 · Tauri capability 未落，`withGlobalTauri` 仍开启

- `apps/desktop/capabilities/` **不存在**（`find apps/desktop -maxdepth 3 -name capabilities` 无结果）；`tauri.conf.json` 仍是 `withGlobalTauri: true`、`security.csp: null`。
- `open_app_window`/`ipc_call`/`app_request` 只靠 window label 判断；没有 capability 文件把 app 窗口允许的命令限定为 `app_request`。要么 app 窗口无法调用 `app_request`（功能失败），要么 global Tauri 暴露过多 core/plugin 命令（安全失败）。
- 因此“per-app capability-only app_request”并未实现；这是 M4 独立窗口安全边界。

**修法**：新增 `apps/desktop/capabilities/app-window.json`（`windows:["zk-app-*"]`，只允许 `app_request`），主窗口 capability 仅 `ipc_call`/`open_app_window`；`tauri.conf` 移除 `withGlobalTauri:true`（或 app window 单独 `withGlobalTauri:false`）；`csp` 至少对 app window 去掉 null；补 capability 单测/手动验证（app 调其他命令 403）。

### 8.4 剩余 P1

#### P1-1 · 退役/20GB/日志/queue 仍缺失

- 没有 `app-retire` job（docs/06:99、spec:32）；`app_calls`/DB 文件/UI/日志的真删保留期没有调度。
- 没有 20GB 全局 DB 配额；没有 `app_runtime_logs` 表；没有 global/per-user 排队 queue（semaphore `try_acquire` 直接 429，spec 的 64 排队未实现）。
- 修：加 `app-retire` job（每小时、幂等、审计、保留 30d、真删 DB/logs/calls/wasm/UI）；20GB 总量检查（`app_databases.sum(size_bytes)`）；`app_runtime_logs` 表/全局 cap；queue + 429 Retry-After。

#### P1-2 · SQL 5s / DB 写入配额仍不严格

- `zk-kit-wasm` 只设置 `busy_timeout(5s)`（`crates/zk-kit-wasm/src/lib.rs:131`），这是锁等待，不是“单条 SQL 5s”；没有 per-query interrupt/进度 handler，长 SELECT 仍可跑到 guest 30s。
- `app_db::enforce_quota` 只在 guest 跑完/前检查；guest 写超 100MB 时先落盘再报错，未回滚。
- 修：在 `db.exec/query` 用 `sqlite3_progress_handler`/interrupt + 5s deadline；写前后均配额；超限回滚/拒绝；补长 SQL/大写入测试。

#### P1-3 · `app_runtime_limits` 仅 fuel；UI 入口/静态资源仍有固定 index 假设

- `app_runtime_limits` 只读取 `fuel`；`max_concurrency`/`max_memory_bytes` 未生效；`app_runtime_logs` 缺失。
- `fetchAppHtml`/`open_app_window` 固定 `/index.html`（`store.ts:161-164`、`main.rs:47`），忽略 `X-ZeoKeel-App-Ui`/manifest 的入口；`ui_manifest` 可含非 index 入口，窗口会 404。
- 修：按 app 配置读取 limits；open/asset 返回明确 `ui_entry`，窗口 URL/iframe 使用 entry；补多入口/无 index 测试。

#### P1-4 · UI/安全负向 e2e 缺失，golden 仍 ignored

- `apps-e2e.sh` 仍只跑 server happy path；`admin-e2e` 的 `m4-admin-apps.spec.ts` 只测 seeded app 的 approve/grant/delist；`login-e2e` 3 个用例无 M4 app；没有 `surface.apps`/Tauri 窗口/静态 UI 的 Playwright；没有 AI/network/file/其他 DB/limits/SQL/can_open 负向。
- `cargo test` 的 `go_and_rust_guests_share_frame_semantics` 仍 `#[ignore]`；`memory_limit_is_memory_error` 接受 `Memory|Fuel|Timeout`，不证明内存限额。
- 修：补 employee/can_open/open/asset/Tauri capability 负向、每个 limit 的错误码、SQL/AI/network/file 用例；把 golden 纳入 CI（或明确证据脚本）；memory 断言收紧。

#### P1-5 · `purge` 非 superadmin、rollback 不校验 schema 兼容、窗口 partition

- `admin_action` 的 purge 只 `require_admin`，docs/04 要求 `super_admin`；rollback 直接按 created_at 取第二个 release，不校验 `db_schema_version` 兼容（回滚到旧 schema 后 request 会 `DB_SCHEMA_DOWNGRADE`）。
- `open_app_window` 有 `incognito`/`data_directory`，但没有 per-app partition；docs/04:222 明确独立 partition。
- 修：purge 仅 super_admin；rollback 校验 DB schema 兼容/迁移；Tauri 窗口加独立 partition（或 ADR 说明 data_directory 等价）；补 rollback/schema 测试。

### 8.5 R2 证据

| 命令 | 结果 |
| --- | --- |
| `bash scripts/apps-e2e.sh` | **PASS**（EXIT 0）`apps e2e ok`；仍 happy path |
| `bash scripts/admin-e2e.sh` | **PASS**（EXIT 0）9 passed，M4 admin 只 approve/grant/delist |
| `bash scripts/login-e2e.sh` | **PASS**（EXIT 0）3 passed，无 M4 client |
| `cargo test -p zk-kit-wasm -p zk-srv-apps -p zk-srv-app-db --offline` | **PASS**（EXIT 0）；golden 1 ignored |
| `cargo test --workspace --offline` | **PASS**（EXIT 0） |
| `zk compose --edition enterprise --profile default --channel official --check` | **PASS**：`compose: up to date (16 files)` |
| `zk validate` | **PASS**：69 packages scanned |
| `bash scripts/schema-check.sh` / `zk db lock --check` | **PASS** |
| `find apps/desktop -maxdepth 3 -name capabilities` | **无结果**（P0-1） |
| `cat apps/desktop/tauri.conf.json` | `withGlobalTauri:true`、`csp:null`（P0-1） |

### 8.6 结论

- **P0：1**（Tauri capability）。
- **P1：5**（退役/20GB/日志/queue；SQL5s/写配额；runtime limits/UI entry；UI/安全 e2e/golden；purge/rollback/partition）。
- **M4 实现仍不可 APPROVED**：修 P0/P1 后再复评。

- 关联 spec：`specs/changes/2026-10-30-wasm-app-center`。
- 复评结论（第二轮）：**CHANGES-REQUESTED**
