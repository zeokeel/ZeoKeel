# composer（Phase 2a）· 认知走查（intuition walkthrough）

> 方法：`docs/27` §2.8（3 persona × 3 top task × 每步 4 问）；本走查是**专家走查**，不等于用户测试。
> 摩擦项引用 `intuition.md` 的 INT ID；P0 未解决不得进入实现/发布。
> 状态：`active`（2026-11-01，实现前专家走查 + 实现后复核）。

## 1. Personas

| ID | Persona | 特征 | 关注点 |
| --- | --- | --- | --- |
| P1 | 首次使用者 | 不懂代码、第一次用 ZeoKeel | 输入框旁能直接看到发送/引用/附件；不培训能发出第一条带文件的消息 |
| P2 | 高频键盘用户 | 每天用、键盘流、熟悉 ⌘K | `/` 与 ⌘K 命令一致；Enter/Shift+Enter/Esc/↑↓ 符合预期；焦点不丢 |
| P3 | 出错/辅助技术用户 | 键盘 + 读屏、离线/上传失败 | 菜单/用量/模型可读；附件错误可恢复；不白屏 |

## 2. Top tasks（频率 × 失败代价）

| ID | Task | 频率 | 失败代价 | 对应 surface |
| --- | --- | --- | --- | --- |
| T1 | 发出第一条消息（可含 `@` 引用/附件） | 极高 | 高 | composer |
| T2 | 用 `/` 执行命令或处理附件失败/超限 | 高 | 中 | composer + 命令目录 |
| T3 | 键盘/读屏完成 T1（含 Esc/IME） | 高 | 中 | composer + a11y |

## 3. Walkthrough 与摩擦表

每步回答：① 知道目标吗？② 看得到可操作控件吗？③ 能预判反馈吗？④ 出错能恢复吗？

| # | Persona/Task | 步骤 | 卡点 | INT | 严重度 | 最小修法 | 状态 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| F1 | P1/T1 | 输入 `/`：用户不知道 composer 支持命令 | 可发现性不足 | INT-01 | P1 | placeholder/键盘提示保留 Enter/Shift+Enter；`/` 与 `@` 作为明显按钮旁的可发现输入提示（走查确认按钮 `引用文件`/`添加附件` 常驻可见） | closed |
| F2 | P2/T2 | `/` 菜单与 ⌘K 命令集是否一致 | 两套命令/行为不一致风险 | INT-12 | P0 | 唯一 `buildCommandCatalog` + 唯一 `runCommand`；单测断言 catalog id 集合；两 surface 渲染同一数组 | closed |
| F3 | P2/T3 | 菜单打开时按 ↑↓ 是否移动光标/丢焦点 | 焦点/键盘意外 | INT-09 | P0 | 菜单打开时 ↑↓/Enter/Esc `preventDefault`；option `tabIndex=-1`；关闭后 focus 回 textarea | closed |
| F4 | P3/T2 | 附件超限/类型错误后草稿是否还在 | 可恢复性 | INT-04/06 | P1 | 客户端预校验 → 行内 `role=alert`；失败 chip 可移除/重试；草稿与其它 chip 保留 | closed |
| F5 | P2/T1 | 发送带引用后用户消息显示原始块 | 认知负担/信任 | INT-05/07 | P1 | `AssistantThread` 把引用块渲染为 chips（隐藏原文）；无块消息行为不变 | closed |
| F6 | P3/T3 | IME 组词中 Enter 是否误发送 | 数据一致性 | INT-02 | P0 | `compositionstart→end` 期间 Enter/`/`/`@` 不触发；e2e 合成 composition 事件回归 | closed |
| F7 | P1/T1 | 模型/用量是否可理解 | 状态可见 | INT-03/07 | P2 | 模型只读标签点击进设置；用量显示本轮+累计，无 usage 显示「用量未报告」（不估算） | closed |
| F8 | P3/T3 | 菜单/附件是否 axe 可读 | 可达性 | INT-08/10 | P1 | 菜单 `listbox/option` + `aria-controls/activedescendant`；错误 `role=alert`；axe 0 serious（`CMP2-A11Y-01`） | closed |

> **无 P0 摩擦**：F2/F3/F6 三个 P0 卡点在实现前已用最小修法关闭（唯一 catalog、焦点保持、IME 保护），实现后由 e2e/单测复核。

## 4. 用户测试计划（未执行 → UNVERIFIED）

- 对象：≥5 名真实用户（非实现者，至少 2 名不熟悉本产品）
- 任务：T1/T2/T3；think-aloud；不提示操作路径
- 指标：任务成功率 / 首次点击正确率 / 完成时间 / 错误数 / SEQ-SUS
- 证据：`target/evidence/2026-11-01-client-composer-surface/usability/{script.md,notes/*.md,scores.json}`
- 结论：**UNVERIFIED**（未执行；执行前不得声称"符合直觉/直观"——`docs/11` §8.6、`docs/27` §2.8）
