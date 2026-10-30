# cap-vector (`vector.store`)

**功能说明**：Embeddings + vector store 能力，已实现（round 3）：rig-core embedding model +
进程内 cosine 索引。

- `configure(model)` / `configure_from_env()`（`ZEOKEEL_EMBEDDING_BASE_URL/_API_KEY/_MODEL/_DIMS`）后
  `available()==true`；`unconfigure()` 清空。
- API：`insert`/`insert_many`/`search`/`clear`/`len`；未配置时 `available()==false` +
  `unavailable_reason()`，调用返回 `CAPABILITY_UNAVAILABLE` 错误。
- 个人版 embedding model 由 `zk-llm-rig::embedding_model()` 构造；enterprise 网关（Responses-only）无
  embeddings 路由，故 enterprise 保持不可用直到新增 embedding provider。
- 索引为进程内（重启丢失）；SQLite 持久化/FTS 待后续。
- 架构注：本包是第三个 rig 适配包（ADR-007 目前只写 llm-rig/agent-rig），需补 ADR 措辞。
