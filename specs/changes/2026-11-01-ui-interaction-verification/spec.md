# 2026-11-01-ui-interaction-verification · 行为规格

> 只写"系统必须怎样"，不写实现选型；实现见 `design.md`。基线契约见本目录 `interaction.md`。

## 1. 需求

### R1 · 交互契约与交互 ID

- **WHEN** 任何客户端 surface（页面/面板/弹窗/菜单/输入区）新增或修改交互行为
- **THEN** 该变更目录必须包含 `interaction.md`，声明：表面/入口、元素与语义、状态模型、交互条目、视觉/ARIA 基线、预算与一致性约束
- **AND** 每个交互条目有唯一 ID（`<SURFACE>-<AREA>-<NN>`），并标注优先级 `P0`/`P1`/`P2`；`P0`/`P1` 必须给出测试落点与证据路径
- **AND** `interaction.md` 顶部声明 `状态：active | planned`；只有 `active` 契约参与门禁；`planned` 契约在实现开始时改 `active` 并补测试
- **AND** 状态模型至少覆盖：空/加载/错误/离线/流式中/已取消（六态）+ `busy`/`disabled`/`composing`/`menu-open`（按 surface 适用）

### R2 · 基线契约（本变更交付）

- **WHEN** 本变更完成
- **THEN** 本目录 `interaction.md` 为 composer / settings dialog / inspector / command palette / sessions 五个已实现 surface 提供 `active` 基线契约，含 19 个交互 ID（P0 = 11），每个 P0/P1 有 e2e 标签与证据
- **AND** 基线只覆盖现有能力：不把 Phase 2a/2b 未实现功能写成已验收
- **AND** 在途 spec（composer Phase 2a、side-panels Phase 2b）各自补 `interaction.md`（`planned`），其 R1–R7 交互细节拆成 ID，实现时改 `active`

### R3 · 真人点击 harness

- **WHEN** 执行 `pnpm e2e:ui`
- **THEN** 以独立 Playwright 配置启动真实 `serve` 载体（真实 SQLite + fake upstream，`e2e/serve-m1.sh`），运行 `e2e/interaction/*.spec.ts`
- **AND** 全部定位使用 role/accessible name/label（禁止 CSS 类选择器作为主定位）；键盘用真实按键序列；文件/拖放用 `filechooser`/`DataTransfer`；IME 用 `compositionstart` + `isComposing` keydown 模拟
- **AND** 每个测试带 `@interaction:<ID>` 标签；测试末尾写 `target/evidence/2026-11-01-ui-interaction-verification/interaction/<ID>.json`（含 ID、测试名、时间、通过标记）
- **AND** 失败保留 trace/screenshot；不允许 `waitForTimeout`、`force: true`、无 `@known-issue` 的 `test.skip`
- **AND** 主 `pnpm e2e` 不重复运行交互套件（主 `playwright.config.ts` 忽略 `**/interaction/**`）

### R4 · 视觉与 ARIA 基线

- **WHEN** `pnpm e2e:ui` 运行
- **THEN** 关键 surface 使用 `toMatchAriaSnapshot` 断言结构/角色/状态（composer、settings dialog、inspector、command palette）
- **AND** 至少 1 个稳定视觉基线（composer 空态，light/1280）使用 `toHaveScreenshot`；禁用动画、`maxDiffPixelRatio ≤ 0.002`；基线文件提交仓库
- **AND** 每个 surface 的 axe 扫描 0 serious/critical；结果失败时输出违规 id 列表
- **AND** 视觉基线更新只能由人确认后执行 `pnpm e2e:ui:update`，不得在 CI 自动接受

### R5 · 交互门禁

- **WHEN** 执行 `python3 scripts/ui-interaction-gate.py`
- **THEN** 对每个 `active` 契约：P0/P1 ID 必须至少被一个 `@interaction:<ID>` 测试覆盖；测试标签不得引用未定义 ID；每个 P0/P1 ID 必须有已生成的证据 JSON
- **AND** 扫描 `e2e/interaction/**/*.ts`，发现 `waitForTimeout` / `force: true` / 无 `@known-issue` 的 `test.skip` 时失败
- **AND** 对 `draft`/`approved` 的 `planned` 契约只输出 WARN，不阻塞；对 `in-progress`/`done` 契约严格
- **AND** 输出 `checked contracts/tests/ids` 摘要；exit 0 表示全部满足

### R6 · 文档与一致性（SSOT）

- **WHEN** 本变更完成
- **THEN** `docs/11` §8.6 定义交互契约/证据/门禁口径；`docs/19` §11 定义交互 ID 强制规则；`docs/27` §2.5/§2.6 定义唯一交互模式清单、token-only、六态、焦点/Esc/IME、错误分层、动效、i18n、响应式与视觉/ARIA 基线规则
- **AND** `AGENTS.md` §6 增加 `pnpm e2e:ui` / gate 命令；§8 增加"UI 变更必须交交互契约与证据"
- **AND** `KNOWN-LIMITATIONS.md` 记录：视觉基线本机通过/CI 首跑 UNVERIFIED；IME 为模拟；MCP/CLI/Storybook 未采用
- **AND** 文档不得复制第二份规范；`docs-check`/`docs-command-check` 绿

### R7 · Skill 与 agent 提示词

- **WHEN** 任意 agent 接到 UI 任务
- **THEN** 仓库内存在 `.agents/skills/zeokeel-ui-interaction/SKILL.md`，规定：先契约后代码、测试先行、失败读 trace/截图/aria diff、证据落盘、fresh-context 评审
- **AND** `agent-rig::default_preamble` 包含客户端 UI 完成定义、交互不变量、禁止捷径、循环与停止条件；`docs/39` 记录原文与规则
- **AND** 提示词变更不引入运行期插件/依赖（C2）；不把 DSH 运行时代码引入仓库（C15）

### R8 · CI

- **WHEN** PR 触发 CI
- **THEN** `.github/workflows/ci.yml` 新增 `ui-interaction` job：安装依赖 + chromium、运行 `pnpm e2e:ui`、运行 gate、上传证据 artifact
- **AND** 本地可复现命令与 CI 一致；hosted-runner 首次执行在 acceptance 中标 `UNVERIFIED` 直到实跑
- **AND** 不修改既有 job 的职责与超时预算

## 2. 边界与错误

| 场景 | 期望行为 | 错误码/提示 |
| --- | --- | --- |
| 契约 `active` 但有 P0/P1 无测试标签 | gate 失败并列出 ID | `missing-tag` |
| 测试标签引用未定义 ID | gate 失败并列出标签 | `unknown-id` |
| 测试通过但未生成证据 JSON | gate 失败并列出 ID | `missing-evidence` |
| 测试含 `waitForTimeout`/`force: true`/裸 `test.skip` | gate 失败并给文件:行 | `anti-pattern` |
| 契约 `planned`/spec 仍 draft | 只 WARN，不阻塞 | `planned-skip` |
| 视觉基线缺失 | e2e 生成并失败（Playwright 默认）；首次需人工 `-u` | 由 Playwright 报告 |
| 视觉像素差异 | e2e 失败，附 actual/expected/diff；不得自动接受 | 由 Playwright 报告 |
| `serve` 启动失败/端口占用 | e2e 失败并保留 serve 日志 | 由 webServer 报告 |
| IME 模拟在 headless 下不生效 | 测试失败；允许改为 L1 单测并调整契约（需评审） | 由测试报告 |
| axe serious/critical | e2e 失败并列出违规 id | 由测试报告 |

## 3. 权限与可见性

- 角色：仅影响开发/CI 工具链与文档；不改变个人/企业客户端权限、审批、网关、商店语义。
- 内容可见性：证据写入 `target/evidence/`（不入库）；快照写入 `e2e/interaction/`（入库）；无用户数据。
- 服务端是否下发策略：否（纯客户端开发期验证，C6 不受影响）。

## 4. 数据与兼容

- 无数据库/契约/生成物变更；无产品运行期数据。
- 新增仓库文件：`playwright.interaction.config.ts`、`e2e/interaction/**`、`scripts/ui-interaction-gate.py`、`.agents/skills/zeokeel-ui-interaction/SKILL.md`、`specs/templates/interaction.md`、本 spec 与基线 `interaction.md`。
- 修改仓库文件：`docs/11`、`docs/19`、`docs/27`、`docs/39`、`docs/README`、`AGENTS.md`、`KNOWN-LIMITATIONS.md`、`package.json`、`playwright.config.ts`、`tsconfig.json`、`.github/workflows/ci.yml`、`scripts/new-change.sh`、`packages/client/agent-rig/src/lib.rs`、两份在途 spec。
- 快照文件为新增；删除/重命名需同步 `interaction.md` 基线路径。
- 幂等：gate 只读；e2e 不写产品数据；证据文件每次覆盖。
- 回滚：删除新增文件、还原修改文件即可；不涉及数据迁移/发布。

## 5. 验收断言（可执行）

- [ ] `pnpm e2e:ui` exit 0；5 个 spec 全绿；证据 JSON = 19 个。
- [ ] `python3 scripts/ui-interaction-gate.py` exit 0（active 契约覆盖完整）。
- [ ] `pnpm exec tsc -p tsconfig.json --noEmit`（或 `tsc -b`）exit 0；`pnpm -r lint` exit 0。
- [ ] `python3 scripts/docs-check.py`、`python3 scripts/spec-check.py`、`python3 scripts/review-check.py`、`python3 scripts/docs-command-check.py --self-test && python3 scripts/docs-command-check.py` 全绿。
- [ ] `bash scripts/i18n-check.sh` exit 0。
- [ ] 主 `pnpm e2e` 不包含 `e2e/interaction`（testIgnore 生效）；`playwright test --list -c playwright.interaction.config.ts` 列出 5 个 spec。
- [ ] `cargo check -p zk-agent-rig` + `cargo test -p zk-agent-rig` exit 0；preamble 含 UI 段。
- [ ] 在途两份 spec 有 `interaction.md`（planned）与 acceptance ID 表。
- [ ] CI YAML 可解析且新增 job 引用本地同名命令（hosted runner UNVERIFIED）。

## 6. 已定决策

- [x] D1 范围：完整闭环（用户 2026-11-01 指令）。
- [x] D2 工具：Playwright + axe + 自建契约/门禁；不引入 Storybook/SaaS/MCP。
- [x] D3 视觉：只 composer 最小基线；其余 aria snapshot + axe。
- [x] D4 在途 spec：只补 planned 契约与验收/任务接线。
