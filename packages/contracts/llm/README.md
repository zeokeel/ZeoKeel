# zk-contract-llm

LLM provider 契约：`LlmProvider::complete` + 封闭 `LlmError`。企业版网关 provider 与个人版本地
provider 都实现同一契约；流式在 M1 扩展。规范见 `docs/02` §5。
