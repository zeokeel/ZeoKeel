# cap-vector (`vector.store`)

**功能说明**：Embeddings + vector store 能力（`vector.store`）：OpenAI-compatible embedding model +
cosine 索引，可选本地 JSON 持久化。

实现位于共享 rig 适配层 `crates/zk-kit-media`（`kit.media`）；本包只依赖该 kit + 自己的结果 DTO，
不直接依赖 `rig-core` / `zk-llm-rig`（C4/C17）。测试使用 kit 的 `test-support` 假模型（无网络）。

- `configure(model)` / `configure_with_dir(model, dir)` / `configure_from_env()` 后
  `available()==true`；`unconfigure()` 清空模型与内存索引（磁盘文件保留）。
  env：`ZEOKEEL_EMBEDDING_BASE_URL/_API_KEY/_MODEL/_DIMS`（模型）与 `ZEOKEEL_VECTOR_DIR`（持久化目录）。
- API：`insert`/`insert_many`/`search`/`clear`/`len`；未配置时 `available()==false` +
  `unavailable_reason()`，调用返回 `CAPABILITY_UNAVAILABLE` 错误。
- 持久化（opt-in）：目录存在时索引写入 `vector-index.json`（`{version, documents:[{id,text,embedding}]}`），
  insert/clear 原子保存（同目录 temp 文件 `0600` → rename，目录 `0700`）；configure 时加载。
  上限 `MAX_DOCUMENTS`(10k) / `MAX_INDEX_BYTES`(16MB)，超限返回 `VectorError::Capacity`。
  文件缺失 → 空索引；损坏/不可读/超限 → 降级为空索引（不 panic），原因经
  `persistence_reason()` 暴露并写 stderr，下一次成功保存会修复文件。
- 宿主接线：personal boot 调用生成 glue 的 `zk_composed::configure_capabilities_from_env()`
  （进而调用本包 `configure_from_env()`）；enterprise boot 不配置（C7/ADR-031，网关无 embeddings 路由），
  故 enterprise 保持不可用。
- 剩余：无跨进程写锁/跨设备同步（本地文件，最后一写者胜）、无 SQLite/FTS；索引仍受单进程内存限制。
