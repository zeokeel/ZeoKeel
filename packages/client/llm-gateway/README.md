# zk-llm-gateway (`llm.provider.default`)

**功能说明**：企业 AI provider，只访问登录服务端的 `POST /v1/responses`。客户端不持有上游 key，
不填上游 base_url；AI 只走登录网关（ADR-031 / C7）。

## 两个出口

1. **rig 模型（企业 agent 主路径，`agent-rig` 组合使用）**
   - `model() -> Result<rig_core::DynModel<Completion>, LlmError>`：用登录注入的
     `set_credentials(server_url, access_token)` + bootstrap 的 `set_model(model)` 构造 rig 原生
     OpenAI Responses 模型（`OpenAIConfig::new(token).with_base_url("{server}/v1").client().responses(model)`），
     实际请求路径 = `{server}/v1/responses`。
   - `model_from_config(provider, base_url, api_key, model)`：组合 glue 的 settings hot-swap 入口；
     空参数回落到登录注入的共享配置。`provider` 只接受 `gateway` / `openai` / `openai_responses` /
     `responses` / 空；`mock` / `ollama` 等本地 provider 明确报错（企业版不得自配 provider）。
   - 未登录（server URL / access token / bootstrap model 任一为空）时 `model()` 返回
     **ADR-064 启动占位**：真实 Responses 模型指向关闭的本地端口 `127.0.0.1:9`，使
     `zk_composed::build()` 能启动并渲染登录页；登录前发起的 turn 在 transport 层明确失败，
     不会静默用 mock 回答。登录后由 `model_from_config`/`configure_llm` 换成真实模型。
   - `model_from_config` 是登录后的严格工厂：server/token/model 缺失或 provider 非网关时
     返回 `LlmError::InvalidRequest`（不降级占位），让登录/bootstrap 错误在 UI 暴露。
   - rig 原生 Responses 支持流式 text/reasoning/tool-call 增量、function tools（JSON Schema）、
     `usage`（input/output/total/cached/reasoning）与 `tool_choice`；无需本包手写协议解析。

2. **兼容门面（`provider()` / `LlmProvider`）**
   - 保留给 `zk-contract-llm` 的旧实现与 `m2-gateway-chat` e2e；只解析非流式
     `output[].content[].text` 与 SSE 文本 delta。
   - 工具调用/usage 不在该门面内（契约只有 prompt 字符串）；企业 agent 走上面的 rig 模型。

## 接线顺序（重要）

`zk compose` 生成的 enterprise glue 在 `build()` 里调用 `model()`，而登录发生在之后（enterprise 首屏是登录页）。
`model()` 在未登录时返回占位模型，所以 `build()` 可正常启动；`AUTH_CONFIGURE` 之后由 desktop 调用
`Runtime::configure_llm(server_url, access_token, model, "gateway")`，经本包 `model_from_config` 热切换为
真实模型。当前 `AUTH_CONFIGURE` 只更新本包共享配置（`set_credentials`/`set_model`），不会替换已构建的
rig 模型；desktop 侧需补 `runtime.configure_llm(...)`（`apps/desktop` 属其他修复任务，见 task-7）。

## 其他

- 错误不泄露上游 body；不做 Chat Completions、不做本地 provider 降级。
- 测试：`tests/conformance.rs`（旧门面 fake SSE）+ `tests/rig_responses.rs`（rig 模型经本地 fake
  gateway 验证流式 text、tool-call round-trip、usage、bearer 注入与请求体）。
