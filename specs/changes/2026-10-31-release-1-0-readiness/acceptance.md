# 2026-10-31-release-1-0-readiness · 验收与证据

- 验收人：lead（AI agent）+ 独立评审 `release-reviewer`（phase-3 终评）
- 日期：2026-10-31
- PR/commit：`v1.0.0-rc.1` tag（annotated）→ rc.1 最终 commit；GitHub Release `v1.0.0-rc.1`

## 1. 验收命令与结果

| 命令 | 结果 | 证据路径 |
| --- | --- | --- |
| `python3 scripts/spec-check.py` | pass（0 errors） | CI `docs` job；`target/evidence/release-1.0-rc1/lead/` |
| `python3 scripts/docs-check.py` | pass（0 errors / 0 warnings, 41 files） | CI `docs` job |
| `zk validate` | pass | CI `rust` job `Baseline manifest + generation gates` |
| `zk compose --edition personal --profile default --channel official --check` | pass（17 files up to date） | 同上 |
| `zk capability --check` | pass | 同上 |
| `zk lint --duplicates` | pass（61 ≤ 61 ratchet） | 同上 |
| `cargo test --workspace` | pass（438 tests） | CI `rust` job `Test (cargo test)` |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | pass | CI `rust` job |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` | pass | CI `rust` job `Rustdoc` |
| `cargo test --workspace export_schema && git diff --exit-code` | pass（generated stable） | CI `rust` job `Generated schemas stable` |
| `pnpm -r typecheck && pnpm -r lint && pnpm -r test:unit && pnpm -r build` | pass | CI `web` job |
| e2e（m1/schedule/agent/login/admin/client-upgrade/apps/updates/oem/srv-auth） | 10/10 pass | `target/evidence/release-1.0-rc1/final-e2e/` |
| `bash scripts/security-audit.sh`（本机非 strict） | exit 2：pass=10 / fail=0 / unverified=1（gate 8 本机工具）/ unverified_external=3（gate 7/9/10） | `target/evidence/release-1.0-rc1/ci/final/security.json` |
| CI `security.yml` strict（hosted runner，PG + 工具 + 镜像 + trivy） | **exit 0：pass=12 / fail=0 / unverified=0 / unverified_external=2**（gate 7 WORM 部署控制、gate 9 release-only SBOM） | security run artifact `security-evidence` |
| release workflow（tag `v1.0.0-rc.1`） | 三平台真实安装器 + 镜像 tar + SBOM + SHA256SUMS + dist 归档 + GitHub Release | GitHub Release `v1.0.0-rc.1`；run evidence `release-evidence` |

## 2. 手工检查

- [x] 客户端六态/键盘/a11y：B12 axe 矩阵（client m2/m3/m4 + admin 10 页 + portal zh/en）0 serious/critical；m1 reduced-motion 在真实 serve carrier 下执行；证据 `target/evidence/release-1.0-rc1/phase-b/b12b14/`。
- [x] 权限/越权/非法输入：srv-auth-test 全端点权限矩阵 pass；admin e2e 11/11；WASM 应用沙箱与 zip hostile-entry 测试 pass。
- [x] 离线/超时/重试/取消：agent rig e2e（fake upstream）、client-upgrade e2e（真实 426 + UpgradeGate）pass。
- [x] 升级/回滚/兼容：企业版 1:1 更新 e2e + 个人版 GitHub Releases 更新链路本地验证；Docker upgrade/rollback drill 为环境项（UNVERIFIED，见 §5）。
- [x] 组合裁剪 A18：personal `targets=["desktop"]` 生成 server stub（exit 2，零 srv deps）；enterprise compose 生成真实 server，CI 断言 srv 依赖存在且 stub marker 不存在。

## 3. 文档与决策

- [x] SSOT 文档已更新：`KNOWN-LIMITATIONS.md`（发布状态/交付物/残余）、`docs/37-release-1.0-checklist.md`、`docs/21` §10 gate 10 口径、`docs/18` 依赖审计策略、`docs/reviews/2026-10-31-release-1-0-readiness.md`。
- [x] ADR/D 已记录：ADR-068（WORM 部署控制）、ADR-069（MCP deferral 历史）、ADR-070（agent-team orchestration）；安全/依赖审计策略写入 `deny.toml`、`.cargo/audit.toml`、`KNOWN-LIMITATIONS.md` §4。
- [x] `docs/README.md` 已登记新主题：不适用（无新增主题文档；release 状态由 37/KNOWN-LIMITATIONS 承载）。
- [x] 变更 spec 已归档到 `specs/archive/`（终评 APPROVED 后归档；本文件为归档前验收记录）。

## 4. 回滚

- 回滚步骤：客户端降级到上一 rc/正式版（个人版 GitHub Releases、企业版 1:1 更新服务端回滚步骤见 `docs/20`）；服务端 `docker load` 上一镜像 + `deploy/docker-compose.yml` 回滚；数据库迁移只前向修复、不回退历史迁移。
- 数据影响：无破坏性迁移；回滚需保留 `master.key` 与备份（审计/用量保留期不变）。
- 已验证回滚：本地 `scripts/backup-restore.sh` + `scripts/upgrade-drill.sh`（非 Docker）pass；Docker full drill 在 CI 为环境项（UNVERIFIED）。

## 5. 剩余风险 / 后续

| 项 | 影响 | 后续 spec |
| --- | --- | --- |
| Docker upgrade/rollback drill、macOS 公证、Windows 真机、真实 provider/媒体在线调用、500 用户生产容量、镜像层内 SBOM | 环境/证书/凭证限制；已在 Release 说明与 `KNOWN-LIMITATIONS.md` §2 标 UNVERIFIED | post-RC 发布/运维项 |
| 基础镜像 68 项无上游修复 HIGH/CRITICAL（1 CRITICAL） | 低-中；`debian:bookworm-slim` 上游无修复，gate 10 按 fixable-only 判定，残余登记 `KNOWN-LIMITATIONS.md` §2 | debian 更新或 distroless 迁移 |
| zip 4.6 / zstd 0.13 / rsa 0.9 advisory（API 未使用或自产输入） | 低；`deny.toml`/`.cargo/audit.toml` 有窄理由 ignore | 升级 zip/wasmtime/sqlx 后移除 |
| personal×official rc.1 安装器未在本 Release run 产出 | 低；需 `workflow_dispatch edition=personal`（C1/C13） | 发布运维 |
| cargo-machete 仅 advisory（生成 glue/derive 误报） | 低 | 工具改进 post-RC |

## 6. 结论

- [x] `done`：证据齐全，可合并；独立评审 phase-3 终评见 `docs/reviews/2026-10-31-release-1-0-readiness.md`
- [ ] `abandoned`：原因：不适用

## 6. 复用证据（C17）

- 搜索记录：`zk find` / `docs/generated/CAPABILITY-INDEX.md`；Phase B 复用 `zk-kit-tools`（`SessionTool`）、`zk-kit-server`（master_key/audit）、`ui-kit` 组件与 contracts。
- 复用清单：MCP client 复用 `zk-contract-connector`；键环复用 `zk-kit-server`；CI setup 复用 composite action `.github/actions/setup`；release dist 复用 `scripts/build-dist.sh`。
- 新增能力登记：`zk connector-mcp`（tool.mcp 角色）、`zk-kit-server` master_key（`@capability kit.server`）；均已在能力索引 `zk capability --check` 通过。
