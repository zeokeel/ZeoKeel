# 2026-10-30-oem-delivery · 任务清单

| # | 任务 | 写作用域（文件/目录） | 依赖 | 验证 | 状态 |
| --- | --- | --- | --- | --- | --- |
| 1 | docs/09 schema + `ChannelFile`/`zk channel validate` | `tools/zk`, `docs/09`, `brands/official/channel.json` | — | `cargo test -p zk`; `zk channel validate --index oem/fixtures --all` | ✅ |
| 2 | 生成物 `brand.ts`/`brand.json`/`channel.json` + 逐字节 check | `tools/zk`, `apps/*/src/generated`, `shared` | 1 | `zk compose --channel official --check`; `zk channel verify --channel official --dist <fixture>` | ✅ |
| 3 | OEM fixture（index/.gitmodules/pin/manifest/deep） | `oem/fixtures`, `docs/23` | 1 | `bash scripts/oem-e2e.sh`（索引部分） | ✅ |
| 4 | demo 渠道 `oem-demo` + 构建矩阵 | `brands/oem-demo`, `editions/profiles`, `scripts/oem-e2e.sh` | 2,3 | `zk compose --edition enterprise --channel oem-demo`; `oem-e2e.sh` | ✅ |
| 5 | dist 产物树/CLIENT-RELEASE/CHANNEL/SHA256/SBOM/脚本 | `scripts`, `deploy`, `.github/workflows` | 4 | `bash scripts/oem-e2e.sh`（产物部分） | ✅ |
| 6 | portal + `/api/updates/manifest` 1:1 + e2e | `deploy/portal`, `packages/server/updates`, `e2e/m5-portal.spec.ts` | 5 | Playwright + server e2e | ✅ |
| 7 | 签名 dry-run + upgrade drill | `scripts/upgrade-drill.sh`, `.github/workflows`, `docs/20` | 5 | `bash scripts/upgrade-drill.sh --dry-run` | ✅ |
| 8 | 文档/验收/归档 | `docs/09/20/23`, acceptance | 1–7 | docs/spec/review; `oem-e2e.sh` | ☐ |

## 2. 写作用域

- 只写：`tools/zk`、`brands/`、`oem/`、`deploy/`、`scripts/{oem-e2e,upgrade-drill}.sh`、`e2e/m5-*`、`packages/server/updates`、`docs/{03,09,20,23,27}`、`.github/workflows/*`、生成物。
- 不写：M0–M4 迁移历史、契约破坏性字段。

## 3. 验收前检查

- [ ] `zk channel validate --index oem/fixtures --all`
- [ ] `zk channel verify --channel official --dist <fixture>`
- [ ] `zk compose --channel official --check` / `--channel oem-demo --check`
- [ ] `bash scripts/oem-e2e.sh`
- [ ] `bash scripts/upgrade-drill.sh --dry-run`
- [ ] `docs-check` / `spec-check` / `review-check`
- [ ] 独立评审 APPROVED
