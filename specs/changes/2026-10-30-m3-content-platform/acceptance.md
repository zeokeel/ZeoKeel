# 2026-10-30-m3-content-platform · 验收

- 状态：`in-progress`（task 1 完成；实现未完成）
- 验收人：pending
- 日期：pending
- PR/commit：pending
- 证据目录：`target/evidence/m3/`

## 1. 自动验证（实现后填写真实结果）

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
