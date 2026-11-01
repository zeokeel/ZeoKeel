# 2026-11-01-client-compact-command · 任务清单

> 状态：`draft`（D1/D2/D6 已定 Lead 2026-11-01；未独立评审前不得改代码，C16/C19）。
> 串行：本 spec 最先实现；plan-mode/skill-commands 随后 rebase 共享文件。

## 0. 文档先行（C12）
| # | 任务 | 写作用域 | 依赖 | 验证 | 状态 |
| --- | --- | --- | --- | --- | --- |
| 0 | docs/19 compact 分隔/失败/流式；docs/26 §5 IPC/错误码/marker；KNOWN 无自动压缩/原始仍占 DB | `docs/19`、`docs/26`、`KNOWN-LIMITATIONS.md` | — | `python3 scripts/docs-check.py` | ☐ |

## 1. 任务
| # | 任务 | 写作用域 | 依赖 | 验证 | 状态 |
| --- | --- | --- | --- | --- | --- |
| 1 | 契约 `session.compact`/`session.compaction_get` + DTO/错误码；不放宽 ChatSendPayload | `packages/contracts/ipc/src/lib.rs` | 0 | `cargo test -p zk-contract-ipc`；`cargo test --workspace export_schema` | ☐ |
| 2 | 生成物 schema/TS | `shared/contracts/schema/*`、`ts/*`（生成器写） | 1 | export_schema 幂等；schema-check | ☐ |
| 3 | kit `zk-kit-compaction` summarizer（provider/gateway、token/摘要上限、错误） | `crates/zk-kit-compaction/{Cargo.toml,src/lib.rs}` | 1 | `cargo test -p zk-kit-compaction` | ☐ |
| 4 | host compacted view（raw+marker 构造/回退；`messages_in` 原始语义不变；search/fork/audit raw）+ app 单测（压缩后原始可见） | `apps/desktop/src/ipc.rs`、`apps/desktop/web/tests/command-compact.test.tsx`（新） | 1,3 | `cargo test -p zk-desktop`；`pnpm --dir apps/desktop/web test:unit` | ☐ |
| 5 | host compact/get/replay/streaming/noop/失败/归属；`session.compacted_view` | `apps/desktop/src/ipc.rs` | 3,4 | `cargo test -p zk-desktop` | ☐ |
| 6 | UI 菜单真实化+CompactDivider+actions（raw 消息+compacted view 分隔） | `packages/ui/ui-chat/web/src/{AssistantComposer.tsx,CompactDivider.tsx}`、`apps/desktop/web/src/{App.tsx,state/controller.ts,i18n.tsx}` | 5 | typecheck/lint/i18n；UI 单测 | ☐ |
| 7 | e2e（成功/noop/失败/流式/next turn/axe）+ 交互证据 | `e2e/m1-serve.spec.ts`、`e2e/interaction/command-compact.spec.ts`（新） | 5,6 | `pnpm e2e`；`pnpm e2e:ui --grep command-compact`；`pnpm e2e:ui:gate` | ☐ |
| 8 | 收尾：acceptance 证据、interaction/intuition 改 active | 本 spec `acceptance.md` | 1–7 | spec-check/docs-check | ☐ |

## 2. 写作用域与预算
- 只写：`packages/contracts/ipc/src/lib.rs`、`crates/zk-kit-compaction/**`、`apps/desktop/src/ipc.rs`、`apps/desktop/web/tests/command-compact.test.tsx`、
  `packages/ui/ui-chat/web/src/{AssistantComposer.tsx,CompactDivider.tsx}`、`apps/desktop/web/src/{App.tsx,state/controller.ts,i18n.tsx}`、
  `e2e/m1-serve.spec.ts`、`e2e/interaction/command-compact.spec.ts`、`docs/19`、`docs/26`、`KNOWN-LIMITATIONS.md`、本 spec 目录。
- **预算 12**（Lead 2026-11-01 书面批准例外；实现/测试；docs 共享首写不计入后续）：契约1、kit2、host1、ui-chat2、app3、app 测试1、e2e2 =12。
- 不写：plan/skill 后端、自动压缩、server、rig history API。

## 3. 提交顺序
0 → 1 → 2 → 3 → 4 → 5 → 6 → 7 → 8。

## 4. 验收检查
- [ ] `spec-check`/`docs-check`；`cargo test -p zk-contract-ipc -p zk-kit-compaction -p zk-desktop`；export_schema 幂等；schema-check。
- [ ] `pnpm -r typecheck/lint/test:unit/build`；i18n；`pnpm e2e`；`pnpm e2e:ui --grep command-compact`；`pnpm e2e:ui:gate`。
- [ ] 无 migration；`messages_in` 原始语义保留；compacted view 仅 replay/分隔；search/fork/审计 raw；错误码登记；无第二套摘要/菜单；C18 归属不变；Lead 12 文件例外已记录。
- [ ] interaction/intuition planned→active；用户测试 UNVERIFIED。
