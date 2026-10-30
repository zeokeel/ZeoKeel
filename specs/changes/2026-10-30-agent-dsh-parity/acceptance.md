# 2026-10-30-agent-dsh-parity · 验收

- 状态：`in-progress`（spec 阶段）
- 验收人：pending
- 证据：`target/evidence/agent-parity/`

## 检查

- [ ] DSH 工具矩阵 diff=0；prompt 来源文档
- [ ] `cargo test --workspace` / clippy
- [ ] agent mock e2e：多步/审批/取消/最大步数/未知工具
- [ ] 每工具正/负测试
- [ ] Playwright 工具+审批+provider 设置
- [ ] axe 0 serious/critical
- [ ] 独立评审 APPROVED
