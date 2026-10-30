# 2026-10-31-round4-data-and-ui · 设计

- 状态：`approved`

## 1. Usage

contract-session → agent 契约；store v4 ALTER + JSON；desktop append_message 带 usage；web parseMessages 已有 usage 字段则直接恢复，否则补映射。

## 2. Settings

RuntimeSettings 两段 Option；From<&AppSettings> 仍存在但仅供兼容；桌面 publish 参数改为 `PublishScope { agent: bool, tools: bool }`；agent-rig 逐段 resolve。

## 3. Changes

Rust 用 std::process::Command（固定参数，不拼接 shell）执行 git status；解析 porcelain v1 -z：`XY path\0`；add/del 只用每行状态粗略计数（A/??→add, D→del），不做行级 diff；上限 500。契约结果类；web 打开 tab 时调用并缓存，错误显示可重试。

## 4. T22

host 构造 DTO struct 后 serde_json::to_value；web validate 用生成的 schema const。
