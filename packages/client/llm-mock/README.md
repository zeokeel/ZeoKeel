# zk-llm-mock

确定性 mock LLM provider（包 id `zeokeel/llm-mock`，角色 `llm.provider.default`，契约 `zk-contract-llm`）。

- 实现 `LlmProvider::complete`；不联网、不读密钥，行为由输入决定。
- 解析：先找最后一次出现的 `USER_INPUT:` 标记，取用户输入行；忽略 demo prompt 里尾随的指令行
  （以 `如需`/`如果`/`If you need` 开头）；没有标记时取 prompt 最后一个非空行。
- 决策：输入含 `读取 ` 或 `read ` 且其后有非空 token → `TOOL fs.read <token>`
  （token 去掉首尾引号/标点）；否则 `（mock）收到：<输入>`。
- 工厂：`provider()`；直接 `MockLlm::new()`。

```bash
cargo test -p zk-llm-mock --offline
```
