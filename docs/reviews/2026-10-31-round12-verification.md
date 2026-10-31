# Round-12 最终独立审计（目标 1–4 / 文档真源 / 全量门禁 / 残余）

- 评审对象：`HEAD 3c4f21b`（首评；`ci(e2e): run schedule auto-run e2e; add round-12 final audit spec`）。复评：`HEAD 3cb1ca2`（§10）；**终评对象：`HEAD 84ccf3f`（`docs(12): mark roadmap exit checklists as historical plan with evidence/UNVERIFIED pointers`；父 3cb1ca2；§11）**。**Post-approval note：`HEAD b717565` 为 CI-only（e2e job 追加 agent/login/admin；见 §12），不在 84ccf3f 评审对象内，不改变 APPROVED。** 终评工作树 clean（唯一未跟踪文件即本记录）。
- 评审者：**auditor-build**（独立终审；实现者未参与本记录；round-12 audit only）。
- 独立性：本记录由未实现 round-12 功能的 auditor-build 完成；Lead/其他 agent 提供实现与本地 e2e 证据，本审计独立复跑门禁/e2e/命令探测。
- 关联 spec：`specs/changes/2026-10-31-round12-final-audit`（proposal/spec/design/tasks/acceptance；proposal 已链接本记录）。
- 评审日期：2026-10-31
- 结论：**APPROVED**（终评 HEAD **84ccf3f**：P0=0 / P1=0；首评 P1-1/P1-2 与复评 P1-2R 均已修复；P2/backlog/UNVERIFIED 保留）。`docs/12` 顶部历史计划状态块已加（M0–M4 done + evidence pointers；M5/M6 软件/本地完成 + Docker/多平台/公证/私有 OEM UNVERIFIED；当前 gates 指向 AGENTS §6/docs24/reviews），docs-check/spec-check/docs-command 0，review-check 终评后 0。目标 1 achieved（软件/本地 e2e，交付环境 UNVERIFIED）；目标 2/3/4 partial/backlog，不得宣称全部完成（矩阵见 §3，残余见 §8）。详见 §11。

## 1. 审计范围

- 目标 1–4 对照：`docs/README`、`AGENTS.md`、`docs/01/07/09/12/19/23/24/38`、`docs/generated/AGENT-TOOL-MATRIX.md`、各 spec/acceptance/archive。
- 命令真源：`AGENTS.md` §6、`docs/03`、`docs/18`、`.github/workflows/{ci,security}.yml`、`scripts/check.sh`、`scripts/docs-command-check.py`、实际 `zk` 命令面。
- spec/acceptance/review：`specs/changes/**`、`specs/archive/**`、`docs/reviews/**`、`review-check/spec-check`。
- 全量门禁与 e2e：cargo/zk/schema/pnpm/docs/matrix/sbom + m1/schedule/agent/login/admin/client-upgrade/apps/updates/oem/srv-auth。
- 重点命令探测：`pnpm test:e2e`、`zk compose --edition enterprise ... --check`、cargo 供应链工具可用性。

## 2. 全量门禁复跑（HEAD 3c4f21b，独立执行）

| 命令 | 结果 |
| --- | --- |
| `cargo fmt --all -- --check` | **EXIT 0** |
| `cargo clippy --workspace --all-targets -- -D warnings` | **EXIT 0**（`target/evidence/audit/round12/clippy.log`） |
| `cargo test --workspace` | **EXIT 0**（419 blocks / **430 passed / 0 failed**） |
| `cargo test --workspace export_schema` + `git diff --exit-code` | **EXIT 0 / 0** |
| `bash scripts/schema-check.sh` | **EXIT 0** |
| `zk validate` / `zk compose --edition personal ... --check` | **0 / 0**（16 files） |
| `zk capability --check` / `zk docs --check` / `zk db lock --check` / `zk lint --duplicates` | **0 / 0 / 0 / 0**（45 capabilities；duplicates 61≤61） |
| `zk sbom --edition personal ... --check` | **EXIT 0** |
| `python3 scripts/agent-tool-matrix.py --check` | **EXIT 0** |
| `python3 scripts/docs-check.py` / `spec-check.py` | **0 / 0**（40 docs） |
| `python3 scripts/docs-command-check.py --self-test` / 正式 | **0 / 0** |
| `pnpm -r typecheck` / `lint` / `test:unit` / `build` | **0 / 0 / 0 / 0** |
| `python3 scripts/review-check.py` | 写本记录前 **1**（缺 round-12 记录，预期）；本记录 CHANGES-REQUESTED 后仍 1 |
| e2e 独立复跑 | 见 §4.2（全部 0） |

## 3. 目标 1–4 achievement matrix

| 目标 | 判定 | 证据 | 残余/限制 |
| --- | --- | --- | --- |
| **1. 企业版插件 + 服务端集成完整** | **achieved（软件/本地 e2e）**，交付环境 **UNVERIFIED** | `editions/enterprise.toml` 组合；`apps/server/src/main.rs` 合并 auth/users/content/skills/connectors/apps/updates/wallet/pricing/gateway/audit/settings/jobs；`login-e2e` 3、`admin-e2e` 11、`apps-e2e`、`updates-e2e`、`oem-e2e` 全 0；archive `m3-content-platform`/`wasm-app-center`/`oem-delivery` acceptance `completed`。 | Docker/镜像/多平台安装包/真实 OEM 私有索引/Windows/Tauri 真机 UNVERIFIED；M2 旧 acceptance 状态过期（P1-2）。 |
| **2. Agent 功能完整** | **partial（显式 backlog）** | `docs/generated/AGENT-TOOL-MATRIX.md`：多数 implemented；approval/subagent（sync+async）/schedule/hooks/workflow 等已测；`agent-e2e` 9、`schedule-e2e` 1 passed；`docs/38` §5.4 记录剩余。 | `terminal` partial（单会话）、`pwsh-persistent` unsupported、`goal` partial（无 round driver）、`ralph` partial（无 fresh context）、`agent-team` experimental/disabled、subagent fork 仅快照；见 §6 backlog。 |
| **3. rig-core 0.45 能力全支持** | **partial/backlog** | `docs/38` §5.2/§5.3：hooks、vector/embedding、image/audio/transcription/rerank、completion/usage/memory/extractor 已适配；capability 动态 available/unavailable；无网络 fake 测试。 | `docs/38` §5.4 明确：run-state 序列化/Recorder/observe/telemetry/effect、`ModelCatalog` 上下文窗口/压缩未接；rig `pdf`/`epub` **有意不启用**（ADR-067，lopdf MSRV/GPL）；enterprise media/embeddings 按 C7 不提供；真实 provider UNVERIFIED。 |
| **4. 一切可配置** | **partial（已文档化 fallback）** | `AppSettings` 契约 + 设置 UI（provider/model/agent/tools/schedule/theme/workspace/connectors/update）；`docs/19` 调度/自动化段；`docs/38` config 表；schedule 热更新 e2e；settings 校验/schema。 | 媒体/vector/web search/hooks/memory 等能力配置仍是 env（`docs/38` 明确 fallback），不在 UI；企业部署/OEM 配置走 env/channel；`settings` 不暴露所有 effective 状态。**没有发现未文档化的 schedule env 隐藏配置**；其他 env-only 均已在 docs/38/24 记录。 |

**结论**：目标 1 的代码/本地集成成立；目标 2/3/4 **不能声称已完成**，须以 partial + 显式 backlog/UNVERIFIED 接受。最终 acceptance/销售口径不得写"目标 2/3/4 全部 achieved"。

## 4. 文档/spec 真源审计

### 4.1 命令真源（P1-1）

- **`pnpm test:e2e` 不存在**：`AGENTS.md:185`、`docs/18-strictness.md:175` 列为当前 gate；实际根 `package.json scripts` 只有 `e2e`（无 `test:e2e`），`pnpm test:e2e` → **EXIT 1 `Command "test:e2e" not found`**。
- **`zk compose --edition enterprise ... --check` 不支持**：`AGENTS.md:199` 列为当前命令；实际 `zk` 返回 **EXIT 1**："compose --check only validates the committed baseline personal×default×official"；docs/24 §1.2 只承诺 personal 基线 `--check`。
- `cargo nextest/deny/machete/public-api/geiger` 在本机均不存在；`AGENTS/docs/18` 将其列为 CI 门禁，CI `security.yml` 仅安装 deny/audit/machete/geiger，nextest/public-api 未安装/未运行。这些应标环境 UNVERIFIED/backlog，而非当前绿。
- 已确认诚实：`zk api`/`zk lock`/`zk verify-lock`/`zk metadata snapshot` 在 docs/03/24 等已统一标 backlog；`zk api`/`zk lock`/`zk verify-lock`/`zk metadata` 实测 `unknown command`；`docs-command-check.py` 0（但它只校验 `zk` 命令，不能发现上述 pnpm/cargo/参数级 false claim）。
- `docs/09` SBOM 已实现 `zk sbom`；`zk sbom --check` 0；CI artifact step 存在。

### 4.2 spec/acceptance/review 状态

- **12 rounds（round3–round11）**：proposal approved；acceptance 均 `done`；review 记录 round2–round11 最新结论 **APPROVED**；round12 acceptance `in-progress`（本终审后由 Lead 更新）。
- **archive**：`m3-content-platform`、`wasm-app-center`、`oem-delivery`、`agent-parity` acceptance `completed`。
- **P1-2 历史 milestone acceptance 状态不闭合**：
  - `specs/changes/2026-10-30-m0-scaffold-minimal-demo/acceptance.md` 无 `状态`、checklist 未归档（proposal review `n/a（C19 前遗留）`）；
  - `2026-10-30-m1-personal-mvp/acceptance.md:4` **状态：进行中**；
  - `2026-10-30-m2-enterprise-server/acceptance.md:1-12` 全部 `[ ]`，且 `:10/:12` 仍把 **enterprise `zk compose --check`** 与 **`zk api --check`** 当 gate（后者明确 backlog，前者不支持）；
  - `2026-10-30-workspace-scoped-conversations/acceptance.md:7-8` 仍有未勾选项。
  - 这些与最终“所有 spec/acceptance 均 done/APPROVED 或显式 backlog/UNVERIFIED”冲突。修法：M0/M1（pre-C19 遗留）标 `done/legacy` 或归档；M2/workspace 按实际证据标 done 或明确 UNVERIFIED，并删除不存在/不支持的 gate 行；`hardening-release` 已显式 `blocked`，属合法环境阻塞，可保留。
- `docs/12-roadmap.md` 的 M0–M6 退出条件 checkbox 全部仍未勾选（如 `:109-115`、`:198-201`），与最终状态不符；应作为 P2 文档状态更新，不能作为当前完成证明。

## 5. 全量 e2e 复跑（独立执行；全部 EXIT 0）

| 套件 | 结果 |
| --- | --- |
| `bash scripts/m1-e2e.sh` | **0**（mock 7 + serve 7；含 schedule settings 持久化 spec） |
| `bash e2e/serve-schedule.sh` | **0**（1 passed；真实 auto-run 产生 assistant 消息） |
| `bash scripts/agent-e2e.sh` | **0**（9 passed） |
| `bash scripts/login-e2e.sh` | **0**（3 passed） |
| `bash scripts/admin-e2e.sh` | **0**（11 passed） |
| `bash scripts/client-upgrade-e2e.sh` | **0**（1 passed） |
| `bash scripts/apps-e2e.sh` | **0**（upload/approve/open/request/restart/delist/retire） |
| `bash scripts/updates-e2e.sh` | **0**（manifest/channel/version/artifact/portal） |
| `bash scripts/oem-e2e.sh` | **0**（channel/index/pin/compose/brand/residue） |
| `bash scripts/srv-auth-test.sh` | **0**（真实 PG auth/billing/audit/migrations/SIGTERM） |

证据：`target/evidence/audit/round12/{m1-e2e,schedule-e2e,agent-e2e,login-e2e,admin-e2e,client-upgrade-e2e,apps-e2e,updates-e2e,oem-e2e,srv-auth-test}.log`；e2e 后 personal compose `--check` 0，工作树恢复 clean。

## 6. P0 / P1 / P2

**P0：无。**

**P1（阻塞 APPROVED）**

| # | 位置 | 问题 | 修法 |
| --- | --- | --- | --- |
| P1-1 | `AGENTS.md:185`、`docs/18-strictness.md:175`（`pnpm test:e2e`）；`AGENTS.md:199`（enterprise compose `--check`）；`docs/18:180`（cargo nextest 等） | **已修复（3cb1ca2）**：根 `package.json` 添加 `test:e2e` alias，`pnpm test:e2e` 实测 0；AGENTS/docs18 改 `pnpm e2e`；AGENTS 删除 enterprise `--check` 改 write compose + cargo check；docs18 对 nextest/public-api 标 UNVERIFIED。 | 已完成；AGENTS 中 `cargo nextest/deny/machete` 仍无注释（P2-3 环境项，非阻塞）。 |
| P1-2 | `specs/changes/2026-10-30-m0.../acceptance.md`（无状态）；`m1.../acceptance.md:4`（进行中）；`m2.../acceptance.md:1-12`（全未勾选，`:10/:12` 含 enterprise compose `--check`、`zk api --check`）；`workspace.../acceptance.md:7-8`（未勾选）；`docs/12-roadmap.md:109-115,198-201`（全部未勾选） | **部分修复（3cb1ca2）**：M0/M1/M2/workspace acceptance 已标 done 并加 UNVERIFIED/环境备注，M2 错误 gate 已改；**但 `docs/12-roadmap.md` 未改**（`git diff 3c4f21b..3cb1ca2 -- docs/12-roadmap.md` 为空，`:59-65/84-89/109-115/134-137/179-183/198-201` 仍全部 `[ ]`），与 commit body 声称“M2/M5/M6 标历史计划/证据指针”不符。 | 对 docs/12 M0–M6 未完成 checkbox 加“历史计划/证据指针”或按实际完成勾选；或明确整份 roadmap 为历史计划（顶部标注），不要保留全空 checkbox 作为当前状态。 |
| P1-2R | `docs/12-roadmap.md:59-65,84-89,109-115,134-137,179-183,198-201` | **已修复（84ccf3f）**：顶部新增“历史计划/退出条件清单”状态块，M0–M4 done + acceptance/review evidence pointers，M5/M6 软件/本地完成 + Docker/多平台/公证/私有 OEM UNVERIFIED，当前 gates 指向 AGENTS §6/docs24/reviews；未勾选项明确保留为历史计划 not false-green。 | 已关闭；P1=0。 |

**P2（非阻塞；最终 acceptance 需保留为 residual/UNVERIFIED）**

| # | 位置 | 问题 | 建议 |
| --- | --- | --- | --- |
| P2-1 | 目标 2/3/4（§3） | 目标 2/3/4 是 **partial/backlog**，不能宣称全部完成：agent 工具 partial/unsupported/experimental；rig run-state/telemetry/effects/ModelCatalog 未接；pdf/epub 有意不启用；媒体/embeddings enterprise 不提供；部分能力配置仍 env-only。 | 在最终 acceptance/交付说明保留 §3 的 achieved/partial/backlog/UNVERIFIED 口径，禁止 false all-achieved 表述。 |
| P2-2 | `.github/workflows/ci.yml` e2e job（lines ~80-103） | CI e2e 只跑 `scripts/m1-e2e.sh` + `e2e/serve-schedule.sh`；agent/login/admin/client-upgrade/apps/updates/oem 仅本地跑，CI 不覆盖。 | 逐项加 CI 或明确这些仅本地证据；最终 acceptance 不要写成“CI 全 e2e 绿”。 |
| P2-3 | `AGENTS.md:191`/`docs/18:180-185`；security.yml | cargo nextest/public-api 未安装/未执行；deny/machete/geiger 只在 security workflow 安装，未在本机/主 CI 复跑。 | 标 UNVERIFIED/backlog 或加 CI 安装/执行。 |
| P2-4 | `scripts/docs-command-check.py` | 仅解析 `zk <command>`，不能发现 pnpm/cargo/`zk compose` 参数级 false claims（如 P1-1）。 | 增加命令/脚本存在性与参数校验，或人工评审清单。 |
| P2-5 | `docs/12-roadmap.md` | M0–M6 退出条件 checkbox 全部未勾选/未归档，作为历史计划未如实反映最终状态。 | 更新 checkbox 或明确“历史计划，完成状态见 archive/acceptance”。 |
| P2-6 | M0/M1 proposal review `n/a（C19 前遗留）`；M2/workspace acceptance checklist | 历史 milestone review/acceptance 证据不可与 round 评审混用；M2/workspace review APPROVED 但 acceptance 未归档。 | 在最终 acceptance 明确“pre-C19 legacy”或归档到 specs/archive，避免 false current status。 |
| P2-7 | `.github/workflows/ci.yml` integration/docker job；docs/09 SBOM | GitHub runner 的 PG/systemd、Docker 镜像/多平台、SBOM 打包进交付 tar 均未在本地验证；本地 srv-auth/SBOM 0 不等于 CI/交付绿。 | 保持 UNVERIFIED；真实 runner/Docker 环境补证据。 |
| P2-8 | `specs/changes/2026-10-31-round12.../tasks.md`、acceptance | 终审前 tasks 全 pending、acceptance in-progress；本记录写后由 Lead 更新 done/APPROVED。 | Lead 更新后归档。 |

## 7. UNVERIFIED（环境/交付项，不得写成已完成）

- GitHub hosted runner 的实际 CI 结果（integration/e2e/docker）；Docker 镜像构建/多平台安装包/公证；Tauri 真机、Windows 运行时。
- 真实模型/媒体 provider（image/audio/transcription/rerank/embeddings）的在线调用；真实调度长时压测。
- 私有 OEM 索引仓/submodule/deploy-key；真实客户交付环境。
- `cargo-deny`/audit/machete/geiger/public-api 本机执行（CI security 仅部分安装）。
- `zk api`/组合锁/metadata snapshot（docs/24 显式 backlog）。
- SBOM 打包进交付 tar/镜像内 SBOM（`docs/09` 已标 UNVERIFIED）。

## 8. 残余 backlog（最终 acceptance 必须保留）

- **目标 2**：`terminal` 单会话、`pwsh-persistent` unsupported、`goal` 无 round driver、`ralph` 无 fresh context、`agent-team` experimental/disabled、subagent fork 快照。
- **目标 3**：rig run-state/Recorder/observe/telemetry/effect、ModelCatalog 上下文窗口/压缩；pdf/epub 有意 disabled（ADR-067）；enterprise media/embedding 按 C7 不可用。
- **目标 4**：媒体/vector/web search/hooks/memory 等 env-only 能力配置（已文档 fallback）；UI 未展示所有 effective 配置状态；企业/OEM 部署 env/channel。
- **命令/交付**：`zk api`/组合锁/metadata（backlog）；Docker/Tauri/多平台/OEM 私有索引（UNVERIFIED）。

## 9. 结论与复评建议

- **首评：P0=0；P1=2；P2=8**。目标 1 代码/本地 e2e 成立；目标 2/3/4 为 **partial/backlog**，不可宣称全部完成；全量门禁/e2e（本机可执行项）独立复跑通过。
- **复评（3cb1ca2）：P1-1 已修复；P1-2 的 acceptance 部分已修复，但 docs/12 未改 → 复评时剩余 P1=1**。
- **终评（84ccf3f）：docs/12 历史计划状态块已补，P1-2R 关闭 → P0=0/P1=0，APPROVED**。最终 acceptance 保留 §7/§8 的 UNVERIFIED/backlog，不写 false-green。
- 本记录是唯一新增文件；未修改产品代码/生成物/committed 文件。`review-check` 终评后应为 0。

## 10. 复评（HEAD 3cb1ca2）

- 复评对象：`HEAD 3cb1ca2`（`docs(final-audit): executable command truth and historical acceptance closure`；父 3c4f21b）。工作树 clean（唯一未跟踪文件 = 本记录）。
- 复评结论：**CHANGES-REQUESTED（P0=0；P1=1 剩余：docs/12 roadmap 状态未修复；P2=8 保留）**。

### 10.1 P1 复核

| 首评 P1 | 复评结果 |
| --- | --- |
| P1-1 `pnpm test:e2e`/enterprise `--check`/cargo nextest claim | **已修复**：`package.json` 新增 `test:e2e` alias，独立跑 `pnpm test:e2e` → **EXIT 0**（m1 7+7 passed）；AGENTS/docs18 改 `pnpm e2e`；AGENTS 删除 enterprise `--check`，改 write compose + cargo check；docs18 对 nextest/public-api 标 UNVERIFIED。`docs-check`/`spec-check`/`docs-command-check` 均 0。 |
| P1-2 historical acceptance | **部分修复**：M0/M1/M2/workspace acceptance 已标 done + UNVERIFIED/环境备注，M2 错误 gate 已改；**但 `docs/12-roadmap.md` 无任何 diff**：`git diff 3c4f21b..3cb1ca2 -- docs/12-roadmap.md` 为空，`:59-65/84-89/109-115/134-137/179-183/198-201` 仍全部 `[ ]`，无历史计划/证据指针。commit body 声称已标历史计划与实际不符。 |

### 10.2 复评门禁（3cb1ca2，独立执行）

| 命令 | 结果 |
| --- | --- |
| `pnpm test:e2e`（新 alias） | **EXIT 0**（m1 7+7 passed） |
| `python3 scripts/docs-check.py` / `spec-check.py` | **0 / 0** |
| `python3 scripts/docs-command-check.py --self-test` / 正式 | **0 / 0** |
| `python3 scripts/review-check.py` | 写本记录后仍 **1**（CHANGES-REQUESTED，需 docs/12 修复+复评） |
| 复评 diff 范围 | 仅 AGENTS/docs18/package.json/M0/M1/M2/workspace acceptance；**无 docs/12**；首评本轮全量门禁/e2e 结果不受影响（本 diff 无产品代码/生成物/迁移）。 |

### 10.3 剩余 P1/P2

- **P1-2R（已于 84ccf3f 修复）**：`docs/12-roadmap.md` 顶部新增历史计划状态块；M0–M4 done + 证据指针；M5/M6 软件/本地完成 + UNVERIFIED；未勾选项明确为历史计划，not false-green。P1 关闭。
- **P2**：目标 2/3/4 partial/backlog；CI e2e 只覆盖 m1+schedule（其他仅本地）；AGENTS cargo nextest/deny/machete 注释不完整；docs-command-check 仅 zk 命令；CI runner/Docker/OEM/SBOM tar/真实 provider 等 UNVERIFIED（详见 §7/§8）。docs/12 顶部 “backog” 为 typo（cosmetic P2）。

- 终评结论：**APPROVED（HEAD 84ccf3f；P0=0 / P1=0；P2/backlog/UNVERIFIED 保留）**；review-check 终评后应 0。

## 11. 终评（HEAD 84ccf3f）

- 终评对象：`HEAD 84ccf3f`（`docs(12): mark roadmap exit checklists as historical plan with evidence/UNVERIFIED pointers`；父 3cb1ca2）。工作树 clean（唯一未跟踪文件 = 本记录）。
- 终评验证：
  - `docs/12-roadmap.md` 顶部新增状态块（`:7-11`）：历史计划/退出条件清单，不是当前施工状态；M0–M4 done 且指向 `specs/changes/2026-10-30-*/acceptance.md` 与 `docs/reviews/`；M5/M6 软件/本地完成，Docker/多平台/公证/私有 OEM UNVERIFIED；未勾选 M2/M5/M6 保留为历史计划 not false-green；当前 gates 指向 AGENTS §6/docs24/reviews。
  - `git diff 3cb1ca2..84ccf3f` 仅 `docs/12-roadmap.md` 6 行新增；无产品代码/生成物/迁移/命令改动。
  - `python3 scripts/docs-check.py` → **0**；`spec-check.py` → **0**；`docs-command-check.py` self-test+正式 → **0**。
  - 首评全量门禁/e2e（HEAD 3c4f21b）独立结果保持有效（430 tests、全部 e2e 0、srv-auth 0、SBOM --check 0）；P1-1 的 `pnpm test:e2e` alias 在 3cb1ca2 已独立验证 0。
- 终评结论：**APPROVED（HEAD 84ccf3f；P0=0 / P1=0；P2/backlog/UNVERIFIED 保留）**。

## 12. Post-approval CI-only change（HEAD b717565）

- 对象：`HEAD b717565`（`ci(e2e): run agent, login and admin e2e jobs`；父 `dd2ac9e`），仅 `.github/workflows/ci.yml` +12 行；**无产品/runtime/contracts/generated/docs 改动**。此提交在 round-12 评审对象 `84ccf3f` 之后，不改变已批准对象的结论。
- 内容：e2e job 在 m1 + schedule 之后追加 `scripts/agent-e2e.sh`、安装/启动本地 PostgreSQL、运行真实 PG 的 `scripts/login-e2e.sh` + `scripts/admin-e2e.sh`；既有 m1/schedule 步骤与 artifact 上传不变。
- 复核：
  - YAML `yaml.safe_load` 成功；e2e steps 顺序为 m1 → agent → schedule → PostgreSQL install/start → login+admin → upload。
  - 本机已独立跑过 agent-e2e **0**、login-e2e **0**、admin-e2e **0**（见 §4.2）；CI 真实 PG 依赖 `sudo -u postgres`/localhost:5432，与脚本本机假设一致。
  - `docs-check`/`spec-check`/`review-check` 不受影响（本 commit 无 docs/spec/reviews 变化）；review-check 当前 **0**。
- 结论：**APPROVED 仍成立（P0=0 / P1=0）**；该 CI-only 改动无产品/runtime/docs 行为，不构成 P0/P1。**GitHub hosted runner 上的 PostgreSQL 安装/启动与三条 e2e 仍未实际执行 → UNVERIFIED**，不得据此写“CI 全 e2e 已绿”；仍以 evidence policy 为准。
