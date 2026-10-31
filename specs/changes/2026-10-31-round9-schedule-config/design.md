# 2026-10-31-round9-schedule-config · 设计

- 状态：`approved`

## 1. 调度
contract ScheduleSettings；agent-rig published RuntimeSettings agent 已含；automation::start 改为接受 Arc<AtomicBool> enabled + AtomicU64 scan_seconds（从 settings 读取并热更新）；desktop ipc settings.set 调用 `automation::configure(...)`（executor handle stored in AppState/static）。
简化：automation 使用全局 RwLock<ScheduleConfig>，executor 线程每次扫描读；start 在 boot 时总是启动（内部 disabled 时休眠），env fallback 仅当 settings 文件不存在。settings.set 更新全局配置。
## 2. UI
ui-settings 增加 schedule 开关/数字输入；protocol parse defaults。
## 3. CI
integration job with postgres service; run `bash scripts/srv-auth-test.sh`.
