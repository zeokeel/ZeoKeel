# @zeokeel/ui-settings-provider (`ui.slot.settings-provider`)

**功能说明（M1）**：个人版本地模型 provider 设置。

- 字段：provider 类型（mock/OpenAI 兼容/Ollama）、model、base_url、api_key。
- 只被 `@zeokeel/ui-settings` 通过 `renderProvider` 渲染；enterprise 构建期 exclude。
- 不持有状态：值/错误/回调由 settings 表单注入。
- 企业版不得出现此包（C1）。
