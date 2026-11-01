# <surface> · 直觉判定（INT）

> 本文件与 `interaction.md` 同目录，是 UI 变更的**第七件**（`docs/19` §11.1、`docs/27` §2.7、`docs/11` §8.6）。
> 只写本 surface 的直觉判据；原则定义不在本文件复制，链接 SSOT。
> `active` 契约：≥3 条 INT，每个 P0 交互 ID 至少映射 1 条；质量由 reviewer 走查 + 用户测试判定，不由 gate 自动评分。

- 表面 / 入口：<composer / settings / inspector / command palette / sessions / ...>
- 归属包：<@zeokeel/ui-*>
- 状态：`active`（参与门禁）| `planned`（实现开始时改 active）
- 证据根：`target/evidence/<change-id>/intuition/`
- 走查：同目录 `intuition-walkthrough.md`

| ID | 原则 | 场景 / top task | 用户直觉期望（可观察） | 反例 | 验证方式 | 证据 |
| --- | --- | --- | --- | --- | --- | --- |
| INT-01 | 可发现性 | <首次发送> | <主操作可见/不 hover 也能发现> | <hover-only> | e2e + review | <测试/走查> |
| INT-02 | 预期一致 | <Esc/Enter/Tab> | <符合平台/产品惯例> | <Esc 无效> | e2e | <测试> |
| INT-03 | 状态可见 | <保存/发送> | <100ms busy/完成/错误> | <无反馈> | e2e | <测试> |

## 覆盖说明

- P0 交互 ID → INT 映射：<ID: .. → INT-..>
- 用户测试：<UNVERIFIED / 日期+人数+证据路径>
- 未解决 P0：<无 / 列表；有则不得进入实现/发布>
