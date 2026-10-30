# 2026-10-30-agent-dsh-parity · 验收

- 状态：`in-progress`（spec 阶段）
- 验收人：pending
- 证据：`target/evidence/agent-parity/`

## 已落地证据

- `docs/generated/AGENT-TOOL-MATRIX.md`（`scripts/agent-tool-matrix.py --check`）
- `docs/39-agent-prompts.md`
- `scripts/agent-e2e.sh`：真实 personal + fake upstream，5 passed（审批/拒绝/取消/ask-user/bash/多步）
- `cargo test --workspace` / clippy 绿
- SSRF/redirect/私网负向；worktree 注入/逃逸负向
- 实时 ToolCallStarted/Finished；取消标志；审批 fail-closed；ask-user IPC/UI

## 仍未完成

- rig hooks/extractor/vector/loaders/多 provider
- agent-team（禁用）、call-timeout 策略包、schedule 执行器、workflow DAG、subagent fork/异步 control
- 工具参数/名称与 DSH 完全对齐
- 逐工具 Playwright 全矩阵（当前 3 条 agent e2e）

## 检查

- [ ] DSH 工具矩阵 diff=0；prompt 来源文档
- [ ] `cargo test --workspace` / clippy
- [ ] agent mock e2e：多步/审批/取消/最大步数/未知工具
- [ ] 每工具正/负测试
- [ ] Playwright 工具+审批+provider 设置
- [ ] axe 0 serious/critical
- [ ] 独立评审 APPROVED
