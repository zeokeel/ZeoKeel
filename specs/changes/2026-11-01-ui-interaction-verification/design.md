# 2026-11-01-ui-interaction-verification · 技术设计

> 规范细节链接 `docs/`，不在这里复制第二份规范。组合/manifest 以 `24` 为准，API 以 `04` 为准。

## 1. 影响面

| 类型 | 变更 |
| --- | --- |
| 包 | 无产品包新增/删除；仅 `zk-agent-rig` 提示词常量改动 |
| 契约 | 无（不新增 capability/契约；不改 `@generated`） |
| 路由 | 无 |
| 迁移 | 无 |
| 生成物 | 无；新快照文件不是 `@generated`（人工 review） |
| edition/profile/channel | `personal × default × official`（个人版基线）；交互套件只跑 personal serve 载体 |
| 文档 | `docs/11` §8.6、`docs/19` §11、`docs/27` §2.5/§2.6、`docs/39` §3、`docs/README` 阅读路径、`AGENTS.md` §6/§8、`KNOWN-LIMITATIONS.md` §3.4、`docs/reviews/2026-11-01-ui-interaction-verification.md` |

### 1.5 复用检查（C17，必填）

| 要实现的点 | 现有能力（capability id / 路径） | 结论 | 归属 |
| --- | --- | --- | --- |
| 真实 serve + SQLite + fake upstream | `e2e/serve-m1.sh`、`e2e/m1-serve.spec.ts` | 复用（webServer 直接调用） | 测试基础设施 |
| axe 扫描 | `@axe-core/playwright`、`e2e/m1-a11y.spec.ts` 模式 | 复用（提 helper） | 测试基础设施 |
| 键盘/焦点/IME 断言 | `ui-kit/focus.ts`、`apps/desktop/web/tests/composer-keyboard.test.tsx` | 复用行为；e2e 补真人旅程 | 测试基础设施 |
| 视觉/ARIA 断言 | Playwright 1.64.1 内置 `toMatchAriaSnapshot`/`toHaveScreenshot` | 复用（空实现，无新库） | 测试基础设施 |
| UI 交互模式 | `ui-kit` primitives、`docs/27` §2 | 扩展（文档化唯一模式） | docs |
| 证据口径 | `docs/11` §8、`target/evidence/` | 复用（新增子目录） | docs |

- 搜索证据：`rg -n "toHaveScreenshot|toMatchAriaSnapshot|axe-core|focus trap|interaction" e2e apps packages scripts docs`；`zk find kit.*`（无 UI 交互 capability）。
- 若"新建"：新脚本/新 e2e 目录是唯一实现，不复制既有测试；不新增产品 capability。
- 新增可复用能力：无（测试基础设施不登记 `@capability`）。

## 2. 方案

### 2.1 文件布局

```text
specs/templates/interaction.md                       # 第六件模板（通用）
specs/changes/<id>/interaction.md                    # 每 surface 契约（active/planned）
scripts/ui-interaction-gate.py                       # ID ↔ 标签 ↔ 证据 + 反模式门禁
playwright.interaction.config.ts                     # 独立 config：真实 serve，testDir=e2e/interaction
e2e/interaction/helpers.ts                           # 页面旅程/axe/aria/证据 helper
e2e/interaction/composer.spec.ts                     # CMP-*
e2e/interaction/settings.spec.ts                     # SET-*
e2e/interaction/inspector.spec.ts                    # INS-*
e2e/interaction/commands.spec.ts                     # CMD-*
e2e/interaction/sessions.spec.ts                     # SES-*
e2e/interaction/__screenshots__/...                  # toHaveScreenshot 基线（入库）
.agents/skills/zeokeel-ui-interaction/SKILL.md
```

### 2.2 关键流程

```text
spec + interaction.md (active) ──> e2e spec（带 @interaction:<ID> 标签）
        │                                  │
        └──────── gate.py ─────────────────┘
                 │  ├─ ID 在契约中且 P0/P1 有标签
                 │  ├─ 每个 P0/P1 有证据 JSON（测试末尾写）
                 │  ├─ 标签不引用未定义 ID
                 │  └─ 无反模式（waitForTimeout/force/skip）
                 ▼
        exit 0 → CI 通过；失败 → 列出 ID/文件:行
```

### 2.3 Playwright 配置要点

- `testDir: "e2e/interaction"`；`workers: 1`、`fullyParallel: false`、`retries: 0`。
- `webServer`: `bash e2e/serve-m1.sh`，`url: http://127.0.0.1:8788/healthz`，`reuseExistingServer: false`。
- `use`: `baseURL: http://127.0.0.1:8788`、`locale: "zh-CN"`、`colorScheme: "light"`、`reducedMotion: "reduce"`、`trace: "retain-on-failure"`、`screenshot: "only-on-failure"`。
- `expect.toHaveScreenshot`: `animations: "disabled"`、`caret: "hide"`、`maxDiffPixelRatio: 0.002`。
- `snapshotPathTemplate`: `"{testDir}/__snapshots__/{testFilePath}/{arg}{ext}"`，保证基线集中、可 review。
- 主 `playwright.config.ts` 的 `testIgnore` 增加 `"**/interaction/**"`，避免 `pnpm e2e` 在 mock 组合下误跑。
- `tsconfig.json` include 增加 `playwright.interaction.config.ts`；e2e 代码受 ESLint 严格规则（禁 `any`/`as`/floating promise）。

### 2.4 证据

- 测试末尾调用 `recordInteraction(id)`：写 `target/evidence/2026-11-01-ui-interaction-verification/interaction/<ID>.json`（`{id,title,at,passed}`），覆盖写。
- 只有测试通过才会执行到末尾 → 证据存在 ≈ 该 ID 本次通过。
- Playwright 的 trace/screenshot 留在 `test-results/`；CI 上传 `target/evidence/**`。
- acceptance 用「ID → 测试 → 证据」表，不写手工勾选。

### 2.5 门禁算法（scripts/ui-interaction-gate.py）

1. 扫描 `specs/changes/*/interaction.md`；读同目录 `proposal.md` 状态。
2. `active` 且 spec 状态 ∈ {`in-progress`,`done`} → 强制；`planned` 或 spec `draft`/`approved` → WARN 跳过。
3. 从契约表格行提取 `[A-Z]{2,6}-[A-Z]{2,6}-\d{2}` 与 `P0|P1`。
4. 扫描 `e2e/interaction/**/*.ts` 的 `@interaction:<ID>` 标签与反模式。
5. 检查每个 P0/P1 ID 的 `target/evidence/.../<ID>.json` 存在。
6. 未定义标签、缺失标签、缺失证据、反模式 → 汇总输出 + exit 1；否则打印计数 + exit 0。
7. 附带 `--self-test`：用内置 fixture 验证解析/反模式/覆盖判定。

### 2.6 提示词

`default_preamble()` 追加一段（不引入新文件/依赖）：

```text
Client UI changes: produce an interaction contract first (specs/changes/<id>/interaction.md with
unique IDs), then tests tagged @interaction:<ID>, then implementation. Done means: real-browser
click-through on the serve carrier, passing axe, aria/visual baselines, evidence under
target/evidence/. Reuse ui-kit primitives and tokens; never add a second focus/toast/dialog/menu.
Forbidden: CSS-selector tests, waitForTimeout, force clicks, auto-accepting snapshots, claiming
"manually verified". If a test exposes a design gap, stop and update the contract first.
```

`docs/39` 记录原文、移植位置与规则；不改 DSH 身份段。

### 2.7 文档一致性

- `docs/27` 增加「交互模式清单」：Dialog/Menu/Popover/Tooltip/Toast/Empty/Error/InlineEdit/Confirm/Loading 的唯一实现（ui-kit）与禁止事项。
- `docs/19` §11：`interaction.md` 必填字段、ID 规则、`active/planned`、P0/P1 证据映射；链接 `docs/11` §8.6，不复述 gate 算法。
- `docs/11` §8.6：证据/命令/门禁/反模式；`§8.4` 增加命令；`§9.2` 增加契约检查项。

## 3. 备选与取舍

| 方案 | 优点 | 缺点 | 结论 |
| --- | --- | --- | --- |
| A：Playwright + 自建契约/门禁 | 零新依赖、确定性、可离线、契合 C2/C15 | 需自维护 gate/契约 | **采用** |
| B：Storybook play + Chromatic | 组件级体验好、视觉云成熟 | 新基础设施、SaaS、组件包需大改 | 放弃（可作为后续独立评审） |
| C：Playwright MCP/CLI 自主点击 | 探索快、能发现未知路径 | 不可复现、token 高、不能当门禁 | 仅未来探索工具，不落地 |
| D：只加截图存档 | 成本最低 | 无断言、无回归、仍靠肉眼 | 放弃（现状问题即此） |

## 4. 安全与隐私

- 不新增网络/密钥/权限；测试使用本地 fake upstream、临时 HOME；不提交用户数据。
- 证据/截图仅本地 `target/`（gitignore）；快照不含 user data（空态/结构）。
- 不改变服务端弱管控/客户端权限语义。

## 5. 发布与回滚

- 不涉及发布/灰度/迁移。
- 回滚：删除新增 harness/gate/skill/契约文件，还原 docs/package.json/playwright/tsconfig/ci/agent-rig；无数据影响。
- 快照删除不影响产品构建。

## 6. 测试计划

| 层 | 用例 | 命令 |
| --- | --- | --- |
| T0 组合 | 无 manifest 变更 | `zk validate && zk compose --edition personal --profile default --channel official --check` |
| T1 单包 | `zk-agent-rig` preamble 测试 | `cargo test -p zk-agent-rig` |
| T2/T3 | 5 个交互 spec（真实 serve） | `pnpm e2e:ui` |
| 门禁 | 契约/标签/证据/反模式 + self-test | `python3 scripts/ui-interaction-gate.py --self-test && python3 scripts/ui-interaction-gate.py` |
| 静态 | TS/ESLint/文档 | `pnpm exec tsc -p tsconfig.json --noEmit`、`pnpm -r lint`、`docs-check`/`spec-check`/`review-check`/`docs-command-check`、`i18n-check` |
| CI | 新 job | 本地命令同上；hosted-runner UNVERIFIED |

## 7. 文档更新

- [x] `docs/11` §8.6 + §8.4/§9.2
- [x] `docs/19` §11
- [x] `docs/27` §2.5/§2.6
- [x] `docs/39` §3
- [x] `docs/README` 阅读路径
- [x] `AGENTS.md` §6/§8
- [x] `KNOWN-LIMITATIONS.md` §3.4
- [x] 评审记录 `docs/reviews/2026-11-01-ui-interaction-verification.md`

## 8. 风险

| 风险 | 概率 | 影响 | 缓解 | 验证 |
| --- | --- | --- | --- | --- |
| 视觉基线跨环境差异 | 中 | 高 | 只截稳定区域/禁动画/阈值；CI 首跑 UNVERIFIED | `pnpm e2e:ui` 本地；CI 首跑记录 |
| IME 模拟失真 | 中 | 中 | e2e 模拟 + L1 状态机测试；真实输入法列 UNVERIFIED | 测试 + KNOWN-LIMITATIONS |
| gate 解析脆弱 | 低 | 中 | 固定列名 + self-test + 只依赖 ID/优先级 | `--self-test` |
| 共享 serve 状态干扰 | 中 | 中 | 独立建会话、不保存设置、workers=1 | 本地全绿 |
| 新增 CI job 超时 | 低 | 中 | 只跑 5 spec；timeout 30–45min | CI 首跑 |
