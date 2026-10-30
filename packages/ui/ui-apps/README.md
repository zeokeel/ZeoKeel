# @zeokeel/ui-apps (`ui.slot.apps`)

**功能说明（M4）**：企业客户端应用目录。

- 目录来自 `GET /api/client/v1/apps`
- 打开：`POST /apps/{id}/open`，展示版本/错误页（APP_DELISTED/APP_UNAVAILABLE）
- 应用 UI 静态资源由服务端提供；客户端不执行应用 WASM（ADR-062）
- 独立窗口/`app_request` IPC 由 app 宿主实现，面板只读目录
