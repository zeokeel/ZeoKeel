# 2026-10-31-round9-schedule-config · 验收

- 状态：`done`（C19 独立终评 APPROVED at HEAD d83d965；首评 P1-1 已修复）
- 验收人：lead；C19 独立评审：auditor-build

## 1. 验收结果

| 命令 | 通过条件 |
| --- | --- |
| cargo fmt/clippy/test --workspace | 0 |
| zk/schema/docs/matrix gates | 0 |
| pnpm gates | 0 |
| e2e agent/m1/login/admin | 0 |
| srv-auth-test | 0 |
| C19 评审 | APPROVED（auditor-build 独立；P0/P1=0；P2 见 review §8.3） | docs/reviews/2026-10-31-round9-verification.md |

## 1.1 实现与复评记录

- ScheduleSettings `{enabled=false, scan_seconds=10 (5..=3600)}` 入 `AgentSettings`；设置页“调度/自动化”开关+间隔，settings.set 校验/持久化/错误回传；schema bounds 5..3600。
- 执行器常驻、disabled 时 idle；settings.set 热更新 enabled/period，无需重启；**显式保存设置（含默认 disabled/10）优先于 env**，env `ZEOKEEL_ENABLE_SCHEDULE_AUTORUN` 仅在无 settings 文件的新装环境生效。
- CI integration job 运行真实 PG `srv-auth-test.sh`（本地 0；GitHub hosted runner UNVERIFIED）。
- 首评 P1（显式默认无法关闭 env 自动调度）修复：override 由 settings 文件存在/显式保存决定，不再按值比较；false-green 测试重写。
- 非阻塞 P2（review §8.3）：GitHub runner 未验证；热更新无真实 desktop/serve e2e；docs/19 无 schedule 说明；fresh install+env 时 UI 未显示 effective fallback。

## 2. UNVERIFIED

- 真实模型调度压测、Docker/多平台、Tauri 真机、cargo-deny、私有 OEM。
