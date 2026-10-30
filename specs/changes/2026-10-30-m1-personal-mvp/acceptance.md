# 2026-10-30-m1-personal-mvp · 验收与证据

- 日期：2026-10-30
- 状态：进行中（由 goal 循环推进）

## 命令与证据

| 命令 | 结果 | 证据 |
| --- | --- | --- |
| `cargo test --workspace` | PASS（55 passed；含 workspace v3 迁移） | `cargo test --workspace --offline` |
| `zk compose --check` / `capability --check` | PASS | `zk validate`（22 packages）、compose 15 files、capability 9 entries |
| `pnpm --dir apps/desktop/web build` | PASS | `dist/`（无 prototype/dense） |
| `pnpm exec playwright test` | PASS（12 tests：mock/serve/workspace API/security） | `target/evidence/m1/` |
| axe | PASS（e2e AxeBuilder；生产 UI 0 serious/critical；UX 骨架另测 0） | `e2e/m1-mock.spec.ts` + `screenshots/axe.json` |

## 人工路径

- [x] 新建会话 → 发送“读取 demo-data/hello.txt” → 工具卡 + 正文
- [x] 取消流式 → 已生成内容保留（message.status 持久化）
- [x] 重启后会话/设置恢复（SQLite + JSON）
- [x] 键盘操作 + 焦点可见（e2e + modal focus trap）

## UI shell wiring

- `AppShell` + FirstRun（boot/error/首用/layout/header/status/toast）已从 `apps/desktop/web` 移入 `@zeokeel/ui-shell`（`shell.frame`），app 只经 registry 接入（App 约 139 行）。
- 开发原型/截图已移出 `apps/desktop/web` 到 `research/ui-prototypes/`。

## Command palette

- `@zeokeel/ui-commands` 接入：⌘K/Ctrl+K 打开、搜索过滤、方向键/Enter/Esc、动作由 app 注入。
- Vitest 36 tests；Playwright mock flow + axe 0 serious/critical。

## Inspector

- `workspace.files` IPC（workspace 边界内、深度≤2、≤300 项、跳过 `.git`）→ `@zeokeel/ui-inspector`；⌘B 开关；Playwright 真 serve 文件树 + axe 0 serious/critical。

## Update check package

- `zk-client-update-github`（`update.client`，personal）：读 GitHub `latest.json`、semver 比较、按平台选资产；企业版 exclude。
- 仍缺：设置页“检查更新”入口、下载/校验/安装。
