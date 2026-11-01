# 2026-11-01-ui-intuition-heuristics · 验收与证据

- 验收人：lead + `reviewer`（独立评审：APPROVED，P0=0/P1=0）
- 日期：2026-11-01
- PR/commit：工作树（branch `release/1.0.0-rc.1`，基线 HEAD `3bcfcee` + 本变更未提交改动）
- 证据根：`target/evidence/2026-11-01-ui-intuition-heuristics/`
- 评审：`docs/reviews/2026-11-01-ui-intuition-heuristics.md#结论`（**APPROVED**）

> 状态：`in-progress`（本地规范落地完成）。用户测试（≥5 人）与 `pnpm e2e:ui` 的并发回归仍 **UNVERIFIED**；在完成前不得声称“符合直觉/直观”。

## 1. 验收命令与结果

| 命令 | 结果 | 证据路径 |
| --- | --- | --- |
| `python3 scripts/docs-check.py` | **exit 0**（0 errors / 0 warnings / 41 files） | `logs/docs-check.log` |
| `python3 scripts/docs-command-check.py --self-test && ...` | **exit 0 / exit 0** | `logs/docs-command-check.log` |
| `python3 scripts/spec-check.py` | **exit 0**（0 errors / 2 warnings：本 spec `in-progress`、另一个 `in-progress`） | `logs/spec-check.log` |
| `python3 scripts/review-check.py` | **exit 0**（33 records） | `logs/review-check.log` |
| `python3 scripts/ui-interaction-gate.py --self-test` | **exit 0**（12 checks，含 missing-intuition / too-few / missing-walkthrough） | `logs/gate-self-test.log` |
| `python3 scripts/ui-interaction-gate.py --strict` | **exit 0**：contracts=4（enforced=2），tests=7，tags=34，ids=54，**intuition=2 contracts/24 rows** | `logs/gate-strict.log` |
| `python3 scripts/ui-interaction-gate.py`（normal） | **exit 0**（baseline proposal in-progress，强制 active） | `logs/gate-normal.log` |
| `cargo test -p zk-agent-rig` | **exit 0**：53 passed（含 `default_preamble_mentions_ui_interaction` 10 关键词；context 裁剪 fixture 修正后仍通过） | `logs/cargo-test-agent-rig.log` |
| `bash scripts/i18n-check.sh` | **exit 0** | `logs/i18n-check.log` |
| `bash scripts/new-change.sh ui-intuition-smoke` | **exit 0**；生成 8 文件（五件套 + interaction + intuition + intuition-walkthrough），临时目录已删 | 终端输出 |
| `pnpm e2e:ui` | **29 passed / 5 failed（UNVERIFIED，非本变更引起）**：并发 Phase 2 改动导致 `CMP-VIS-01` composer 高度 117→116、`SET-*` “打开设置”解析到 2 个按钮；本变更未改产品/e2e 代码 | `logs/pnpm-e2e-ui-2.log` |
| 用户测试（≥5 人 think-aloud） | **UNVERIFIED（未执行）** | 待执行后填 `.../usability/` |

## 2. 直觉检查（INT → 证据）

| 检查 | 结果 | 证据 |
| --- | --- | --- |
| 基线 `intuition.md` 12 条 INT | 通过 | `specs/changes/2026-11-01-ui-interaction-verification/intuition.md` |
| P0 → INT 映射 11/11 逐条 | 通过（CMP 4、SET 2、INS 1、CMD 3、SES 1） | 同上 “P0 → INT 映射”表 |
| 3 persona × 3 top task 走查 | 通过（含 3×3 矩阵；无未决 P0） | `.../intuition-walkthrough.md` |
| 摩擦项（P1/P2） | 已记录；P1 不阻塞本规范，后续 UI spec 修复 | `.../intuition-walkthrough.md` §4 |
| gate 存在性检查 | 通过（2 contracts/24 rows） | `logs/gate-strict.log` |
| 用户测试 ≥5 人 | **UNVERIFIED** | `.../usability/`（未生成） |

## 3. 文档与决策

- [x] `docs/27` §2.7（INT-01..12 判定）+ §2.8（3 persona × 3 top task × 4 问、用户测试）
- [x] `docs/19` §11.1（`intuition.md` 第七件 / `intuition-walkthrough.md` 第八件；P0 映射；active/planned）
- [x] `docs/11` §8.6/§9.2（证据、存在性门禁、reviewer 走查）
- [x] `AGENTS.md` **C20 直觉交互先行** + §8 + §13 步骤（C1–C19 未改）
- [x] `docs/README.md` 阅读路径
- [x] `KNOWN-LIMITATIONS.md` §3.4（质量由 reviewer/用户测试；gate 只存在性；用户测试 UNVERIFIED）
- [x] `docs/39` §3.1 + `agent-rig::default_preamble`（单测锁定）
- [x] `.agents/skills/zeokeel-ui-interaction` description + Step 0 走查
- [x] `specs/templates/{intuition,intuition-walkthrough}.md` + `scripts/new-change.sh` + `specs/README`（6/7/8 编号统一）
- [x] 评审记录 APPROVED 且 proposal 已链接；P1-1/P1-2 已修复并复评

## 4. 回滚

- 回滚步骤：删除新增模板/基线/评审文件；还原 docs/AGENTS/skill/prompt/gate/new-change.sh；`git checkout -- <modified>`。
- 数据影响：无（无迁移/无产品数据；用户测试证据目录 target gitignore）。
- 已验证回滚：否（未执行完整演练；改动均为文档/模板/提示词/门禁，可逆）。

## 5. 剩余风险 / 后续

| 项 | 影响 | 后续 |
| --- | --- | --- |
| `pnpm e2e:ui` 因并发 Phase 2 UI 改动 29/5（CMP-VIS/SET-*） | 中 | Phase 2 稳定后更新基线/选择器并重跑；与本规范无关，不得据此判定本规范失败 |
| 用户测试未执行 | 高 | 发布前 ≥5 人 think-aloud；结果回填；未跑前 **UNVERIFIED** / 禁止“符合直觉”结论 |
| 门禁只查存在性/≥3 行/非空，不评质量 | 中 | reviewer 3-persona 走查 + 用户测试判定；`docs/11` 已明确 |
| gate 证据不绑定本次运行（旧证据可残留） | 中 | CI 先跑 e2e 再 gate；后续可加 run-id（P2） |
| 在途 spec 的 `intuition.md` 仍 planned | 中 | 实现时改 active 并补 P0 映射/走查 |
| 并发 Phase 2 评审记录仍引用旧“第六/七件”编号 | 低 | 该记录 owner 后续修正；历史记录不影响本规范 |

## 6. 结论

- [x] `done`（本地规范落地完成；用户测试与 e2e 并发回归 UNVERIFIED）
- [ ] `abandoned`：原因：待填

## 7. 复用证据（C17）

- 搜索记录：`rg -n "intuition|INT-|cognitive|walkthrough|usability|heuristic" docs specs AGENTS.md scripts`（变更前 0 命中）。
- 复用：`docs/11` §8.6 证据口径、`interaction.md`/gate 基础设施、`docs/27` §2.5、`docs/19` §11、`specs/templates/interaction.md`、`scripts/new-change.sh`、`docs/reviews` + `review-check.py`。
- 新增能力登记：无（SSOT/模板/测试基础设施，不新增 `@capability`）。
