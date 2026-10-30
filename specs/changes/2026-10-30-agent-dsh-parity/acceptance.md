# 2026-10-30-agent-dsh-parity · 验收

- 状态：`implementation-in-progress`（R6 CHANGES-REQUESTED；等待 R7 复评）
- 验收人：pending
- 证据：`target/evidence/agent-parity/`

## 已落地证据

- `docs/generated/AGENT-TOOL-MATRIX.md`（`scripts/agent-tool-matrix.py --check`）
- `docs/39-agent-prompts.md`
- `scripts/agent-e2e.sh`：真实 personal + fake upstream，6 scenarios（审批/拒绝/取消/ask-user/bash/present/多步）
- `cargo test --workspace` / clippy 绿
- SSRF/redirect/私网负向；worktree 注入/逃逸负向
- 实时 ToolCallStarted/Finished；取消标志；审批 fail-closed；ask-user IPC/UI

## 仍未完成 / P2 manual

- rig hooks/vector/media 为 capability 占位（`available()==false`、`CAPABILITY_UNAVAILABLE`），需产品确认 P2 或配置 provider
- subagent 异步 send/interrupt；agent-team（实验禁用）
- 工具名/参数与 DSH 完全对齐
- `call-timeout` 为 agent 级环境策略（C3 豁免）

## 检查

- [x] DSH 工具矩阵 diff=0；prompt 来源文档
- [x] `cargo test --workspace` / clippy
- [x] agent mock e2e：多步/审批/取消/最大步数/未知工具
- [x] 每工具正/负测试
- [x] Playwright 工具+审批+provider 设置
- [x] axe 0 serious/critical
- [ ] 独立评审 APPROVED
