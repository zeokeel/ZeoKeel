# 2026-11-01-ui-interaction-verification · 任务清单

> 每个任务必须能独立提交、独立验证；没有 `verify` 的任务不算任务。
> 状态：`draft`（用户 2026-11-01 已要求完整落地；独立评审通过后改 `in-progress`）。

## 0. 写作用域总表

- 本变更只写：`specs/changes/2026-11-01-ui-interaction-verification/**`、`specs/templates/interaction.md`、
  `docs/11-testing-and-evidence.md`、`docs/19-client-ux-flows.md`、`docs/27-ui-design-system.md`、`docs/39-agent-prompts.md`、
  `docs/README.md`、`AGENTS.md`、`KNOWN-LIMITATIONS.md`、`docs/reviews/2026-11-01-ui-interaction-verification.md`、
  `scripts/new-change.sh`、`scripts/ui-interaction-gate.py`、`playwright.interaction.config.ts`、`playwright.config.ts`、
  `tsconfig.json`、`e2e/interaction/**`、`package.json`、`.github/workflows/ci.yml`、
  `.agents/skills/zeokeel-ui-interaction/SKILL.md`、`.agents/skills/zeokeel-change/SKILL.md`、
  `packages/client/agent-rig/src/lib.rs`、
  `specs/changes/2026-11-01-client-composer-surface/{interaction.md,acceptance.md,tasks.md}`、
  `specs/changes/2026-11-01-client-side-panels/{interaction.md,acceptance.md,tasks.md}`。
- 明确不写：产品契约/生成物/迁移/服务端/管理台/企业 UI/`@generated`；不新建运行期包；不改既有 e2e 的断言（只加 ignore/配置）。
- 并行：docs（0/1/7/8）与 harness（2/3/4/5/6）可并行；`package.json`/`playwright.config.ts`/`tsconfig.json` 由 harness owner 独占；
  `docs/39`+`agent-rig` 由 prompt owner 独占；`docs/11/19/27/README/AGENTS/KNOWN` 由 docs owner 独占。

## 1. 任务表

| # | 任务 | 写作用域（文件/目录） | 依赖 | 验证 | 状态 |
| --- | --- | --- | --- | --- | --- |
| 0 | 文档 SSOT：`docs/11` §8.6 + §8.4/§9.2；`docs/19` §11；`docs/27` §2.5/§2.6；`docs/README` 阅读路径；`AGENTS.md` §6/§8；`KNOWN-LIMITATIONS` §3.4 | `docs/11-testing-and-evidence.md`、`docs/19-client-ux-flows.md`、`docs/27-ui-design-system.md`、`docs/README.md`、`AGENTS.md`、`KNOWN-LIMITATIONS.md` | — | `python3 scripts/docs-check.py && python3 scripts/docs-command-check.py --self-test && python3 scripts/docs-command-check.py` | ☐ |
| 1 | 契约模板 + 基线契约：`specs/templates/interaction.md`；本目录 `interaction.md`（5 surface / 19 ID / P0 = 11）；`scripts/new-change.sh` 复制可选模板 | `specs/templates/interaction.md`、`specs/changes/2026-11-01-ui-interaction-verification/interaction.md`、`scripts/new-change.sh` | — | `bash scripts/new-change.sh ui-interaction-smoke && ls specs/changes/*-ui-interaction-smoke/interaction.md`（随后删除临时目录，不提交） | ☐ |
| 2 | harness 配置：`playwright.interaction.config.ts`；主 `playwright.config.ts` ignore；`tsconfig.json` include；`package.json` scripts | `playwright.interaction.config.ts`、`playwright.config.ts`、`tsconfig.json`、`package.json` | 1 | `pnpm exec tsc -p tsconfig.json --noEmit`；`pnpm exec playwright test -c playwright.interaction.config.ts --list` | ☐ |
| 3 | e2e 真人点击：`e2e/interaction/{helpers,composer,settings,inspector,commands,sessions}.ts` + 快照基线 | `e2e/interaction/**` | 1,2 | `pnpm e2e:ui` exit 0；5 spec；19 ID 证据 JSON | ☐ |
| 4 | 门禁脚本：`scripts/ui-interaction-gate.py`（解析 active/planned、标签、证据、反模式、self-test） | `scripts/ui-interaction-gate.py` | 1,3 | `python3 scripts/ui-interaction-gate.py --self-test && python3 scripts/ui-interaction-gate.py` exit 0 | ☐ |
| 5 | CI：`.github/workflows/ci.yml` 新增 `ui-interaction` job（install + chromium + `pnpm e2e:ui` + gate + artifact） | `.github/workflows/ci.yml` | 2,3,4 | `python3 -c "import yaml,pathlib;yaml.safe_load(pathlib.Path('.github/workflows/ci.yml').read_text())"`（或 `ruby -ryaml` 等价） | ☐ |
| 6 | Skill：`.agents/skills/zeokeel-ui-interaction/SKILL.md`；`zeokeel-change` 增加 UI 交叉引用 | `.agents/skills/zeokeel-ui-interaction/SKILL.md`、`.agents/skills/zeokeel-change/SKILL.md` | 1,3,4 | `python3 scripts/docs-check.py`（frontmatter/name 校验） | ☐ |
| 7 | 提示词：`agent-rig::default_preamble` 增加 UI 段（完成定义/不变量/禁止/循环/停止）；`docs/39` §3 记录；`cargo test` 覆盖 | `packages/client/agent-rig/src/lib.rs`、`docs/39-agent-prompts.md` | — | `cargo test -p zk-agent-rig`；`rg -n "interaction contract" packages/client/agent-rig/src/lib.rs docs/39-agent-prompts.md` | ☐ |
| 8 | 在途 spec 接线：composer Phase 2a + side-panels Phase 2b 各加 `interaction.md`（planned），acceptance 加 ID 映射表，tasks 加门禁命令 | `specs/changes/2026-11-01-client-composer-surface/{interaction.md,acceptance.md,tasks.md}`、`specs/changes/2026-11-01-client-side-panels/{interaction.md,acceptance.md,tasks.md}` | 0,1 | `python3 scripts/spec-check.py`；`python3 scripts/ui-interaction-gate.py`（planned 只 WARN） | ☐ |
| 9 | 独立评审：fresh-context reviewer 写 `docs/reviews/2026-11-01-ui-interaction-verification.md`（评审对象 SHA256、独立性、P0/P1/P2、结论 APPROVED） | `docs/reviews/2026-11-01-ui-interaction-verification.md`、本目录 `proposal.md`（状态/评审字段） | 0–8 | `python3 scripts/review-check.py` exit 0 | ☐ |
| 10 | 验收与证据：跑全部门禁，填 `acceptance.md`（命令/结果/证据/回滚/残余）；CI 首跑标 UNVERIFIED | 本目录 `acceptance.md` | 0–9 | `python3 scripts/spec-check.py && python3 scripts/docs-check.py && python3 scripts/review-check.py` | ☐ |

## 2. 写作用域

- 见 §0；并行/重叠已列。`docs/11/19/27/README/AGENTS/KNOWN` 只允许 docs owner 写；`e2e/interaction/**`、`playwright*.ts`、`tsconfig.json`、`package.json`、`scripts/ui-interaction-gate.py`、`.github/workflows/ci.yml` 只允许 harness owner 写；`agent-rig`/`docs/39` 只允许 prompt owner 写；`docs/reviews/...` 只允许 reviewer 写。
- 本 spec 目录（除 `interaction.md` 外）由 Lead 写；`interaction.md` 由 Lead 写、docs owner 只读引用。

## 3. 提交顺序

1. 1（模板 + 基线契约）→ 2（config）→ 3（e2e）→ 4（gate）→ 5（CI）
2. 0（docs）与 7（prompt）可任意并行
3. 6（skill）依赖 1/3/4
4. 8（在途 spec）依赖 0/1
5. 9（review）依赖 0–8
6. 10（acceptance）依赖 0–9

## 4. 验收前检查

- [ ] `python3 scripts/spec-check.py`（draft warning 可接受）
- [ ] `python3 scripts/docs-check.py`；`python3 scripts/docs-command-check.py --self-test && python3 scripts/docs-command-check.py`
- [ ] `python3 scripts/review-check.py`
- [ ] `python3 scripts/ui-interaction-gate.py --self-test && python3 scripts/ui-interaction-gate.py`
- [ ] `pnpm e2e:ui`；`pnpm exec tsc -p tsconfig.json --noEmit`；`pnpm -r lint`；`bash scripts/i18n-check.sh`
- [ ] `cargo test -p zk-agent-rig`
- [ ] 主 `pnpm e2e` 不重复跑 interaction（testIgnore）
- [ ] 无手改 `@generated`；无新增 capability；无运行期插件加载（C2）
- [ ] 在途两份 spec 有 planned 契约与 ID 表
- [ ] `acceptance.md` 已填证据路径；CI 首次执行标 UNVERIFIED
