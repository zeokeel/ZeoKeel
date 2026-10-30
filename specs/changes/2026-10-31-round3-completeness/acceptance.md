# 2026-10-31-round3-completeness · 验收

- 状态：`in-progress`（实现与 P1/P2 修复完成；C19 复评待）
- 验收人：lead；独立评审：auditor-rust（C19）

## 1. 待填

| 命令 | 通过条件 | 证据 |
| --- | --- | --- |
| cargo fmt/clippy/test --workspace | 0/0/0（本机已跑） | 日志 |
| export_schema + schema-check + generated diff clean | 0/0（62 schemas，稳定） | shared/contracts |
| zk validate/compose personal --check/capability/docs/db lock/lint duplicates | 全 0 | 日志 |
| pnpm -r typecheck/lint/test:unit/build | 0/0/0/0（105 tests） | 日志 |
| m1/agent/login/admin e2e | 14 passed/15 skipped · 9 passed · 4 passed · 11 passed（lead 本机；reviewer 于 HEAD 复核同结果） | target/evidence/{m1,m2,agent-rig}；reviewer 日志 target/evidence/audit/logs-rust/r28-*-e2e.log |
| 设置超时/向量持久化/Ajv 负向/内核取消/重复门禁 | 单测通过（contract/agent-rig/desktop/cap-vector/jobs/automation/zk） | 各 crate tests |
| C19 评审 | APPROVED（初次 CHANGES-REQUESTED P1 已修复：Tauri 事件桥改内核 task） | docs/reviews/2026-10-31-round3-verification.md |

## 1.1 复审修复记录

- P1 Tauri 事件桥：`run_window` 经 `Bootstrap::spawn_task` 注册 `zk.desktop.tauri-events` / `zk.desktop.open-app`，内核 shutdown 可 abort/await（reviewer 复核正确）。
- P2 安全：全局 runtime settings 改为非密 `RuntimeSettings{agent,tools}`，provider key/base_url 不进入 registry（测试断言序列化不含 secret）。
- P2 向量：insert_many 原子批、目录 0700 收紧、model/dims/width 指纹不兼容降级、容量前置检查、去掉算术 allow。
- P2 jobs：`wait` 10s 有界 + Drop 停止；P2 文档：docs/24 关闭顺序、docs/27/28 inspector changes、docs/25 去重措辞、proposal 重复评审行已修。
- P2 其余（usage 持久化、T22 host 手写 JSON、cargo-deny 本地 UNVERIFIED）保留在 review §3，不阻塞。

## 2. UNVERIFIED

- 真实媒体 provider、Docker/多平台、真实 git 变更数据源（若实现）、Windows/macOS 打包。
