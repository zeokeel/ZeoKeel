# configs/ — 严格检查基线（scaffold 时使用）

这些是 `docs/18-strictness.md` 的参考配置，**在仓库骨架创建时复制到对应位置**：

| 文件 | 目标位置 |
| --- | --- |
| `tsconfig.base.json` | 仓库根，各前端包 `extends` |
| `eslint.config.js` | 仓库根（flat config） |
| `rust-workspace-lints.toml` | 合并进根 `Cargo.toml`；每个 crate `[lints] workspace = true` |

约定：

- 禁止放宽全局检查；例外必须行内 + 理由 + issue。
- TS 唯一妥协是 `skipLibCheck: true`（第三方 d.ts）；自有包必须全绿。
- Rust 测试模块可局部 `#[allow(clippy::unwrap_used, clippy::expect_used)]`。

## 2026-10-31 落地状态（fix round）

- 根 `tsconfig.base.json`、`eslint.config.js`、各 `apps/*/web|packages/*/*/web/tsconfig.json` 已落地；
  每个 web facet 都有 `lint` / `typecheck` / `test:unit` 脚本（`pnpm -r lint|typecheck|test:unit` 全绿）。
- 根 `tsconfig.base.json` 与本文档参考配置的唯一差异：**关闭 `isolatedDeclarations`**。
  原因：`apps/desktop/web/src/generated/registry.ts` 由 `zk compose` 生成（含对象 spread / 索引访问），
  在 noEmit 的前端构建下无法满足 isolatedDeclarations，且生成器改动属 `tools/zk` 范围（不在本轮 write scope）。
  如 `docs/18` 要求生成物也能独立 emit d.ts，需先改生成器，再在每个 `web/` 包启用该选项。
- `configs/eslint.config.js` 保留为参考；实际生效的是根 `eslint.config.js`（flat config，type-aware）。
