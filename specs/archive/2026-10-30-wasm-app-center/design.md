# 2026-10-30-wasm-app-center · 设计（M4）

- 状态：`approved`

## 1. 包/契约/角色

- `packages/contracts/app`：App/Release/Open/Frame/Manifest；schemars 生成 `app.json`/`app-frame.json`/`app-manifest.json`；C10 前端 Ajv。
- `packages/contracts/ipc`：新增 `app_request` 命令/DTO（信封、响应、错误码）；Tauri capability 文件只允许该命令；窗口其他 IPC 命令一律 403；serve postMessage 只接受同 DTO。
- `packages/server/apps`（`zk-srv-apps`，role `srv.apps`）：管理/员工/客户端路由、发布内核、轮询/资产。
- `packages/server/app-runtime`（`zk-srv-app-runtime`，role `srv.app.runtime`）：wasmtime 执行、限额、SQL 闸门、帧、asset。
- `packages/server/app-db`（`zk-srv-app-db`，role `srv.app.db`）：每 app SQLite、schema version、表数/大小、pinned connection 接口。
- `crates/zk-kit-wasm`：prototype runtime 抽公共实现（`@capability wasm.runtime`）；`srv.app-runtime` 与 prototype 共用，禁止复制。
- `packages/admin/admin-apps`（`admin.page.apps`）；`packages/ui/ui-apps`（`ui.slot.apps`）；四件套齐全并注册 zk.toml/editions/docs16。
- 迁移 `0012_apps`；全局编号追加。

## 2. 运行时设计

- `zk-kit-wasm::Runtime::run(input) -> output`：wasmtime Engine/Module 缓存（LRU 16）；单请求 Store；epoch ticker 100ms；fuel 10M；StoreLimits memory 64MB。
- 执行前：校验 wasm magic/大小/sha256；模块编译；导入检查（只允许四个 host function）；`_start` 导出。
- 帧：从 stdin 读完整请求；输出上限 8MiB；畸形/非 JSON → `APP_BAD_REQUEST`。
- SQL 闸门：词法归一化 + 单语句 + 首关键字白名单 + 禁止关键字/`;`/注释；参数绑定；DDL 永不通过；SQL 5s 用 interrupt handle。
- DB：`srv.app-db` 每连接 `rusqlite`/`sqlx` SQLite 文件；`db.exec/query` 不暴露路径；表数 `sqlite_master` 检查前后；文件大小 fs metadata；写前配额检查。
- Host-managed schema：manifest `db_migrations` 仅 CREATE TABLE/INDEX，host 在事务内执行、更新 `schema_version`、失败回滚并保留旧 DB；guest 只 DML；迁移在 guest 启动前完成。
- 并发：`tokio::sync::Semaphore` per-app/user/global + queue；超时含排队；429 + Retry-After。
- 审计：upload/approve/list/delist/request/limit/purge；写失败 fail-closed。

## 3. 发布/生命周期

- 员工上传：multipart → manifest 校验（name/version/entry/ui 文件清单/大小）→ wasm magic/sha → pending；同 owner 同名版本 409。
- 管理审批：approve → release approved；list → status approved + deployment；delist；rollback 切换 current_release（DB 策略：如 schema_version 不兼容则拒绝）。
- DB schema：`app_databases.schema_version`；guest manifest 声明 `db_schema_version`；升级不允许降级；失败保留旧版本+旧 DB。
- 退役：`app-retire` job 每小时；软删 content，保留 30 天后删除 release/DB/logs/calls；审计。
- 迁移只追加，不改 M3 迁移。

## 4. 客户端

- 静态 UI 资源清单由 `GET /apps/{id}/assets` 提供（release ui zip 解包到服务端只读目录/对象）；客户端不执行 WASM。
- 桌面：Tauri `WebviewWindow` + 自定义 scheme；channel 配置 `app_origin_scheme`；注册 privileged/partition/capability（仅 `app_request`）；导航闸门。
- serve：sandbox iframe + CSP + postMessage nonce/origin/source 校验；仅 e2e。
- 错误页：APP_DELISTED/APP_UNAVAILABLE/网络失败；重试。

## 5. 测试计划

- Rust：kit golden（Go/Rust frame）、SQL 闸门表驱动、限额、DB 隔离、manifest、迁移。
- 集成：`scripts/apps-e2e.sh` 真 PG+server+Go guest；上传/审批/打开/写/重启/下架/退役。
- UI：admin apps 创建/上传/审批/上下架 + axe；client apps 打开/错误页 + axe。
- 独立评审 audit-arch R1/R2。
