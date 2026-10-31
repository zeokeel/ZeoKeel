# stage2-embedding-config · 设计

- 状态：`approved`
- 契约→glue→desktop→UI；glue 函数放生成模板（compose lib）并 regenerated；密钥不进入 RuntimeSettings；settings.get 已 mask provider，扩展 mask embedding.api_key。
