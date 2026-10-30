# zk-srv-content (`srv.content`)

**功能说明（M3）**：内容发布内核。

- owns `contents/content_releases/content_grants/content_deployments/content_installs`
- 审批/状态机/授权/强制更新/删除 tombstone/配额
- 归档存 PG BYTEA；服务端只审归档元数据（ADR-061）
- 审计写 kit 哈希链；技能格式校验在 `zk-srv-skills`，不重复状态机
