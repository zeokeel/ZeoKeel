# 2026-10-30-m0-scaffold-minimal-demo · 行为规格

> 规范细节链接 `docs/`；本文件只写本次行为与验收断言。

## R1 单一 workspace

- WHEN 在仓库根执行 `cargo metadata --offline` THEN 成功，members 覆盖 `packages/*/*`、`crates/*`、`apps/*`、`tools/*`。
- WHEN 执行 `cargo test --workspace --offline` THEN 全部测试通过。
- 工具链：`rust-toolchain.toml` 使用本机已安装的 stable（1.99.1）；不触发网络下载。

## R2 `zk validate`

- WHEN 读取根 `zk.toml` + `editions/personal.toml` + `profiles/default.toml` + 包清单
  THEN 校验：包 id 唯一、角色已登记、契约已登记、singleton 恰好一个提供者、`requires` 的角色有提供者。
- WHEN 缺角色 / 同一 singleton 两个 provider / 角色声明契约与注册表不一致
  THEN 退出码非 0，并指出角色/包/契约。
- WHEN 全部合法 THEN 退出码 0，并输出包/角色数量。

## R3 `zk compose`

- WHEN 执行 `zk compose --edition personal --profile default --channel official`
  THEN 生成：
  `crates/zk-composed/{Cargo.toml,src/lib.rs}`、
  `apps/desktop/Cargo.toml`、`apps/server/Cargo.toml`（由 `tools/zk/templates/` 复制）、
  `apps/{desktop,server}/src/generated/packages.rs`、
  `shared/contracts/{packages,roles}.json`、
  `docs/generated/PACKAGE-INDEX.md`。
- 所有生成物首行含 `@generated`；除模板复制外内容由解析结果决定；重复执行字节一致。
- WHEN 加 `--check` THEN 不写文件，仅比较；不一致退出码非 0。

## R4 最小 demo（桌面）

- WHEN 启动 `zk-desktop` 并输入“读取 demo-data/hello.txt”
  THEN 返回 JSON：`text` 含文件正文；`tool_calls` 恰好一条，`name="fs.read"`、`args` 含路径、`result` 含正文。
- WHEN 输入普通文本（无读取意图） THEN 返回 mock 文本，`tool_calls` 为空。
- WHEN 工具路径含 `..` 或绝对路径 THEN 工具返回错误，turn 不崩溃，UI 显示错误。
- 前端：静态 HTML/JS（`apps/desktop/web/`）；Tauri 主进程只暴露一个 `ipc_call` 命令（业务命令表由数据驱动）。

## R5 最小 demo（服务端）

- WHEN `GET /healthz` THEN 200 `{"status":"ok","edition":"personal","packages":[...]}`。
- WHEN `GET /readyz` THEN 200。
- WHEN `GET /api/meta` THEN 返回 edition/包清单/版本。
- 服务端**不执行 agent**（ADR-028），只做骨架与健康检查。

## R6 `zk capability` / `zk find`

- WHEN 执行 `zk capability [--check]` THEN 扫描 `packages/contracts/*` 与 `crates/zk-kit-*` 的 `@capability` 注释，
  生成/校验 `shared/contracts/capability-index.json` + `docs/generated/CAPABILITY-INDEX.md`。
- WHEN 执行 `zk find <keyword>` THEN 在索引中按 id/摘要过滤并打印。
- WHEN 新增公共项无 `@capability` THEN `--check` 失败（M0 门禁雏形；完整规则见 `25`）。

## R7 文档与门禁

- `python3 scripts/docs-check.py`、`python3 scripts/spec-check.py` 均 0 error。
- 本 spec 的 `acceptance.md` 填命令、结果、证据路径；不做“口述验证”。

## 错误与边界

| 场景 | 期望 |
| --- | --- |
| 未知角色 / 未知契约 / 包 id 重复 | `zk validate` 非 0，明确报文 |
| singleton 0 或 ≥2 provider | `zk validate` 非 0 |
| 生成物被手改 | `zk compose --check` 非 0 |
| 工具越权路径 | `Denied` 错误，无 panic |
| 离线构建 | `--offline` 可用；不新增网络依赖 |
