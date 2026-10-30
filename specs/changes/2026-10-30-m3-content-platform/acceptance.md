# 2026-10-30-m3-content-platform · 验收

- 状态：`in-progress`（task 1 完成；实现未完成）
- 验收人：pending
- 日期：pending
- PR/commit：pending
- 证据目录：`target/evidence/m3/`

## 1. 自动验证（实现后填写真实结果）

### task 8 evidence (partial, 2026-10-30)

- `cargo test -p zk-desktop installs_valid_archive`：valid zip 安装且 tree_sha256 64 位；重复 409；`../evil` 拒绝；zip symlink 拒绝；remove 清理目录。
- 安装回执：`POST /skills/{name}/{version}/install` 写 content_installs + audit；`srv-store-test.sh` 验证 200。
- tombstone：删除后 `/skills/tombstones` 返回名称；客户端启动时拉取并调 `skills.remove`（代码路径已接）。
- OAuth PKCE/authorize URL/token exchange/refresh/error 单测：`apps/desktop/web/tests/oauth.test.ts`（3 passed）。
- tombstone e2e：安装 → 管理台删除 → 客户端刷新后目录为空。
- force update e2e：管理台上传 1.1.0 + 强制更新 → 客户端显示“更新” → 更新后“已安装”。
- 员工上传/非 owner 403/转交后上传 200：`srv-store-test.sh` 已覆盖。
- OAuth 授权回调 UI 已接：fake OAuth provider e2e（授权跳转→回跳→换 token→“已授权”）；token/pending 单测通过。

### task 9 evidence (2026-10-30)

- `bash scripts/admin-e2e.sh`：8 passed（M2 6 + M3 skills/connectors，axe 0 serious/critical）
- 截图：`target/evidence/m3/admin-skills.png` / `admin-connectors.png`

### task 7 evidence (2026-10-30)

- 新增 `@zeokeel/ui-skills` / `@zeokeel/ui-connectors`，接入 registry、⌘K 命令、shell overlay。
- Rust IPC `skills.install`：base64 → zip → 安全解压（路径/链接/总量）→ 规范化 tree_sha256 → 原子落盘；`skills.installed` 列表。
- `bash scripts/login-e2e.sh`：3 passed（登录、网关聊天、客户端安装技能 + 启停连接器）；axe 0 serious/critical；截图 `target/evidence/m3/client-skills.png` / `client-connectors.png`。
- 仍缺 task8：恶意 zip/树哈希/OAuth 刷新客户端测试。

### task 6 evidence (2026-10-30)

- `bash scripts/srv-store-test.sh`：创建 connector → 全员 grants → OAuth public client → 客户端目录含 OAuth → enable/disable → 删除；`store ok: content schema + skill archive + connectors`
- `secret` 配置被 400 拒绝；目录不含 secret/token（ADR-060）
- `bash scripts/admin-e2e.sh`：8 passed（M2 6 + M3 skills/connectors）
- 截图：`target/evidence/m3/admin-connectors.png`

### task 5 evidence (2026-10-30)

- `bash scripts/admin-e2e.sh`：7 passed（M2 6 + M3 skills 1）
- M3 真人操作：登录 → 技能页 → 创建 → 审批 → 管理 → 上传 zip 1.0.0 → 全员可安装 → axe 0 serious/critical → 删除
- 修复：admin 登录误用 client LoginRequest schema（device_id）→ 新增 AdminLoginRequest schema/Ajv；admin hint 对比度 4.47→合规
- 截图：`target/evidence/m3/admin-skills.png`

### task 4 evidence (2026-10-30)

- `bash scripts/srv-store-test.sh`：上传 zip（SKILL.md）→ 归档 sha256；同版本 409；`../evil` zip 400；下载 archive 校验 `X-ZeoKeel-Skill-Sha256` 与内容；`store ok: content schema wired + skill upload/download`
- 服务端只读 central directory，不解压（ADR-061）

### task 3 evidence (2026-10-30)

- `bash scripts/srv-store-test.sh`：创建（pending）→ approve → grants(all,can_install) → client catalog 1 项 → DELETE 软删 → catalog 0 项；`store ok: content schema wired`
- 路由：`/api/admin/skills` 列表/创建/approve/reject/delist/transfer/grants/force-update/delete + `/api/client/v1/skills`
- `cargo test --workspace` / clippy：pass

### task 2 evidence (2026-10-30)

- `cargo run -q -p zk -- validate`：enterprise 33 / personal 14 packages，57 scanned
- `cargo build -q -p zk-server`：pass
- `bash scripts/srv-store-test.sh`：`store ok: content schema wired`（5 张 content 表）
- `zk db lock --check`：pass（10 migrations，含 0010_content）

### task 1 evidence (2026-10-30)

- `cargo test -p zk-contract-content -p zk-contract-skill -p zk-contract-connector -p zk-kit-content`：pass
- `bash scripts/schema-check.sh`：pass（6 schemas）
- `zk capability --check`：up to date (13 entries)
- `cargo run -q -p zk -- validate`：56 packages


| 命令 | 期望 | 结果 |
| --- | --- | --- |
| `cargo test --workspace` | 全绿 | task1: content/skill/connector + zk-kit-content tests pass |
| `bash scripts/srv-store-test.sh` | 上传/审批/可见/安装/更新/删除/配额/恶意 zip 全通过 | pending |
| `bash scripts/admin-e2e.sh` | 技能页面操作 + axe 0 serious/critical | pending |
| `pnpm --dir apps/desktop/web test` | 客户端安装/冲突/删除六态 | pending |
| `pnpm exec playwright test e2e/m3-*.spec.ts` | 全绿 | pending |
| `python3 scripts/docs-check.py` / `spec-check.py` / `review-check.py` | 0 errors | pending |

## 2. 退出条件（docs/12 M3）

- [ ] 员工上传 → 管理员审批 → 全员/指定可见并安装 → 新版本强制覆盖 → 删除后客户端自动移除
- [ ] 非归属人无法更新；管理员转交后权限正确
- [ ] 连接器密钥不出现在客户端日志/存储；令牌过期后自动刷新
- [ ] 审计可查完整链路（谁上传/谁审批/谁安装）
- [ ] 恶意 zip（路径穿越/symlink/超量/重复名）拒绝且清理
- [ ] tree_sha256 不匹配拒绝安装
- [ ] 配额超限返回 `CONTENT_QUOTA_EXCEEDED`

## 3. 手工检查

- [ ] 管理台上传真实 zip，SKILL.md 必填错误提示明确
- [ ] 客户端安装后来源/版本/哈希可见
- [ ] 强制更新低版本提示不可忽略
- [ ] 删除内容后客户端目录被移除
- [ ] 审计页面可按 action 过滤

## 4. 回滚 / 剩余风险

- 回滚：应用层 `rollback` 到历史 release；DB 只前向修复。
- 剩余风险：zip 库漏洞、PG BYTEA 容量、客户端解压权限；mitigation 见 design。
- 结论：pending
