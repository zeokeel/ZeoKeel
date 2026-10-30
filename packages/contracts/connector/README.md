# zk-contract-connector

**功能说明**：connector 契约（C4 seam）。

- 仅 DTO/枚举与 serde/schemars；不含实现、网络、存储。
- 依赖方只依赖本 crate；实现方在 `packages/server/connector`（或 `zk-srv-content` 等）。
