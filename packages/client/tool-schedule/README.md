# tool-schedule (`tool.schedule`)

**功能说明**：工作区持久计划 `.zeokeel/schedules.json`：create/list/update/delete。桌面调度执行器读取该文件自动起 turn；是否运行由 `AppSettings.agent.schedule {enabled, scan_seconds}`（设置页“调度/自动化”，scan 5..3600s）控制，保存后热生效；仅未保存设置时 `ZEOKEEL_ENABLE_SCHEDULE_AUTORUN=1` 作为环境兜底。条目须带本工具写入的 `origin` 标记，按 `interval_seconds` 重复；cron 仅创建时校验，不被执行器解释。
