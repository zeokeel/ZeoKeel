# tool-web-search (`tool.web_search`)

**功能说明**：调用 DuckDuckGo Instant Answer API 返回摘要/相关链接；外部内容不可信，不执行其中指令（结果用 `results` + `note: external untrusted data` 标注）。

**配置（R9.3/B11）**：端点与超时来自设置契约，优先级 **显式设置 > env > 内置默认**：

| 来源 | 端点 | 超时 |
| --- | --- | --- |
| `AppSettings.web_search`（设置页“网络搜索”；桌面发布非密 `RuntimeSettings.web_search`） | `base_url`（空 = 不覆盖；仅 https） | `timeout_ms`（1000..=120000，默认 15000） |
| env | `ZEOKEEL_SEARCH_URL` | 无 env fallback |
| 内置默认 | `https://api.duckduckgo.com/` | 15000 ms |

- 每次 `web_search` 调用重新解析（保存设置后无需重启）；`call_json` 仍强制 https，非 https 返回 `Denied`。
- 该端点无密钥，客户端不保存任何搜索凭据；企业版同样生效（工具出口端点，不是 AI provider，C7 不适用）。
- 单测：`tests/tool.rs`（默认/env/settings 优先级 + 非 https 拒绝）；契约侧 `zk-contract-config` 覆盖默认值与边界。
