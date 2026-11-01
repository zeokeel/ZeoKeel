# 2026-11-01-ui-interaction-verification · 验收与证据

- 验收人：lead（实现者）+ `reviewer`（独立评审，P0=0/P1=0/P2=10）
- 日期：2026-11-01
- PR/commit：工作树（branch `release/1.0.0-rc.1`，基线 commit `551468924a487534d537a4a90b818265e5113ed8` + 本变更未提交改动）
- 证据根：`target/evidence/2026-11-01-ui-interaction-verification/`
- 评审：`docs/reviews/2026-11-01-ui-interaction-verification.md#结论`（**APPROVED**）

> 状态：本地证据齐全（`in-progress`）；hosted-runner CI 首跑、跨平台视觉、归档为 **UNVERIFIED/待执行**，不得据此声称 1.0.0 已发布或 CI 已绿。

## 1. 验收命令与结果

| 命令 | 结果 | 证据路径 |
| --- | --- | --- |
| `pnpm e2e:ui` | **exit 0**，19 passed（5 files，44.3s） | `logs/pnpm-e2e-ui.log`、`interaction/*.json`（19） |
| `pnpm e2e`（主套件，真实 serve/security/a11y/mcp） | **exit 0**，17 passed（1.0m）；不重复跑 interaction | `logs/pnpm-e2e-main.log` |
| `python3 scripts/ui-interaction-gate.py --self-test` | **exit 0**（9 checks） | `logs/gate-self-test.log` |
| `python3 scripts/ui-interaction-gate.py`（proposal=in-progress，强制 active） | **exit 0**：contracts=4（enforced=1），tests=6，tags=19，ids=54，enforced P0/P1=19；3 份 planned 仅 WARN | `logs/ui-interaction-gate.log` |
| `pnpm exec tsc -p tsconfig.json --noEmit` | **exit 0** | `logs/tsc.log` |
| `pnpm exec eslint e2e/interaction playwright.interaction.config.ts playwright.config.ts --max-warnings 0` | **exit 0**（根 e2e 手动接线；`pnpm -r lint` 不含根 e2e） | `logs/eslint-e2e.log` |
| `pnpm -r lint` | **exit 0** | `logs/pnpm-r-lint.log` |
| `bash scripts/i18n-check.sh` | **exit 0**（`i18n-check ok`） | `logs/i18n-check.log` |
| `python3 scripts/docs-check.py` | **exit 0**（0 errors / 0 warnings / 41 files） | `logs/docs-check.log` |
| `python3 scripts/spec-check.py` | **exit 0**（0 errors / 1 warning：本 spec `in-progress` 尚未置 `approved` 的提示） | `logs/spec-check.log` |
| `python3 scripts/review-check.py` | **exit 0**（31 records，含本记录） | `logs/review-check.log` |
| `python3 scripts/docs-command-check.py --self-test && python3 scripts/docs-command-check.py` | **exit 0 / exit 0** | `logs/docs-command-check.log` |
| `cargo test -p zk-agent-rig` | **exit 0**，53 passed（含 `default_preamble_mentions_ui_interaction`） | `logs/cargo-test-agent-rig.log` |
| `pnpm exec playwright test -c playwright.config.ts --list` | **exit 0**：46 tests / 19 files，**0** 个 interaction 匹配 | `logs/playwright-list-main.log` |
| `pnpm exec playwright test -c playwright.interaction.config.ts --list` | **exit 0**：19 tests / 5 files | `logs/playwright-list-ui.log` |
| CI YAML 解析 | **exit 0**；仅新增 `ui-interaction` job，既有 job 未改 | `logs/ci-yaml.log` |
| CI `ui-interaction` job（hosted runner） | **UNVERIFIED**（未在本次本地执行；首跑后回填） | CI run URL 待填 |

## 2. 交互验收（ID → 测试 → 证据）

> 契约：本目录 `interaction.md`（19 ID = P0 11 + P1 8，`active`）。全部通过；证据位于 `target/evidence/2026-11-01-ui-interaction-verification/interaction/<ID>.json`。

| ID | 优先级 | 测试 | 证据 | 结果 |
| --- | --- | --- | --- | --- |
| CMP-FOCUS-01 | P0 | `e2e/interaction/composer.spec.ts` | `interaction/CMP-FOCUS-01.json` | 通过 |
| CMP-KEY-01 | P0 | 同上 | `interaction/CMP-KEY-01.json` | 通过 |
| CMP-IME-01 | P0 | 同上 | `interaction/CMP-IME-01.json` | 通过 |
| CMP-PERSIST-01 | P0 | 同上 | `interaction/CMP-PERSIST-01.json` | 通过 |
| CMP-A11Y-01 | P1 | 同上 | `interaction/CMP-A11Y-01.json` | 通过 |
| CMP-VIS-01 | P1 | 同上（`__screenshots__/composer.spec.ts/composer-empty.png`） | `interaction/CMP-VIS-01.json` | 通过 |
| SET-OPEN-01 | P0 | `e2e/interaction/settings.spec.ts` | `interaction/SET-OPEN-01.json` | 通过 |
| SET-ESC-01 | P0 | 同上 | `interaction/SET-ESC-01.json` | 通过 |
| SET-ARIA-01 | P1 | 同上 | `interaction/SET-ARIA-01.json` | 通过 |
| SET-A11Y-01 | P1 | 同上 | `interaction/SET-A11Y-01.json` | 通过 |
| INS-TOGGLE-01 | P0 | `e2e/interaction/inspector.spec.ts` | `interaction/INS-TOGGLE-01.json` | 通过 |
| INS-ARIA-01 | P1 | 同上 | `interaction/INS-ARIA-01.json` | 通过 |
| INS-A11Y-01 | P1 | 同上 | `interaction/INS-A11Y-01.json` | 通过 |
| CMD-OPEN-01 | P0 | `e2e/interaction/commands.spec.ts` | `interaction/CMD-OPEN-01.json` | 通过 |
| CMD-NAV-01 | P0 | 同上 | `interaction/CMD-NAV-01.json` | 通过 |
| CMD-ESC-01 | P0 | 同上 | `interaction/CMD-ESC-01.json` | 通过 |
| CMD-A11Y-01 | P1 | 同上 | `interaction/CMD-A11Y-01.json` | 通过 |
| SES-NEW-01 | P0 | `e2e/interaction/sessions.spec.ts` | `interaction/SES-NEW-01.json` | 通过 |
| SES-A11Y-01 | P1 | 同上 | `interaction/SES-A11Y-01.json` | 通过 |

## 3. 文档与决策

- [x] `docs/11` §8.4/§8.6/§9.2（交互契约/证据/门禁口径；命令）
- [x] `docs/19` §11（契约字段/ID 规则/active-planned/P0-P1 映射；基线 5 surface）
- [x] `docs/27` §2.5/§2.6（唯一交互模式清单/一致性规则/视觉与 ARIA 基线）
- [x] `docs/39` §3（提示词原文/落位/规则）+ `agent-rig::default_preamble`（单测锁定）
- [x] `AGENTS.md` §6.1/§8（`pnpm e2e:ui`、gate、UI 交互证据义务）
- [x] `KNOWN-LIMITATIONS.md` §3.4（本地通过/CI 首跑 UNVERIFIED/IME 模拟/未采用 MCP·Storybook）
- [x] `docs/README.md` 阅读路径（要改客户端交互 → 19 §11）
- [x] `specs/README.md` + `specs/templates/interaction.md` + `scripts/new-change.sh`（第六件模板）
- [x] 评审记录 **APPROVED** 且 proposal 已链接；在途 spec（composer/side/activity）补 `planned` 契约
- [x] 无手改 `@generated`；无新增 capability；无运行期插件加载（C2）

## 4. 回滚

- 回滚步骤：删除新增 `playwright.interaction.config.ts`、`e2e/interaction/**`、`scripts/ui-interaction-gate.py`、`.agents/skills/zeokeel-ui-interaction/`、`specs/templates/interaction.md`、本 spec 目录与 activity 的 `interaction.md`；还原修改的 docs/AGENTS/KNOWN/package.json/playwright.config/tsconfig/ci.yml/new-change.sh/agent-rig/在途 spec；`git checkout -- <modified>`。
- 数据影响：无数据库/契约/运行期数据；快照与证据仅为测试产物（证据在 `target/`，不入库）。
- 已验证回滚：**否**（未执行完整回滚演练；改动均为新增文件 + 可还原修改，无迁移/无状态变更）。产品构建不依赖新增文件。

## 5. 剩余风险 / 后续

| 项 | 影响 | 后续 |
| --- | --- | --- |
| hosted-runner `ui-interaction` job 首跑 UNVERIFIED | 中 | CI 首跑后回填结果；若字体/AA 导致视觉基线差异，调整阈值/容器或把视觉降为 advisory（需评审） |
| 视觉基线仅 Linux/Chromium 本机生成 | 中 | 跨平台渲染/真机 UI 未验证；`KNOWN-LIMITATIONS` §3.4 已登记 |
| IME 为合成事件模拟（真实中文输入法未跑） | 中 | 真机输入法手测记录；后续可加 L1 状态机测试 |
| planned 契约（composer 15 ID / side 14 ID / activity 5 ID）未执行 | 中 | 各自 spec 实现时改 `active` 并补测试/证据 |
| 企业版/管理台 surface 未覆盖 | 中 | 后续按同一模板补 `interaction.md` |
| 评审 P2-01…P2-10（非阻塞） | 低 | 见评审记录 §3：docs 措辞/模板路径/契约预算字段/证据新鲜度/根 e2e lint 接线等；后续小步或独立 spec |
| `spec-check` 对本 spec 报 1 warning（`in-progress` 尚未 `approved`） | 低 | 归档/收尾时按 C16 处理；当前 0 errors |
| 既有 `ui-sessions` 手写 focus trap（P2-02） | 低 | 既有债；后续 T21 迁移 `ui-kit` 唯一实现 |

## 6. 结论

- [x] `done`（本地证据齐全，可合并；hosted-runner CI 首跑与归档 UNVERIFIED，见 §5）
- [ ] `abandoned`：原因：待填

## 7. 复用证据（C17）

- 搜索记录：`rg -n "toHaveScreenshot|toMatchAriaSnapshot|axe-core|focus trap|interaction" e2e apps packages scripts docs`；`zk find kit.*`（无 UI 交互 capability）。
- 复用清单：`e2e/serve-m1.sh`（真实 serve + fake upstream）、`e2e/m1-a11y.spec.ts` 的 axe/焦点模式、`@axe-core/playwright`、Playwright 1.64.1 `toMatchAriaSnapshot`/`toHaveScreenshot`、`ui-kit/focus.ts`、`docs/11` §8 证据口径。
- 新增能力登记：无（测试/门禁基础设施，不新增 `@capability`；无第二套产品实现）。
