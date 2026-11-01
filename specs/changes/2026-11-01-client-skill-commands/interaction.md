# command-skill-commands · 交互契约

- 入口：composer `/` 的「技能」组 + `/name` Instructions
- 归属：`@zeokeel/ui-chat`、ui-kit、desktop-web
- 权威：docs/19 §2.1b/§11、docs/27、docs/11 §8.6
- 状态 active；证据根 target/evidence/2026-11-01-client-skill-commands/interaction/
- 测试 e2e/interaction/command-skill.spec.ts

- 状态：`active`（实现阶段）
## 1. 元素
| 元素 | role/name | 键盘 | 默认焦点 |
| --- | --- | --- | --- |
| 技能组/候选 | listbox option / 技能名+描述 | ↑↓/Enter/Tab/Esc | 否 |
| Instructions chip | button /「移除技能：<name>」 | Delete/Enter | 否 |
| 空/错误/不可用 | status/alert | 可读 | 否 |

## 2. 状态
empty/loading/error/offline/streaming/cancelled + disabled/inserted/removed。技能组 loading/空/不可用；插入后 chip；流式可插入不发送。

## 3. 条目
| ID | 前置 | 动作 | 期望 | 优先级 | 测试 | 证据 |
| --- | --- | --- | --- | --- | --- | --- |
| CMD3S-SKILL-01 | `/` 菜单 | 打开技能组；过滤；选 `/name`；移除；发送；enterprise 未安装 | personal `.agents/skills`/enterprise `$ZEOKEEL_HOME/skills/<name>/<version>` 已安装；不可用禁用；host 经 generated facade；插入可移除 Instructions ≤8 KiB；发送进入上下文；未知/超限错误；无第二套扫描 | P0 | command-skill.spec.ts | CMD3S-SKILL-01.json |
| CMD3S-KEY-01 | composer | `/sk`→↑↓/Enter；删除 chip；Esc | 键位同 Batch1；chip 键盘可达；焦点回 composer | P1 | 同上 | CMD3S-KEY-01.json |
| CMD3S-ERR-01 | 技能组 | 空/加载/错误/离线/不可用/超限 | 可读/可重试；不白屏/假插入；Instructions 保留 | P1 | 同上 | CMD3S-ERR-01.json |
| CMD3S-VIS-01 | 技能组/Instructions | light/1280 | 截图基线（技能名 mask/0.002） | P2 | 同上 | — |

## 4. 视觉/ARIA
`e2e/interaction/__snapshots__/command-skill.spec.ts/skill-menu-light-1280.png`；axe 0 serious。

## 5. 门禁
实现开始 active；`pnpm e2e:ui --grep command-skill` + gate。
