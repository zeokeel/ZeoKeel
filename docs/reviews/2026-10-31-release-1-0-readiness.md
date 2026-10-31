# 2026-10-31-release-1-0-readiness · 独立评审（阶段 1：spec/design/tasks；含复评）

- 评审对象（首评，2026-10-31T13:14）：`HEAD d0a1c95` + 未跟踪五件套快照
  （proposal `3398d991…`、spec `99b1d446…`、design `d54973f7…`、tasks `97da60d4…`、acceptance `f527700e…`；
  同批复核 docs/37 `2ac8b43b…`、docs/18 `2ee55673…`、docs/21 `2035724e…`、docs/24 `6c782c5c…`、docs/33 `db8836f8…`）。
  **首评快照已被实现者于 13:15–13:16 编辑（proposal/spec/design/tasks），按 `docs/reviews/README.md` 记录失效并复评。**
- 复评对象（2026-10-31T13:22）：`HEAD d0a1c95` 未变 + 五件套新快照 proposal `6c0dfaa7…`、spec `b8c7edc4…`、
  design `96c2473f…`、tasks `ba565203…`、acceptance `f527700e…`（docs/37/18/21/24/33 未变，hash 同上）。
- 评审者：release-reviewer（独立只读评审；仅可写 `docs/reviews/`；未实现本变更任何文件）。
- 独立性：评审者未参与 proposal/spec/design/tasks 撰写，也未参与 A/B/C 阶段实现；事实核验与复评由评审者
  独立执行（§2/§6），命令与 exit code 均为本次评审采集。实现者另有其人（Lead/其他 teammate）。
- 关联 spec：`specs/archive/2026-10-31-release-1-0-readiness/`（proposal/spec/design/tasks/acceptance，未归档）。
- 评审日期：2026-10-31；复评日期：2026-10-31（同日两轮）。
- 结论：**APPROVED**（复评：P0=0 / P1=0；P2×9 为文档润色/施工期验证项，不阻塞 A 阶段施工与 rc.1 发布；
  P2 与实现证据由阶段 3 终评确认）。

## 1. 范围与方法

- 已读：`AGENTS.md` §2（C1–C19）/§6/§9–§12；`docs/README.md` §2–§6；`docs/11` §9；`docs/12`；`docs/18` §3/§7/§8/§10；
  `docs/21` §10；`docs/24` §1.2；`docs/37`；`docs/33`；`docs/38` §5.4；`docs/03` §CI；`docs/10`、`docs/20`；
  `.github/workflows/{ci,security,release}.yml`；`scripts/{security-audit,review-check,docs-check}.py|sh`；
  `target/evidence/release-check/`（delivery 审计、rust 退出码）；`target/evidence/release-1.0-rc1/spikes/rig-0.45.md`；
  M6 spec/acceptance（13 项来源）。
- 方法：spec 契约完整性 → 与 docs/SSOT 对照（C1–C19）→ 对 proposal 关键事实做 ≤6 条独立只读核验（§2）→
  P0/P1/P2 与可执行性 → 首评 CHANGES-REQUESTED → 实现者修 P0/P1 → 复评（§6）→ APPROVED。
- 并发说明：评审期间实现者并行推进 A1/A2/A3/A6/A10/A15/A16/A18（`git status` 显示 `.github/workflows/*`、
  版本文件、`packages/server/jobs/src/lib.rs`、`scripts/build-dist.sh`、admin ui-kit、server 契约等未提交改动）。
  因此 §2 用 **HEAD 对象命令**（`git show HEAD:…`/`git grep … HEAD`）保证可复现；涉及工作树的命令标注执行时刻。
  实现证据与最终 diff 由阶段 3 终评复核，不在本次 spec 评审结论内。

## 2. 事实核验（首评 6 条，只读命令 + exit code）

| # | 待核声明（来自 proposal） | 命令 | 结果 | 判定 |
| --- | --- | --- | --- | --- |
| 1 | 0 git tag、未发布 | `git tag \| wc -l` | `0`，exit 0 | 真（C3/tag 未发生） |
| 2 | 仓库无 LICENSE 文本 | `git cat-file -e HEAD:LICENSE` | `fatal: path 'LICENSE' does not exist`，exit 128 | 真（R6 待补） |
| 3 | `review-check` 红 / 评审链接无效 | `python3 scripts/review-check.py`（13:14） | exit 1：`review record not found: docs/reviews/2026-10-31-release-1-0-readiness.md`（独立只读评审；实施后复评）` | 真（P0-1 证据；13:22 复跑已只剩结论项，证明路径修复，见 §6） |
| 4 | `security-audit.sh` 诚实非 pass | `bash scripts/security-audit.sh /tmp/sec2.json`（13:14；脚本未改） | exit 2；JSON `checks=331 high_fail=0 unverified=3 status=unverified` | 真（R4 现状；3 个依赖工具 unverified） |
| 5 | CI 未执行已声明门禁 | `git grep -nE 'cargo doc\|cargo geiger\|pnpm -r typecheck' HEAD -- .github/workflows/` | exit 1（HEAD 无匹配；工作树已由实现者并发补 typecheck/geiger/doc） | 真（R2 依据：HEAD 上 doc/geiger/typecheck 均不在 CI） |
| 6 | rustdoc 红（唯一红门禁） | `cat target/evidence/release-check/rust/04-doc.exit` + `04-doc.log`；`git show HEAD:packages/server/jobs/src/lib.rs`（320/339 行） | `EXIT=101`；`:339` 链接私有 `STOP_WAIT_TIMEOUT`（定义于 `:320`） | 真（A1 依据；评审者未重跑长构建，证据为 HEAD 当日 release-check） |

## 3. P0（首评 2 项，复评均已修复）

### P0-1 · 评审链接被 `review-check` 解析坏 → C19 门禁无法变绿（`proposal.md:7`）

- 首评现状：`- 评审：`…md`（独立只读评审；实施后复评）``；`scripts/review-check.py:45-52` 只 `strip("`")` + `split("#")`，
  尾随括号被当作路径后缀 → 记录存在也 exit 1；CI `docs` job（HEAD `ci.yml:125` 执行 review-check）必红。
- 复评处置：`proposal.md:7` 已改为纯路径 `docs/reviews/2026-10-31-release-1-0-readiness.md`；13:22 复跑
  `review-check` 已能解析到本记录，仅剩"结论需 APPROVED"（§6）。**状态：fixed（已核验）。**

### P0-2 · proposal §3 非目标与 spec R9（用户 D2）直接矛盾（`proposal.md:34` ↔ `spec.md:63-71`）

- 首评现状：§3 写"不补齐 agent/rig 缺口、另立 spec"，与 D2"补齐全部功能缺口（R9）"及 spec R9.1/R9.2 冲突；
  spec R9.1 还留"agent-team 或 ADR 不做"逃生口。
- 复评处置：`proposal.md:34` 改为"按用户 D2 补齐 R9；仅 by-design 排除 pdf/epub（ADR-067）、企业媒体/embeddings（C7）、
  cordis（C2）、Windows 签名（ADR-033）、服务端自动升级（ADR-041）"；`spec.md:63-71` R9 改为"全部可行项"并删除
  agent-team 逃生口（改为 §6 F1 冻结模型）；`spec.md:5` 声明"行为决策已冻结（§6），实现不得再留未决行为"。
  **状态：fixed（已核验）。** 残留：`tasks.md:33` B5 行仍留旧括号（记为 P2-6，非行为歧义，spec/F1 权威）。

## 4. P1（首评 7 项，复评均已处置）

| # | 首评问题（位置） | 复评处置（证据位置） | 状态 |
| --- | --- | --- | --- |
| P1-1 | R3 要求 tag push 触发/GH Release，但 HEAD release.yml 仅 workflow_dispatch、`contents: read`、无 `gh release`；A4 未含 trigger/权限（`spec.md:22-27`、`tasks.md:14`） | `spec.md:24-27` 改为"推送 tag 或 workflow_dispatch …（tag push 触发、`permissions: contents: write`）…创建 GitHub Release …不得 `\|\| true`"；工作树 release.yml 已并发大改，A4 保留"tag/version 校验、GH Release、真实产物"（`tasks.md:14`）。最终由 A4/C3 验收 | fixed（spec 层；施工期验证） |
| P1-2 | R4 以"docs/21 §10 13 项"验收，但 docs/21 §10 只有 6 条，13 项枚举在 M6 spec；A6 未写明迁移（`spec.md:28-33`、`docs/21:131-138`） | `spec.md:31-32` 明确"docs/21 §10 逐项列出 13 项（从 M6 spec 迁移枚举，SSOT 在 docs/21）"；A6 写作用域含 `docs/21`（`tasks.md:16`）。docs/21 实文迁移由 A6 完成 | fixed（spec 层；A6 验证） |
| P1-3 | R2 覆盖 AGENTS§6/docs/03，但 A5 只写 docs/18；AGENTS:191 nextest、docs/03:385 nextest-as-CI、security.yml 装 geiger 不调用（`spec.md:15-20`、`tasks.md:15`） | `spec.md:17` 明确覆盖 `AGENTS.md` §6/docs/03/docs/18；`tasks.md:15` A5 写作用域加 `docs/03` 且注明 AGENTS §6 真值由 ci-release → docs-version；工作树 ci/security 已加 doc/geiger/typecheck 步骤 | fixed（spec 层；A2/A3/A5 验证） |
| P1-4 | 文档真源漏项：AGENTS §12/docs/12:29 陈旧、docs/10:125 死链 dev-all.sh，A12/A14 未覆盖（`AGENTS.md:301`、`docs/12:29`、`docs/10:125`） | `spec.md:52-54` R7 覆盖 `AGENTS.md`（§1+§12）/docs/10/docs/12/docs/20；`tasks.md:24` A14 含 docs/10 并注明 AGENTS §12 由 docs-version 同步；A12 覆盖 AGENTS/docs/12 | fixed（spec 层；A12/A14 验证） |
| P1-5 | tasks A4 依赖 A10/A17 但提交顺序把 A4 排在其前（`tasks.md:14` vs `:57-61`） | `tasks.md:58` 改为"A10 版本 → A2/A3/A5 CI（A4 依赖 A10/A17，排在其后）"；design §2.1 同步（A4 在 A10/A17 之后）。A4 具体步骤位置由执行时排程 | fixed（顺序已消歧；施工期验证） |
| P1-6 | 已 approved spec 仍带 3 个未决行为决策（rig API/agent-team/裁剪）（`spec.md:66`、`:106-110`） | `spec.md:109-119` 新增 §6 F1–F7 已冻结决策（B5 编排模型/B8 权威/B3 轮次/B10 压缩/B6 不可实现项/R8.2 裁剪/版本文案）；`spec.md:121-124` §7 仅余外部环境项（D4）；design `§2.3` 冻结行为同步 | fixed（已核验） |
| P1-7 | R8.2 允许"ADR + 发布说明"豁免 personal targets 缺陷（`spec.md:57-59`、`editions/personal.toml:3`、`tools/zk/src/main.rs:747-757`） | `spec.md:61` R8.2 改为"personal 裁剪可验证：plan 闭包/生成物/发布产物三处断言，**不接受 ADR 豁免**"；`spec.md:81` 错误表加裁剪断言；`tasks.md:27` 新增 A18（`targets` 过滤 + 三处断言）；design `W8`/§2.3/§6 同步 | fixed（spec 层；A18 验证） |

## 5. P2（建议项；不阻塞施工，A/C 阶段同期修）

| # | 位置 | 问题 | 修法 | 状态 |
| --- | --- | --- | --- | --- |
| P2-1 | `proposal.md:28,44,93` | 版本文案残留 `1.0.0`（Goa1/用户场景/W5 行），与 D1（`:77`）=`1.0.0-rc.1` 及 spec F7 不一致。 | 统一 `1.0.0-rc.1`（"正式 1.0.0 待 rc 验收后"按 F7 表述）。 | open |
| P2-2 | `proposal.md:17` | 事实小误：称 docs/20 引用 `scripts/dev-all.sh`；实际 docs/20:31 是 day0-check.sh，dev-all.sh 在 docs/10:125（A14/R7 仍正确覆盖）。 | 改 attribution；不影响任务范围。 | open |
| P2-3 | `spec.md:37-41`、M6 `acceptance.md:1` | docs/37 各条"已勾/UNVERIFIED"判定表未预列；M6 acceptance 第 1 项要求 security `pass`，本机无 Docker/trivy 时保持 blocked。 | A9/A13 验收时列判定表并写 UNVERIFIED 原因/后续命令。 | open |
| P2-4 | `design.md:54-55` | §2.0 阶段表"B \| B1 工具补齐"重复两行（编辑残留）。 | 删重复行。 | open |
| P2-5 | `design.md:26`、`design.md:129` | §1.5 rig 行仍写"否则记录 UNVERIFIED"、§8 风险写"回问用户"，与 F5（仅 live runner checkpoint 不可实现，其余必须实现；最终验收前确认）不一致。 | 同步 F5 措辞。 | open |
| P2-6 | `tasks.md:33` | B5 仍留"（或 ADR 不做，需用户确认）"旧逃生口，与 spec R9.1/F1（agent-team 实现）冲突（spec 权威，无行为歧义）。 | 删除括号，改为"按 F1 实现 + ADR 记录编排语义"。 | open |
| P2-7 | `tasks.md:16`（A6 验证列） | A6 verify 仅 `security-audit.sh`，未含 docs/21 §10 13 项枚举检查（R4 要求迁移）。 | verify 加 `rg`/docs 断言（docs/21 含 13 项 ID）。 | open |
| P2-8 | `acceptance.md:3-5` | 验收占位 `<name>/<YYYY-MM-DD>/<hash>`；C1 需填证据路径并归档，docs/11 §9.6 要求 acceptance 链接本评审记录。 | C1 前替换占位、补证据指针/回滚验证/评审链接。 | open |
| P2-9 | `scripts/review-check.py:33-40` | `latest_conclusion()` 取"含『结论』且含 APPROVED/CHANGES-REQUESTED 的末行"，正文措辞可能误触发（首评已实测：记录 §9 原句曾被解析为 APPROVED）。门禁有无意识 false-green 风险。 | 只解析固定字段行（`^- 结论：`），并给记录加 schema 断言。 | open |

## 6. 复评记录（2026-10-31T13:22，触发：Lead 修复后请求复评）

- 门禁复跑（评审者执行）：
  - `python3 scripts/review-check.py` → **exit 1**，但错误已从"review record not found:…（独立只读评审…）"变为
    "latest review conclusion is CHANGES-REQUESTED (need APPROVED)" → **P0-1 路径解析已修复**；本记录更新为 APPROVED 后应为 0。
  - `python3 scripts/docs-check.py` → **exit 0**（0 errors / 0 warnings / 40 files，13:22 复跑）。
- P0-2 核验：`proposal.md:34` 已按 D2 改写（`grep -n '仅以下 by-design'` 命中）；`spec.md:63-71` R9 无逃生口；
  `spec.md:5` 声明行为冻结；design §2.3/§6、tasks B5 说明（其残留措辞列 P2-6）。
- P1 处置核验（§4 证据列逐条对照新快照）：R2/R3/R4/R7/R8/R9 文本与 A5/A14/A18/任务顺序已更新；
  `target/evidence/release-1.0-rc1/spikes/rig-0.45.md` 存在（含 `pub(crate)`/`RunOrigin`/`build_run` 证据 3 处命中，
  对应 F5 的"仅 live runner 中途 checkpoint 不可实现"）。
- 复评结论：**APPROVED**（P0=0 / P1=0）。P2×9 不阻塞 A 阶段施工；实现证据（A1/A6/A18/CI/发布/安全）由阶段 3 终评
  按 `acceptance.md` 复核，若届时 P2 或证据未落实则本记录回退为 CHANGES-REQUESTED。

## 7. C19 评审检查表（docs/11 §9.2）

- [x] 唯一 SSOT：design/proposal 链接 docs，不复制规范；R4 明确 13 项 SSOT 回写 docs/21（P1-2）。
- [x] 来源/许可可接受；无参考仓库依赖（C15）；无运行期插件/切换（C2）。
- [x] C3–C5：不新增包/角色；R9.2 若新增 `zk-kit-*` 需先 ADR（design.md:35）并登记 `@capability`。
- [x] C6/C7/C8 边界写清：spec §3 企业 AI 只走网关、媒体/embedding 隐藏/拒绝、角色不变。
- [x] 组合基线符合 docs/24 §1.2（personal×default×official 入库；enterprise 仅 compose+cargo check）。
- [x] R8.2 组合裁剪无豁免（F6/A18 三处断言），不再有"文档声称已裁剪、实现未裁剪"的缺口。
- [x] 可实现/可测试：R1–R9 + §6 冻结决策 + design §6 的 T0–T5 命令具体；e2e/UNVERIFIED 口径明确。
- [x] C12/C16/C19：用户 D1–D4 已拍板；proposal `approved` 且已链接本评审记录；P0/P1 已复评关闭。

## 8. 关联 spec 与背景记录

- `specs/archive/2026-10-31-release-1-0-readiness/`（本评审对象，五件套，未归档）。
- 基线评审：`docs/reviews/2026-10-31-round12-verification.md`（HEAD 门禁/目标部分 partial 的残余）。
- M6 缺口来源：`docs/reviews/2026-10-30-m6-implementation.md`、`specs/changes/2026-10-30-hardening-release/`（13 项/acceptance blocked）。
- 交付事实证据：`target/evidence/release-check/delivery/audit-notes.md`、`target/evidence/release-check/rust/04-doc.log`。

## 9. 当前结论

结论：**APPROVED**（阶段 1 复评：P0=0 / P1=0；P2×9 为文档润色与施工期验证项，不阻塞 A 阶段施工与 rc.1 发布；
阶段 3 终评将复核实现证据与 P2 收敛，必要时回退本结论）。

## 10. 阶段 3 最终验收（2026-11-01）

- 验收对象：annotated tag `v1.0.0-rc.1` → commit `33f59a139317bdac15bbf8ceda549258ede8ef17`（远端 tag object
  `8a88c81f…`、tagger 2026-10-31T14:54:46Z；远端 `main` 同 commit；工作树 clean）。
- 验收证据（2026-11-01 01:0x +08，评审者独立只读核验）：
  - **CI run 41210867692**（main, `33f59a1`）= success，**9/9 jobs**（rust/web/integration/e2e/a11y/e2e-oem/
    e2e-client-upgrade/docker/docs）；触发器已收敛为 push main + PR main（tag 仅 release workflow，不再三重复跑）。
  - **Security run 41210867640** = success；strict artifact（`security-evidence`）exit 0、overall
    `unverified_external`、**pass=12 / fail=0 / unverified=0 / unverified_external=2**（gate 7 WORM 部署控制 ADR-068；
    gate 9 产物 SBOM release-only），gate 1 / gate 10（trivy fixable-only，68 unfixed 登记 KNOWN-LIMITATIONS §2）/ gate 14 = pass。
  - **Release run 41212324783**（tag event, `33f59a1`）= success；installers linux/windows/macos 全 success
    （macos-15），assemble dist success，GitHub Release success；`upgrade-drill` job failure 为环境项
    （continue-on-error），已在 release body + KNOWN-LIMITATIONS §2 明确 UNVERIFIED 并指向 `upgrade-drill-evidence` artifact。
  - **GitHub Release `v1.0.0-rc.1`**（prerelease，published 2026-10-31T15:39:00Z）：15 assets = AppImage/deb/dmg/nsis +
    server/postgres/caddy 镜像 tar + CycloneDX SBOM + `SHA256SUMS` + BUILD-STATUS/VERSION/CHANNEL/CLIENT-RELEASE +
    `KNOWN-LIMITATIONS.md` + 完整 dist archive（315,956,897 B，sha256 `ad4e7671…`）。
  - B2 复核：Release 的 `KNOWN-LIMITATIONS.md` digest `0f93a3d4…` == 仓库 `sha256sum KNOWN-LIMITATIONS.md`
    （`0f93a3d4…`）；§1 tag/Release/hosted runner = VERIFIED，§2 交付物 CI-verified、真机/公证/provider 等 UNVERIFIED。
  - B5 复核：`SHA256SUMS` 25 条；release job "Verify the Release really contains the real artifacts" 下载并解包
    archive 后执行 `sha256sum -c SHA256SUMS`，**25/25 全部 OK**（run log 14324–14368）；13 个 loose asset 的
    GitHub API digest 与 SHA256SUMS 逐条一致（13/13）；12 条 `deploy/*` 在 archive 内。
  - B3/B4/B6/B7 复核：release body 含 upgrade-drill UNVERIFIED / real provider / 500-user / personal scope /
    `KNOWN-LIMITATIONS` 引用；`docs/37` 行 2/行 6 已更新；`acceptance.md` 已填命令/结果/回滚/风险/C17；
    证据引用改为 CI strict artifact + `target/evidence/release-1.0-rc1/ci/final/security.json`，旧 `security2` 标非权威。
  - 剩余 UNVERIFIED（honest，均不得声称已验证）：Docker upgrade/rollback drill 实跑、macOS 公证、Windows 真机、
    真实 provider/媒体/embedding 在线调用、500 用户生产容量、镜像层内 SBOM、personal×official release run。
- P2（不阻塞，建议 1.0.0 前修）：`KNOWN-LIMITATIONS.md:152-153` 仍写 "cargo deny/cargo machete 仅在 security.yml
  （hosted runner 未实跑）"，与 §1 VERIFIED 矛盾（security run 已实跑）；该句为保守表述、非 false-green，
  随 1.0.0 文档更新修正即可（修正会改变 rc.1 release asset 与仓库 digest 一致性，故不在 rc.1 重发）。
- 关联 spec：`specs/archive/2026-10-31-release-1-0-readiness/`（已归档）；`specs/changes/2026-10-30-hardening-release/`
  M6 acceptance 仍 `blocked`（Docker/证书/真机/CI 环境项）。
- 最终验收结论：APPROVED

## 11. Post-approval hotfix 复核（2026-11-01）

- 复核对象：hotfix commit/tag `v1.0.0-rc.1` → `5c0f29d4e918a22113ce6c3c77ed9f4e4f3c5cd6`（remote annotated tag object
  `a4f32aa86a0e1a6b96bbdeb269c3757dcb4088b1`，tagger 2026-10-31T16:55:47Z；remote `main` 同 commit；工作树 clean）。
  Delta vs 已批准 `33f59a1`：`8a61838`（Windows GUI subsystem、启动失败可见化、WebView2 构建目标）+ `5c0f29d`
  （boot fallback 样式作用域化），6 文件 / +116 行（CHANGELOG、main.rs、index.html、main.tsx、types.ts、vite.config.ts）。
- 代码 delta 核验：`apps/desktop/src/main.rs:7-11` `#![cfg_attr(all(not(debug_assertions), target_os = "windows"),
  windows_subsystem = "windows")]`；`apps/desktop/web/index.html` `#zk-boot` 作用域启动卡片 + 10s watchdog
  （`10000`，展示 UA/地址/错误文本）；`apps/desktop/web/src/main.tsx:29` `StartupErrorBoundary`
  （`getDerivedStateFromError`/`componentDidCatch`，失败显示可截图错误面板）；`apps/desktop/web/vite.config.ts:16-17`
  `target: "es2020"`、`cssTarget: "chrome105"`。
- 远端证据（只读复核）：CI `41219684321` = success（9/9 jobs；e2e log 14 passed、a11y 15 passed）；
  security `41219684298` = success（strict artifact exit 0、pass=12 / fail=0 / unverified=0 / unverified_external=2）；
  release `41222029806` = success（linux/windows/macos installers、dist、GitHub Release 全 success；
  `upgrade-drill` 仍为环境项 failure + continue-on-error，body 标 UNVERIFIED）。
- Release 刷新核验：`v1.0.0-rc.1` prerelease（published 2026-10-31T17:30:50Z，15 assets）；Windows
  `ZeoKeel_1.0.0-rc.1_x64-setup.exe` sha256 `feaf270f…`（8,768,902 B）；dist archive `1689d57c…`（315,956,339 B）；
  `KNOWN-LIMITATIONS.md` digest `0f93a3d4…` == 仓库 `sha256sum KNOWN-LIMITATIONS.md`（B2 一致性保持）；
  release body Pipeline run = `41222029806`；SHA256SUMS 25 条在 release job 解包 archive 后 `sha256sum -c` 全部 OK；
  13 个 loose asset 的 GitHub API digest 与 SHA256SUMS 13/13 一致。
- CHANGELOG：`1.0.0-rc.1` 段含 `### Windows 客户端修复（2026-10-31 hotfix）`（cmd 控制台/白屏/WebView2 target）；
  `docs/37` Windows 未签名声明不变；Windows 真机安装/升级仍 UNVERIFIED（真源 `KNOWN-LIMITATIONS.md` §2 与
  release body；`docs/37` 未单列真机行）。
- 结论：hotfix delta **APPROVED**；本记录既有 APPROVED 继续有效，artifact approval 现指 `5c0f29d4…`（tag `v1.0.0-rc.1`）。
- P2 非阻塞 follow-up：(1) release body 的 Changelog 被 `.github/workflows/release.yml:551-561` 的
  `changelog_section()` 在首个 `###` 处截断，当前 body 只显示版本标题+引言、**未展示 hotfix 子节**；建议修 generator
  或 `gh release edit` 补 notes（不影响已发布二进制与校验和）；(2) `docs/37` 无独立 Windows 真机行，真机 UNVERIFIED
  以 `KNOWN-LIMITATIONS.md` §2 为准（Lead 描述中 "docs/37 Windows 真机项" 与文件实际内容略有偏差）。


