# 2026-10-30-workspace-scoped-conversations · 验收

- [x] T1 契约测试绿
- [x] T2 迁移/工作区/隔离测试绿（5 tests）
- [x] T3 agent cwd/tool root 测试绿
- [x] T4 workspace IPC + context 已通；新路径无 conversation 失败；旧 auto-create/兼容方法已删除
- [x] T5 UI DSH 式平铺 workspace/会话、默认 `~`、首用、切换/隔离/移除确认；Playwright 9 passed + axe
- [ ] 默认 Workspace = `~`（home canonicalize）且不自动建会话；列表平铺/会话行平铺；home 不可用进入选择器（无 data 回退）
- [ ] docs-check/spec-check/review-check/compose --check 绿
