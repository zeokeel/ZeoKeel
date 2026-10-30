# 2026-10-31-round3-completeness · 设计

- 状态：`approved`

## 1. 超时配置

contract ToolSettings → desktop boot/settings.set 应用：`ApprovalBroker::set_timeout`；agent-rig `AgentRuntimeConfig.tools` 每轮读取（替代直接 env）。命令默认超时（zk-kit-tools DEFAULT_TIMEOUT_MS）仍由工具实现使用，后续可扩展。

## 2. 向量持久化

`VectorStore` 在 kit/cap 内增加可选 `Persistence { path }`；序列化 {version, documents:[{id,text,embedding}]}；写入 temp+rename；加载失败返回空并记录 reason；`configure_from_env` 读 `ZEOKEEL_VECTOR_DIR`。

## 3. T22

contracts: FileEntry/MetaInfo/ChatSendResult/QuestionDef 放 session/ipc/agent 合适位置；schemars export；schema-ts 生成 interfaces+SCHEMAS；desktop validate.ts 增加对应 schema key，protocol parser 先 Ajv 后映射。

## 4. 内核

bootstrap `spawn_task` 返回 TaskHandle；schedule executor 改为 async loop + stop Notify，内核 task 取消时置位；server jobs loop 增加 stop AtomicBool/CancellationToken，shutdown 时置位并等待；Tauri setup 中事件转发改为 `bootstrap.spawn_task`（或在 shutdown 中等待）。

## 5. UI

ui-sessions 删除 paletteOpen/命令按钮复制；通过 UiActions.openCommands()（shell 已暴露）或新增 action；admin 页面用 ui-kit translate 注入（admin 目前独立 i18n，可复用本地字典）；inspector changes 增加 `workspace.changes` IPC（git status 解析）或禁用 tab + docs 标注。
