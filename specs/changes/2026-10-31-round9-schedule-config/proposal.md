# 2026-10-31-round9-schedule-config · 提案

- 状态：`approved`
- Owner：lead（auditor-rig / auditor-web）
- 日期：2026-10-31
- 关联：docs/38 §3；docs/24；round-8 review §8.3 P2-1；objective 2/4
- 评审：docs/reviews/2026-10-31-round9-verification.md
- 变更预算：调度配置化 + CI 真 PG 集成 + 文档真源

## 1. 背景

- `schedule` 自动执行器目前只能通过环境变量 `ZEOKEEL_ENABLE_SCHEDULE_AUTORUN=1` 开启、扫描周期固定 10s；这是用户可见 agent 功能的隐藏配置，违反"所有项目可配置"。
- `scripts/srv-auth-test.sh` 已覆盖真实 PG 的认证/计费/审计/迁移等关键不变量，但 CI 无 postgres job，未被执行（round-8 P2-1）。
- 工具矩阵 `call-timeout` 行仍写 env-only，实际已由 `AppSettings.tools` 配置（文档滞后）。

## 2. 目标

- `AgentSettings.schedule { enabled: bool, scan_seconds: u64 }`（默认 disabled、10s，范围 5..=3600），settings UI 可配置并持久化；自动化执行器读取已发布设置（env 兜底/兼容），启动/停止随设置热更新（settings.set 后生效，无需重启）；`meta.get` 可暴露当前值？至少 settings 可见。
- docs/38、工具矩阵、设置文档同步；测试覆盖默认/范围/热更新（enabled 从 false→true 需重启任务？实现允许下次内核启动生效并文档说明，或运行时 apply）。
- CI 新增 `integration` job：PostgreSQL service，构建 server 后运行 `scripts/srv-auth-test.sh`（允许较长超时），失败即红；本地证据。
- 矩阵 `call-timeout` 行改为 `AppSettings.tools.tool_timeout_ms`（env fallback），schedule 行指向 settings。
- 全量门禁 + C19 APPROVED。

## 3. 非目标
- 真实模型长时调度压测；zk api；M6。
