# Contributing

1. 先读 `AGENTS.md` 与 `docs/README.md`；无 spec 不开工（C16）。
2. 复用优先：`zk find`/能力索引；禁止复制实现（C17）。
3. 严格检查：`cargo test/clippy`、`pnpm test`、`zk validate`、`compose --check`、`schema-check`、`docs-check`、`spec-check`、`review-check`。
4. 提交 Conventional Commits；PR 写验证命令/证据/影响的包/契约/数据。
5. 不提交密钥/target/node_modules；生成物只由生成器写。
