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
