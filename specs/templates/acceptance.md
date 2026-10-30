# <change-id> · 验收与证据

- 验收人：<name>
- 日期：<YYYY-MM-DD>
- PR/commit：<链接/hash>

## 1. 验收命令与结果

| 命令 | 结果 | 证据路径 |
| --- | --- | --- |
| `python3 scripts/spec-check.py` | <pass/fail> | <stdout/日志> |
| `python3 scripts/docs-check.py` | <pass/fail> | <stdout/日志> |
| `zk validate` | <pass/fail> | <日志> |
| `zk compose --edition <E> --profile <P> --channel <C> --check` | <pass/fail> | <日志> |
| `zk capability --check` | <pass/fail> | <日志> |
| `zk lint --duplicates`（或 jscpd ratchet） | <pass/fail> | <报告> |
| `cargo nextest run -p <pkg>` | <pass/fail> | `target/evidence/...` |
| `pnpm -r typecheck && pnpm -r lint` | <pass/fail> | <日志> |
| <其它> | | |

## 2. 手工检查

- [ ] <UI 六态/键盘/a11y/错误提示>
- [ ] <权限/越权/非法输入>
- [ ] <离线/超时/重试/取消>
- [ ] <升级/回滚/兼容>

## 3. 文档与决策

- [ ] SSOT 文档已更新：<路径/节>
- [ ] ADR/D 已记录：<编号>
- [ ] `docs/README.md` 已登记新主题：<是/否/不适用>
- [ ] 变更 spec 已归档到 `specs/archive/`

## 4. 回滚

- 回滚步骤：<命令/镜像/迁移>
- 数据影响：<无/可逆/需备份>
- 已验证回滚：<是/否；证据>

## 5. 剩余风险 / 后续

| 项 | 影响 | 后续 spec |
| --- | --- | --- |
| <P2/未做> | <低> | <id 或 无> |

## 6. 结论

- [ ] `done`：证据齐全，可合并
- [ ] `abandoned`：原因：<...>

## 6. 复用证据（C17）

- 搜索记录：<`zk find ...` 输出 / 索引条目链接>
- 复用清单：<capability id → 调用点>
- 新增能力登记：<索引条目 / 无>

