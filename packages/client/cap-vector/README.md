# cap-vector (`vector.store`)

**功能说明**：Embeddings + vector store 能力（`vector.store`）：OpenAI-compatible embedding model +
进程内 cosine 索引。

实现位于共享 rig 适配层 `crates/zk-kit-media`（`kit.media`）；本包只依赖该 kit + 自己的结果 DTO，
不直接依赖 `rig-core` / `zk-llm-rig`（C4/C17）。测试使用 kit 的 `test-support` 假模型（无网络）。

- `configure(model)` / `configure_from_env()`（`ZEOKEEL_EMBEDDING_BASE_URL/_API_KEY/_MODEL/_DIMS`）后
  `available()==true`；`unconfigure()` 清空模型与索引。
- API：`insert`/`insert_many`/`search`/`clear`/`len`；未配置时 `available()==false` +
  `unavailable_reason()`，调用返回 `CAPABILITY_UNAVAILABLE` 错误。
- 宿主接线：personal boot 调用生成 glue 的 `zk_composed::configure_capabilities_from_env()`
  （进而调用本包 `configure_from_env()`）；enterprise boot 不配置（C7/ADR-031，网关无 embeddings 路由），
  故 enterprise 保持不可用。
- 索引为进程内（重启丢失）；SQLite 持久化/FTS 待后续。
