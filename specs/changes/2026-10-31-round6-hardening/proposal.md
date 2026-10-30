# 2026-10-31-round6-hardening · 提案

- 状态：`approved`
- Owner：lead（auditor-web / auditor-rust）
- 日期：2026-10-31
- 关联：round-5 review §4 P2；docs/04 §7、docs/09、docs/19、docs/21
- 评审：docs/reviews/2026-10-31-round6-verification.md
- 变更预算：round-5 P2 收尾 + Tauri 企业路径 + 426 真实 e2e

## 1. 背景 / 问题

round-5 APPROVED，P2-1/2/3/4/5/6/7/8/9/10 为非阻塞但影响"完美"目标：
- store.ts 直连 426 未统一进门禁；相对 portal_url 潜在错误；426 envelope 与 docs/04 §7 不一致。
- 客户端头 platform 固定 `desktop`、无 `X-Request-Id`；min_client_version 全局而非按渠道；docs/19 门禁动作与实现不一致。
- 无真实 server 426→强制升级 e2e；OEM dir-hash fallback 无限制；Tauri CSP 未允许企业 server，外链打开不可用（P2-9）。
- changes add/del 是文件级计数，UI 文案易误读。

## 2. 目标

- 所有 web→server fetch 的 426 统一 → UpgradeRequiredError/门禁；相对 portal_url 基于 server_url 解析；强制升级有"查看版本说明"（构建信息）/可配置文案，docs 对齐。
- 426 错误统一 envelope（code/message/retryable/request_id）并带 `X-Request-Id` 回显；客户端头带真实 platform。
- `min_client_version` 支持渠道级配置（server channel settings + env fallback）。
- OEM pin：git 索引有 gitlink 用 gitlink；非 git 才允许 dir-hash；git 索引缺 gitlink/SHA-256 不回退并明确报错；channel_validate 双路径测试。
- Tauri：CSP `connect-src` 允许企业 server（https/自定义 server）与 ipc；新增受控 `system.open_url` 命令（仅 http/https，固定 argv），UpgradeGate 在 Tauri 走命令、浏览器走 window.open；能力文件与测试。
- 真实 server 426 e2e（`ZEOKEEL_MIN_CLIENT_VERSION=9.9.9` + desktop serve + Playwright 断言强制升级页、不重试、portal 链接）。
- changes UI 文案明确"文件级变更数"；docs/27 同步。

## 3. 非目标
- 子代理异步 send/interrupt、`zk api`（下一轮）。

## 4. 成功指标
- P2-1/2/3/4/5/6/7/8/9/10 关闭或文档明确；真实 426 e2e 绿；Tauri CSP/外链实现有测试；全量门禁 + C19 APPROVED。
