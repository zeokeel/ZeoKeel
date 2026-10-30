# zk-llm-gateway (`llm.provider.default`)

**功能说明（M2 P4）**：企业 AI provider，只访问登录服务端的 `POST /v1/responses`。

- 客户端不持有上游 key，不填上游 base_url；base_url 来自 bootstrap/服务端地址。
- 认证用企业 access token（env/boot 注入；UI 接线后由 auth session 提供）。
- 解析 Responses 非流式 `output[].content[].text`；错误不泄露上游 body。
- 工具调用暂由 agent-loop 本地解析（后续 turn context 扩展）。
- 不做 Chat Completions、不做本地 provider 降级。
