# 2026-11-01-client-plan-mode · 任务清单

> 状态 draft；D3/D4/D6 已定；串行 compact→**本 spec**→skill-commands；共享文件后写者 rebase。

## 0. 文档
| # | 任务 | 作用域 | 依赖 | 验证 | 状态 |
| --- | --- | --- | --- | --- | --- |
| 0 | docs/19 plan mode/审核；docs/26 IPC/plan 文件/错误码；**docs/16 plan tab order 50**；KNOWN 无 sandbox/不自动执行/不传播 plan 到 child | docs/19、docs/26、docs/16、KNOWN | — | docs-check | ☑ |

## 1. 任务
| # | 任务 | 作用域 | 依赖 | 验证 | 状态 |
| --- | --- | --- | --- | --- | --- |
| 1 | 契约 plan.get/update + DTO/错误码（additive） | contracts/ipc/src/lib.rs | 0 | cargo test -p zk-contract-ipc；export_schema | ☑ |
| 2 | 生成物 schema/TS | shared/contracts/* | 1 | export_schema 幂等；schema-check | ☑ |
| 3 | tool-plan 包四件套 exit_plan_mode（≤64 KiB/文件/pending） | packages/client/tool-plan/**（新） | 1 | cargo test -p zk-tool-plan；zk validate | ☑ |
| 4 | agent-rig plan mode 过滤（只读+exit；禁 mutating+**全部 delegation**+schedule/MCP；逐 delegation 阻断测试（含 **`subagent_fork` 专项 + child mutating 0**；派生 `subagent*` 前缀/role 集合 + 静态清单兜底））+ generated facade | agent-rig/src/lib.rs、tools/zk/src/main.rs | 3 | cargo test -p zk-agent-rig；zk compose --check | ☑ |
| 5 | host plan 状态/审核/文件/注入 | apps/desktop/src/ipc.rs | 1,3,4 | cargo test -p zk-desktop | ☑ |
| 6 | UI PlanCard + plan tab + actions | ui-chat/PlanCard.tsx、ui-inspector/index.tsx、ui-kit/context.tsx | 5 | typecheck/lint/test:unit | ☑ |
| 7 | app 接线/controller/i18n | apps/desktop/web/src/{App.tsx,state/controller.ts,i18n.tsx} | 5,6 | typecheck/lint/i18n | ☑ |
| 8 | e2e/交互证据 | e2e/interaction/command-plan.spec.ts（新） | 5–7 | pnpm e2e；e2e:ui --grep command-plan；gate | ☑ |
| 9 | 收尾 acceptance、active | acceptance.md | 1–8 | spec-check/docs-check | ☑ |

## 2. 预算 16
contract1、tool pkg 5（zk.toml/Cargo/src/tests/README）、agent-rig1、generator1、host1、PlanCard1、inspector1、ui-kit context1、App1、controller1、i18n1、e2e1 =16。共享 docs 由 compact 首写，本 spec 追加不计重复；protocol 并入 App。
不写：compact/skill 后端、sandbox/企业策略、自动执行。

## 3. 顺序
0→1→2→3→4→5→6→7→8→9。

## 4. 验收
- [ ] spec-check/docs-check；cargo tests/export_schema；zk validate/compose --check/capability（tool.plan 登记）。
- [ ] pnpm typecheck/lint/test:unit/build/i18n；pnpm e2e；e2e:ui --grep command-plan；gate。
- [ ] mutating 0/只读可用；**delegation 六项（含 subagent_fork）各工具阻断且 child mutating 0**；派生规则/静态兜底测试；计划恢复；无沙箱/企业策略；错误码登记；interaction/intuition active；用户测试 UNVERIFIED；Lead 书面批准 16 文件预算例外已记录 proposal §8。

## 6. 实现备注（2026-11-01，impl-settings）

- 新包 `packages/client/tool-plan`（role `tool.plan`，editions personal/enterprise，四件套 + 2 单测）：`exit_plan_mode` 校验非空/≤64 KiB、要求 mode active、写 `pending_review`+through_turn；`plans` 模块唯一文件实现（原子写、64 KiB 上限、approve/reject/continue/off 状态机），host 经生成 facade 复用。
- agent-rig：`set_plan_mode`（contract Agent 默认 no-op）；动态工具构造时白名单/黑名单过滤（静态六项 delegation + `subagent_*` 前缀 + mutating + schedule/MCP）；被禁工具保留可见、emits ToolCallStarted/Finished(Failed) 并返回可读 `plan/*-blocked`；4 个策略单测（只读/exit 允许、mutating、六项 delegation、schedule/MCP）。
- 契约：`plan.get`/`plan.update` + `PlanView/PlanGetResult/PlanUpdateResult/Session...` payload；5 新 schema（132 total）；facade 增加 `zk_tool_plan`、`Runtime::set_plan_mode`；`zk.toml`/editions 注册 `tool.plan`。
- host `plan.rs`：plan.get/update（enter/approve/reject/continue/off、feedback ≤4 KiB、归属/错误码、原子写）；run_turn 开始按 `.zeokeel/plans.json` 设置 plan mode（Drop 复位）。
- UI：`/plan`/`/plan off` 真命令（submit 拦截 + 菜单 execute）；PlanCard（turnTail）+ 右栏 plan tab order 50；turn 结束后刷新 plan 状态（exit_plan_mode 是工具调用）；i18n zh/en；fixtures/单测更新。
- e2e：`e2e/interaction/command-plan.spec.ts` 4 个 P0/P1；`e2e/serve-m1.sh` fake upstream 增加 `提交计划`/`写入` 触发（唯一关键字，不影响既有测试）。
