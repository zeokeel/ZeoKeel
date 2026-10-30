# 2026-10-30-m3-content-platform · 任务清单

## 1. 任务表

| # | 任务 | 写作用域 | 依赖 | 验证 | 状态 |
| --- | --- | --- | --- | --- | --- |
| 0 | 规格/评审（R1–R6） | `specs/...`, `docs/reviews/...` | — | `spec-check`/`review-check` | ✅ |
| 1 | 内容/技能契约 + content 迁移 | `packages/contracts/{content,skill}/`, `packages/server/content/migrations/` | — | `cargo test -p zk-contract-content -p zk-contract-skill` | ☐ |
| 2 | `zk-srv-content` 内核（状态机/审批/配额/树哈希/审计） | `packages/server/content/` | 1 | `bash scripts/srv-store-test.sh` | ☐ |
| 3 | `zk-srv-skills` 上传校验/目录/下载 | `packages/server/skills/` | 2 | `bash scripts/srv-store-test.sh`（恶意 zip 用例） | ☐ |
| 4 | 管理台 skills 页面包 | `packages/admin/admin-skills/` | 3 | `pnpm --dir apps/admin/web build` | ☐ |
| 5 | 管理台 skills e2e + axe | `e2e/m3-admin-skills.spec.ts` | 4 | Playwright | ☐ |
| 6 | 连接器契约/服务端/管理台 | `packages/contracts/connector/`, `packages/server/connectors/`, `packages/admin/admin-connectors/` | 2 | `bash scripts/srv-store-test.sh` | ☐ |
| 7 | 客户端 `ui-skills`/`ui-connectors` + 安装/冲突/删除 | `packages/ui/ui-skills/`, `packages/ui/ui-connectors/`, bootstrap | 3,6 | Vitest + Playwright | ☐ |
| 8 | 强制更新/回滚/配额/审计链路 | `packages/server/content/`, `packages/server/skills/`, `apps/desktop/src/ipc.rs` | 2 | `bash scripts/srv-store-test.sh` | ☐ |
| 9 | 创建 `scripts/srv-store-test.sh` + 恶意 zip 用例 | `scripts/srv-store-test.sh` | 2 | `bash scripts/srv-store-test.sh` | ☐ |
| 10 | 文档/验收 | `docs/08`, `docs/12`, `docs/16`, `AGENTS.md`, `acceptance.md` | 1–9 | docs/spec/review | ☐ |

## 2. 写作用域

- 本 change 只写：`packages/contracts/{content,skill,connector}/`、`packages/server/{content,skills,connectors}/`、`packages/admin/{admin-skills,admin-connectors}/`、`packages/ui/{ui-skills,ui-connectors}/`、`apps/admin/web/package.json`、`apps/desktop/web/src/{auth,App,protocol}.ts(x)`、`zk.toml`、`editions/enterprise.toml`、`e2e/m3-*.spec.ts`、`scripts/srv-store-test.sh`、`docs/08`、`docs/12`、`docs/16`、`AGENTS.md`。
- 生成物（`registry.ts`、`crates/zk-composed/**`、`shared/contracts/**`）由 `zk compose`/生成器写。
- 并行任务不重叠；任务 2/3 串行，4/5 串行，6 可并行，7 依赖 3/6。

## 3. 提交顺序

1. 契约 + content 迁移
2. `zk-srv-content` 内核 + curl 测试
3. `zk-srv-skills` 校验/下载 + 恶意 zip 测试
4. admin skills 页面 + registry + e2e/axe
5. connectors 服务端/页面
6. 客户端 `ui-skills`/`ui-connectors` 安装/冲突/删除 + bootstrap
7. 强制更新/回滚/配额/审计完整链路
8. 文档/验收

## 4. 验收前检查

- [ ] `python3 scripts/spec-check.py`
- [ ] `python3 scripts/docs-check.py`
- [ ] `bash scripts/srv-store-test.sh`（任务 9 创建；恶意 zip 用例）
- [ ] `bash scripts/admin-e2e.sh`
- [ ] `pnpm --dir apps/desktop/web test`
- [ ] `zk validate`
- [ ] `zk compose --edition enterprise --profile default --channel official --check`
- [ ] `zk api --edition enterprise --profile default --channel official --check`
- [ ] `bash scripts/schema-check.sh`
- [ ] `zk capability --check`
- [ ] 无手改 `@generated`
- [ ] 复用检查已做（`zk find`/能力索引；新增能力已登记 `@capability`）
- [ ] `acceptance.md` 已填真实证据
