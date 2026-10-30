# 2026-10-30-wasm-app-center · 提案（M4）

- 状态：`draft`
- 依据：`docs/12` §M4、`docs/07` §7、`docs/04`、`docs/21`、`docs/06`、`experiments/wasm-poc`、`docs/15` T11
- 关联：M3 `content` 聚合（`kind=app` 复用 contents/releases/deployments/grants）

## 1. 背景

M3 已完成技能/连接器。M4 让员工上传 WASM 小应用，管理员审核/上架，企业客户端在独立窗口使用；服务端执行 WASM，每应用独立 SQLite，100MB/16 表限额，无网络/文件/AI。

## 2. 目标

- 服务端 `srv.apps`：WASM 上传/版本/审核/上下架/退役；wasmtime 执行；每应用 DB；帧 ABI；资源限额；审计。
- 管理台：应用列表/版本/上下架/运行诊断。
- 客户端：应用目录、独立 origin 窗口、请求信封、错误页/下架拒绝。
- 示例应用：Go(wasip1) 表单/数据处理各一；golden 帧 conformance。
- 先把 P2 wasm-poc 证据归档到 `target/evidence/m0/wasm/`。

## 3. 非目标

- 不做 AI 宿主函数、网络、文件 preopen、wasip2、Rust guest GA（GA 前再加）。
- 不做运行时插件加载；不做部门/用户组；不做跨应用共享 DB。

## 4. 成功指标

- 示例发布 → 员工独立窗口打开 → 写 DB → 服务重启后仍在。
- 下架后新打开 403；已打开窗口返回 `APP_DELISTED`。
- AI/网络/其他应用 DB 均不可达；内存/CPU/时间/并发耗尽有稳定错误码。
- 管理台/客户端 Playwright + axe；独立评审 APPROVED。

## 5. 风险

- wasmtime 50 依赖较重；离线可用但构建时间增加。
- 客户端“独立 origin”在 Tauri/serve 下需要 per-app origin 方案（`zk://app-<id>`/本地回环子域）；M4 用独立窗口 + 应用隔离页，不在同一 DOM 执行 guest UI。
- 100MB/16 表限额需在宿主侧强制执行；不能用 guest 自报。

## 6. 需用户拍板

- M4 客户端窗口采用 Tauri 多窗口 + `zk-app://<id>` 自定义 origin，还是 serve 模式回环路径？（默认：桌面多窗口，serve 用独立路径/iframe sandbox 作 e2e 载体）
- 应用 UI 二进制由服务端返回给客户端，客户端在隔离 WebView 渲染；是否接受 M4 先 spool 到临时文件再窗口？（默认接受，窗口关闭删除）

## 7. 变更预算

服务端 2 个包（`srv.apps`/`srv.app-runtime` 可先合并为 `srv.apps` 内含 runtime）、1 迁移、1 管理台包、1 客户端 UI 包、1 示例 guest、e2e 2 条。
