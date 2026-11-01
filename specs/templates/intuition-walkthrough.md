# <surface> · 认知走查（intuition walkthrough）

> 方法：`docs/27` §2.8；3 persona × 3 top task × 每步 4 问。
> 本走查是专家走查，不等于用户测试；用户测试见 §4（未执行标 UNVERIFIED）。
> 摩擦项引用同目录 `intuition.md` 的 INT ID；P0 未解决不得进入实现/发布。

## 1. Personas

| ID | Persona | 特征 | 关注点 |
| --- | --- | --- | --- |
| P1 | 首次使用者 | <特征> | <关注> |
| P2 | 高频用户 | <特征> | <关注> |
| P3 | 出错/辅助技术用户 | <特征> | <关注> |

## 2. Top tasks（频率 × 失败代价）

| ID | Task | 频率 | 失败代价 | 对应 surface |
| --- | --- | --- | --- | --- |
| T1 | <task> | 高/中/低 | 高/中/低 | <surface> |
| T2 | <task> | | | |
| T3 | <task> | | | |

## 3. Walkthrough 与摩擦表

每步回答：① 知道目标吗？② 看得到可操作控件吗？③ 能预判反馈吗？④ 出错能恢复吗？

| # | Persona/Task | 步骤 | 卡点 | INT | 严重度 | 最小修法 | 状态 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| F1 | <P1/T1> | <步骤> | <卡点> | INT-0x | P0/P1/P2 | <修法> | open/closed |

> 无 P0 摩擦时必须写“无 P0 摩擦”；有 P0 时实现前必须修，且在本表记录关闭证据。

## 4. 用户测试计划（未执行 → UNVERIFIED）

- 对象：≥5 名真实用户（非实现者）
- 任务：T1/T2/T3；think-aloud
- 指标：成功率 / 首次点击 / 时间 / 错误数 / SEQ-SUS
- 证据：`target/evidence/<change-id>/usability/{script.md,notes/*.md,scores.json}`
- 结论：<UNVERIFIED / 日期 + 结果>
