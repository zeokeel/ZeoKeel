# 2026-10-30-wasm-app-center · 提案（M4）

- 状态：`approved`
- 评审：`docs/reviews/2026-10-30-m4-spec.md`
- 依据：`docs/12` §M4、`docs/07` §7、`docs/04` §2.6/§9、`docs/21`、`docs/06`、ADR-062、`experiments/wasm-poc`、`docs/15` T11
- 关联：M3 `content` 聚合（`kind=app` 复用 contents/releases/deployments/grants）

## 1. 背景

M3 已完成技能/连接器。M4 让员工上传 WASM 应用，管理员审核/上架，企业客户端在独立 origin 窗口使用；服务端 wasmtime 执行，每应用独立 SQLite（100MB/16 表），无网络/文件/AI。

## 2. 目标

- 服务端 `srv.apps`/`srv.app-runtime`/`srv.app-db`：员工上传/版本/审核/上下架/退役；wasmtime 执行；每应用 DB；帧 ABI；限额/配额；审计。
- 管理台：应用列表/版本/上下架/诊断。
- 客户端：应用目录、`app_request` 信封、独立窗口（静态 UI）、错误页/下架拒绝；**不执行应用 WASM**（ADR-062）。
- 示例：Go(wasip1) 表单/数据处理各一；golden frame conformance。
- 归档 P2 wasm-poc 证据到 `target/evidence/m0/wasm/`。

## 3. 非目标

- 不做 AI 宿主函数、网络、文件 preopen、wasip2、客户端 UI WASM 沙箱。
- 不做运行时插件加载；不做部门/用户组；不做跨应用共享 DB。
- M4 不做 wasmtime 常驻实例池/预热。

## 4. 成功指标

- 示例发布 → 员工独立窗口打开 → 写 DB → 服务重启后仍在。
- 下架后新打开 403；已打开窗口 request 返回 410。
- AI/网络/其他应用 DB 均不可达；内存/CPU/时间/并发/体积耗尽有稳定错误码。
- 管理台/客户端 Playwright + axe；独立评审 APPROVED。

## 5. 风险

- wasmtime 50 构建重；离线可用但 CI 时间增加。
- Tauri 自定义 scheme/partition/capability 需要平台验证；serve 用 sandbox iframe 只能作 e2e 载体，不作为生产隔离。
- 100MB/16 表/超时/并发必须宿主强制，不能由 guest 自报。
- UI 静态资源 XSS：CSP + 独立 origin + 无 token；需安全测试。

## 6. 需用户拍板

- 客户端窗口方案：Tauri 多窗口 + 渠道配置 `zk-app://<app_id>/`（默认）；serve 仅 e2e iframe。若不接受 M4 用静态 UI 替代“UI 编进 wasm”，需回退 ADR-062。
- 上传面：员工上传 wasm+静态 UI；管理员审批/上架。默认接受。

## 7. 变更预算

服务端 3 包（apps/runtime/app-db）+ 1 kit + 1 迁移；管理台 1 包；客户端 1 包；Go 示例 2 个；e2e 脚本 1 个。
