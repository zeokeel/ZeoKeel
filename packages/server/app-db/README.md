# zk-srv-app-db (`srv.app.db`)

**功能说明（M4）**：每应用 SQLite。

- 路径 `ZEOKEEL_APP_DB_DIR/<app_id>.sqlite` 由服务端生成
- host-managed migration：manifest 只允许 CREATE TABLE/INDEX，事务执行，schema_version 原子
- 100MB/16 表/DB 隔离；guest 只通过宿主 `db.*`
