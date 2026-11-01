# 2026-11-01-ui-intuition-heuristics · 提案

- 状态：`in-progress`
- Owner：lead
- 日期：2026-11-01
- 关联：用户 2026-11-01 要求「落地规范，让 AI 每次写新功能都能看到、知道怎么做、做好约束」；前置 `2026-11-01-ui-interaction-verification`（行为/证据闭环）；本变更落定 `docs/27` §2.7/§2.8、`docs/19` §11.1、`docs/11` §8.6、`AGENTS.md` C20、`docs/39` §3.1、skill 扩展
- 评审：`docs/reviews/2026-11-01-ui-intuition-heuristics.md#结论`（APPROVED，P0=0/P1=0）
- 变更预算：**18 个文件**（文档 7、spec 7、模板 2、门禁/脚本 2；不含本 spec 目录与生成物）

## 1. 背景 / 问题

上一轮已把「行为正确、可回归」做成闭环（`interaction.md` + `pnpm e2e:ui` + gate + 证据）。但**行为对 ≠ 符合真人直觉**：

- e2e 能证伪焦点、Esc、IME、双击提交、数据恢复；不能判断“用户是否知道下一步”“图标是否一眼看懂”“三个选项是否让人犹豫”。
- 当前 `docs/27` §2.5/§2.6 只覆盖一致性/视觉/ARIA；`docs/19` §11 只覆盖交互 ID；没有任何 SSOT 定义“直觉”的判据、走查方法、用户测试与证据。
- AI 每次写新功能时，提示词里没有“先做认知走查”的约束；模型默认直接写实现，产出的 UI“能用但不像人设计的”。
- 不做的后果：每个新 surface 继续靠个人感觉；交互行为测试全绿但用户第一次用就卡住；无法评审、无法复用、无法回归。

## 2. 目标

- **G1 SSOT**：`docs/27` §2.7「直觉交互判定（INT-*）」+ §2.8「认知走查与用户测试」；`docs/19` §11.1 定义契约必填的 `intuition.md` / `intuition-walkthrough.md`；`docs/11` §8.6 定义证据与门禁口径。
- **G2 强制可见**：`AGENTS.md` 新增 **C20 直觉交互先行** + §8 + §13 工作流步骤；`agent-rig::default_preamble` 与 `docs/39` 增加“先认知走查、无走查不得实现/不得声称直觉”的提示；skill 描述与步骤覆盖走查。
- **G3 模板**：`specs/templates/intuition.md`（INT 表）与 `intuition-walkthrough.md`（3 persona × top task × 4 问）；`scripts/new-change.sh` 自动复制。
- **G4 证据**：每个 active UI 变更目录必须有 `intuition.md`（≥3 条 INT，覆盖 P0 旅程）与 `intuition-walkthrough.md`（3 persona/3 top task/摩擦表）；用户测试（≥5 人）在发布前提供证据，未跑则标 UNVERIFIED。
- **G5 门禁**：`scripts/ui-interaction-gate.py` 对 active 契约检查两个文件存在、INT 行数、走查存在；INT 检查**不伪装成 e2e 标签门禁**（直觉质量由 reviewer 判定）。
- **G6 基线**：为 5 个已实现 surface 写 `intuition.md` + `intuition-walkthrough.md`，作为样板。
- **G7 独立评审**：C19 评审记录 APPROVED；P0/P1 修复后复评。

## 3. 非目标

- 不实现任何新 UI 功能；不改产品运行期行为/契约/数据库/生成物。
- 不把“直觉”做成可全自动化的 CI 断言；e2e 只做可观察代理（焦点/Esc/确认/撤销/busy/文案键/空态）。
- 不做真实用户招募/测试执行（本次只提供脚本/模板；用户测试结果标 UNVERIFIED，后续执行回填）。
- 不替代 WCAG/平台 HIG；不新增设计系统组件、不重写 `ui-kit`。
- 不改既有 Phase 2 spec 的实现范围/提案状态；只加 `intuition.md`/`intuition-walkthrough.md`（planned 时可选，active 时必填）。

## 4. 用户与场景

| 用户 | 场景 | 期望 |
| --- | --- | --- |
| AI coding agent | 接到任何新 UI 功能 | 在 AGENTS/skill/prompt 中看到“先走查再契约再代码”；知道 INT 表、走查文件、证据路径 |
| 实现者 | 写 composer/设置/右栏等新交互 | 有模板可填；P0 摩擦未解决就不进入实现 |
| 评审者 | 评审 UI diff | 用 3 persona 走查 + INT 表 + 用户测试证据判断，而不是只看“测试绿” |
| 个人/企业用户 | 第一次用/高频用/出错/用键盘 | 不需要猜、不会卡死、出错能恢复；体验符合平台与产品惯例 |

## 5. 成功指标

- `AGENTS.md` C20、§8、§13 均出现“认知走查/INT/用户测试”约束；`docs/39` 与 preamble 同步。
- 5 个 baseline surface 有 `intuition.md`（12 条 INT）与 `intuition-walkthrough.md`（3 persona × 3 top task）。
- gate `--strict` 对 active 契约检查两个文件存在 + INT 行数 ≥3；缺文件/缺行 exit 1；`--self-test` 覆盖。
- 新 spec 的 `tasks.md`/`acceptance.md` 有 INT 证据步骤；`docs-check`/`spec-check`/`review-check` 绿。
- 评审 APPROVED；P0/P1=0；用户测试脚本可执行（实际执行 UNVERIFIED）。

## 6. 风险与未知

| 风险/未知 | 影响 | 缓解/验证 |
| --- | --- | --- |
| “直觉”主观，INT 判定可能流于形式 | 中 | 每条 INT 有反例 + 验证方式；P0 必须 reviewer 走查；用户测试必须真人 |
| 门禁只能查存在性，不能查质量 | 中 | 明确“存在性 gate + reviewer 质量判定 + user test 证据”；不冒充自动化 |
| 用户测试未执行 | 中 | 模板/脚本落地；acceptance 标 UNVERIFIED；发布前必须跑 ≥5 人 |
| 新增 C20 与现有 C1–C19 编号体系 | 低 | 追加编号；仅 AGENTS/docs 引用，不重排旧编号 |
| 在途 spec 未填 INT 文件 | 中 | planned 只 WARN；实现开始时改 active 并补 |
| 与 Phase 2 提交/评审并发 | 中 | 只改 docs/AGENTS/skill/prompt/templates/gate + 新文件，不碰 Phase 2 产品文件 |

## 7. 需要用户拍板

- [x] **D1 范围**：按用户 2026-11-01「可以，落地规范…做好约束」执行：SSOT + C20 + skill + prompt + 模板 + gate + 基线 + 独立评审；不做产品功能、不做真实用户测试。
- [x] **D2 不自动化直觉**：e2e 只做代理；INT 质量由 reviewer 走查 + 用户测试判断。
- [x] **D3 文件位置**：`intuition.md`/`intuition-walkthrough.md` 与 `interaction.md` 同目录（第七/第八件）；INT ID 前缀 `INT-`。
- [x] **D4 阈值**：active 契约至少 3 条 INT；P0 旅程每条至少 1 条 INT；3 persona × 3 top task；用户测试 ≥5 人（发布前）。

## 8. 变更预算与拆分

- 文档 7：`docs/27`、`docs/19`、`docs/11`、`docs/README`、`AGENTS.md`、`KNOWN-LIMITATIONS.md`、`docs/39`。
- spec 7：本目录 6 + 基线 `intuition.md`/`intuition-walkthrough.md`（2，计入 spec 目录外的前置 spec 目录）。
- 模板 2：`specs/templates/intuition.md`、`intuition-walkthrough.md`；`scripts/new-change.sh` 1。
- 门禁/脚本 2：`scripts/ui-interaction-gate.py`、self-test。
- prompt 1：`packages/client/agent-rig/src/lib.rs`；skill 1：`.agents/skills/zeokeel-ui-interaction/SKILL.md`。
- 合计 **18**（不含本 spec 目录与被改动文档的上下文行）。

## 9. 复用与已有实现（C17）

- 已搜索：`rg -n "intuition|INT-|cognitive|walkthrough|usability|heuristic" docs specs AGENTS.md scripts`；当前 0 命中。
- 可复用：`docs/11` §8.6 证据口径、`interaction.md`/gate 基础设施、`docs/27` §2.5 唯一交互模式、`docs/19` §11 ID 规则、`specs/templates/interaction.md`、`scripts/new-change.sh`。
- 新增可复用能力：SSOT 判定表（docs/27 §2.7）+ 两个模板 + gate 检查；不是产品 capability，不登记 `@capability`。
- 为什么不能复用：现有交互闭环只覆盖“行为可执行”，没有“直觉/走查/用户测试”的规范与模板；必须在同一闭环上补一层，不能另建第二套流程。
