# 2026-11-01-client-settings-ia · 任务清单

> 每个任务必须能独立提交、独立验证；没有 `verify` 的任务不算任务。

## 0. 跨 spec 顺序与文件 owner（评审 P1-1）

- 实施顺序：**settings-ia → approval-tool-rendering → config-approval-mcp → mcp-config**（串行；后者不得在前者未合并时改共享文件）。
- 共享文件 owner（只允许当前 spec 在窗口内改）：
  - `packages/contracts/ipc/src/lib.rs`：settings-ia 先加 settings DTO；config/mcp 之后各自追加段落，禁止重写他人段落。
  - `packages/ui/ui-kit/web/src/types.ts`：settings-ia 加 settings 类型；approval 之后加 chat 类型；config/mcp 之后加 pane 类型。
  - `apps/desktop/web/src/i18n.tsx`：settings-ia 加 settings 键；approval 加 boot/approval 键；config 加 config 键。
  - `apps/desktop/src/ipc.rs`：settings-ia 只改 SETTINGS_GET/SET 分支；approval 只改 APPROVAL 分支；config/mcp 只加新分支。
  - `docs/19`：settings-ia 改设置节；approval 改 §4；config 改连接器节。
- 冲突时：后序 spec 先 rebase 前序合并结果，并在 tasks 勾选记录。

## 1. 任务表

| # | 任务 | 写作用域（文件/目录） | 依赖 | 验证 | 状态 |
| --- | --- | --- | --- | --- | --- |
| 0 | 文档先行：`docs/16`（多贡献 slot 排序/空列表/`editions` 过滤/内建 pane）、`docs/24`（registry 生成规则 + contribution editions 过滤）、`docs/19`（设置分页/冲突态）落定 | `docs/16-ui-packages.md`、`docs/24-composition-and-runtime-spec.md`、`docs/19-client-ux-flows.md` | — | `python3 scripts/docs-check.py` | ☑ |
| 1 | 契约：`SettingsGetResult`/`SettingsSetResult`/`SettingsSetPayload.expected_revision` + `SettingsStore::revision()/save_with_revision()`（默认实现兼容） | `packages/contracts/ipc/src/lib.rs`、`packages/contracts/config/src/lib.rs` | 0 | `cargo test -p zk-contract-ipc -p zk-contract-config` | ☑ |
| 2 | 存储：`store-settings-json` 旁挂 `settings.meta.json`（原子写、hash 校验、损坏/并发/故障注入：settings 成功+meta 失败回滚） | `packages/client/store-settings-json/**` | 1 | `cargo test -p zk-store-settings-json` | ☑ |
| 3 | 宿主：`settings.get/set` 返回 revision、`expected_revision` 冲突拒绝不写盘；settings 分支单测 | `apps/desktop/src/ipc.rs`、`apps/desktop/src/settings_*.rs` | 1,2 | `cargo test -p zk-desktop` | ☑ |
| 4 | 生成器：`UiContributionDecl.editions`（可选、缺省全部）+ `ui.settings.pane` 多贡献 list slot（按 edition 过滤）→ 有序 `SettingsPanes`（空列表、personal/enterprise 快照） | `tools/zk/src/main.rs`、`shared/contracts/ui-contributions.json`（生成物） | 0 | `zk compose --edition personal --profile default --channel official --check`、`cargo test -p zk` | ☑ |
| 5 | ui-kit：共享 `SettingsForm`/`SettingsErrors`/`SettingsPaneProps`/`SettingsPaneContribution`（不含 edition 分支） | `packages/ui/ui-kit/web/src/{types.ts,index.tsx}`、`packages/ui/ui-kit/tests/*` | 4 | `pnpm --dir packages/ui/ui-kit test:unit` | ☑ |
| 6 | ui-settings 外壳重构：左导航 + pane 渲染 + 窄窗顶部 tab + 六态/焦点/Esc/单一保存 | `packages/ui/ui-settings/web/src/**`、`packages/ui/ui-settings/tests/*`、`packages/ui/ui-settings/zk.toml` | 4,5 | `pnpm --dir packages/ui/ui-settings test:unit` | ☑ |
| 7 | 内建 pane 迁移：general/models/agent/tools/search/embedding(`editions=["personal"]`)/media(`editions=["personal"]`)/enterprise(`editions=["enterprise"]`)/about 拆 pane 并在 `zk.toml` 声明 order；provider 迁到 `ui.settings.pane`(order 21, personal) | `packages/ui/ui-settings/web/src/panes/*`、`packages/ui/ui-settings/web/src/index.tsx`、`packages/ui/ui-settings/zk.toml`、`packages/ui/ui-settings-provider/**` | 6 | `pnpm -r typecheck && pnpm -r build` | ☑ |
| 8 | 企业 pane：server_url/登出/网关模型；C14 回归（渠道锁定只读、改地址需重登、enterprise 无 personal pane） | `packages/ui/ui-settings/web/src/panes/*`、`apps/desktop/web/src/App.tsx` | 7 | `pnpm --dir apps/desktop/web test:unit` | ☑ |
| 9 | 前端 revision 接线：get 初始化、save 带 expected_revision、冲突 i18n + 重载；`settings_saved` 回写基线 | `apps/desktop/web/src/{state/controller.ts,store.ts,protocol.ts,i18n.tsx}` | 3,6 | `pnpm --dir apps/desktop/web test:unit` | ☑ |
| 10 | 生成物门禁：schema/TS 重生成 + 校验 | 生成物（`shared/contracts/**`） | 1,3 | `cargo test --workspace export_schema && git diff --exit-code`；`bash scripts/schema-check.sh` | ☑ |
| 11 | e2e + a11y：分页遍历/保存/刷新/窄窗/axe/焦点回位/C14 | `e2e/m1-a11y.spec.ts`、`e2e/*.spec.ts` | 7,8,9 | `pnpm e2e`（m1-a11y + 设置用例） | ☑ |
| 12 | 文档回写：`docs/16`/`docs/28`/`docs/19` 现状、parity 报告 §7 状态 | `docs/16-ui-packages.md`、`docs/28-ui-module-map-and-skeleton.md`、`docs/19-client-ux-flows.md`、`research/client-dsh-parity-2026-11-01.md` | 6–11 | `python3 scripts/docs-check.py` | ☑ |
| 13 | 收尾：acceptance 证据、全量门禁 | `specs/changes/2026-11-01-client-settings-ia/acceptance.md` | 1–12 | `zk compose ... --check && zk capability --check && bash scripts/i18n-check.sh`；`cargo public-api diff`（契约变更；本机未安装 → UNVERIFIED，CI 必跑） | ☑ |

## 2. 写作用域

- 本变更只写：上表列出的契约/生成器/ui-kit/ui-settings/ui-settings-provider/store-settings-json/desktop/e2e/docs。
- 明确不写：`packages/client/agent-rig`、`packages/ui/ui-chat`、`packages/contracts/{agent,tool,connector}`、server/admin、生成物（由生成器写）。
- 并行任务是否重叠：任务 1/2/4 可并行（不同 crate）；任务 3 依赖 1/2；任务 6 依赖 4/5；任务 7/8 依赖 6；任务 9 依赖 3/6；任务 10 依赖 1/3；任务 11 依赖 7–9。

## 3. 提交顺序

0. 任务 0（文档先行）
1. 任务 1/2/4（契约/存储/生成器，可并行）
2. 任务 3/5（宿主/共享类型）
3. 任务 6（外壳）
4. 任务 7/8（pane 迁移/企业）
5. 任务 9/10（接线/生成物）
6. 任务 11/12/13（测试/文档/证据）

## 4. 验收前检查

- [ ] `python3 scripts/spec-check.py`
- [ ] `python3 scripts/docs-check.py`
- [ ] 变更涉及的 T0/T1 命令全绿
- [ ] 无手改 `@generated`
- [ ] 复用检查已做（`zk find`/能力索引；本 spec 不新增 capability，`zk capability --check` 保持绿）
- [ ] PR 只包含本 spec 范围
- [ ] `acceptance.md` 已填证据路径（`target/evidence/2026-11-01-client-settings-ia/`）
- [ ] `cargo public-api diff`（契约 DTO 新增；本机未安装 → UNVERIFIED，CI/有工具时必跑，见 P2-14.3）

## 5. 实现备注（2026-11-01，impl-settings）

- 任务 0 的 `docs/24` 由本实现按任务表写入（team task-5 写作用域未列该文件，但任务表要求；已同步 lead）。
- 任务 10：`export_schema` + `schema-ts` 已重生成并二次运行字节不变（`target/evidence/.../generated-sha-*.txt`）；
  `git diff --exit-code` 需提交后才有意义（共享工作树未提交），记为 UNVERIFIED-until-commit。
- 任务 11：`pnpm e2e` 原样复跑依赖已提交的生成基线（`scripts/restore-baseline.sh` 用 `git checkout --` + `--check`）；
  已用等价序列跑完 mock + serve 两阶段（`e2e-manual.log`/`e2e-retry.log`），本 spec 相关用例全绿；提交后需原样复跑一次。
- 任务 12：`docs/16`/`docs/28`/`docs/19` 已回写；`research/client-dsh-parity-2026-11-01.md` §7 不在本实现写作用域，未改（留给 Lead/研究文件 owner）。
- 任务 13：`bash scripts/i18n-check.sh` 仍红且为**基线既有**问题（`apps/desktop/web/src/main.tsx:47,49`，归 approval spec task #11）；
  本 spec 未改该文件、新增文案全部走 i18n（证据 `i18n-attribution.txt`）。`cargo public-api diff` 本机未安装 → UNVERIFIED。
- 新增实现约束（已写入 `docs/16` §2.4 / `docs/24` §3.1）：`[[ui.contributions]].editions` 匹配集 = `base_edition` 链减去基线根 edition
  （`personal`），确保 `editions=["personal"]` 不继承进 enterprise/OEM、`editions=["enterprise"]` 仍对 OEM 生效。
