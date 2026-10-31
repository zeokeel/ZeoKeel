# 2026-10-31-release-1-0-readiness · 提案（1.0 发布就绪）

- 状态：`approved`（用户 2026-10-31T13:08 拍板）
- Owner：lead（Agent Teams）
- 日期：2026-10-31
- 关联：`docs/37`、`docs/11`、`docs/12` M6、`docs/09`、`docs/18`、`docs/21`、ADR-033/037/041/059/067、`docs/reviews/2026-10-31-round12-verification.md`
- 评审：`docs/reviews/2026-10-31-release-1-0-readiness.md`
- 变更预算：跨域；按 §8 工作流分片提交，每片独立 `verify`；>10 文件，按片拆 spec 任务

## 1. 背景 / 问题

HEAD `d0a1c95` 的实测结论（2026-10-31，证据 `target/evidence/release-check/`）：

- **软件本地门禁基本全绿**：Rust fmt/clippy/438 tests/schema、TS 26 包 typecheck/lint/153 tests、13 项 `zk`/治理门禁、10 套 e2e（真实 PG）全通过；唯一红门禁是 `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps`（exit 101，`packages/server/jobs/src/lib.rs:339` 链接私有常量）。
- **发布工程未起步**：0 git tag；版本全为 `0.1.0`；`docs/37` 7 项全未勾；M6 acceptance `blocked`、M6 评审 CHANGES-REQUESTED；无 Docker/trivy/证书/真机/真实 provider key；`deploy/updates/clients/...AppImage` 是 18 字节占位；`build-dist.sh` 无真实输入即 exit 2；release workflow 从未运行。
- **CI 从未运行**：本地领先 `origin/main` 270 个提交（远端仅初始提交 `2258944`），GitHub Actions 无运行记录；`ci.yml` 未执行 `cargo doc`、`pnpm -r typecheck/lint/test:unit`，`security.yml` 安装 geiger 但从不调用、a11y job 只有 `echo`，apps/updates/oem e2e 不在 CI。
- **文档真源失真**：`CHANGELOG.md` 把 M0–M6 写成完成（M6 实际 blocked）；根 `README.md` 仍是"M0 骨架"；`AGENTS.md` §1 仍写"M5–M6 待做"；`docs/20` 引用不存在的 `scripts/day0-check.sh`、`scripts/dev-all.sh`；仓库无 LICENSE 文本（仅 Cargo.toml 声明 MIT）；无 known limitations。
- **工程债**：2 处 `as T` 被 `eslint-disable` 压制（`packages/admin/ui-kit/web/src/index.ts:39,76`，ADR-059 例外已在 M2 硬退出条件后过期）；personal 组合仍生成 `apps/server` 目标（`targets` 解析后未使用）；`security-audit.sh` 诚实地 exit 2（3 个依赖审计工具 unverified），`docs/21` §10 13 项门禁无可执行证据；威胁模型无独立评审记录。

不做的后果：项目停留在"内部 Beta"，无法产出/验证任何正式交付物，且文档口径与事实冲突会持续误导后续施工与发布。

## 2. 目标

1. **门禁全绿且 CI 强制执行**：修 rustdoc 1 处；`cargo doc -D warnings`、`pnpm -r typecheck/lint/test:unit`、a11y、apps/updates/oem e2e 进入 CI；`geiger`/`nextest`/`public-api` 要么 CI 真跑，要么从"当前门禁"清单移除并标 UNVERIFIED（禁止 false claim）。
2. **发布流水线可产出真实交付物**：`release.yml` 校验 tag/version、创建 GitHub Release、上传三平台安装包 + Docker 镜像 + SBOM；`build-dist.sh` 只接受真实产物；在授权远端实跑一次。
3. **安全门禁可执行**：`docs/21` §10 13 项落为逐项可执行检查/测试（本地可跑者本地跑，需外部工具者 CI 跑），`security-audit.sh` 输出逐项 `pass/fail/unverified`；补威胁模型独立评审记录。
4. **M6 演练补齐**：backup→restore（master.key/data 回灌；PITR 视环境 UNVERIFIED）、upgrade/rollback drill（Docker 环境 CI）、perf 预算真实值、可复现构建覆盖 server/desktop/admin dist（磁盘/时间允许时）。
5. **版本与文档一致**：`1.0.0`（或用户选定）同步 `Cargo.toml`/`package.json`/`tauri.conf.json`/`CHANGELOG`/`docs/37`；`README`/`AGENTS` 状态与事实一致；新增 known limitations 与 LICENSE；修 `docs/20` 死链脚本；历史归档 acceptance 不改结论。
6. **残余项可执行且不 false-green**：所有环境阻塞项（Docker 镜像、macOS 公证、Windows 真机、真实 provider/媒体在线调用、500 用户压测、GitHub hosted runner 实跑）都有 CI/后续可执行路径与明确 UNVERIFIED 记录。
7. **本地可产出部分作为 RC 证据**：若本机 Tauri 构建可行，产出 Linux AppImage/deb + SBOM + checksums；否则记录原因并保留 CI 路径。

## 3. 非目标

- **按用户 D2 补齐 R9 功能缺口**（工具/rig/配置/a11y/i18n/真实环境 harness）；仅以下 by-design 项不在补齐范围：`pdf`/`epub`（ADR-067：lopdf MSRV / GPL 冲突）、企业版媒体/embeddings（C7：企业 AI 只走网关 Responses）、`tool-cordis` 运行期插件（C2）、Windows 代码签名（ADR-033）、服务端自动升级（ADR-041）。rig 0.45 若仅有私有 API（如 live runner 检查点），实现可验证子集并将不可实现项写入 known limitations，附 rig 源码证据，需用户在最终验收前确认该口径。
- 不做 Windows 代码签名（ADR-033）；不做服务端自动升级（ADR-041）。
- 不改已归档/历史 acceptance 的结论；不回改历史迁移（只前向）。
- 不引入运行期插件加载/运行期 edition 切换（C2）。
- 不在本次新增产品功能（除发布门禁所必需的最小接线）。

## 4. 用户与场景

| 用户 | 场景 | 期望 |
| --- | --- | --- |
| 内部/试用用户 | 安装 1.0.0 客户端（Linux/macOS/Windows） | 可安装、可升级、版本号一致、known limitations 明示 |
| 企业管理员 | 导入镜像 + `compose up` + 门户下载客户端 | 有可验证镜像 tar/compose/SBOM，升级/回滚有演练证据 |
| 发布负责人 | 打 tag 触发 release workflow | 三平台包/镜像/SBOM/SHA256 一次产出，`zk channel verify` 过，GitHub Release 生成 |
| 安全/合规评审 | 看 13 项安全门禁与威胁模型 | 每项有可执行命令与状态，不 false-green |
| 维护者 | 看 README/AGENTS/CHANGELOG | 当前状态与事实一致，能按文档复跑门禁 |

## 5. 成功指标

- `cargo fmt --check` / `clippy --all-targets --all-features -D warnings` / `cargo test --workspace` / `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` / `cargo test --workspace export_schema` + `git diff --exit-code` 全 0。
- `pnpm -r typecheck` / `lint` / `test:unit` / `build` 全 0；`pnpm e2e` 与 §6 e2e 套件全 0。
- `zk validate/compose --check/capability --check/docs --check/db lock --check/lint --duplicates/sbom --check` + docs/spec/review/docs-command/matrix/schema 全 0。
- `release.yml` 语法与步骤可审计；授权远端实跑后产出真实三平台包 + 镜像 + SBOM + SHA256SUMS，`zk channel verify` 通过。
- 版本/文档/CHANGELOG/docs/37/README/AGENTS 一致；`docs-check` 0 warning；known limitations 覆盖全部残余 UNVERIFIED。

## 6. 风险与未知

| 风险/未知 | 影响 | 缓解/验证 |
| --- | --- | --- |
| 根分区仅余 ~13G（96% 用），冷构建三平台/两次可复现构建可能不足 | 高 | 优先 CI 矩阵，本地只做必要构建；先跑 `df` 预算检查 |
| 无 Docker/trivy/证书/真机/真实 provider key | 高 | 镜像/公证/真机/在线调用交 CI 或后续环境；明确 UNVERIFIED，禁止声称已验证 |
| 公开仓推送不可逆（270 commits + 1.0 release） | 高 | 需显式授权 D3；先本地全绿再推 |
| 1.0 口径与功能 partial 不符 | 中 | W5 强制 known limitations 覆盖；发布说明不得写"功能全部完成" |
| 跨工作流改 `.github/workflows/` 与 `docs/` 写入冲突 | 中 | 互斥写作用域 + 依赖排序；Lead review 最终 diff |
| CI 首次运行可能暴露本地未发现的问题（缺系统库/网络/runner 差异） | 中 | 先 dry-run 语法检查；按失败日志迭代修复 |

## 7. 需要用户拍板

- [ ] **D1 版本号**：`1.0.0`（按 docs/37，推荐）/ `1.0.0-rc.1`（先 RC，等 CI 实跑后转正）/ 保持 `0.1.0` 公开 Beta。
- [ ] **D2 功能范围**：仅发布工程（agent/rig partial 写 known limitations，推荐）/ 同时补齐功能缺口（另立多轮 spec，显著延长）。
- [ ] **D3 远端授权**：是否授权 push 到公开仓 `zeokeel/ZeoKeel` 并触发 GitHub Actions/release（270 个本地提交、远端仅初始提交；推送不可逆，release 会公开分发）。
- [ ] **D4 环境项口径**：Docker 镜像、macOS 公证、Windows 真机、真实 provider 在线调用、500 用户压测、hosted runner 实跑允许以"CI 可执行路径 + UNVERIFIED 记录"收尾。

> **已拍板（2026-10-31T13:08，用户）**：
> - D1 版本号 = **`1.0.0-rc.1`**（先 RC，CI/发布实跑后按需转 1.0.0）。
> - D2 功能范围 = **同时补齐全部功能缺口**（R9；done 口径见 spec §1 R9）。
> - D3 远端 = **授权 push 到公开仓 `zeokeel/ZeoKeel` 并触发 GitHub Actions/Release**（先分支后 main/tag）。
> - D4 环境项 = **允许"CI 可执行路径 + UNVERIFIED"收尾**（Docker/公证/真机/真实 provider/压测/hosted runner）。

## 8. 变更预算与拆分

- 预计改动文件：~40–60（含 docs/CI/scripts/版本文件）；按 7 个工作流分片，每片 ≤10 文件/≤400 行，独立 verify + 提交。
- 是否需要拆成多个 spec：不拆（同一发布目标）；工作流之间用任务依赖排序。

| # | 工作流 | 主要产物 | 写作用域（相对仓库根） |
| --- | --- | --- | --- |
| W1 | Rust 门禁修复 | rustdoc 1 处、CI rust job 加 `cargo doc` | `packages/server/jobs/`、`.github/workflows/ci.yml` |
| W2 | CI/发布流水线 | ci/security/release workflow 补齐；tag/version 校验、GH Release、a11y、e2e 矩阵 | `.github/workflows/` |
| W3 | 安全门禁 | `scripts/security-audit.sh` 13 项可执行化、威胁模型评审记录 | `scripts/security-audit.sh`、`docs/reviews/`、`docs/33` |
| W4 | M6 演练 | backup/restore、upgrade-drill、perf、reproducible 脚本与证据 | `scripts/`、`target/evidence/m6/`（不入库） |
| W5 | 版本与文档 | 1.0.0、CHANGELOG、README、AGENTS、docs/37、known limitations、LICENSE、docs/20 | 根文件、`docs/`、`Cargo.toml`、`package.json`、`apps/desktop/tauri.conf.json` |
| W6 | 严格性债务 | 删除 2 处 `as T` 压制（改为运行时校验边界） | `packages/admin/ui-kit/web/src/`、必要时 `packages/contracts/` |
| W7 | 发布候选 | 本地 Linux 安装包/SBOM/checksums 或 UNVERIFIED 说明；dist dry-run 审计 | `scripts/`、`deploy/`、`target/evidence/` |
| B | R9 功能补齐 | 工具多会话/pwsh-persistent/goal/ralph/agent-team、rig 可验证子集（effect 记录/usage/messages_in/catalog window/compaction）、config 全量、a11y/i18n、真实环境 harness | 见 spec R9 与 tasks B1–B15 |

## 9. 复用与已有实现（C17）

- 已搜索：`docs/generated/CAPABILITY-INDEX.md`（45 capability）、`docs/generated/PACKAGE-INDEX.md`（personal 55 / enterprise 86 包）、`scripts/` 现有 `security-audit.sh`/`backup-restore.sh`/`upgrade-drill.sh`/`perf-budget.sh`/`reproducible-build.sh`/`sbom.sh`/`build-dist.sh`、`docs/21` §10 门禁表。
- 可复用：`scripts/schema-check.sh`、`scripts/docs-command-check.py`、`scripts/agent-tool-matrix.py`、`zk sbom`、`zk channel verify`、`scripts/restore-baseline.sh`、`e2e/*.spec.ts`（a11y 已有 `m6-a11y.spec.ts`）。
- 本次新增可复用能力：无新 `@capability`；只扩展既有脚本/CI/文档。W6 若引入 admin 响应运行时校验，优先扩展 `packages/admin/ui-kit/web/src/validation.ts`（已有 Ajv 入口），不新建 utils。
- 为什么不能复用/扩展：rustdoc 修复是编译错误修复；CI/发布 workflow 无同类实现；版本/文档需就地更新。W6 若无法在契约层生成响应 schema，需在 spec 冻结最小方案（禁止新增第二套校验实现）。
