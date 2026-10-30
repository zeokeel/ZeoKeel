# 2026-10-30-wasm-app-center · 规格（M4）

- 状态：`draft`
- 依据：`docs/07` §7、`docs/04` §2.5、`docs/21`、ADR-061 信任边界

## 1. 数据

- 复用 `contents(kind='app')`、`content_releases`、`content_deployments`、`content_grants`。
- 新表 `app_databases(app_id PK, path, size_bytes, quota_bytes default 104857600, updated_at)`。
- 新表 `app_calls(id, app_id, user_id, request_id, status, duration_ms, fuel, at)`（诊断/限额，保留 30 天）。
- 新表 `app_runtime_logs(id, app_id, level, message, at)`（仅诊断，最多 1000 行/应用，guest 不可读他人）。

## 2. API

- 管理：`GET/POST /api/admin/apps`、`POST /api/admin/apps/{id}/versions`（wasm ≤32MB）、`approve|reject|delist|rollback|delete`、`PUT /grants`、`GET /diagnostics`。
- 客户端：`GET /api/client/v1/apps`（approved+可见）、`POST /api/client/v1/apps/{id}/open`（返回 session_id/ui_wasm?）、`POST /api/client/v1/apps/{id}/request`（帧信封）。
- 错误码：`APP_NOT_FOUND`、`APP_DELISTED`、`APP_FORBIDDEN`、`APP_BAD_WASM`、`APP_LIMIT_MEMORY`、`APP_LIMIT_FUEL`、`APP_LIMIT_TIME`、`APP_CONCURRENCY`、`APP_DB_QUOTA`、`APP_DB_TABLES`、`APP_BAD_REQUEST`。
- 权限：管理写 super_admin+CSRF；员工仅可见/可开；下架后 open 403、request 410。

## 3. 执行

- wasmtime 50 LTS embedding；wasip1；零 preopen；无网络。
- 帧 ABI：stdin `RS<len>\n<JSON>`，stdout 同格式；单请求单实例（M4 不做常驻池）。
- 宿主函数：`db.exec`、`db.query`、`log`、`asset.read`；禁止 AI/网络/文件。
- 限额：fuel 默认 10M、墙钟 5s、内存 64MB/实例、并发 4/应用、DB 100MB/16 表；耗尽返回稳定错误码并审计。
- 每应用 DB 文件在服务端 `ZEOKEEL_APP_DB_DIR/<app_id>.sqlite`，0600；只通过宿主 db 函数访问；退役软删→保留 30 天→真删。
- WASM 只校验 magic/大小/哈希；不在服务端执行上传前代码；执行时 wasmtime 编译错误 → `APP_BAD_WASM` 并审计。

## 4. 客户端

- 目录展示；打开 → 服务端返回 UI wasm/入口；客户端独立窗口（Tauri）或 sandbox iframe（serve）加载隔离页；窗口关闭清理临时物。
- 下架/无权限：错误页；已打开请求返回 `APP_DELISTED` 并显示。

## 5. 验收断言

- `bash scripts/apps-e2e.sh`：登录 → 管理台上传示例 app → 审批/上架 → 员工目录可见 → open → request 写 DB → 重启 server → 数据仍在 → 下架 → open 403 → 已打开 request 410。
- 限额：fuel 耗尽/内存超限/并发超限/DB 表 >16/DB >100MB 均返回对应错误码。
- 安全：示例无法导入 AI/网络/文件；无法读其他 app DB；日志不含 secret。
- 管理台/客户端 Playwright + axe 0 serious/critical。
