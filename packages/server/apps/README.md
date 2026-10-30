# zk-srv-apps (`srv.apps`)

**功能说明（M4）**：WASM 应用发布/生命周期。

- 员工上传 manifest+wasm+ui zip（pending，owner）
- 管理审批/上架/下架/grants/诊断/回滚/退役
- 客户端目录/open/request/assets
- 执行交给 `srv.app.runtime`，DB 交给 `srv.app.db`；服务端不执行业务代码以外逻辑。
