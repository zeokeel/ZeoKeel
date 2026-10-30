# zk-contract-ipc

本地 IPC 命令/响应/事件信封 + 稳定命令名常量。Tauri 与 HTTP serve 载体共用同一批 DTO；
UI 只通过 `CommandRequest`/`UiEvent` 通信。规范见 `docs/05` §6。
