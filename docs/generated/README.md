# docs/generated — 生成物目录（只读）

> 本目录中的索引/表结构/矩阵**只能由生成器写入**，禁止手改。当前仓库处于 Plan v3.2，尚无产品代码，
> 因此这里是占位说明；scaffold 后本目录文件由 `zk docs` 写入（数据来自 `zk plan`/`zk api`/`zk db lock`），能力索引用 `zk capability`：

| 文件 | 生成者 | 内容 |
| --- | --- | --- |
| `PACKAGE-INDEX.md` | `zk docs`（数据来自 `zk plan`） | 包 id、角色、契约、edition、渠道 |
| `CONTRACT-INDEX.md` | `zk docs`（数据来自 `zk api`） | 契约 id/crate/版本/单实例校验 |
| `PROFILE-MATRIX.md` | `zk docs`（数据来自 `zk plan`） | edition × profile × channel 闭包 |
| `schema.md` | `zk docs`（数据来自 `zk db lock`） | 表结构与迁移版本 |
| `CAPABILITY-INDEX.md` | `zk capability` | 可复用能力 id/签名/路径/消费方（`25` §3） |

- **writer 唯一**：本目录由 `zk docs` 写；`CAPABILITY-INDEX.md` 由 `zk capability` 写（`24` §3）。`zk plan`/`zk api`/`zk db lock` 只提供数据，不直接写本目录。
- 生成物内容一致性用 `zk docs --check` 校验（`--check` 不写源文件）；`scripts/docs-check.py` 不检查本目录内容。
