# 2026-10-31-round10-truth-and-p2 · 设计

- 状态：`approved`

## 1. docs
直接编辑权威文档，保持与 CI 实际命令一致；docs-check。
## 2. request-id
kit 函数加 `is_ascii_alphanumeric || ._-` 校验。
## 3. async
agent-rig 测试用 spawn hook 失败分支（已有测试 hook？若无则加）与真实线程 stress。
## 4. schedule e2e
m1-serve 已有 settings 测试；扩展检查 schedule 字段。
