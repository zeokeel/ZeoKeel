# 2026-11-01-ui-intuition-heuristics · 行为规格

> 只写"系统必须怎样"，不写实现选型；实现见 `design.md`。长期规范见 `docs/27` §2.7/§2.8、`docs/19` §11.1、`docs/11` §8.6。

## 1. 需求

### R1 · 直觉判定 SSOT

- **WHEN** 任何人/AI 需要判断一个客户端交互是否符合真人直觉
- **THEN** 唯一权威是 `docs/27` §2.7「直觉交互判定（INT-*）」：≥12 条原则，每条含 `ID / 原则 / 判据 / 反例 / 验证方式（e2e proxy / review / user test）`
- **AND** `docs/27` §2.8 定义认知走查（3 persona × top task × 4 问）与用户测试（≥5 人、think-aloud、指标/证据），不得在其它文档复制第二份
- **AND** 所有 INT ID 以 `INT-` 开头、全仓唯一；判定必须引用 INT ID

### R2 · 契约必填的直觉文件

- **WHEN** 任何客户端 surface 的交互变更进入 `active`（实现/验收）
- **THEN** 变更目录必须同时有：
  - `intuition.md`：≥3 条 INT 行，覆盖该 surface 的 P0 旅程（每个 P0 交互 ID 至少映射 1 条 INT）；每行含场景、用户直觉期望、验证方式、证据；
  - `intuition-walkthrough.md`：3 类 persona（首次/高频/键盘+读屏或出错）× 3 个 top task，逐步回答 4 问（知道目标？看得到控件？能预判反馈？能恢复？），并给出摩擦表（INT ID、严重度 P0/P1/P2、修法）。
- **AND** `planned` 契约可选，实现开始时必须补齐并把状态改 `active`；否则不进入实现/验收
- **AND** 两个文件与 `interaction.md` 同目录；模板为 `specs/templates/intuition.md`、`specs/templates/intuition-walkthrough.md`；`scripts/new-change.sh` 自动复制

### R3 · 强制可见（AI 每次写新功能都能看到）

- **WHEN** 任意 agent 开始一轮任务
- **THEN** `AGENTS.md` 最高约束表含 **C20 直觉交互先行**；§8 测试说明与 §13 工作流含“先走查、无走查不得实现/不得声称直觉”的步骤；`docs/README` 阅读路径可定位
- **AND** `agent-rig::default_preamble` 包含“新 UI 功能先做认知走查 + INT 表；e2e 不能证明直觉；无走查/user test 不得声称直观”的段落；`docs/39` 逐字记录并单测锁定
- **AND** `.agents/skills/zeokeel-ui-interaction` 的 description 与步骤覆盖认知走查、`intuition.md`、`intuition-walkthrough.md`、用户测试脚本
- **AND** `specs/templates/interaction.md` 指明同目录两个直觉文件（UI 变更必填）

### R4 · 门禁（存在性，不冒充质量）

- **WHEN** `scripts/ui-interaction-gate.py` 运行
- **THEN** 对每个 enforced（`active` 且 proposal `in-progress`/`done`，或 `--strict`）契约：
  - 同目录 `intuition.md` 必须存在，且解析出 ≥3 个 `INT-\d{2}` 行；
  - 同目录 `intuition-walkthrough.md` 必须存在且非空（≥64 bytes；persona/top task/摩擦表由模板与评审核对）；
  - 缺文件/缺行 → 失败并列出 change id；
  - INT 行**不**参与 `@interaction` 标签/证据检查（直觉质量不由脚本判定）。
- **AND** 输出 `intuition` 计数；`--self-test` 覆盖“缺 intuition.md”“INT 行不足”“缺 walkthrough”三类失败
- **AND** 文档明确：门禁只查存在性/最低数量；质量由 reviewer 走查 + 用户测试证据判断（`docs/11` §8.6）

### R5 · 评审与用户测试

- **WHEN** UI 变更提交评审
- **THEN** 评审者（fresh context、非实现者）按 `docs/27` §2.8 以 3 persona 走 3 个 top task，核对 `intuition.md` 覆盖与 `intuition-walkthrough.md` 摩擦项；P0 摩擦未解决 → CHANGES-REQUESTED
- **AND** P0 旅程在发布前必须有 ≥5 名真实用户的 think-aloud 证据（脚本/记录/指标：成功率、首次点击、时间、错误、SEQ/SUS）；未执行标 **UNVERIFIED**，不得声称“符合直觉”
- **AND** 评审记录/acceptance 引用 INT ID、走查结论、用户测试证据路径；禁止“模型自述/看起来不错”作证据

### R6 · 基线与在途 spec

- **WHEN** 本变更完成
- **THEN** 5 个已实现 surface（composer / settings / inspector / command palette / sessions）在 `specs/changes/2026-11-01-ui-interaction-verification/` 下有 `intuition.md`（12 条 INT）与 `intuition-walkthrough.md`（3 persona × 3 top task），作为样板
- **AND** composer/side-panels/activity 三份在途 spec 在实现开始时补两个文件（planned 阶段可暂缺，gate 只 WARN）
- **AND** 未来任何 UI 变更由 `new-change.sh` 自动生成两个模板文件

## 2. 边界与错误

| 场景 | 期望行为 | 错误码/提示 |
| --- | --- | --- |
| active 契约缺 `intuition.md` | gate 失败并列出 change id | `missing-intuition` |
| `intuition.md` INT 行 <3 | gate 失败并列出数量 | `intuition-too-few` |
| 缺 `intuition-walkthrough.md` / 空文件 | gate 失败 | `missing-walkthrough` |
| planned 契约缺两个文件 | 只 WARN | `planned-skip` |
| 用户测试未执行 | acceptance 标 UNVERIFIED；禁止“符合直觉”结论 | `user-test-unverified` |
| 评审未做走查/只写“看起来不错” | 评审记录不合格；C19 不通过 | `walkthrough-missing` |
| INT 行无验证方式/证据 | 评审 P1；质量不达标 | `int-unsupported` |
| 用户测试 <5 人/非真实用户 | 不作为发布证据 | `user-test-insufficient` |

## 3. 权限与可见性

- 角色：仅影响开发/评审/发布证据；不改变个人/企业客户端权限、审批、网关、商店语义。
- 内容可见性：走查/用户测试记录可写 `docs/reviews/` 或 `target/evidence/`；不得包含用户隐私/密钥；用户测试需匿名化。
- 服务端是否下发策略：否（纯客户端设计/验证规范）。

## 4. 数据与兼容

- 无数据库/契约/生成物变更；无产品运行期数据。
- 新增文件：`docs/27` §2.7/§2.8、`docs/19` §11.1、`docs/11` §8.6 增量、`AGENTS.md` C20/§8/§13、`docs/README`、`KNOWN-LIMITATIONS`、`docs/39` §3.1、skill、两个模板、gate 增量、基线 `intuition.md`/`intuition-walkthrough.md`、本 spec。
- 修改文件：`scripts/new-change.sh`（复制模板）、`packages/client/agent-rig/src/lib.rs`、`specs/templates/interaction.md`、`.agents/skills/zeokeel-ui-interaction/SKILL.md`。
- 兼容：旧 spec 无两个文件不受影响（非 UI 变更不要求）；在途 spec 未实现前 planned 只 WARN。
- 幂等：gate 只读；模板复制可重复执行（存在则跳过）。
- 回滚：删除新增文件、还原修改文件；无数据/发布影响。

## 5. 验收断言（可执行）

- [ ] `python3 scripts/docs-check.py`、`python3 scripts/spec-check.py`、`python3 scripts/review-check.py`、`python3 scripts/docs-command-check.py --self-test && python3 scripts/docs-command-check.py` 全 0。
- [ ] `python3 scripts/ui-interaction-gate.py --self-test` 全 0；`--strict` 对 baseline active 契约检查 `intuition` 通过。
- [ ] `pnpm e2e:ui` 仍 19 passed（本变更不改 e2e 行为）。
- [ ] `cargo test -p zk-agent-rig` 通过（preamble 新段锁定）。
- [ ] `AGENTS.md` 含 C20；`docs/27` §2.7/§2.8；`docs/19` §11.1；`docs/11` §8.6；`docs/39` §3.1；skill description 含“walkthrough/INT”。
- [ ] 基线 5 surface 有 `intuition.md`（12 INT）与 `intuition-walkthrough.md`（3×3）。
- [ ] `new-change.sh` 生成临时 spec 后包含 `interaction.md`/`intuition.md`/`intuition-walkthrough.md`（随后删除临时目录）。
- [ ] 评审记录 APPROVED，P0/P1=0；用户测试标 UNVERIFIED。

## 6. 已定决策

- [x] D1 范围：SSOT + C20 + skill + prompt + 模板 + gate 存在性 + 基线 + 独立评审；不做产品功能/真实用户测试。
- [x] D2 不自动化直觉：e2e 只做代理；质量由 reviewer 走查 + 用户测试判定。
- [x] D3 文件：与 `interaction.md` 同目录的两个文件；INT 前缀 `INT-`。
- [x] D4 阈值：active ≥3 条 INT；P0 旅程各 ≥1；3 persona × 3 top task；用户测试 ≥5 人。
