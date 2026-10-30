# 2026-10-30-wasm-app-center · 规格（M4）

- 状态：`draft`
- 依据：`docs/07` §7、`docs/04` §2.6/§9、`docs/06`、`docs/21`、ADR-062

## 1. 数据

- 复用 `contents(kind='app')`、`content_releases`、`content_deployments`、`content_grants`；release 额外存 `wasm bytea`、`ui_manifest jsonb`、`ui_archive bytea`、`wasm_sha256`、`ui_sha256`。
- `app_databases(app_id PK, schema_version, size_bytes, quota_bytes default 104857600, updated_at)`；`path` 不入库，由服务端 `ZEOKEEL_APP_DB_DIR/<app_id>.sqlite` 唯一生成。
- **host-managed schema/migration**：release manifest 携带 `db_schema_version` + `db_migrations:[{version,sql}]`；host 在 guest 启动前校验（仅 `CREATE TABLE`/`CREATE INDEX`，≤32 条、≤64KB total、表数 ≤16）并在事务内按版本执行；`app_databases.schema_version` 原子更新；失败回滚、保留旧 DB 与旧 schema_version；不允许降级；guest 永远只 DML（禁 DDL）。
- `app_calls(id, app_id, user_id, request_id, status, duration_ms, fuel, at)` 保留 30 天，后台清理。
- `app_runtime_logs(id, app_id, level, message, at)` 每 app 最多 1000 行，全局 10 万行；每日清理。
- `app_runtime_limits(app_id, max_concurrency default 4, max_memory_bytes 64MB, fuel default 10M, updated_at)`；全局/用户并发来自 system_settings。

## 2. API（唯一真源 docs/04 §2.6/§9）

- 员工：`POST /api/client/v1/apps`（multipart manifest JSON + wasm + ui zip；`apps:publish`；owner=当前用户；pending）；`GET /api/client/v1/apps`、`POST /{app_id}/open`、`POST /{app_id}/request`、`GET /{app_id}/assets/{path}`（均 `apps:use`）。
- 管理：`GET/POST /api/admin/apps`、`POST /api/admin/apps/{id}/versions`、`approve|reject|delist|rollback|delete|purge`、`PUT /grants`、`GET /diagnostics`；写 `apps:admin` + super_admin + CSRF。
- open 头：`X-ZeoKeel-App-Version`、`X-ZeoKeel-App-Ui`；不返回可执行 guest。request 信封严格按 docs/04 §9。
- 错误：`APP_NOT_FOUND`、`APP_DELISTED`、`APP_FORBIDDEN`、`APP_BAD_WASM`、`APP_BAD_MANIFEST`、`APP_LIMIT_MEMORY`、`APP_LIMIT_FUEL`、`APP_LIMIT_TIME`、`APP_CONCURRENCY`、`APP_DB_QUOTA`、`APP_DB_TABLES`、`APP_BAD_REQUEST`、`RATE_LIMITED`、`APP_UNAVAILABLE`；超限 429 + `Retry-After`。
- 权限：`apps:publish`（员工上传自己的 pending）、`apps:use`（可见/打开/请求）、`apps:admin`（管理）；非 owner 上传同名/新版本 403；转交仅 admin。

## 3. 执行/限额（宿主强制）

- wasmtime 50 LTS embedding；wasip1 core module；零 preopen；零 socket；无 env/时钟配置；guest 导出 `_start`。
- 帧：stdin/stdout `RS<len>\n<JSON>`；单请求单实例。
- 宿主函数封闭：`db.exec`、`db.query`、`log`、`asset.read`；无 `ai.*`、network、file、env。
- guest SQL：单语句；仅 SELECT/INSERT/UPDATE/DELETE；**禁 DDL/ATTACH/PRAGMA/VACUUM/load_extension/readfile/writefile/TRIGGER/VIRTUAL TABLE**；显式参数绑定；注释/多语句/大小写绕过拒绝；单条 SQL 5s。host-managed migration 是唯一 DDL 入口（见 §1）。
- DB：只操作当前 app 的 pinned connection；`db.*` 不接受路径/attach 名；表 ≤16、文件 ≤100MB；超限 fail-closed。
- 限额：请求 1MiB、响应 8MiB；端到端 60s（含排队）、guest 30s、SQL 5s；每应用并发 4、每用户并发 8、全局并发 32、排队 64；超限 429+Retry-After；内存 64MB/实例、fuel 10M。
- 配额：上传 wasm ≤32MB、UI zip ≤16MB；每用户 pending ≤10、总应用 ≤200；DB 总磁盘 ≤20GB；日志/调用按保留期清理。
- 退役：job `app-retire`（每小时）软删→保留 30 天→真删 wasm/UI/DB/logs/calls；审计；手动 purge 仅 super_admin。
- 执行失败/编译错误/非零退出/畸形帧 → `APP_BAD_WASM`/`APP_UNAVAILABLE`，fail-closed 且审计。

## 4. 客户端隔离

- 桌面：每应用独立 Tauri `WebviewWindow`，origin `zk-app://<app_id>/`（渠道配置、跨渠道唯一、≠深链 scheme），创建前注册 privileged、独立 partition；capability 只允许 `app_request`；导航只允许本 origin；窗口关闭销毁 partition/临时资源。
- serve：仅 e2e；`<iframe sandbox="allow-scripts">`（无 allow-same-origin）+ CSP，host 校验 `event.origin === "null"`、`event.source === iframe.contentWindow`、一次性 session nonce；postMessage 只接受信封白名单。
- 页面拿不到 token；host 注入 Bearer；静态 UI 只从 release `ui/` 清单取；资源路径拒绝 `..`/绝对/URL。
- 下架：open 403；已打开 request 410 → 错误页；写数据不再落库。

## 5. 验收断言

- `bash scripts/apps-e2e.sh`：员工上传 → admin 审批/上架 → 目录可见 → open → request 写 DB → 重启 server 数据仍在 → 下架 → open 403 → 已打开 request 410。
- 限额：fuel/内存/时间/并发/请求/响应/DB 表/DB 大小/上传配额逐项有命令和错误码。
- 安全：guest 无法 import ai/network/file；`VACUUM INTO`、多语句、`ATTACH`、`PRAGMA`、`load_extension` 拒绝；跨 app DB 隔离；UI 资源路径穿越拒绝。
- 管理台/客户端 Playwright + axe 0 serious/critical。
