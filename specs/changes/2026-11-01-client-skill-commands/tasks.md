# 2026-11-01-client-skill-commands · 任务清单

> 状态 done；D5/D6 已定；串行 compact→plan-mode→**本 spec**；共享文件后写者 rebase。

## 0. 文档
| # | 任务 | 作用域 | 依赖 | 验证 | 状态 |
| --- | --- | --- | --- | --- | --- |
| 0 | docs/19 技能组/Instructions；docs/26 IPC/错误码/来源（personal/enterprise 路径）；KNOWN enterprise 未安装/不自动执行 | docs/19、docs/26、KNOWN | — | docs-check | ✅ |

## 1. 任务
| # | 任务 | 作用域 | 依赖 | 验证 | 状态 |
| --- | --- | --- | --- | --- | --- |
| 1 | 契约 skill.list/read + DTO/错误码（additive） | contracts/ipc/src/lib.rs | 0 | cargo test -p zk-contract-ipc；export_schema | ✅ |
| 2 | 生成物 schema/TS | shared/contracts/* | 1 | export_schema 幂等；schema-check | ✅ |
| 3 | tool-skill 扩展 list（默认 text 兼容旧换行 + `format:"json"`）/frontmatter/user-invocable + read 8 KiB（单实现）；enterprise path helper | packages/client/tool-skill/src/lib.rs | 1 | cargo test -p zk-tool-skill（text/json 兼容/路径/版本） | ✅ |
| 3b | `zk-composed` generated facade 暴露技能读取（`tools/zk/src/main.rs` 生成输入；不手改生成物） | tools/zk/src/main.rs | 3 | `zk compose --check`；host 不 direct import tool_skill | ✅ |
| 4 | host skill.list/read 经 facade；personal `.agents/skills` / enterprise `$ZEOKEEL_HOME/skills/<name>/<version>`；归属/版本/越界/不可用/超限 | apps/desktop/src/{ipc.rs,skill.rs} | 3,3b | cargo test -p zk-desktop；zk compose --check | ✅ |
| 5 | UI 技能候选/Instructions/发送块/六态/键盘 | ui-chat/{AssistantComposer.tsx,composerRefs.ts,AssistantThread.tsx}、ui-kit/{context.tsx,types.ts} | 4 | typecheck/lint/test:unit | ✅ |
| 6 | app 接线/controller/i18n | apps/desktop/web/src/{App.tsx,protocol.ts,validate.ts,i18n.tsx} | 5 | typecheck/lint/i18n | ✅ |
| 7 | e2e/交互证据 | e2e/interaction/command-skill.spec.ts（新） | 4–6 | pnpm e2e；e2e:ui --grep command-skill；gate | ✅ |
| 8 | 收尾 acceptance、active | acceptance.md | 1–7 | spec-check/docs-check | ✅ |

## 2. 预算 10（≤10）
contract1、tool-skill1、host ipc1、generator facade1、AssistantComposer1、ui-kit context1、App1、controller1、i18n1、e2e1 =10。共享 docs 首写不计重复；protocol 并入 App。
不写：compact/plan、商店后端/新扫描/自动执行；host 不 direct import tool_skill（C4）。

## 3. 顺序
0→1→2→3→4→5→6→7→8。

## 4. 验收
- [x] spec-check/docs-check；cargo test --workspace；export_schema 幂等/schema-check。
- [x] pnpm typecheck/lint/test:unit/build/i18n；pnpm e2e；e2e:ui --grep command-skill；gate。
- [x] 来源/权限正确/无第二套扫描；Instructions 可移除/发送上下文/8 KiB；错误码登记；interaction/intuition active；用户测试 UNVERIFIED。
