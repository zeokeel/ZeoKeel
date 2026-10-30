# docs/generated — 生成物目录（只读）

> 本目录中的索引/矩阵/表结构**只能由生成器写入**，禁止手改。权威 writer 表见 `docs/24` §3。
> `--check` 只校验、不写源文件；`scripts/docs-check.py` 不检查本目录内容。

| 文件 | 生成器 | 内容 |
| --- | --- | --- |
| `PACKAGE-INDEX.md` | `zk compose` | 当前组合（基线 personal×default×official）的包 id、角色、crate、契约、基数 |
| `CONTRACT-INDEX.md` | `zk docs` | 契约 id → crate → 角色/editions → 实现包（来自 `[contracts]`/`[roles]`/manifest role 声明） |
| `PROFILE-MATRIX.md` | `zk docs` | edition × profile × channel 闭包：status、包数、generated EDITION、personal_only_hits、excluded_from_personal |
| `CAPABILITY-INDEX.md` | `zk capability` | 可复用能力 id/路径（contract/kit/package `@capability`，`docs/25` §3） |
| `AGENT-TOOL-MATRIX.md` | `scripts/agent-tool-matrix.py` | DSH ↔ ZeoKeel 工具覆盖矩阵（研究/对照用，不是产品 SSOT） |

- **writer 唯一**：`zk docs` 写 `CONTRACT-INDEX.md` + `PROFILE-MATRIX.md`；`zk compose` 写 `PACKAGE-INDEX.md`
  （及 `shared/contracts/{packages,roles,ui-contributions,brand}.json`、两端 generated glue）；
  `zk capability` 写 `CAPABILITY-INDEX.md` + `shared/contracts/capability-index.json`。
- `zk api`（`shared/contracts/{api-catalog,http-routes,openapi,permissions,errors}.json`）与 `schema.md`
  （迁移表结构）**尚未实现**，见 `target/evidence/audit/build.md`；`docs/24` §3 的对应行仍是 M0 待办。
- 生成物内容一致性：`zk docs --check`、`zk compose ... --check`、`zk capability --check`；
  CI 需在 drift job 中同时运行三者（Lead 负责 `.github/workflows/ci.yml`）。
