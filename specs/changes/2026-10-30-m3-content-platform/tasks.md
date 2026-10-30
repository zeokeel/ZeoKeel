# 2026-10-30-m3-content-platform · 任务清单

## 1. 任务表

| # | 任务 | 写作用域 | 依赖 | 验证 | 状态 |
| --- | --- | --- | --- | --- | --- |
| 0 | 规格与六轮评审 | `specs/...`, `docs/reviews/...` | — | `spec-check`/`review-check` | ✅ |
| 1 | 契约 + `zk-kit-content` + schema/@capability | `packages/contracts/{content,skill,connector}/`, `crates/zk-kit-content/`, `shared/contracts/schema/`（生成） | 0 | `cargo test -p zk-contract-content -p zk-contract-skill -p zk-contract-connector -p zk-kit-content`; `bash scripts/schema-check.sh`; `zk capability --check` | ✅ |
| 2 | content 迁移 `0010_content` + 组合根接线 + 测试脚本骨架 | `packages/server/content/migrations/`, `tools/zk/templates/app_server.Cargo.toml`, `apps/server/src/main.rs`, `zk.toml`, `editions/enterprise.toml`, `apps/admin/web/package.json`, `apps/desktop/web/package.json`, `scripts/srv-store-test.sh`, `docs/24` | 1 | `zk db lock --check`, `cargo check -p zk-server`, `bash scripts/srv-migration-test.sh` | ✅ |
| 3 | `zk-srv-content` 内核 | `packages/server/content/` | 2 | `bash scripts/srv-store-test.sh` | ✅ |
| 4 | `zk-srv-skills` 归档元数据/目录/下载 | `packages/server/skills/` | 3 | `bash scripts/srv-store-test.sh` | ✅ |
| 5 | admin skills 页面 + registry 接线 | `packages/admin/admin-skills/`, `apps/admin/web/src/app.tsx`, `tools/zk/src/main.rs`, `apps/desktop/web/src/app.css`, `docs/16` | 4 | `zk compose --edition enterprise --profile default --channel official --check`, `pnpm --dir apps/admin/web build` | ✅ |
| 6 | connectors 契约/服务端/页面 | `packages/server/connectors/`, `packages/admin/admin-connectors/`, `apps/admin/web/src/app.tsx` | 4 | `bash scripts/srv-store-test.sh`, `pnpm --dir apps/admin/web build` | ✅ |
| 7 | 客户端 `ui-skills`/`ui-connectors` 安装/冲突/删除/OAuth | `packages/ui/ui-skills/`, `packages/ui/ui-connectors/`, `apps/desktop/web/src/{App,protocol,auth}.ts(x)`, `tools/zk/src/main.rs`, `apps/desktop/web/package.json` | 4,6 | `pnpm --dir apps/desktop/web test`, `pnpm --dir apps/desktop/web build` | ☐ |
| 8 | 客户端恶意 zip / 树哈希 / OAuth 测试 | `packages/ui/ui-skills/tests/`, `packages/ui/ui-connectors/tests/`, `crates/zk-kit-content/tests/` | 7 | `pnpm --dir apps/desktop/web test` | ☐ |
| 9 | 管理台 e2e + axe + admin-e2e 接入 | `e2e/m3-admin-skills.spec.ts`, `e2e/m3-admin-connectors.spec.ts`, `scripts/admin-e2e.sh` | 5,6 | `bash scripts/admin-e2e.sh`, `pnpm exec playwright test e2e/m3-*.spec.ts` | ☐ |
| 10 | 文档/验收 | `docs/04`, `docs/08`, `docs/12`, `docs/16`, `AGENTS.md`, `acceptance.md` | 1–9 | `docs-check`/`spec-check`/`review-check` | ☐ |

## 2. 写作用域

- 本 change 只写：`packages/contracts/{content,skill,connector}/`、`crates/zk-kit-content/`、`packages/server/{content,skills,connectors}/`、`packages/admin/{admin-skills,admin-connectors}/`、`packages/ui/{ui-skills,ui-connectors}/`、`apps/admin/web/{package.json,src/app.tsx}`、`apps/desktop/web/{package.json,src/app.css,src/App.tsx,src/protocol.ts,src/auth.ts}`、`apps/server/src/main.rs`、`tools/zk/templates/app_server.Cargo.toml`、`tools/zk/src/main.rs`、`zk.toml`、`editions/enterprise.toml`、`e2e/m3-*.spec.ts`、`scripts/{srv-store-test,admin-e2e}.sh`、`docs/{04,08,12,16,24}`、`AGENTS.md`、`specs/...`、`acceptance.md`。
- 生成物（`registry.ts`、`crates/zk-composed/**`、`shared/contracts/**`、`apps/server/Cargo.toml`）由 `zk compose`/生成器写，不手改。
- 任务 2 的迁移/组合根必须先于 3/4/6；5/6 可并行但 `app.tsx` 串行合并；7 依赖 4/6；8 依赖 7。

## 3. 提交顺序

1. 契约 + kit-content + conformance
2. `0010_content` 迁移 + 组合根/模板/edition 接线 + 脚本骨架
3. `zk-srv-content`
4. `zk-srv-skills`（归档元数据）
5. admin skills + registry
6. connectors
7. 客户端安装/冲突/删除/OAuth
8. 恶意 zip/树哈希测试
9. e2e/axe
10. 文档/验收

## 4. 验收前检查

- [ ] `zk db lock --check`
- [ ] `bash scripts/srv-migration-test.sh`
- [ ] `bash scripts/srv-store-test.sh`
- [ ] `bash scripts/admin-e2e.sh`
- [ ] `pnpm --dir apps/desktop/web test`
- [ ] `zk validate`
- [ ] `zk compose --edition enterprise --profile default --channel official --check`
- [ ] `zk api --edition enterprise --profile default --channel official --check`
- [ ] `bash scripts/schema-check.sh`
- [ ] `zk capability --check`
- [ ] 无手改 `@generated`
- [ ] `acceptance.md` 已填真实证据
