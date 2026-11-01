# 2026-11-01-ui-intuition-heuristics · 任务清单

> 每个任务必须能独立提交、独立验证；没有 `verify` 的任务不算任务。

## 1. 任务表

| # | 任务 | 写作用域（文件/目录） | 依赖 | 验证 | 状态 |
| --- | --- | --- | --- | --- | --- |
| 0 | 文档 SSOT：`docs/27` §2.7（INT-01..12 判定表）/§2.8（认知走查/用户测试）；`docs/19` §11.1（`intuition.md`/`intuition-walkthrough.md` 必填/ID/证据）；`docs/11` §8.6/§9.2（证据/评审）；`docs/README` 阅读路径；`KNOWN-LIMITATIONS` 直觉验证边界 | `docs/27-ui-design-system.md`、`docs/19-client-ux-flows.md`、`docs/11-testing-and-evidence.md`、`docs/README.md`、`KNOWN-LIMITATIONS.md` | — | `python3 scripts/docs-check.py && python3 scripts/docs-command-check.py` | ☑ |
| 1 | 强制可见：`AGENTS.md` 新增 **C20 直觉交互先行** + §8 一条 + §13 工作流步骤；`docs/39` §3.1 记录提示词增量 | `AGENTS.md`、`docs/39-agent-prompts.md` | 0 | `python3 scripts/docs-check.py` | ☑ |
| 2 | 提示词：`agent-rig::default_preamble` 追加“认知走查 + INT + 用户测试”段；单测锁定关键词 | `packages/client/agent-rig/src/lib.rs` | 1 | `cargo test -p zk-agent-rig` | ☑ |
| 3 | 模板/脚本：`specs/templates/intuition.md`、`intuition-walkthrough.md`；`scripts/new-change.sh` 复制两个模板（幂等）；`specs/templates/interaction.md` 指向同目录两个文件 | `specs/templates/intuition.md`、`specs/templates/intuition-walkthrough.md`、`scripts/new-change.sh`、`specs/templates/interaction.md` | — | `bash scripts/new-change.sh ui-intuition-smoke` 生成三文件（随后删除）；`bash -n scripts/new-change.sh` | ☑ |
| 4 | 基线：`specs/changes/2026-11-01-ui-interaction-verification/intuition.md`（12 条 INT，5 surface）+ `intuition-walkthrough.md`（3 persona × 3 top task × 摩擦表） | 两个新文件 | 0,3 | `python3 scripts/ui-interaction-gate.py --strict` | ☑ |
| 5 | 门禁：`scripts/ui-interaction-gate.py` 对 enforced active 契约检查 `intuition.md` 存在/≥3 INT 行 + `intuition-walkthrough.md` 存在；自测 3 类失败；输出 `intuition` 计数 | `scripts/ui-interaction-gate.py` | 4 | `python3 scripts/ui-interaction-gate.py --self-test && python3 scripts/ui-interaction-gate.py --strict` | ☑ |
| 6 | Skill：`.agents/skills/zeokeel-ui-interaction/SKILL.md` 增加 Step 0 认知走查、两个文件、用户测试脚本、评审 prompt；description 含 walkthrough/INT | `.agents/skills/zeokeel-ui-interaction/SKILL.md` | 0,3 | `python3 scripts/docs-check.py`（frontmatter/name/description） | ☑ |
| 7 | 独立评审：fresh-context reviewer 按 3 persona 走查 5 surface，评审 SSOT/模板/gate/prompt，写 `docs/reviews/2026-11-01-ui-intuition-heuristics.md`（SHA256/结论 APPROVED） | `docs/reviews/2026-11-01-ui-intuition-heuristics.md`、本目录 `proposal.md` 状态/评审字段 | 0–6 | `python3 scripts/review-check.py` | ☑ |
| 8 | 验收：跑全部门禁（docs/spec/review/gate/e2e/cargo），填 `acceptance.md`；用户测试标 UNVERIFIED | 本目录 `acceptance.md` | 0–7 | `python3 scripts/spec-check.py && python3 scripts/docs-check.py && python3 scripts/review-check.py` | ☑ |

## 2. 写作用域

- 本变更只写：`docs/27`、`docs/19`、`docs/11`、`docs/README`、`AGENTS.md`、`KNOWN-LIMITATIONS.md`、`docs/39`、`specs/templates/intuition.md`、`specs/templates/intuition-walkthrough.md`、`specs/templates/interaction.md`、`scripts/new-change.sh`、`scripts/ui-interaction-gate.py`、`.agents/skills/zeokeel-ui-interaction/SKILL.md`、`packages/client/agent-rig/src/lib.rs`、`specs/changes/2026-11-01-ui-interaction-verification/{intuition.md,intuition-walkthrough.md}`、`docs/reviews/2026-11-01-ui-intuition-heuristics.md`、本 spec 目录。
- 明确不写：产品 UI/契约/生成物/迁移；Phase 2 spec 实现范围；既有 `interaction.md`（基线契约不动，避免破坏已评审 hash）；不新增 `@capability`；不引入新依赖。
- 并行：0/1/3/6 与 2/4/5 可并行；`AGENTS.md`/`docs/39`/`agent-rig` 串行（prompt owner）；`scripts/ui-interaction-gate.py` 由 harness owner 独占；`docs/27/19/11` 由 docs owner 独占。

## 3. 提交顺序

1. 任务 0（SSOT）→ 3（模板/脚本）→ 4（基线）→ 5（门禁）
2. 任务 1（AGENTS/docs39）→ 2（prompt）
3. 任务 6（skill）
4. 任务 7（review）→ 8（acceptance）

## 4. 验收前检查

- [ ] `python3 scripts/spec-check.py`
- [ ] `python3 scripts/docs-check.py`；`python3 scripts/docs-command-check.py --self-test && python3 scripts/docs-command-check.py`
- [ ] `python3 scripts/review-check.py`
- [ ] `python3 scripts/ui-interaction-gate.py --self-test && python3 scripts/ui-interaction-gate.py --strict`
- [ ] `pnpm e2e:ui`（19 passed，不回归）
- [ ] `cargo test -p zk-agent-rig`
- [ ] 基线两个文件存在；`new-change.sh` 生成三文件
- [ ] 无手改 `@generated`；无新增依赖/运行期加载（C2）
