# 2026-10-30 · M6 加固/发布实现对抗性验证

- **评审对象**：`scripts/{security-audit,backup-restore,upgrade-drill,fault-injection,perf-budget,reproducible-build,sbom}.sh`、`e2e/m6-a11y.spec.ts`、`.github/workflows/security.yml`、`docs/33-37`、`CHANGELOG.md`、`CONTRIBUTING.md`、M6 spec/acceptance
- **评审者**：audit-arch（独立只读；仅写本记录）
- **结论**：CHANGES-REQUESTED
- **关联 spec**：`specs/changes/2026-10-30-hardening-release`
- 证据日期：2026-10-30；本记录为唯一新增文件，未改产品代码/spec。

## 1. 结论

**CHANGES REQUIRED（CHANGES-REQUESTED）。**

M6 脚本能跑出“绿色”，但多个绿是**证据语义错误**：

- `security-audit.sh` 只有静态 grep + 三个依赖工具；`cargo deny/audit/pnpm audit` 在本机是 `unverified`，JSON 却 `status: pass`；13 项安全门禁（认证/CSRF/SSRF/WASM/路径/命令/密钥/审计链/隐私）没有命令/证据。
- `reproducible-build.sh` 只比较 `zk`；`zk-server`/`zk-desktop`/admin/image 全在 `unverified`，却 `status: pass`。
- `backup-restore` 明确输出 `PITR/WAL UNVERIFIED`；没有 master.key 恢复、`/data` 回灌、整机 smoke。
- `fault-injection` 只有 DB down、audit 表 rename、kill -9；无 OOM/磁盘/网络/迁移中 kill、无账/审计/迁移一致性断言。
- `perf-budget` 的 `image_size_bytes`/`gateway_p50/p95` 为 `null` 仍 `status: pass`；mock 首 token 未测。
- a11y 只扫 admin 页面；client/portal 无矩阵；`security.yml` 的 a11y job 只有 echo。
- `upgrade-drill --full` 实际只跑 backup + dry-run，无 upgrade/rollback/restore。
- `admin-e2e.sh` 本轮 exit 1（M3 连接器失败）；M6 a11y 单测通过但整体非绿；CHANGELOG 仍 0.1.0，未 1.0。

## 2. 证据命令与结果

| 命令 | 结果 |
| --- | --- |
| `bash scripts/security-audit.sh` | **EXIT 0**：`26 findings, high_fail=0`；JSON 里 `cargo deny/audit/pnpm audit` 均 `unverified`，但 `status:"pass"` |
| `bash scripts/backup-restore.sh` | **PASS**：32 tables；输出 `PITR/WAL UNVERIFIED`；无 master.key 恢复、/data 回灌 |
| `bash scripts/fault-injection.sh` | **PASS**：db-down/audit-fail-closed（实际 500）/kill9 rows=2；无 OOM/磁盘/网络/迁移中 kill |
| `bash scripts/perf-budget.sh` | **PASS**：server 871ms、health 0.877ms；`image_size_bytes:null`、`gateway_p50/p95:null`，无 mock 首 token；status pass |
| `bash scripts/reproducible-build.sh` | **PASS**：仅 `zk` 两次 sha 相同；`unverified:["zk-server","zk-desktop","admin dist","image tar"]`，status pass |
| `bash scripts/upgrade-drill.sh --full` | **PASS**：先 backup-restore，再 dry-run；无 upgrade/rollback/restore |
| `bash scripts/admin-e2e.sh` | **EXIT 1**：10 passed / 1 failed（M3 admin connectors）；M6 admin a11y 已跑 |
| `bash scripts/login-e2e.sh` | **PASS**：4 passed |
| `cargo test --workspace` / clippy | **PASS** |
| `python3 scripts/spec-check.py` / docs / review | PASS（M6 draft warning；review 11 records） |
| `security.yml` a11y job | 只有 `echo "axe matrix runs via ..."`，未执行矩阵 |
| `CHANGELOG.md` | 仍 `0.1.0`，无 `1.0.0`/M6 release section completion |

## 3. P0（阻塞）

### P0-1 · security-audit 是 false green：依赖审计 unverified、13 项门禁未实现

- `scripts/security-audit.sh` 仅：Rust secret regex、`unsafe {`、`.unwrap/.expect`、`cargo deny/audit/pnpm audit` 存在则跑；**没有**：
  - 认证绕过/JWT 吊销/refresh reuse/CSRF/越权矩阵；
  - SSRF/rebinding、zip 穿越/炸弹/链接、WASM 逃逸、命令注入/env 白名单；
  - master.key/JWT/更新签名/OAuth 生命周期；
  - 审计哈希链/WORM、钱审计矩阵；
  - 隐私/日志运行期脱敏、错误响应/崩溃 dump；
  - pnpm audit 失败阈值、镜像扫描。
- 本机 JSON：`dependency-audit:cargo:deny|audit|pnpm:audit` 全部 `severity=unverified/status=unverified`，但顶层 `status:"pass"`、`high_fail=0`。
- 因此 `security-audit.sh` 0 高危不能证明 docs/21 §10 的 13 项门禁。

**修法**：13 项每项一条可执行命令/测试并进 JSON（rule/severity/owner/status），缺工具=红；强制 `cargo deny/audit`/`pnpm audit --prod`/trivy（或明确 `UNVERIFIED` 且**整体 fail**）；补 auth/CSRF/SSRF/zip/WASM/命令/密钥/审计链/隐私用例；静态规则扩展到 TS/config。

### P0-2 · reproducible-build 只比较 `zk`，却报 pass

- `scripts/reproducible-build.sh` 只 `cargo clean -p zk; cargo build -p zk`，比较 `target/debug/zk` sha；JSON 明确 `unverified:["zk-server","zk-desktop","admin dist","image tar"]`，但 `status:"pass"`。
- 验收 #6 要求 zk/server/desktop/admin dist sha 完全一致（镜像 UNVERIFIED）；当前未满足。
- 修：分别两次冷构建 `zk`/`zk-server`/`zk-desktop`/admin dist；镜像无 Docker 标 UNVERIFIED 且整体状态不得 pass；两次 sha 必一致；补 toolchain/lock/remap 断言。

## 4. P1（应修）

1. **backup/PITR 不完整**：`backup-restore` 输出 PITR/WAL UNVERIFIED；只 pg_dump，不是 spec 的 basebackup+WAL+recovery.signal；`/data` 只 tar 未回灌；master.key 恢复未验证；补真实 PITR 路径（或整体 fail/unverified）与 restore smoke。
2. **fault-injection 不完整**：只有 DB down/rename audit_log/kill9；未覆盖迁移中/事务中 kill、OOM、磁盘满、网络分区；audit 失败用 rename 表，不是 spec 的 REVOKE INSERT fail-closed；无账/审计/迁移一致性断言。
3. **perf false pass**：`image_size_bytes`/`gateway_p50/p95` null 仍 pass；mock 首 token 未测；500 用户容量未执行；应 fail 或明确 UNVERIFIED 且整体不 pass。
4. **a11y 矩阵不完整**：`e2e/m6-a11y.spec.ts` 只 admin；client/portal 无 axe/键盘/i18n 矩阵；`security.yml` a11y job 仅 echo，不执行。
5. **upgrade --full 名不副实**：只 backup-restore + dry-run；无 upgrade/rollback/restore；补真实（或明确 UNVERIFIED 并整体 fail）。
6. **admin-e2e 本轮 exit 1**：M3 admin connectors 失败；M6 a11y 单测通过但整体非绿；验收不能写 admin-e2e pass。
7. **1.0 文档/清单未完成**：CHANGELOG 仍 0.1.0；docs/37 全未勾选；版本决策/known limitations/Windows 未签名声明未落。
8. **SBOM/license 不完整**：`scripts/sbom.sh` 只 cargo metadata；缺 pnpm/deep OEM/license allowlist/deny；镜像 SBOM 无。
9. **security-audit 静态规则误报/漏报**：medium unwrap 26 条未在报告处置；TS/config 未扫；无 route permission 表比对实现。
10. **docs/34 声明 PITR/加密/密钥分离，但脚本未验证**；文档与证据不一致。

## 5. P2（建议）

- `security-audit` 输出 13 项独立 status，不要只 `high_fail=0`。
- a11y 矩阵按页面/客户端/门户分文件，CI 真跑而非 echo。
- perf 生成 JSON 字段齐全（mock_first_token/image_size/gateway p50/p95/500）。
- SBOM 固定 CycloneDX 1.5/工具版本；license allowlist/deny。
- 1.0 checklist 绑定 M0–M5 证据路径。
- 环境阻塞项（Docker/证书/平台/500）在 acceptance 保持 UNVERIFIED；有环境后补跑。

## 6. M6 退出条件对照

| docs/12 M6 退出 | 当前 |
| --- | --- |
| 安全评审无高危未修复 | **FAIL/UNVERIFIED**：依赖审计 unverified；13 项未跑 |
| 备份恢复演练成功；故障注入不损坏 | **PARTIAL**：pg_dump restore 32 表；PITR/WAL UNVERIFIED；故障仅 3 项 |
| 性能预算达标；可复现通过 | **FAIL**：image/gateway null 仍 pass；repro 仅 zk |
| 三平台+镜像+门户+文档齐备 | **UNVERIFIED/PARTIAL**：无 Docker/证书/runner；docs/37 未完成 |

## 7. 明确未验证（环境阻塞，不能写 complete）

Docker 镜像扫描/save/load/compose/PITR 实跑、三平台安装包、macOS 公证、镜像 tar 可复现、500 用户压测。这些属环境阻塞，不等同已通过；acceptance 必须保持 `UNVERIFIED`。

## 8. 结论

- **P0：2**：security-audit false green、reproducible 仅 zk false pass。
- **P1：10**：backup/PITR、fault 矩阵、perf false pass、a11y、upgrade、admin-e2e、1.0 文档、SBOM/license、静态审计处置、docs/34 不一致。
- **M6 实现不能 APPROVED**：先修 P0/P1，环境项标 UNVERIFIED，再由同一评审者复评。

- 关联 spec：`specs/changes/2026-10-30-hardening-release`。
- 评审结论：**CHANGES-REQUESTED**

## 9. 复评（2026-11-01，HEAD c1aebf9 / rc.1 已发布）

- 复核者：release-reviewer（独立只读；仅写本记录 §9；未改产品代码/脚本/spec）。
- 复核对象：`HEAD c1aebf9`（tag `v1.0.0-rc.1`，artifact commit `5c0f29d`；远端 CI/security/release 均已实跑）；
  证据 `target/evidence/release-1.0-rc1/{ci/final,phase-b,security}/`、`target/evidence/m6/`、GitHub Release
  `v1.0.0-rc.1`（15 assets）；本记录历史 §1–§8 保留不改。
- 复核方式：只读命令/文件核验（`gh run view/download`、`gh release view`、读取脚本/证据 JSON/docs）。

### 9.1 P0/P1 逐项状态

| # | 首评问题 | 复评状态 | 证据（file:line / 命令结果） |
| --- | --- | --- | --- |
| P0-1 | security-audit false green、13 项未实现 | **CLOSED** | `scripts/security-audit.sh` 实现 gate1–14（docs/21:131+ §10 共 14 行）；CI security run `41225234844`（head c1aebf9）artifact `security-evidence`：strict exit 0、overall `unverified_external`、**pass=12 / fail=0 / unverified=0 / external=2**（gate7 WORM 部署控制 ADR-068、gate9 release-only SBOM）；本机非 strict `target/evidence/release-1.0-rc1/ci/final/security.json` pass=10 / unverified=1（gate8 工具）/ external=3；旧 `security2/` 已标 superseded（`KNOWN-LIMITATIONS.md:136`），不再作证据 |
| P0-2 | reproducible 仅 `zk` 却 pass | **CLOSED** | `target/evidence/m6/reproducible.json`：zk / zk-server / zk-desktop / admin-dist 四项两次 clean build sha 一致（pass）；server-image-tar + strict-flag-set `unverified`；整体 `status:"unverified"`（无 false pass）；`docs/37:13` 行 5 同口径 |
| P1-1 | backup/PITR 不完整 | **CLOSED（残余 env-UNVERIFIED）** | `target/evidence/m6/backup-restore.json`：12 checks，9 pass（restore 到 fresh DB+/data、ready 470ms、users/audit/ledger/tables 2/1/0/32 一致、audit chain 0 broken、truncated dump 被 `pg_restore` 拒绝）/ 3 unverified（master.key 文件恢复、PITR/WAL、age/KMS 加密+异地），整体 `unverified`；WAL/PITR/加密/离线 key 恢复均列入 `specs/changes/2026-10-30-hardening-release/acceptance.md` §2 与 `docs/37:11`；首评允许"或整体 fail/unverified + restore smoke"，已满足 |
| P1-2 | fault-injection 不完整 | **CLOSED（残余 env-UNVERIFIED）** | `target/evidence/m6/fault-injection.json`：7 pass（DB down→readyz 503 且恢复、audit trigger 失败→HTTP 500 且 0 行、users/ledger/audit 一致、SIGTERM 10s）；5 unverified（OOM/磁盘/网络命名空间/迁移中 kill/PG crash 重启）逐项说明 env 不可本地执行；整体 `unverified` |
| P1-3 | perf false pass（null 仍 pass） | **CLOSED** | `target/evidence/m6/perf.json`：实测 server ready 993ms、health p95 0.61ms、admin 112419B、desktop 345968B、mock first token 83.47ms、gateway p50 87.46/p95 122.66ms；`failures:[]`、`unverified:["image_size","capacity_500_users"]`、note "null is never interpreted as pass"、整体 `unverified`；500 用户本地 smoke pass（`target/evidence/release-1.0-rc1/phase-b/b14-perf/load-smoke-500.json`：0 错误、gateway p95 46.53ms、98.41 req/s；生产容量仍 UNVERIFIED） |
| P1-4 | a11y 仅 admin；CI job 只 echo | **CLOSED** | `.github/workflows/ci.yml` a11y job 实跑 `scripts/login-e2e.sh` + `scripts/admin-e2e.sh`（ci.yml `Client a11y`/`Admin + portal a11y` steps）；run `41225234859` a11y job 全 steps success（client m2/m3/m4 + admin 10 页 + portal，0 serious/critical；log 15 passed），e2e 14 passed |
| P1-5 | upgrade --full 名不副实 | **CLOSED（env-UNVERIFIED，已记录不冒充 pass）** | `scripts/upgrade-drill.sh` `--docker`/`--full` 实现真实 backup→upgrade→rollback→restore（`upgrade-drill.sh:7-11`）；release workflow 在 hosted runner 实跑并上传 `upgrade-drill-evidence`（release run `41222029806`）；因 compose postgres unhealthy 失败（environment），job `continue-on-error`，证据 artifact status=fail、release body/acceptance §2 标 UNVERIFIED，不记 pass |
| P1-6 | admin-e2e exit 1 | **CLOSED** | CI a11y job `scripts/admin-e2e.sh` step success（run `41225234859`；log 15 passed 含 m6-a11y/m5-portal）；M3 连接器失败已不在当前缺口 |
| P1-7 | 1.0 文档/清单未完成 | **CLOSED** | `CHANGELOG.md` `1.0.0-rc.1` + Windows hotfix 子节；`docs/37` 行 2/5/6/7 已更新（CI strict/可复现/SBOM/Release）；`KNOWN-LIMITATIONS.md` 为残余 SSOT；`SPEC` 已归档 `specs/archive/2026-10-31-release-1-0-readiness/` |
| P1-8 | SBOM/license 不完整 | **CLOSED（残余 env-UNVERIFIED 已记录）** | `zk sbom` 覆盖 Cargo.lock + pnpm workspace 直接包（`tools/zk/src/main.rs:421` `pnpm_components`；`docs/09:330-341`）；release dist `SBOM/cyclonedx.json` 已发布并以 SHA256SUMS 校验；license allowlist/deny 在 `deny.toml` + CI `security.yml` 执行；镜像层内 SBOM/deep OEM 产物 SBOM 为 env/release-only residue（`KNOWN-LIMITATIONS.md` §2/§5）；`THIRD-PARTY.md` 未生成（`KNOWN-LIMITATIONS.md:131`）→ P2 |
| P1-9 | 静态规则误报/漏报（unwrap/TS/config/route permission） | **PARTIALLY CLOSED（blocker）** | unwrap 已处置：`Cargo.toml:58-59` `unwrap_used/expect_used=deny`，gate12 log 披露 328 hits 并注明 clippy 为可执行门禁；但 `security-audit.sh:531-535` 的 secret/log/unsafe 静态扫描仍只覆盖 `--include='*.rs'`（**TS/config 未扫**）；**route permission 表比对**仍无实现，且 `docs/11:65` 把"权限矩阵"列为关键门禁失败条件，而 `docs/24:245,272`/`docs/04:171` 明示 `[[routes]]`/`zk api`/permissions catalog 为未实现 backlog → 文档口径冲突 |
| P1-10 | docs/34 与证据不一致 | **PARTIALLY CLOSED（blocker）** | `docs/34-operations-runbook.md:3-5` 仍把 "PG `pg_dump -Fc`/WAL、加密且密钥分离、master.key 离线" 作为既成备份口径，未标 UNVERIFIED；虽然 acceptance §2 已明确 PITR/WAL/加密/master.key 恢复 UNVERIFIED，但 runbook 本身未对齐证据，仍属"文档与证据不一致" |

### 9.2 环境/发布 UNVERIFIED（不计 pass，已在 acceptance §2 + KNOWN-LIMITATIONS §2 记录）

Docker load / upgrade / rollback / restore drill（hosted runner 实跑但 compose postgres unhealthy，artifact 记录 failure，非 pass）、
PITR/WAL（`archive_mode=off`）、备份 age/KMS 加密与异地、master.key 离线恢复、macOS 公证（证书后）、Windows 真机安装/升级、
真实 provider/媒体/embedding 在线调用、500 用户生产容量（仅本地 smoke）、镜像层内 SBOM/deep OEM 产物 SBOM、personal×official release run。

### 9.3 阻塞项（复评 CHANGES-REQUESTED 原因）

1. **P1-9 TS/config 静态扫描缺失**：`scripts/security-audit.sh:531-535` 仅扫 `*.rs`；需扩展到 TS/TSX/配置（secret/log 模式）
   或在 `docs/21` §10 / `docs/18` §7.2 明确记录"不在当前门禁"并说明替代验证（ESLint/tsc/Ajv/schema-check），不得留下
   "静态扫描覆盖所有代码"的错误暗示。
2. **P1-9 route permission 矩阵口径冲突**：`docs/11:65` 声称权限矩阵为门禁失败条件，但 `docs/24:245,272` 与 `docs/04:171`
   说 `zk api`/permissions catalog/运行时 Register 对账未实现（backlog）→ 二选一：实现最小权限矩阵比对（可先复用现有
   `srv-auth-test.sh` 权限断言并在 docs/11 说明），或把 docs/11 该行改为 UNVERIFIED/backlog 并链接 docs/24。
3. **P1-10 docs/34 未对齐**：`docs/34:3-5` 的备份/加密/master.key 口径需标注 UNVERIFIED 或链接 acceptance §2；同时
   `scripts/backup-restore.sh:16-19` 的注释仍写 "server has no master.key/encryption implementation yet"，与 rc.1 已实现的
   AES-256-GCM 密钥环不符（应改为"offline master.key restore drill 未执行"并链接 `scripts/master-key-rotate.sh` 证据）。

### 9.4 P2（不阻塞，随 1.0.0 清理）

- `docs/37:11`（行 3）仍写 fault-injection "3 probes pass" 与 upgrade-drill "无 Docker"，与当前证据（7 pass / 5 unverified；
  hosted runner 已实跑但 env failure）不符，需更新为实际状态。
- `THIRD-PARTY.md` 未生成（`KNOWN-LIMITATIONS.md:131`）；license SPDX 清单应在 1.0.0 前补。

### 9.5 结论

- P0=0（P0-1/P0-2 CLOSED）；P1：8 项 CLOSED（其中 P1-1/P1-2/P1-5/P1-8 的残余为环境/发布 UNVERIFIED 且已记录），
  **P1-9、P1-10 仍 PARTIALLY CLOSED**（上述 3 个阻塞项）。
- 未发现新的安全/可复现/a11y false-green；M6 acceptance 仍 `blocked`（环境项），不因本复评改变。
- 复评结论：**CHANGES-REQUESTED**（修 9.3 三项后可复评 APPROVED）。

## 10. 复评（2026-11-01，blockers 修复后）

- 复核者：release-reviewer（独立只读；本轮仅写本记录 §10；未改产品代码/脚本/spec/docs）。
- 复核对象：`HEAD c1aebf9` + 工作树未提交修复（`docs/11`、`docs/21`、`docs/34`、`docs/37`、`scripts/security-audit.sh`、
  `scripts/backup-restore.sh`、`specs/changes/2026-10-30-hardening-release/acceptance.md`）；复核方式为只读 diff/命令核验。
- 本轮只读核验：`python3 scripts/docs-check.py` → exit 0（0 errors / 0 warnings / 41 files）；
  `python3 scripts/spec-check.py` → exit 0；`python3 scripts/review-check.py` → ok（28 records）；
  `bash -n scripts/security-audit.sh` / `bash -n scripts/backup-restore.sh` → exit 0 / exit 0；
  gate12 扫描 preview（与脚本同 pattern）→ secret-log hits **0**、secret-literal hits **0**。

### 10.1 9.3 阻塞项逐项状态

| blocker | 状态 | 证据（工作树 diff / 文件:行 / 命令） |
| --- | --- | --- |
| P1-9 · TS/config 静态扫描缺失 | **CLOSED** | `scripts/security-audit.sh:531-535` 现扫描 Rust log 宏 + TS/TSX/JS `.mjs` 的 `console.*`/`logger.*` secret 插值（排除 node_modules/dist/build/target），并对 `*.rs,*.ts,*.tsx,*.js,*.json,*.toml,*.yml,*.yaml`（packages/crates/apps/tools/deploy/.github）扫私钥/AKIA/`sk-` literal；本地 preview = 0/0 hits；`docs/21` §10 gate 12 命令文本已同步（diff `docs/21-security-privacy.md:159`） |
| P1-9/P1-10 · route permission 口径冲突 | **CLOSED** | `docs/11-testing-and-evidence.md:65` 已改为 **UNVERIFIED/backlog**，引 `docs/24` §4 与 `docs/04`「唯一真源（计划/未实现）」，并写明替代证据 = `scripts/srv-auth-test.sh` 手工全端点权限矩阵；`docs/24:245,270-272` 确证 `zk api`/permissions catalog/Register 对账为 backlog → 文档口径一致，无 false gate claim |
| P1-10 · docs/34 与证据不一致 | **CLOSED** | `docs/34-operations-runbook.md:5-6` 备份条目已标 **WAL/PITR、age/KMS 加密+异地、master.key 离线恢复演练 UNVERIFIED**，链接 `specs/changes/2026-10-30-hardening-release/acceptance.md` §2 + `KNOWN-LIMITATIONS.md` §2，并注明版本化 AEAD ring/轮换已实现（`crates/zk-kit-server/src/master_key.rs`、`scripts/master-key-rotate.sh`）；`scripts/backup-restore.sh:16-19` 头注释已从"未实现"改为"ring 已实现；offline restore drill NOT executed (UNVERIFIED)" |
| P2 · docs/37 行 3 陈旧数字 | **CLOSED** | `docs/37-release-1.0-checklist.md:11` 现为 fault-injection **7 pass / 5 env-unverified**、upgrade-drill 在 CI release run `41222029806` 实跑但 compose postgres unhealthy → failure + `continue-on-error` + 证据 artifact（UNVERIFIED，不记 pass） |

### 10.2 仍为环境/发布 UNVERIFIED（不计 pass；acceptance §2 + KNOWN-LIMITATIONS §2 已记录）

Docker load / upgrade / rollback / restore drill（hosted runner 实跑失败于 compose postgres unhealthy，artifact 记录，非 pass）、
PITR/WAL（`archive_mode=off`）、备份 age/KMS 加密与异地、master.key 离线恢复、macOS 公证（证书后）、Windows 真机安装/升级、
真实 provider/媒体/embedding 在线调用、500 用户生产容量（仅本地 smoke）、镜像层内 SBOM/deep OEM 产物 SBOM、personal×official release run。

### 10.3 THIRD-PARTY.md（P2 判定）

`THIRD-PARTY.md` 仍未生成（`KNOWN-LIMITATIONS.md:131` 已记录；根 `LICENSE` MIT 与 `deny.toml` license allowlist +
CI `security.yml` cargo-deny license 检查已覆盖依赖许可门禁）。**接受为 P2 非阻塞**：rc.1 已发布、该项属 1.0.0 文档/交付
清单（`docs/37` 行 5 已标"镜像/许可 UNVERIFIED"），不构成本 M6 复评阻塞；1.0.0 前补生成。

### 10.4 结论

- P0=0（P0-1/P0-2 已于 §9 复核 CLOSED）；P1-9/P1-10 两个 PARTIALLY CLOSED 项已 **CLOSED**；其余 8 项 P1 维持 §9 的
  CLOSED/环境-UNVERIFIED-documented 结论。
- 未发现新的安全/可复现/a11y/perf false-green；M6 acceptance 仍 `blocked` 仅因环境项（Docker/PITR/公证/真机/真实 provider/
  生产容量），与本复评一致。
- 复评结论：**APPROVED**（M6 实现复评通过；环境项保持 UNVERIFIED，不写 complete）。


