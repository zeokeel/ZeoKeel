# Round-11 独立评审（SBOM / schedule e2e / async turn / docs-command gate）

- 评审对象：`HEAD db9901efb44873410ebf03fe80131b216947a7ee`（`docs(spec): round-11 acceptance inputs`；工作树 clean，唯一未跟踪文件为本记录）。关键文件 SHA-256 见 §9。
- 评审者：**auditor-rust**（独立评审；未实现 round-11，未修改任何产品/生成物；仅写本记录）。
- 与实现者关系/独立性：round-11 由 Lead 与其他 agent 实现；评审者此前只负责独立任务，未参与本轮 `zk sbom`/`e2e/serve-schedule.sh`/`async_subagent_turn.rs`/`docs-command-check.py` 的编写或评审。
- 关联 spec：`specs/changes/2026-10-31-round11-e2e-and-sbom`（proposal/spec/design/tasks/acceptance）。
- 评审日期：2026-10-31
- 首评结论：**CHANGES-REQUESTED**（P0=0；P1=1；P2=6）。
- 复评日期/结论：**pending**（Lead 修复 P1 后由同一评审者复评）。

## 1. 评审范围

- `zk sbom`：`tools/zk/src/main.rs`（命令解析/组件收集/渲染/`--check`）、`docs/09` §8、`.github/workflows/ci.yml` docker job。
- schedule auto-run：`e2e/serve-schedule.sh`、`e2e/schedule-autorun.spec.ts`、`apps/desktop/src/automation.rs`（只读核对）、真实 serve/SQLite/fake upstream。
- async 真实 turn：`packages/client/agent-rig/tests/async_subagent_turn.rs`、`crates/zk-kit-subagent` `live_count`。
- docs-command gate：`scripts/docs-command-check.py`、`scripts/check.sh`、CI `rust` job、`docs/03`/`docs/06`/`docs/09`/`docs/10`/`docs/13` 的 `zk` 命令声明。
- 证据/验收：`specs/changes/2026-10-31-round11-e2e-and-sbom/{tasks,acceptance}.md`；真实 e2e/gates 复跑。

## 2. 门禁复跑（独立执行）

| 命令 | 结果 |
| --- | --- |
| `cargo fmt --all -- --check` | **EXIT 0** |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | **EXIT 0**（`target/evidence/audit/round11-review/clippy.log`） |
| `cargo test --workspace` | **EXIT 0**（419 个 test result 块，**430 passed / 0 failed**；`target/evidence/audit/round11-review/workspace-tests.log`） |
| `cargo test --workspace export_schema` + `git diff --exit-code` | **0 / 0**（无 schema 漂移） |
| `bash scripts/schema-check.sh` | **0**（schema-ts up to date） |
| `zk validate` / `compose personal --check` / `capability --check` / `docs --check` / `db lock --check` / `lint --duplicates` | **0 / 0 / 0 / 0 / 0 / 0** |
| `zk sbom --edition enterprise --profile default --channel official --output ... --check` | **0**（输出已生成；918 components） |
| `python3 scripts/docs-command-check.py` | **EXIT 0（但存在 false negative，见 P1-1）** |
| `python3 scripts/docs-check.py` / `spec-check.py` | 0 / 0 |
| `python3 scripts/agent-tool-matrix.py --check` | 0 |
| `pnpm -r typecheck` / `lint` / `test:unit` / `build` | **0 / 0 / 0 / 0**（`round11-review/pnpm-*.log`） |
| `bash scripts/m1-e2e.sh`（单独执行） | **EXIT 0（7 passed）** |
| `bash e2e/serve-schedule.sh`（单独执行） | **EXIT 0（1 passed，真实 serve/SQLite/fake upstream）** |
| `bash scripts/agent-e2e.sh` | **EXIT 0（9 passed）** |
| `cargo test -p zk-agent-rig --test async_subagent_turn`（含在 workspace） | **PASS**（`parent_turn_drives_async_subagent_start_status_and_interrupt ... ok`） |
| `python3 scripts/review-check.py`（写本记录前） | **1**（缺本记录）；写后因结论 CHANGES-REQUESTED 仍为 **1**（预期，Lead 修复并复评后转 0） |

说明：`zk compose --edition enterprise ... --check` 按现有实现只允许 personal baseline，返回 1 且提示用非 `--check` 构建；不是本轮回归（CI 使用 `compose enterprise` 非 check）。

## 3. zk sbom（正确性/确定性/CycloneDX/覆盖）

- 生成 `target/evidence/sbom/task69-a.json`：918 components（`pkg:cargo` 892，`pkg:npm` 26）；`bomFormat=CycloneDX`、`specVersion=1.5`、`version=1`；`serialNumber=urn:uuid:...`（版本/变体位合法，见 `derived_serial` 测试及实测）；`metadata.component`；所有组件 `type=library`、purl/bom-ref 一致、按 purl 排序、purl 唯一；Cargo checksum 写 SHA-256，workspace license 来自 `cargo metadata --no-deps --offline`；无 timestamp。
- 两次生成 byte-identical；`--check` 对生成文件返回 0；缺文件/内容不同返回错误（测试覆盖）。
- 覆盖诚实性：实现确实覆盖 Cargo.lock 全部 crate + `pnpm-workspace.yaml` globs 下 26 个直接 `package.json`；文档 §8 status 明确 pnpm transitive、镜像内 SBOM、tar 打包 UNVERIFIED。未发现“伪造 transitive 覆盖”。
- **P2-1**：`--edition/--profile/--channel` 仅经 `resolve()` 校验，不参与组件集合/输出；personal 与 enterprise SBOM **byte-identical**（实测 `cmp=0`）。`docs/09:324` 写“每产物 CycloneDX、深度 OEM 代码包必须出现”，但当前输出是 workspace 全量 lock，不能证明某些 OEM/edition 产物专属包集合；且 OEM fixture/deep 包未单独断言出现。应把文档降级为“工作区级 SBOM”或实现 composition/OEM 过滤与断言。
- **P2-2**：SBOM 单元测试只做字段/排序/`--check` 字节比较，**未用 CycloneDX 1.5 JSON Schema 校验**（本机无外部 schema/网络）；“CycloneDX 有效”目前是结构级证据，不是官方 schema 验证。CI artifact 只在 docker job 生成，真实 Docker/hosted runner 未在本地复跑（UNVERIFIED）。
- **P2-3**：`docs/09:324` 的“进交付目录与 tar；深度 OEM 代码包必须出现”仍作为规则表述，而 `docs/09:334-336` 同时把 tar/深度 OEM 标为 UNVERIFIED；表述互相拉扯（规则 vs 实现状态）应明确“当前仅生成工作区 SBOM，不打包进 tar”。

## 4. schedule auto-run e2e 真实性

- 单独执行 `bash e2e/serve-schedule.sh`：**1 passed**。它创建临时 `ZEOKEEL_HOME`/SQLite、写入 due `schedules.json`（origin=tool-schedule、interval=60、next_run_ms=0）、启动真实 `zk-desktop --serve` + fake OpenAI-compatible upstream，Playwright 观察到自动创建的 "Scheduled task" 会话及 fake upstream 返回的 assistant 消息；不是伪造 UI 数据。
- **P2-4**：该脚本**未接入** `scripts/check.sh` 或 CI `e2e` job（CI 只跑 `scripts/m1-e2e.sh`，见 `.github/workflows/ci.yml:97`）；acceptance 的“schedule e2e”只有本地证据，若未人工执行不会有 gate。建议加入 check/CI 或明确标注非门禁。
- **P2-5（测试隔离）**：schedule 与 m1 套件共用 `test-results/`、`target/evidence/round11`/generated baseline；本评审并发启动 m1+schedule 时，m1 的 mock compose 使 schedule 二进制变成 mock、schedule 首次失败（assistant 为 `（mock）收到`），单独重跑分别 0。脚本应使用独立 `PLAYWRIGHT_*` 输出目录/顺序执行，或 CI 不并发运行二者。
- 未发现真正的“fake auto-run”：单独执行路径通过真实 executor/SQLite/upstream。

## 5. async 真实 turn 集成

- `async_subagent_turn.rs`：parent replay model 调用 `subagent_start`，真实 child 线程经 `RigSubagentRunner` 运行；Phase B 直接调用 kit control tools 启动/查询/中断 slow child；断言 done/cancelled、live_count 归零、kit registry 状态一致。workspace 测试通过。
- 该测试是**真实线程**集成，不是纯 mock 状态计数；没有 async 浏览器 e2e，但 round-11 spec 允许直接 kit registry。未发现 P1。
- **P2-6**：Phase B 直接调用 control tools 而非父模型调用 `subagent_status`/`subagent_interrupt`；spec 允许直接验证，但覆盖的是工具 API + 线程，不是完整模型驱动控制链路。另 stress 仍为单线程顺序驱动（round-8 残留）。

## 6. docs-command-check 门禁（P1）

- **P1-1（阻塞）**：`scripts/docs-command-check.py:44-54` 对不在 `KNOWN_UNIMPLEMENTED` 的未知命令直接 `continue`（注释明确“only tracked backbone commands enforced”），且只取第一个 token（`db` 在 IMPLEMENTED 即放过 `zk db reset`）。这与 spec §4“若命令未实现且同一段落/表格未出现未实现/backlog...报错”不符，造成 false negative：
  - `docs/03:190-200` 仍列 `zk init`、`zk vendor`、`zk brand`、`zk version check`、`zk oem add`、`zk explain`、`zk diff`、`zk why`（均未标 backlog）；`docs/06:72` 的 `zk db reset`；实际 `./target/debug/zk init|vendor|brand|version|oem|explain|diff|why|doctor` 全为 `unknown command`，`zk db reset` 为 `unknown db subcommand: reset`。
  - 本地用临时 docs 验证：`zk init` 无标注 → gate **0**；`zk db reset` → gate **0**；仅 `zk api` 无标注 → gate **1**。即门禁只保护 4 个硬编码命令，不能防止本轮目标所述 false command claim；`docs/command-check=0` 是 false-green 证据。
  - 修法：从唯一 CLI 注册表/`zk --help` 生成 implemented 集合并对**所有**未知命令报错（含子命令/参数粒度）；为 `zk init`、`zk db reset`、已标注 `zk api`、未标注 `zk api` 增加负向自测；扫描范围至少覆盖 `AGENTS.md`/`docs/README.md`（或明确排除并声明）；把 `docs/03:190-196`、`docs/06:72` 等真实标为未实现/删除。
- **P2-7**：即使未知命令改为报错，当前 240 字符窗口的 backlog marker 仍是启发式（`:46-48`），可能被无关段落“计划/未实现”误放行；应记录匹配段落/表格或使用显式 HTML 标记。另 gate 只扫 `docs/*.md`，不扫 AGENTS/README。
- CI/`scripts/check.sh` 确实调用该脚本，但门禁语义不足，不能据此认为 docs command truth 已闭环。

## 7. docs/09 truth / false-green

- **P2-3** 已述：`docs/09:324` 与 `:329-336` 的 SBOM 状态/交付 tar/deep OEM 表述不一致；表格仍像 current rule。
- **P2-8**：`specs/.../tasks.md` 仍全部 `pending`、`acceptance.md` 仍 `in-progress`（本轮 HEAD 是 `docs(spec): round-11 acceptance inputs`，验收尚未回填）；acceptance “docs-command-check/schedule e2e 0” 依赖本记录所列本地执行，CI 未覆盖 schedule、未在 hosted runner 跑 Docker SBOM；不应在修复前声称全绿。
- 已核对为真的部分：SBOM 918 components/排序/确定性/`--check`；async test 430 workspace；m1 7 passed；schedule 1 passed（单独）；agent 9 passed；pnpm 四套 0；fmt/clippy/zk gates 0；docs/spec/schema/matrix 0；`export_schema` 无漂移。

## 8. P0 / P1 / P2

**P0：无。**

| # | 等级 | 位置 | 问题 | 修法 |
| --- | --- | --- | --- | --- |
| P1-1 | P1 | `scripts/docs-command-check.py:44-54`；`docs/03:190-200`；`docs/06:72` | docs-command gate 对未知命令和子命令直接跳过，`zk init`/`zk vendor`/`zk brand`/`zk version`/`zk oem`/`zk explain`/`zk diff`/`zk why`/`zk db reset` 等未实现命令在无 backlog 标注时仍 **gate 0**，与 round-11 §4 目标及 docs 真源冲突；实测上述命令均 unknown。 | 以 CLI 注册表/`zk --help` 驱动 implemented 集合，所有未实现命令/子命令必须标注 backlog 或删除；补临时 docs 负向自测；扫描 AGENTS/README（或声明排除）；同步 docs/03/06/09/10/13 的未实现命令。 |
| P2-1 | P2 | `tools/zk/src/main.rs:176-186,252-...`；`docs/09:324` | SBOM 的 edition/profile/channel 只 resolve 校验，不影响 components；personal/enterprise 输出 byte-identical，无法证明“每产物/OEM 深度包出现”。 | 按 composition/OEM 过滤组件或明确文档为 workspace 级覆盖；增加 edition/OEM fixture 断言。 |
| P2-2 | P2 | `tools/zk/src/main.rs:3933+` tests；CI docker job | 无官方 CycloneDX 1.5 schema 校验；仅字段/字节测试；Docker artifact 未本地/CI 实际验证。 | 加 schema validator（或 CI 用 cyclonedx-cli）与 artifact 提取断言；保留 UNVERIFIED。 |
| P2-3 | P2 | `docs/09:324` vs `:334-336` | SBOM “进交付目录与 tar、深度 OEM 必须出现”与实现状态 UNVERIFIED 冲突。 | 把表格改为当前“workspace SBOM 已实现；tar/镜像/深度 OEM UNVERIFIED”。 |
| P2-4 | P2 | `.github/workflows/ci.yml:97`；`scripts/check.sh` | schedule auto-run e2e 未接入 CI/check.sh，acceptance 的 schedule e2e 仅本地证据。 | 加入 CI 或明确非门禁；记录证据路径。 |
| P2-5 | P2 | `e2e/serve-schedule.sh`；`scripts/m1-e2e.sh` | 两套 e2e 共享 generated baseline/test-results，本评审并发运行时互相干扰（m1 的 mock compose 使 schedule 用 mock）；单独均 0。 | 独立 compose 目录/PLAYWRIGHT 输出目录，或 CI 顺序执行并加隔离说明。 |
| P2-6 | P2 | `packages/client/agent-rig/tests/async_subagent_turn.rs:176-220` | Phase B 直接调用 control tools，未覆盖父模型驱动 status/interrupt；stress 仍单线程顺序。 | 增加模型驱动控制链或明确 synthetic 说明。 |
| P2-7 | P2 | `scripts/docs-command-check.py:46-48,63` | 240 字符 marker 窗口启发式；只扫 docs 顶层，不扫 AGENTS/README；可能误放行无关“计划”文本。 | 段落/表格级标记；扫描范围与排除项显式化；负向自测。 |
| P2-8 | P2 | `specs/.../tasks.md`；`acceptance.md` | tasks 全 pending、acceptance in-progress；CI hosted/Docker/SBOM artifact 未验证。 | C19 通过后回填 tasks/acceptance；标注 UNVERIFIED 或补 CI 运行证据。 |

## 9. 评审对象哈希（关键文件）

| 文件 | SHA-256 |
| --- | --- |
| `tools/zk/src/main.rs` | `b41f5e301d819f0c566a58ef1313010b9723222d948e20f9358b21e1f62ee9fc` |
| `e2e/serve-schedule.sh` | `9d78b5a01a0e785b67d11575e2901839135e503bca688e49e897fd45a65e04f6` |
| `e2e/schedule-autorun.spec.ts` | `a67371573bd0abce1cccf3a5c5d0a29b6f00228027a09a13ba0a9abdd6ccf3ba` |
| `packages/client/agent-rig/tests/async_subagent_turn.rs` | `ca92a3b093801f7f5f87b191ca0e228bb496a0f3ab600634c996babb447e80d6` |
| `scripts/docs-command-check.py` | `845f514f620e2859b3fec356012f0de4ccea7003d9dd79957e8c707f6acf69bb` |
| `scripts/check.sh` | `b7c93e57c8bad360fb5b50e95a921c6473a051ecce5388d946cac5e8a7134b8b` |
| `.github/workflows/ci.yml` | `bc97b57663db9f3185f4967ee13683a020888e25b4408775a6e24cedb0b1fe74` |
| `docs/09-delivery-and-oem.md` | `5b19341e0d6de670800e4e4355fcbfc68c0ee4d26bfe353aa791ec84b12a33e3` |
| `docs/03-composition-and-ci.md` | `3a004c72bc6899c81cf8c9c95a99e5ebf83180fd41b4437be85c138eeae8d96b` |
| `specs/.../proposal.md` | `e252e81098782fd0a08fee6e70a3a65fe5d4ac434cef9052a4b721d3988cbcd3` |
| `specs/.../spec.md` | `d8f165fe3bde82440beb8b8e6bc498930d7f701c156e630669465954ce7f11e3` |

## 10. UNVERIFIED（环境限制）

- Docker 镜像构建、docker job 真实 hosted runner、SBOM 打包进镜像/tar；真实 OEM 私有索引/submodule；Tauri 真机/Windows；真实模型长时调度压测；cargo-deny 本地。
- CycloneDX 官方 schema validator 未安装/未联网（结构级检查已做，不称 schema 验证）。
- **本评审并发测试说明**：m1/schedule 首次并发运行失败（m1 的 mock compose 污染 schedule binary；trace artifact 冲突），单独运行均 0；该并发失败不计为产品 P1，但已记录测试隔离 P2-5。

## 11. 结论与状态

- 最新结论：**CHANGES-REQUESTED**（P0=0；P1=1；P2=8）。阻塞项为 docs-command-check 语义 false negative：未实现命令/子命令仍可在无 backlog 标注时通过 gate，导致 docs 真源仍含 false current claims（`zk init` 等，docs/03:190-200、docs/06:72）。
- 写本记录前 `review-check.py` 为 **1**（缺本记录）；写后仍为 **1**（本记录结论 CHANGES-REQUESTED），Lead 修复 P1 并复评改最新结论为 APPROVED 后应转 0。
- 本记录是唯一新增文件；未修改产品代码/生成物/committed 文件。

## 12. 复评（HEAD 78c259c，2026-10-31）

- 复评对象：`HEAD 78c259ca2cb124f08d45b3bb95e22aa96e5a38d1`（`fix(round11): docs command gate, backlog annotations, SBOM truth`；父 `db9901e`）。工作树 clean（唯一未跟踪文件 = 本记录）。
- 复评日期/结论：2026-10-31；**APPROVED**。P1-1 已闭环；原 P2-1/P2-3 已随文档修正闭环；P2-2/P2-4/P2-5/P2-6/P2-7/P2-8 作为非阻塞残余保留。
- 变更范围（8 files）：`scripts/docs-command-check.py`、`scripts/check.sh`、`docs/03`/`06`/`09`/`10`/`13`/`24`；无产品代码/契约/schema 改动。

### 12.1 P1-1 复核（已修复）

| 项 | 证据 |
| --- | --- |
| 实现的命令集合 | `IMPLEMENTED` 为 top-level + 子命令映射；`db` 仅允许 `lock`，`channel` 仅允许 `validate`/`verify`（`scripts/docs-command-check.py:16-28`） |
| 未知命令/子命令 | `scan_text` 对未知 top-level 或未允许子命令，无 backlog 标注时写 error（`:39-50`）；self-test 含 `zk init`、`zk db reset`、未标注 `zk api` 失败与 annotated pass（`:58-79`） |
| 扫描范围 | 顶层 `docs/*.md`（排 reviews/research/archive/generated/14）+ `AGENTS.md` + `README.md`（`:89-95`） |
| 实际 gate | `python3 scripts/docs-command-check.py --self-test` → 0；正常 gate → 0 |
| 负向实测 | 临时 docs：`zk init` → gate 1；`zk db reset` → gate 1；`zk api`（未标注）→ gate 1；`zk api`（未实现标注）→ gate 0 |
| 命令实测 | `./target/debug/zk init|vendor|brand|version|oem|explain|diff|why|doctor` 均 unknown；`zk db reset` → `unknown db subcommand: reset`；docs 现在均标注 backlog（`docs/03:190-200`、`docs/06:72` 等） |
| CI/check | `scripts/check.sh` 先跑 `--self-test` 再跑 gate；CI `rust` job 已调用 gate |

### 12.2 P2 残留复核

- **P2-1 已闭环**：`docs/09:324` 改为“命令已实现；进 tar/镜像打包 UNVERIFIED”；`:334` 明确 components 来自全 workspace lock + pnpm direct，personal/enterprise 输出可能相同、edition 仅校验组合不做过滤；不再声称每 edition 必然不同。
- **P2-3 已闭环**：`docs/09:324` 与 `:329-336` 不再把 tar/deep OEM 当当前已交付能力；明确 UNVERIFIED。
- **P2-2 保留**：仍无 CycloneDX 官方 schema validator/CI artifact 内容断言；本轮仅结构/确定性测试。
- **P2-4 保留**：schedule auto-run e2e 仍只由独立脚本执行，未接入 `scripts/check.sh`/CI（CI e2e 只跑 `m1-e2e.sh`）。
- **P2-5 保留**：schedule 与 m1 套件共享 generated baseline/test-results；并发运行时首次失败（m1 mock compose 污染 schedule 二进制），单独均 0；建议隔离或 CI 顺序执行。
- **P2-6 保留**：async real-turn 测试 Phase B 仍直接调用 control tools；stress 仍单线程顺序。
- **P2-7 保留（已缓解）**：marker 窗口扩至 500 字符并覆盖 AGENTS/README，但仍是启发式窗口；负向 self-test 已覆盖主要命令/子命令。
- **P2-8 保留**：round11 `tasks.md` 仍全 pending、acceptance 仍 in-progress；CI hosted/Docker SBOM artifact 未本地验证；Lead/验收回填。

### 12.3 复评门禁（78c259c）

| 命令 | 结果 |
| --- | --- |
| `python3 scripts/docs-command-check.py --self-test` | 0（8 self-test cases） |
| `python3 scripts/docs-command-check.py` | 0 |
| `python3 scripts/docs-check.py` / `scripts/spec-check.py` | 0 / 0 |
| `zk compose --edition personal --profile default --channel official --check` | 0 |
| `bash scripts/check.sh`（含 self-test/gate/docs/spec/review + pnpm build/test） | 0 |
| `python3 scripts/review-check.py`（本记录写后） | 0 |

### 12.4 复评哈希

| 文件 | SHA-256 |
| --- | --- |
| `scripts/docs-command-check.py` | `d4765b0d4f8fa96dc17447053ab139f09a60fefb49840343bb418cd97cb8bdd8` |
| `scripts/check.sh` | `91e8056e2828915dce3373c0412a994f2df4f38e1adefd4e822dc9a20add0eb0` |
| `docs/03-composition-and-ci.md` | `9528dbb68cae74a030d68982b89b87f6b38bdec15ef52567ee882f92cbe25b43` |
| `docs/06-server-runtime.md` | `908d4695cb9bc3a03e3aacee5591052068e8a17e17dc4e394cd2d2ab7f130b9e` |
| `docs/09-delivery-and-oem.md` | `8eaa2ab96b15ca0db4d01b2252187ccd122531df779b8734c6a8f61c5d3a80f8` |
| `docs/10-repo-and-build.md` | `2deedf33015020dae30a566f1e091d16280b66f53bc4064863dc1484b02f7319` |
| `docs/13-risks-and-adr.md` | `4b797028dcaaf9a30355f9834451fa0e267dbd3595ce59277b4811265c3db99a` |
| `docs/24-composition-and-runtime-spec.md` | `6c782c5c7f96d5818898d077c47192bec51f887a83132314ec568f80bec9c903` |

### 12.5 最终结论

- 复评结论：**APPROVED**（HEAD 78c259c；P0=0 / P1=0；P2-2/P2-4/P2-5/P2-6/P2-7/P2-8 为已记录非阻塞残余）。
