# 2026-10-31-round5-upgrade-and-e2e · 设计

- 状态：`approved`

## 1. 426
axum `middleware::from_fn` 读取 HeaderMap 与 Extension 中的 min version；路径前缀判断；构造 Json body `{error:{code:"CLIENT_TOO_OLD",...}}` 与 426；测试用 oneshot router。

## 2. 客户端
protocol/transport 错误类型扩展；App 顶层 `upgrade` 状态优先于 boot/login；portal 链接由 `server_url` 拼 `/portal`；i18n zh/en；不自动刷新。

## 3. e2e
扩展 `e2e/m1-serve.spec.ts`（或新 spec）用 `ZEOKEEL_HOME` 临时目录 + mock provider；usage 断言 `data-usage` 文本；changes 用临时 git repo 作为 workspace（初始化、修改文件）→ 打开 inspector 变更 tab。
