# 2026-10-31-round12-final-audit · 规格

- 状态：`approved`

## 1. CI schedule e2e
- `.github/workflows/ci.yml` e2e job 增加 `bash e2e/serve-schedule.sh`（在 m1-e2e 之后/独立步骤），确保 hosted runner 上可运行（Playwright chromium 已安装）。本地已验证脚本 0；CI runner UNVERIFIED。
## 2. 最终审计
- 复跑所有门禁与 e2e，记录命令/exit/关键输出。
- 对照目标 1–4 与 docs/specs 建立 achievement matrix（achieved/partial/backlog/UNVERIFIED）。
- 审计所有 acceptance/review 记录是否为 APPROVED、status done；所有 backlog/UNVERIFIED 是否在 docs/acceptance 可见。
- 独立评审由 auditor-build 完成（非实现者），P0/P1 必须修复。
