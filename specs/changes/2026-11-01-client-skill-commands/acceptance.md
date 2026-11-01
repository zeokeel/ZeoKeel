# 2026-11-01-client-skill-commands · 验收

- 状态 draft（D5/D6 已定 Lead 2026-11-01；待独立评审）
- 证据根 target/evidence/2026-11-01-client-skill-commands/
- 交互/直觉 planned→active；用户测试 UNVERIFIED。

## 1. 门禁
| 命令 | 结果 | 证据 |
| --- | --- | --- |
| spec-check/docs-check | 待跑 | logs |
| cargo test -p zk-contract-ipc -p zk-tool-skill -p zk-desktop | 待跑 | logs/cargo |
| export_schema 幂等/schema-check | 待跑 | logs/schema |
| zk validate/compose --check/capability | 待跑 | logs/zk |
| pnpm typecheck/lint/test:unit/build + i18n | 待跑 | logs/pnpm |
| pnpm e2e；e2e:ui --grep command-skill；gate | 待跑 | interaction/CMD3S-SKILL-01.json |

## 2. P0
| ID | 优先级 | 测试 | 证据 |
| --- | --- | --- | --- |
| CMD3S-SKILL-01 | P0 | e2e/interaction/command-skill.spec.ts | CMD3S-SKILL-01.json |
| CMD3S-KEY-01 | P1 | 同上 | CMD3S-KEY-01.json |
| CMD3S-ERR-01 | P1 | 同上 | CMD3S-ERR-01.json |
| CMD3S-VIS-01 | P2 | 同上 | — |

## 3. 手工
- [ ] 技能组 personal 本地/enterprise `$ZEOKEEL_HOME/skills/<name>/<version>` 已安装；不可用禁用；无第二套扫描；host 经 generated facade 不 direct import tool_skill（C4）。
- [ ] `/name` 插入可移除 Instructions ≤8 KiB；发送进入上下文；移除后不含；未知/超限错误；`tool-skill list` 旧 text 兼容 + JSON 新格式（agent 工具回归）。
- [ ] enterprise `$ZEOKEEL_HOME/skills/<name>/<version>` 已安装/版本/越界；personal `.agents/skills`；host 经 generated facade（C4）；未安装不读。
- [ ] 六态/i18n/axe/键盘；错误码登记；无自动执行/网络；C18 归属。
- [ ] 用户测试 UNVERIFIED；未跑前不得声称符合直觉。

## 4. 数据/回滚
- [ ] 无 DB/文件迁移；Instructions 仅内存/消息；只读 SKILL.md；回滚移除 UI/IPC 即可。

## 5. 结论
draft；实现后填证据；未执行标 UNVERIFIED。
