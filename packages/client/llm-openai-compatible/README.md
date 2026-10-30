# zk-llm-openai-compatible

OpenAI 兼容 provider（`POST {base_url}/chat/completions`）。个人版用于 DeepSeek/OpenAI/本地 vLLM/Ollama 等。
配置来自本地设置（M1 后续接 UI）或开发环境变量：
`ZEOKEEL_OPENAI_BASE_URL`、`ZEOKEEL_OPENAI_API_KEY`、`ZEOKEEL_OPENAI_MODEL`
（本地开发可用 `.env.local` 的 `ZEOKEEL_DEV_*` 变量，不提交）。

> 密钥只存本机/环境变量，不写日志、不进仓库。企业版只走网关（ADR-031），不使用本包连接上游。
