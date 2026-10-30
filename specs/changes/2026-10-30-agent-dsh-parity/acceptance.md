# 2026-10-30-agent-dsh-parity · 验收

- 状态：`blocked`（R4 CHANGES-REQUESTED；需产品 descope 或补全 rig 全能力）
- 验收人：pending
- 证据：`target/evidence/agent-parity/`

## 已落地证据

- `docs/generated/AGENT-TOOL-MATRIX.md`（`scripts/agent-tool-matrix.py --check`）
- `docs/39-agent-prompts.md`
- `scripts/agent-e2e.sh`：真实 personal + fake upstream，6 scenarios（审批/拒绝/取消/ask-user/bash/present/多步）
- `cargo test --workspace` / clippy 绿
- SSRF/redirect/私网负向；worktree 注入/逃逸负向
- 实时 ToolCallStarted/Finished；取消标志；审批 fail-closed；ask-user IPC/UI

## 仍未完成（R4）

- rig hooks / vector store / dynamic context / 图像/音频/转写/rerank
- schedule 执行器；subagent fork/异步 control；agent-team（实验禁用）
- Playwright max turns / unknown tool / provider 设置
- PDF 正例测试（当前只有 EPUB 正例 + PDF 负例）
- `call-timeout` 为 agent 级环境策略（已豁免独立包）
- 工具名/参数与 DSH 完全对齐

## 检查

- [ ] DSH 工具矩阵 diff=0；prompt 来源文档
- [ ] `cargo test --workspace` / clippy
- [ ] agent mock e2e：多步/审批/取消/最大步数/未知工具
- [ ] 每工具正/负测试
- [ ] Playwright 工具+审批+provider 设置
- [ ] axe 0 serious/critical
- [ ] 独立评审 APPROVED
