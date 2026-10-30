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
