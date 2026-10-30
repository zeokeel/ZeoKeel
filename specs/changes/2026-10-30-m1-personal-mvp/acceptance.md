# 2026-10-30-m1-personal-mvp · 验收与证据

- 日期：2026-10-30
- 状态：进行中（由 goal 循环推进）

## 命令与证据

| 命令 | 结果 | 证据 |
| --- | --- | --- |
| `cargo test --workspace` | 待填 | |
| `zk compose --check` / `capability --check` | 待填 | |
| `pnpm --dir apps/desktop/web build` | 待填 | |
| `pnpm exec playwright test` | 待填 | `target/evidence/m1/` |
| axe | 待填 | `target/evidence/m1/axe.json` |

## 人工路径

- [ ] 新建会话 → 发送“读取 demo-data/hello.txt” → 工具卡 + 正文
- [ ] 取消流式 → 已生成内容保留
- [ ] 重启后会话/设置恢复
- [ ] 键盘操作 + 焦点可见
