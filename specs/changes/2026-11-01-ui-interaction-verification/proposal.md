# 2026-11-01-ui-interaction-verification · 提案

- 状态：`in-progress`
- Owner：lead
- 日期：2026-11-01
- 关联：用户 2026-11-01 明确要求「完整落地」交互验证闭环；`docs/11` §8.6、`docs/19` §11、`docs/27` §2.5/§2.6（本变更落定）；`AGENTS.md` §6/§8；`specs/changes/2026-11-01-client-composer-surface`、`2026-11-01-client-side-panels`、`2026-11-01-client-activity-panels`（在途 spec 补 `interaction.md`）
- 评审：`docs/reviews/2026-11-01-ui-interaction-verification.md#结论`（APPROVED，P0=0/P1=0/P2=10 非阻塞）
- 变更预算：**30 个文件**（文档 8、spec 7、harness 8、门禁/脚本 2、CI 1、skill 2、prompt 2；不含本 spec 目录与生成物）

## 1. 背景 / 问题

当前客户端交付的验收口径是「功能可用 + 单测/e2e 断言绿」，但**交互细节没有成为可验收的一等交付物**：

- e2e 默认只有 1280×820；截图写进 `target/evidence/` 后不比对；全仓 `e2e/ apps/ packages/` 无 `toHaveScreenshot` / `toMatchAriaSnapshot` 使用。
- 验收表以「单测 + 断言清单 + 手工勾选」为主，交互细节（IME、焦点归还、Esc 层叠、双击提交、乐观回滚、刷新恢复）散在 spec 的 prose 里，没有唯一 ID、没有「ID → 测试 → 证据」映射。
- agent 提示词（`docs/39`）只含 identity/persona/workspace/tool guidance，完成定义是「类型检查/测试绿」，没有「交互契约 + 真实浏览器点击 + 证据」义务。
- 交互实现不统一：`ui-kit/focus.ts` 手写 focus trap 与 Radix Dialog 自带行为并存；Tooltip 未进 ui-kit（`docs/27` T21），Esc/焦点/提示行为天然分叉。

**不做的后果**：模型继续交「功能对、交互不像人做的」UI；每个新 surface 都靠人工肉眼回归；跨 surface 的交互行为无法比较、无法回归、无法评审。

## 2. 目标

- **G1 交互契约**：新增 `specs/templates/interaction.md` 与 `specs/changes/<id>/interaction.md` 约定；每个 surface 用唯一交互 ID（如 `CMP-IME-01`）描述状态 × 事件 × 焦点 × 键盘 × 失败恢复，P0/P1 必须映射自动化测试与证据。
- **G2 基线契约**：为已实现的 5 个 surface（composer / settings dialog / inspector / command palette / sessions）写出可执行的基线 `interaction.md`（本 spec 目录内），作为后续 surface 的样板。
- **G3 真人点击 harness**：新增 `playwright.interaction.config.ts` + `e2e/interaction/*.spec.ts`，在真实 `serve` 载体（真实 SQLite + fake upstream）上用角色定位完成点击/输入/键盘/拖拽/文件选择/刷新旅程；带 `@interaction:<ID>` 标签。
- **G4 回归基线**：关键 surface 加 `toMatchAriaSnapshot`（结构/角色/状态）与最小 `toHaveScreenshot`（视觉）；axe 0 serious/critical。
- **G5 门禁**：新增 `scripts/ui-interaction-gate.py`：契约 ID ↔ 测试标签 ↔ 证据文件覆盖率、反模式（`waitForTimeout`/`force: true`/无 issue 的 skip）、active/planned 状态过滤；CI 新 job。
- **G6 一致性规则**：`docs/27` §2.5 定义唯一交互模式清单（Dialog/Menu/Tooltip/Toast/Empty/Error/InlineEdit/Confirm/Loading）与 token-only/六态/焦点/文案规则；`docs/19` §11 定义交互契约与 ID 强制规则；`docs/11` §8.6 定义证据口径。
- **G7 Skill 与提示词**：`.agents/skills/zeokeel-ui-interaction/SKILL.md`；`agent-rig::default_preamble` 增加 UI 交互 DoD/证据/禁止捷径段，同步 `docs/39`。
- **G8 在途 spec 接线**：给 `client-composer-surface`、`client-side-panels` 补 `interaction.md`（`planned`）并更新 acceptance/tasks，使实现者按同一契约施工。

## 3. 非目标

- 不实现 composer `/`、`@`、附件、模型标签等 Phase 2a 功能；不实现右栏文件树/预览/终端/会话操作等 Phase 2b 功能（由各自 spec 负责）。
- 不引入 Storybook、Playwright MCP/CLI、SaaS 视觉服务作为门禁；MCP/CLI 仅作为未来探索工具，不写进 CI。
- 不覆盖企业版登录/网关/商店/管理台 surface 的交互基线（后续按同一模板补）。
- 不做全量六态 e2e（离线/流式中/已取消仍由 L1 单测 + 现有 e2e 覆盖）；本变更只固化「可执行、可回归」的最小闭环。
- 不改产品运行期行为/契约/数据库；不改 `@generated`；不新增 capability。
- 不把视觉基线当唯一判据：视觉差异需人工确认后才更新；aria/行为断言优先。

## 4. 用户与场景

| 用户 | 场景 | 期望 |
| --- | --- | --- |
| 实现 agent | 接到任何 UI 任务 | 先产出 interaction.md，按 ID 先写测试再实现；失败时读 trace/截图/aria diff |
| 评审者 | 评审 UI diff | 只看「契约 + ID → 测试 → 证据」即可判断覆盖，不靠肉眼 |
| 个人用户 | 输入中文、按 Enter、切面板、开设置 | 不误发送、不丢焦点、Esc 行为一致、刷新后状态还在 |
| 维护者 | 改一个按钮/菜单 | 交互回归自动报警，不需要重新手工点一遍 |
| 企业用户 | 同一界面（无本地 provider 差异） | 交互行为一致；无新增权限/管控语义 |

## 5. 成功指标

- `pnpm e2e:ui` 在真实 `serve` 上全绿；5 个 surface 共 19 个交互 ID（P0 = 11）全部有标签与证据。
- `python3 scripts/ui-interaction-gate.py` 0 errors；缺失 ID/标签/证据/反模式均 exit 1。
- 文档门禁 `docs-check`/`spec-check`/`review-check`/`docs-command-check` 全绿。
- e2e 中无 `waitForTimeout`、无 `force: true`、无无 issue 的 `test.skip`；关键断言用 role/label/aria。
- 在途两份 spec 均有 `interaction.md`（planned），acceptance 有 ID 映射表。
- 新增 CI `ui-interaction` job（本地已绿；hosted-runner 首跑记为 UNVERIFIED）。

## 6. 风险与未知

| 风险/未知 | 影响 | 缓解/验证 |
| --- | --- | --- |
| Playwright 视觉基线在 CI 与本机渲染差异（字体/抗锯齿） | 高（flake） | 只在稳定小区域截图、禁用动画、`maxDiffPixelRatio`；基线更新需人工确认；CI 首跑 UNVERIFIED 如实记录 |
| IME 合成事件在 headless 下只能模拟 | 中（测不到真实输入法） | e2e 用 `compositionstart` + `isComposing` keydown 模拟；L1 单测覆盖状态机；真实输入法仍列 UNVERIFIED |
| `serve` 载体构建慢（web build + cargo build） | 中 | 单独 config/webServer；`pnpm e2e:ui` 只跑 5 个 spec；CI 与现有 e2e job 分开 |
| 多个测试共享同一 serve/SQLite，状态互相影响 | 中 | 每个测试独立新建会话；不保存设置；必要处 `page.reload`；workers=1 |
| 视觉基线首次生成可能掩盖问题 | 中 | 首次生成后人工逐张 review；PR 必须附基线文件；不自动接受 `-u` |
| gate 解析 markdown 表格脆弱 | 低 | 契约使用固定列名；gate 只依赖 ID + 优先级 + 标签；有 self-test |
| 在途 spec 仍 draft，gate 不强制其 ID | 低 | `planned` 状态跳过；实现开始时改 `active` 并补测试 |

## 7. 需要用户拍板

- [x] **D1 范围**：按用户 2026-11-01 指令「要完整落地，不要考虑 token 问题」执行完整闭环（docs/spec/harness/gate/skill/prompt/CI/在途 spec 接线），不再拆分 A/B/C。
- [x] **D2 工具取舍**：不引入 Storybook/SaaS/MCP 门禁；用现有 Playwright + axe + 自建契约/门禁。
- [x] **D3 视觉基线**：只对 composer 做最小截图基线；settings/inspector/commands/sessions 用 aria snapshot + axe 优先，避免不稳定像素。
- [x] **D4 在途 spec**：只补 `interaction.md`（`planned`）与验收/任务接线，不改其实现范围与提案状态。

## 8. 变更预算与拆分

- 预计改动文件：文档 8（`docs/11`、`docs/19`、`docs/27`、`docs/39`、`docs/README`、`AGENTS.md`、`KNOWN-LIMITATIONS.md`、`docs/reviews/...`）；spec 7（本目录 6 + 模板 1）；
  harness 8（`playwright.interaction.config.ts`、`e2e/interaction/{helpers,composer,settings,inspector,commands,sessions}.ts`、快照）；门禁/脚本 2（`scripts/ui-interaction-gate.py`、`scripts/new-change.sh` 复制模板）；
  CI 1（`.github/workflows/ci.yml`）；skill 2（新增 + `zeokeel-change` 交叉引用）；prompt 2（`agent-rig/lib.rs`、`docs/39`）；在途 spec 4（两份 `interaction.md` + 两份 acceptance/tasks 增量）。**合计 30**（不含本 spec 目录与生成物）。
- 不拆 spec 的理由：单一目标「让 UI 交互可验收、可回归、可强制」；文档/harness/gate/prompt 是一个闭环，拆开会产生「有契约没门禁」或「有门禁没契约」的中间态。

## 9. 复用与已有实现（C17）

- 已搜索：`rg -n "playwright|axe|toHaveScreenshot|toMatchAriaSnapshot|interaction" e2e apps packages scripts docs`；`zk find kit.*`（无 UI 交互专用 capability）。
- 可复用：`e2e/serve-m1.sh`（真实 serve + fake upstream + 临时 HOME）、`e2e/m1-a11y.spec.ts` 的 axe/focus 模式、`@axe-core/playwright`、Playwright 1.64.1（`toMatchAriaSnapshot`/`toHaveScreenshot` 已具备）、`ui-kit/focus.ts`、`docs/11` §8 既有证据口径。
- 本次新增可复用能力：无产品 capability；新增的是**测试/门禁基础设施**（`e2e/interaction/`、`scripts/ui-interaction-gate.py`、skill），不登记 `@capability`，不新增运行期包/加载（C2）。
- 为什么不能复用：现有 e2e 不覆盖「ID → 测试 → 证据」映射与视觉/ARIA 基线；`m1-a11y` 只覆盖 a11y 矩阵，不能替代真人旅程与门禁。
