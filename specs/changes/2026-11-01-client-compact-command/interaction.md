# command-compact · 交互契约

- 表面 / 入口：composer `/` → `/compact`；会话 compact 分隔
- 归属包：`@zeokeel/ui-chat`（菜单/CompactDivider）+ `ui-kit`（actions）+ desktop-web
- 权威：docs/19 §2.1b/§11、docs/27 §2.5–§2.8、docs/11 §8.6
- 状态：`planned`（实现开始时与 intuition 一起 active）
- 证据根：`target/evidence/2026-11-01-client-compact-command/interaction/`
- 测试：`e2e/interaction/command-compact.spec.ts`

## 1. 元素
| 元素 | role/name | 键盘 | 默认焦点 | 备注 |
| --- | --- | --- | --- | --- |
| `/compact` 命令 | option /「压缩」 | ↑↓/Enter/Tab/Esc | 否 | 指令组 |
| compact 分隔 | separator /「较早消息已压缩」 | 可聚焦展开摘要 | 否 | 不展示 JSON |
| 摘要 | text/按钮 | Enter 展开 | 否 | 摘要/条数/usage |

## 2. 状态
empty/loading/error/offline/streaming/cancelled + busy/noop/disabled。busy 时分隔 loading；error 行内可重试；noop 提示；流式禁用。

## 3. 交互条目
| ID | 前置 | 动作 | 期望 | 优先级 | 测试 | 证据 |
| --- | --- | --- | --- | --- | --- | --- |
| CMD3C-COMPACT-01 | 非流式、有历史 | 执行 `/compact`；查看分隔；重复；失败/超限；流式中尝试 | 成功显示已压缩+摘要/条数/usage；**`messages_in` 原始语义/原始消息可见；compacted view 仅供分隔/replay；search/fork/audit 仍见 raw**；失败/超限/流式不改历史；重复 no-op；焦点回 composer | P0 | command-compact.spec.ts | CMD3C-COMPACT-01.json |
| CMD3C-KEY-01 | composer | `/co`→Enter；Esc；分隔摘要键盘 | 菜单键位与 Batch1 一致；焦点回 composer；分隔可展开 | P1 | 同上 | CMD3C-KEY-01.json |
| CMD3C-ERR-01 | 失败/离线/流式 | 触发错误/超限/离线 | 可读错误/重试；历史不变；不白屏/假成功 | P1 | 同上 | CMD3C-ERR-01.json |
| CMD3C-VIS-01 | compact 分隔 | 渲染 light/1280 | 截图基线（摘要 mask，0.002） | P2 | 同上 | — |

## 4. 视觉/ARIA
`e2e/interaction/__snapshots__/command-compact.spec.ts/compact-divider-light-1280.png`；axe 0 serious。

## 5. 门禁
实现开始改 active；`pnpm e2e:ui --grep command-compact` + `pnpm e2e:ui:gate`。
