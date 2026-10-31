# 2026-10-31-round9-schedule-config · 规格

- 状态：`approved`

## 1. 调度配置
- contracts/config `ScheduleSettings { enabled: bool = false, scan_seconds: u64 = 10 (clamp 5..=3600) }`；`AgentSettings.schedule` serde default；schema/TS 重生成。
- RuntimeSettings 的 agent section 携带 schedule；desktop settings.set 应用后：若 enabled 变化，启动/停止 automation executor（或下次 boot 生效并在 UI 文案说明；优先热更新：executor 通过 AtomicBool/Notify 控制）。
- env `ZEOKEEL_ENABLE_SCHEDULE_AUTORUN` 仅作为未保存设置时的 fallback；显式 settings 优先；scan_seconds 覆盖固定 10s。
- settings UI：开关 + 扫描间隔（秒），范围校验/本地化/持久化，错误回传。
- docs/38 记录配置优先级与热更新语义；matrix schedule 行指向 settings。

## 2. CI 集成
- `.github/workflows/ci.yml` 新增 `integration` job：services.postgres (postgres:18, healthcheck)，安装依赖，`cargo build -p zk-server`，运行 `bash scripts/srv-auth-test.sh`；timeout 30m；不要求 web。
- 本地跑同一脚本记录证据。

## 3. 文档
- matrix `call-timeout` 行改为 `AppSettings.tools.tool_timeout_ms`（env fallback），schedule 行 settings；docs/38 同步。
