# 2026-10-31-round4-data-and-ui · 提案

- 状态：`approved`
- Owner：lead（auditor-rig / auditor-rust / auditor-web / auditor-build）
- 日期：2026-10-31
- 关联：docs/26；docs/28；ADR-066/067；round-3 review §6 剩余 P2
- 评审：docs/reviews/2026-10-31-round4-verification.md
- 变更预算：usage 持久化 + settings 分 section 发布 + workspace.changes IPC/UI + 企业更新/交付脚本复核

## 1. 背景

round-3 已 APPROVED，但仍有明确缺口：
- `AgentReply.usage` 只展示不持久化，重启后丢失（round-3 review P2）。
- `RuntimeSettings` 整文件发布导致"只保存 tools 时默认 agent 快照覆盖 `ZEOKEEL_AGENT_*`"的优先级边界（review P2）。
- inspector changes tab 被禁用，缺 `workspace.changes` IPC 数据源（docs/27/28 已标注待办）。
- 企业插件/交付面（updates/OEM/portal）e2e 未在本轮验证，交付脚本仍有 UNVERIFIED。

## 2. 目标

- session `Message` 增加可选 usage（契约 + SQLite v4 迁移 + 桌面持久化/加载 + UI 恢复显示）；兼容旧 DB/旧消息。
- `RuntimeSettings` 按 section 可选发布；agent-rig 逐 section 合并 settings > env > 默认；补测试消除优先级边界。
- `workspace.changes` IPC（git status 只读、限额）契约/实现/UI：inspector 变更 tab 可用；docs/27/28 更新。
- 企业更新/OEM/portal e2e 在当前代码上复核；发现问题修复或记录 UNVERIFIED。
- T22 剩余 host JSON/FileEntry 镜像/question 校验清理。

## 3. 非目标

- `zk api` 路由真源、M6 Docker/SBOM/多平台证据（环境缺失，继续 UNVERIFIED）。
- 子代理异步 send/interrupt（下一轮）。

## 4. 成功指标

- usage 重启后可见（单测：append/load 跨实例；e2e 重启消息含 usage）。
- tools-only 保存 + env agent 配置 → env 生效（单测）。
- workspace.changes e2e/单测通过，inspector 变更 tab 有数据/空态。
- 全量门禁 + 更新/OEM 脚本结果 + C19 APPROVED。

## 5. 风险

| 风险 | 缓解 |
| --- | --- |
| SQLite 迁移破坏旧库 | 只加列 + user_version 4，旧数据 NULL；备份既有机制 |
| git 命令注入/性能 | 固定 argv `git status --porcelain -z`，超时/条数上限，只读 |
| settings Option 迁移 | serde default + 兼容旧 snapshot/旧 registry |
