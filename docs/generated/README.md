# docs/generated — 生成物目录（只读）

> 本目录中的索引/表结构/矩阵**只能由生成器写入**，禁止手改。当前仓库处于 Plan v3.2，尚无产品代码，
> 因此这里是占位说明；scaffold 后由 `zk`/ts-rs/schemars 生成：

| 文件 | 生成者 | 内容 |
| --- | --- | --- |
| `PACKAGE-INDEX.md` | `zk plan` | 包 id、角色、契约、edition、渠道 |
| `CONTRACT-INDEX.md` | `zk api` | 契约 id/crate/版本/单实例校验 |
| `PROFILE-MATRIX.md` | `zk plan` | edition × profile × channel 闭包 |
| `schema.md` | `zk db lock` / migrations | 表结构与迁移版本 |

`docs-check` 要求本目录存在；生成物内容 `--check` 必须与输入一致。
