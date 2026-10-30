# zk-srv-app-runtime (`srv.app.runtime`)

**功能说明（M4）**：WASM 执行。

- wasmtime 50/wasip1；帧 ABI；SQL 闸门；fuel/epoch/内存/并发
- 仅供 `srv.apps` 调用，不暴露 HTTP
- 错误映射 `APP_LIMIT_*`/`APP_BAD_WASM`/`APP_BAD_REQUEST`
