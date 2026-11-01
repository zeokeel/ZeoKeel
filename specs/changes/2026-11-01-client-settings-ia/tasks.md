# 2026-11-01-client-settings-ia · 任务清单

> 每个任务必须能独立提交、独立验证；没有 `verify` 的任务不算任务。

## 1. 任务表

| # | 任务 | 写作用域（文件/目录） | 依赖 | 验证 | 状态 |
| --- | --- | --- | --- | --- | --- |
| 1 | 契约：`settings.get` 返回 revision、`settings.set` 收 expected_revision（serde default）；宿主实现 + 单测 | `packages/contracts/ipc/src/lib.rs`、`apps/desktop/src/ipc.rs`、`apps/desktop/src/settings_*.rs` | — | `cargo test -p zk-contract-ipc -p zk-desktop` | ☐ |
| 2 | 生成器：新增 `ui.settings.pane` 多贡献 slot，生成有序 `SettingsPanes`；conformance/快照测试 | `tools/zk/src/*.rs`、`tools/zk/tests/*`、`shared/contracts/ui-contributions.json`（生成物） | — | `zk compose ... --check`、`cargo test -p zk` | ☐ |
| 3 | ui-kit：把 `SettingsForm`/校验类型提升为共享类型 + `SettingsPaneProps`/`SettingsPane` 契约（含 `@capability` 注记） | `packages/ui/ui-kit/web/src/{types.ts,index.tsx}`、`packages/ui/ui-kit/tests/*` | 1 | `pnpm --dir packages/ui/ui-kit test:unit && pnpm -r typecheck` | ☐ |
| 4 | ui-settings 外壳重构：左侧分区导航 + 右侧 pane 渲染 + 窄窗顶部 tab + 六态/焦点/Esc（字段先留在包内分文件） | `packages/ui/ui-settings/web/src/**`、`packages/ui/ui-settings/tests/*` | 2,3 | `pnpm --dir packages/ui/ui-settings test:unit && pnpm -r typecheck` | ☐ |
| 5 | pane 贡献：把现有 provider 表单改为 `ui.settings.pane` 贡献；把通用/Agent/工具/检索拆为包内 pane 组件；个人/企业差异保留 | `packages/ui/ui-settings-provider/**`、`packages/ui/ui-settings/web/src/panes/*` | 4 | `pnpm -r test:unit && pnpm -r build` | ☐ |
| 6 | 企业 pane：server_url/登出/网关模型迁入「企业」pane；登出/重登提示不回归 | `packages/ui/ui-settings/web/src/panes/*`、`apps/desktop/web/src/App.tsx` | 4 | `pnpm --dir apps/desktop/web test:unit` | ☐ |
| 7 | 前端 revision 接线：settings.get 保存 revision、save 带 expected_revision、冲突提示与重载 | `apps/desktop/web/src/{state/controller.ts,store.ts,protocol.ts,i18n.tsx}` | 1,4 | `pnpm --dir apps/desktop/web test:unit` | ☐ |
| 8 | e2e + a11y：设置分页遍历/保存/刷新保持/窄窗/axe/焦点回位 | `e2e/m1-a11y.spec.ts`、`e2e/*.spec.ts` | 5,6,7 | `pnpm e2e --grep 设置`（或既有入口） | ☐ |
| 9 | 文档：更新 `docs/16`（slot 表）、`docs/28`（设置分页口径）、`docs/19`（设置状态）；回写 parity 报告 §7 | `docs/16-ui-packages.md`、`docs/28-ui-module-map-and-skeleton.md`、`docs/19-client-ux-flows.md`、`research/client-dsh-parity-2026-11-01.md` | 4 | `python3 scripts/docs-check.py` | ☐ |
| 10 | 收尾：`acceptance.md` 证据、生成物 `--check`、i18n 门禁 | `specs/changes/2026-11-01-client-settings-ia/acceptance.md` | 1–9 | `zk compose ... --check && zk capability --check && bash scripts/i18n-check.sh` | ☐ |

## 2. 写作用域

- 本变更只写：上表列出的契约/生成器/ui-kit/ui-settings/ui-settings-provider/desktop web/e2e/docs。
- 明确不写：`packages/client/**` 运行时、`apps/server/**`、admin SPA、生成物（由生成器写）。
- 并行任务是否重叠：任务 1/2/3 可并行（不同 crate/包）；任务 4 依赖 2/3；任务 5/6 依赖 4，二者写不同 pane 文件可并行；
  任务 7 依赖 1/4；任务 8 依赖 5/6/7。

## 3. 提交顺序

1. 任务 1（契约，先行为后 UI）
2. 任务 2（生成器，slot 基础）
3. 任务 3（ui-kit 共享类型）
4. 任务 4（外壳）
5. 任务 5/6（pane 迁移）
6. 任务 7（revision 接线）
7. 任务 8/9/10（测试、文档、证据）

## 4. 验收前检查

- [ ] `python3 scripts/spec-check.py`
- [ ] `python3 scripts/docs-check.py`
- [ ] 变更涉及的 T0/T1 命令全绿
- [ ] 无手改 `@generated`
- [ ] 复用检查已做（`zk find`/能力索引；新增能力已登记 `@capability`）
- [ ] PR 只包含本 spec 范围
- [ ] `acceptance.md` 已填证据路径
