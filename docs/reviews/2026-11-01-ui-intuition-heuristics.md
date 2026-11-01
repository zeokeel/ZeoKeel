# 2026-11-01-ui-intuition-heuristics · 独立评审记录（C19）

- 评审对象：`/data/ZeoKeel` 工作树（分支 `release/1.0.0-rc.1`，commit `3bcfceea5f56c6e95d8d8bcf2634a73e5883053f`，工作树 dirty；含并发 Phase 2 实现的未提交改动）。本变更自身只涉及 docs/SSOT + gate + prompt + templates + 基线 `intuition.md`/`intuition-walkthrough.md`；逐文件 SHA256 见 §7。
- 评审者：`reviewer`（fresh-context / 非实现者）。
- 与实现者关系/独立性：实现者为 Lead 与 docs/harness/prompt 各 owner；本人未参与本变更任何实现或写作，仅只读检查 + 亲自执行命令 + 自行做 3 persona 走查；本次会话**唯一写入** = 本记录（`docs/reviews/2026-11-01-ui-intuition-heuristics.md`）。未改产品/e2e/spec/docs 任何文件。
- 关联 spec：`2026-11-01-ui-intuition-heuristics`
- 并发说明：工作树同时有另一条 **Phase 2 实现**在改产品文件（`AssistantComposer.tsx`、`App.tsx`、`ui-settings`、IPC/契约等）与 `client-composer-surface` 契约。本记录把并发引起的失败归因到 Phase 2 并标 **UNVERIFIED/环境性 P2**，不作为本变更的 P0/P1。
- 复评日期/结论：2026-11-01；首轮 P1-1/P1-2 已由实现者修复，本次复评逐项核实为 RESOLVED；**复评结论：APPROVED**（P0/P1 = 0）。
- 说明：本记录 SHA256 为复评时快照；批准后 Lead 回填 `acceptance.md`/`tasks.md` 与 proposal 状态属流程状态变更，不改变本记录对内容的评审结论；除此之外的实质变更需复评。

## 1. 亲自执行的命令与结果（复评轮）

| # | 命令 | 退出码 | 关键结果 |
| --- | --- | --- | --- |
| 1 | `python3 scripts/docs-check.py` | 0 | `0 errors, 0 warnings, 41 files` |
| 2 | `python3 scripts/docs-command-check.py --self-test` | 0 | self-test ok |
| 3 | `python3 scripts/docs-command-check.py` | 0 | ok |
| 4 | `python3 scripts/spec-check.py` | 0 | `0 errors, 2 warnings`（interaction-verification in-progress、intuition-heuristics draft） |
| 5 | `python3 scripts/review-check.py` | 0 | `ok (33 records)` |
| 6 | `bash scripts/i18n-check.sh` | 0 | `i18n-check ok` |
| 7 | `python3 scripts/ui-interaction-gate.py --self-test` | 0 | **12 checks** 全 ok（含 missing-intuition / intuition-too-few / missing-walkthrough） |
| 8 | `python3 scripts/ui-interaction-gate.py --strict` | **0** | `checked contracts=4 (enforced=2), tests=7, tags=34, ids=54 (enforced P0/P1=34), intuition=2 contract(s)/24 row(s)`；无 error（含直觉检查）；2 个 planned 契约 WARN skip |
| 9 | `cargo test -p zk-agent-rig` | 0 | 53 passed（40+1+7+1+2+2），0 failed；含 `default_preamble_mentions_ui_interaction`（同时锁 UI 交互段 + 直觉段关键词） |
| 10 | `cargo check -p zk-agent-rig` | 0 | Finished |
| 11 | `cargo clippy -p zk-agent-rig --all-targets -- -D warnings` | 0 | Finished，0 warning |
| 12 | `cargo fmt -p zk-agent-rig -- --check` | 0 | 无 diff |
| 13 | `bash -n scripts/new-change.sh` | 0 | 语法通过 |
| 14 | `bash scripts/new-change.sh ui-intuition-smoke`（临时目录，随后删除） | 0 | 生成 8 文件（五件套 + `interaction.md` + `intuition.md` + `intuition-walkthrough.md`）；已删除，无残留 |
| 15 | `pnpm e2e:ui` | — | **UNVERIFIED/并发**（见 §2）：本轮未重跑（并发 Phase 2 实现进行中）；首轮独立运行 `4 passed / 15 failed`（首败 `CMD-NAV-01` 的 `browserContext.close: ENOENT ... test-results/.playwright-artifacts-0/traces/...`，随后 14× `ERR_CONNECTION_REFUSED 127.0.0.1:8788`）；Lead 本轮报告 `29 passed / 5 failed`（CMP-VIS-01 composer 高度 117→116；SET-*「打开设置」解析到 2 个按钮）。均属并发产品改动，非本变更 |

## 2. 并发 Phase 2 归因（不构成本变更 P0/P1）

- `git show HEAD:specs/changes/2026-11-01-client-composer-surface/interaction.md` = `状态：planned`；工作树该文件被并发改为 `active`（mtime 17:46:06），故 `--strict` 会同时强制其 `CMP2-*` ID；并发实现随后补齐了 `@interaction:CMP2-*` 测试/证据，所以本轮 `--strict` 已 exit 0（`enforced=2`、`tags=34`、`intuition=2/24`）。
- 并发产品文件：`packages/ui/ui-chat/web/src/AssistantComposer.tsx`（+691 行）、`apps/desktop/web/src/App.tsx`、`apps/desktop/web/src/i18n.tsx`、`ui-settings`、IPC/契约等；均非本变更 scope。
- 本变更实质文件：AGENTS、docs/11/19/27/39/README、KNOWN-LIMITATIONS、specs/README、两个 intuition 模板、gate、new-change、agent-rig preamble+test、基线两个 intuition 文件、本 spec、三份 Phase 2 intuition 头注。未触碰产品 UI/e2e/契约。
- e2e 结论：**UNVERIFIED/环境性 P2**；在 Phase 2 工作树稳定后由 Lead 重跑并回填 acceptance。本变更不改 e2e/产品代码，不因此改结论。

## 3. 评审者自己的 3 persona × 3 top task 走查（5 baseline surface）

方法：`docs/27` §2.8；P1 首次/非技术、P2 高频键盘、P3 出错/键盘+读屏；T1 发送消息、T2 改设置、T3 键盘/命令+右栏。每步 4 问：①知道目标？②看得到控件？③能预判反馈？④出错能恢复？依据 `interaction.md` + 代码/i18n（shell、composer、settings、inspector、commands、sessions）。

| Persona/Task | ① 目标 | ② 控件 | ③ 反馈 | ④ 恢复 | 结论 |
| --- | --- | --- | --- | --- | --- |
| P1/T1 发送首条消息 | 通过：placeholder「输入消息，Enter 发送，Shift+Enter 换行」+ 提示行（`i18n.tsx:84`） | 通过：textarea、发送、0/4000 常驻可见 | 通过：Enter 发送、空草稿发送禁用、流式状态可见 | 部分：失败保留草稿无专门 e2e，仅 `CMP-PERSIST-01` 成功刷新 | 无 P0；P1 与走查 F1 一致 |
| P1/T2 改设置并保存 | 通过：侧栏「打开设置」按钮（`ui-sessions:602`） | 通过：dialog + 分区 nav + 单一保存 | 通过：保存中/已保存 toast（`settingsSaving/settingsSaved`） | 部分：错误 key 已能跳 pane + focus（`ui-settings:119-143`），字段级文案仍可加强 | 无 P0；`INT-04` P2 与 F4 一致 |
| P1/T3 键盘/命令+右栏 | 通过：header 有可见「命令面板（⌘K）」按钮 + tooltip（`ui-shell:142`；HEAD 已存在）、「切换右栏（⌘B）」按钮（`ui-shell:145`） | 通过 | 通过 | 通过 | 无 P0；修正后 F3 与代码一致 |
| P2/T1 高频发送 | 通过 | 通过 | 通过（Enter/Shift+Enter 有提示） | 同 P1/T1 | 无 P0 |
| P2/T2 键盘保存 | 通过 | 通过 | 通过 | 通过：Esc 关 Radix dialog 并回 opener（`SET-ESC-01`） | 无 P0 |
| P2/T3 键盘命令流 | 通过 | 通过：Ctrl+K、Ctrl+B 有 header 入口 | 通过：palette listbox + aria-activedescendant | 通过：Esc 回输入框（`CMD-ESC-01`） | 无 P0 |
| P3/T1 出错恢复 | 通过 | 通过 | 通过：role=status/alert 文案 i18n | 部分：同 P1/T1 | 无 P0 |
| P3/T2 读屏设置 | 通过 | 通过：dialog name/heading/close name | 通过 | 通过 | 无 P0 |
| P3/T3 右栏读屏 | 通过 | 通过：tablist 3 tab、close 有 `aria-label=收起右栏`（`ui-inspector:48`） | 通过 | 部分：close 可见面只有 `×`（有 aria-label，无 tooltip） | 无 P0；`INT-10` P2 与 F5 一致 |

**结论**：5 surface 的 P0 旅程没有未解决 P0 摩擦，与 `intuition-walkthrough.md` 的「无 P0」一致；修正后的 3×3 矩阵与 F3 已与代码一致。

## 4. `intuition.md` / `intuition-walkthrough.md` 核验（复评）

- `intuition.md`：12 行 INT（`INT-01`…`INT-12`），与 `docs/27` §2.7 的 12 条原则一一对应；每行有场景/期望/反例/验证方式/证据；用户测试标 **UNVERIFIED**。**P1-2 已修复**：新增「P0 → INT 映射（11/11，逐条）」表，程序化核对 11 个 P0 ID 全部字面出现（CMP-FOCUS/KEY/IME/PERSIST、SET-OPEN/ESC、INS-TOGGLE、CMD-OPEN/NAV/ESC、SES-NEW），每 ID 映射 ≥2 条 INT 且与 `intuition-walkthrough.md` 的 T1–T3 一致。
- `intuition-walkthrough.md`：含 3 persona、3 top task、3×3 走查矩阵（P1/P2/P3 × T1/T2/T3，每格给 4 问结论 + INT）、5 条摩擦（F1–F5，F1/F2 为 P1、F3–F5 为 P2）、用户测试 UNVERIFIED。**P2-1/P2-2 已修复**：F3 改为「header 命令按钮+tooltip 已可见，快捷键提示弱（P2）」，与 HEAD 的 `ui-shell:142` 一致；走查从「3 task 段 + 通用 P3」改为字面 3×3 矩阵。
- 用户测试：未执行并如实标 UNVERIFIED（`intuition.md`、`intuition-walkthrough.md` §5、`KNOWN-LIMITATIONS:90`、`acceptance.md:8,21`）；全仓未发现「已符合直觉/用户测试已通过」类过度声称 → 合规。

## 5. 可见性 / SSOT / C 约束（复评）

- **C20 可见性**：`AGENTS.md:62` C20；`AGENTS.md:269`（§8）+ §13 工作流「先走查/INT 再契约/代码」；`docs/README` 阅读路径；skill description 含 cognitive walkthrough/INT；`agent-rig::default_preamble` 直觉段与 `docs/39` §3.1「原文（逐字）」程序化对比一致，插入点正确，identity/persona 与 `web_fetch` 警告逐字未变，单测锁定关键词（`cognitive walkthrough`/`intuition.md`/`intuition-walkthrough.md`/`user test`/`P0 friction`）→ 通过。
- **SSOT**：直觉原则/判据/反例只在 `docs/27` §2.7 定义（并明确 `INT-01..12` 定义一次、各 surface 引用）；`docs/19` §11.1 只定义必填文件/映射；`docs/11` §8.6 已收敛为证据/门禁口径并链接 19/27（**P2-7 已修复**：不再复制阈值）。gate 只做存在性/行数/非空检查（`INTUITION_MIN_ROWS=3`、`WALKTHROUGH_MIN_BYTES=64`），与 spec R4/design §2.2 一致（**P2-6 已修复**）。**未发现第二份原则表/gate 算法**。
- **文件集编号**：canonical 已统一为 interaction=第六件、intuition=第七件、intuition-walkthrough=第八件；`docs/19:295-296`、`docs/11:176`、`specs/templates/intuition.md:3`、`specs/README.md:47-49,56`、`scripts/new-change.sh:19`、spec proposal D3、composer/side/activity 三份 intuition 头注一致（**P1-1 已修复**）。`rg` 无残留错误编号。
- **用户测试**：≥5 人 think-aloud 未执行并标 UNVERIFIED；禁止「符合直觉」结论 → 通过。
- **C2**：无运行期加载/新依赖；仅提示词常量 + 文档/模板/脚本。**C15**：新增文件不依赖/不引用 channels/Harness/Next（仅 AGENTS C15 自身条文出现词面）。**C16**：本 spec 五件套齐全、proposal 仍 draft；agent-rig 提示词属本变更受审范围。**C17**：扩展既有 gate/templates，未新增 capability/依赖。**C19**：文档先落、非实现者评审 + 复评，proposal 待 APPROVED 后再置 approved/in-progress → 流程符合。

## 6. 评审发现（复评后状态）

### P0（阻塞）：无

### P1（应在 APPROVED 前修复）：无（首轮 2 项均已修复并核实）

| # | 首轮问题 | 复评状态与证据 |
| --- | --- | --- |
| P1-1 | spec 文件集编号自相矛盾 + `specs/README` 未登记直觉模板/第七八件 | **RESOLVED**：编号统一 6/7/8（`docs/19:295-296`、`docs/11:176`、`specs/templates/intuition.md:3`、`specs/README.md:39,47-49,56`、`scripts/new-change.sh:19`、proposal D3、三份 Phase 2 intuition 头注）；`specs/README` §4 文件表新增 3 行；`rg` 无错误旧编号 |
| P1-2 | baseline `intuition.md` 无显式 P0→INT 映射（4 个 P0 ID 未出现） | **RESOLVED**：新增「P0 → INT 映射（11/11，逐条）」表，程序化核对 11 个 P0 ID 全部字面出现且每 ID ≥2 条 INT |

### P2（非阻塞，复评后状态）

| # | 文件:行 | 问题 | 修法 | 状态 |
| --- | --- | --- | --- | --- |
| P2-1 | baseline `intuition-walkthrough.md`（F3） | 原称「⌘K 是隐藏捷径/无可见提示」，与 HEAD 可见 header 按钮 + tooltip 矛盾 | 已改为「按钮可见，快捷键提示弱（P2）」 | RESOLVED |
| P2-2 | baseline `intuition-walkthrough.md` §3 | 原只有 3 task 段 + 通用 P3，非字面 3×3 | 已补 3 persona × 3 top task 走查矩阵，保留 §3.1 细节 | RESOLVED |
| P2-3 | `pnpm e2e:ui` | **UNVERIFIED/并发**：本轮未重跑；首轮独立运行 4/15（首败 trace ENOENT + 14× connection refused），Lead 本轮 29/5（CMP-VIS 117→116、SET-* 两个「打开设置」）；并发产品文件在改 | Phase 2 稳定后由 Lead 重跑回填 acceptance；不阻塞本变更 | OPEN（环境性） |
| P2-4 | gate `--strict`（首轮） | 首轮因并发 composer 契约 active 而 exit 1（30 errors） | 并发实现补齐 CMP2 测试/证据后，本轮 exit 0（enforced=2、intuition=2/24） | RESOLVED |
| P2-5 | `acceptance.md:15-24,28-35,37-44`、`tasks.md` 状态 | 命令结果仍「待跑」、tasks 全 ☐ | Lead 在本记录 APPROVED 后回填实际结果/证据/UNVERIFIED 项 | 待 Lead 执行（流程） |
| P2-6 | `spec.md:36` vs `design.md:51-52` | spec 说 walkthrough 需含 persona/top task/摩擦标记，gate 原只校验 32 bytes | 已统一为 ≥64 bytes，persona/top-task/摩擦表由模板 + 评审核对（gate `WALKTHROUGH_MIN_BYTES=64`） | RESOLVED |
| P2-7 | `docs/11:176` | 原重复 docs/19 §11.1 的阈值/映射要求 | 已收敛为「字段/映射/证据要求见 19 §11.1」+ 证据口径 | RESOLVED |
| P2-8 | `docs/27:80` | 「INT ID 全仓唯一」与 per-surface 复用 INT-01..12 字面冲突 | 已改为「INT-01..12 在 §2.7 定义一次；各 surface 引用，不自造同名编号」 | RESOLVED |
| P2-9 | `scripts/new-change.sh:19-20` | 注释称「幂等：目标已存在则跳过」但目录已存在时脚本提前 exit，分支不可达 | 已删除该幂等表述，改为「仅在本脚本新建的 change 目录内生成」 | RESOLVED |
| P2-10 | `docs/reviews/2026-11-01-client-phase2-intuition.md:27` | 并发 Phase 2 的评审记录仍引用旧编号「第六/七件」 | 建议并发 owner 更正；非本变更文件 | OPEN（并发/非本变更） |

## 7. 评审对象 SHA256（复评时工作树快照）

commit `3bcfceea5f56c6e95d8d8bcf2634a73e5883053f`（工作树 dirty；并发 Phase 2 产品文件不在本表/本变更 scope）。

| SHA256 | 文件 |
| --- | --- |
| a570d48ce23fd2f406f2eb7397e86db4d3e8f3d9c1eb47c9a7ecb3d8aaabbc6b | `specs/changes/2026-11-01-ui-intuition-heuristics/proposal.md` |
| 90ee2bc009df3c3e516c1d676ad3d2134d250460f4d10c6dcfcea29f4dcf6ab7 | `specs/changes/2026-11-01-ui-intuition-heuristics/spec.md` |
| 29a0e0fddf05d23eac73bda720a629bd22a15fa56997e96601ea62ced87f942a | `specs/changes/2026-11-01-ui-intuition-heuristics/design.md` |
| bccd1424e8fa6424b168342a07a1257e20bebbc4a3f35e1a3d31d1d79ed4a2c3 | `specs/changes/2026-11-01-ui-intuition-heuristics/tasks.md` |
| c51e2de8147e05ac8823e4bc1845055ea388e3285b912fcf6492d9577675f638 | `specs/changes/2026-11-01-ui-intuition-heuristics/acceptance.md` |
| 9083dfdc0ca76678fc331b63803d34d286bc9f37d737f1c3ef5b6c07f4448223 | `specs/changes/2026-11-01-ui-interaction-verification/intuition.md` |
| e36eddf57b0f71fc6f244f5580d22126ad9f66332f3bd3d5f16577642abce863 | `specs/changes/2026-11-01-ui-interaction-verification/intuition-walkthrough.md` |
| d587d330db8691cdf49b76ed16acf0110f8f9c139e9cc5e0f14bf11ffd1e91e4 | `specs/templates/intuition.md` |
| 8e620fa06a8a114e75307adc27c77713c17988021dca51782edcb2cdfbbcda7c | `specs/templates/intuition-walkthrough.md` |
| 0fe3a3795e1345bcb2b3cfc77ddcbd705a5fa4e838896414193ed4ff3ceef306 | `specs/templates/interaction.md` |
| ce90d6745c23b20434ccd82886e25716180be4f0d2cbe0dc747ede0e102b2835 | `docs/27-ui-design-system.md` |
| 7403fbcc93f87919b00e99b276d22d7aeefc19d7314af4ca5789d9afbaf35ff1 | `docs/19-client-ux-flows.md` |
| 065579eb17e39c138de956cc286ed9568f7a2db379b470d07f1b6ce2ffbf151a | `docs/11-testing-and-evidence.md` |
| 3c6fef9f321c72ad6449a2e9f586691d56ceb4163c1e53cdb0f9b985aa010285 | `docs/README.md` |
| a76a098fe6b28b19f39a11720ad58df09a142ffff6bb8a9650bc77cc55f9b73b | `AGENTS.md` |
| 7857a3f92926ae6e7b845394ec1edf3c840f6f5e655bd2a46b359170820647b6 | `KNOWN-LIMITATIONS.md` |
| a1aedb997ad19b096ae99c996e8f64cd14406b51c5a29ae3c156e7c22821dc0d | `docs/39-agent-prompts.md` |
| af93ec252723329e60c215f145927a08067b4c08e7ec6f939363a34f878c8df9 | `specs/README.md` |
| a0a431f3fc833249ed9c913bd54906e6de44a6b35034c63a59aeea5be1d8fb0c | `.agents/skills/zeokeel-ui-interaction/SKILL.md` |
| 4e40fbc3846cbf88d59ba096504800c7ac959e881ae72243f16ce540f3c6d365 | `.agents/skills/zeokeel-change/SKILL.md` |
| f6e2c928de37b500f4c194a6460f5d559120b958b4530c12266c92af4e76bdef | `scripts/ui-interaction-gate.py` |
| 640688383dda80f48ed47ffbcb6d4a74b8505b2890818c19642894f459ebe5ca | `scripts/new-change.sh` |
| 0b9e3c925902b835bce8e4ca4e2f1397b467fd1de7d6374988fb68f677d841e5 | `packages/client/agent-rig/src/lib.rs` |
| 4ceee67ca92fb7b9d5cc0c9e6364e5cdb73a189e2e60f8756358149067dd4f8a | `packages/client/agent-rig/tests/context.rs` |

## 8. 复评

- 首轮 P1-1（spec 文件集编号/`specs/README`）与 P1-2（baseline P0→INT 映射）已修复；复评逐项核对：编号 6/7/8 全仓一致、`specs/README` 已登记三模板与第七/八件、baseline 映射表 11/11 字面覆盖；P2-1/P2-2/P2-4/P2-6/P2-7/P2-8/P2-9 一并修复。
- 复评重跑：docs-check 0/0、docs-command-check 0、spec-check 0 errors、review-check 0、gate --self-test 12 checks 0、gate --strict 0（intuition 检查绿）、cargo test/check/clippy/fmt 0、new-change smoke 0。
- 残留：`pnpm e2e:ui` 仍 **UNVERIFIED**（并发 Phase 2 产品改动，Lead 29/5、我首轮 4/15；非本变更）；acceptance/tasks 待 Lead 回填；并发 Phase 2 评审记录仍有旧编号（非本变更文件）。
- 结论：本变更（直觉交互 SSOT/gate/prompt/templates/基线与独立评审）**通过**；用户测试（≥5 人）仍未执行，保持 UNVERIFIED，在完成前不得声称“符合直觉/直观”。

- 结论：**APPROVED**
