# 2026-10-30-workspace-scoped-conversations · 验收

- [x] T1 契约测试绿
- [x] T2 迁移/工作区/隔离测试绿（5 tests）
- [x] T3 agent cwd/tool root 测试绿
- [ ] T4 workspace IPC + context 已通；新路径无 conversation 失败已测；旧 auto-create 兼容路径待 T5 删除
- [ ] T5 UI 首用/切换/重启/不可用目录 e2e + axe（设计评审通过后）
- [ ] 默认 Workspace = `~`（home canonicalize）且不自动建会话；列表平铺/会话行平铺；home 不可用进入选择器（无 data 回退）
- [ ] docs-check/spec-check/review-check/compose --check 绿
