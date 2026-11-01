# activity-panels（Phase 2b 子 spec）· 认知走查（intuition walkthrough）

> 方法：`docs/27` §2.8（3 persona × 3 top task × 每步 4 问）；本走查是**专家走查**，不等于用户测试。
> 摩擦项引用 `intuition.md` 的 INT ID；P0 未解决不得进入实现/发布。
> 状态：`active`（2026-11-01 走查已完成并随实现生效；实现后由 fresh-context 评审复核，见 §5）。

## 1. Personas

| ID | Persona | 特征 | 关注点 |
| --- | --- | --- | --- |
| P1 | 首次使用者 | 不懂代码、第一次用 ZeoKeel | 活动入口在哪；7 个分区分别是什么；能否找到并停止一个后台任务 |
| P2 | 高频键盘用户 | 每天使用、键盘流、熟悉右栏/tab | 切 tab/刷新/停止/删除是否键盘可达；删除确认是否可预期；轮询是否打断阅读 |
| P3 | 出错/辅助技术用户 | 键盘 + 读屏；离线/权限失败/任务已结束 | 错误可读可恢复；aria/焦点正确；stale/失败不白屏；present 预览失败可返回 |

## 2. Top tasks（频率 × 失败代价）

| ID | Task | 频率 | 失败代价 | 对应 surface |
| --- | --- | --- | --- | --- |
| T1 | 打开 activity，查看 jobs/subagents，停止一个运行中任务 | 极高 | 高（后台资源/数据一致性） | activity tab + jobs/subagents |
| T2 | 暂停/编辑/删除一个 schedule，并处理非法输入 | 中 | 高（定时任务/文件） | activity schedule 分区 |
| T3 | 查看 turnTail present、goal.get、切会话/断网后恢复；键盘/读屏完成 T1 | 高 | 中 | turnTail + goal + 轮询/六态/a11y |

### 2.1 3×3 覆盖矩阵（防漏格）

| Persona \ Task | T1 查看/停止 | T2 schedule 编辑/删除 | T3 turnTail/恢复/a11y |
| --- | --- | --- | --- |
| P1 首次使用者 | W1/W2 | W11（N/A + 替代验证） | W6/W10 |
| P2 高频键盘用户 | W3 | W4 | W7 |
| P3 出错/辅助技术用户 | W12 | W5/W8/W9 | W5/W8/W9 |

> 每个格子必须至少一个 W 行（或 N/A + 理由 + 替代验证）；N/A 不得留空。

## 3. Walkthrough 与摩擦表

每步回答：① 知道目标吗？② 看得到可操作控件吗？③ 能预判反馈吗？④ 出错能恢复吗？

| # | Persona/Task | 步骤 | ①目标 | ②控件 | ③反馈 | ④恢复 | 卡点 | INT | 严重度 | 最小修法 | 状态 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| W1 | P1/T1 | 打开右栏找 activity | 知道有后台任务 | tab 名称「活动」可见；刷新按钮常驻 | 打开显示分区/骨架 | 可切回其它 tab | 4th tab 可能被忽略；无空态说明 | INT-01/07/10 | P1 | tab 常驻 + 空态说明「暂无运行中任务」；分区标题/计数 | closed（design §2.3、tasks #6） |
| W2 | P1/T1 | 找到 job 并点「停止任务」→ 内联确认（id/命令/影响） | 想停掉一个任务 | 行展开输出；停止按钮只在 running 行；确认区可见、默认焦点「取消」 | 取消不变；确认后 busy→状态刷新/列表更新 | 未知/已结束显示可读错误，面板仍可用 | 误停已完成/别的 workspace 任务 = 数据一致性 | INT-03/05/06 | **P0** | host jobs 按 cwd `Path::starts_with` 过滤；动作后重新 snapshot + host 当前状态校验；仅 running 可点；**内联二次确认后才发 action**；错误可读 | closed（spec R2/design §2.3、tasks #4/#6/#9） |
| W3 | P2/T1 | 用键盘停止另一个 running job（焦点在行上）→ 确认区 Tab 到「取消/确认」→ Enter | 想不用鼠标 | 行/按钮可 Tab 聚焦；确认区 alertdialog 可读 | Enter 后确认；取消不变；aria-live 播报 | Esc/取消后焦点回停止按钮 | 停止不可逆且误操作风险 | INT-02/05/09 | **P0** | 内联二次确认（默认焦点取消，Esc=取消）；确认后才 action；焦点返回；`ACT2-JOB-01` 增 confirm case | closed（spec R2/interaction `ACT2-JOB-01`、tasks #6/#9） |
| W4 | P2/T2 | 删除 schedule：点删除→内联确认→Enter | 想清理计时任务 | schedule 行删除按钮；确认区可见 | 确认后删除并刷新；取消不变 | 未知 id/`deleted 0` 显示「已不存在」 | 无确认直接删 = 不可逆 | INT-02/05 | **P0** | host 先 `list` 预检；`activity/schedule-invalid`；内联二次确认；取消/确认键盘可达 | closed（spec R6/design §2.3、tasks #4/#6） |
| W5 | P3/T2 | 编辑 interval=30s / prompt 超长 | 想改定时 | 行内编辑控件/开关 | 行内错误（可修文案） | 不写盘、保留原值 | 错误只弹 toast 或写坏文件 | INT-04/06 | P1 | host 预校验 interval≥60、prompt≤32 KiB；行内错误；tool 原子写 | closed（spec R6/design §2.4、tasks #4） |
| W6 | P1/T3 | 读 turnTail present 卡→点「在右栏查看」 | 想看交付文件 | 卡片在消息尾部，文件/描述可见 | 打开右栏 files 预览 | 文件不存在时可读错误、可返回消息 | 卡片与正文混在一起/空卡 | INT-01/06/10 | P1 | present 去重、卡片显示文件名/描述；无 present 不渲染；预览失败可关闭 | closed（spec R7/design §2.3、tasks #7） |
| W7 | P2/T3 | 切会话/关闭 activity 再回来 | 不想后台轮询污染新会话 | 切走即停 | 返回后重新加载当前会话 | 旧响应不覆盖新数据 | 旧轮询回包写错会话 = 数据串台 | INT-06/11 | **P0** | close/切走 clearInterval + AbortController/序号丢弃旧响应；切会话重置 snapshot | closed（design §2.3/§2.4、tasks #6） |
| W8 | P3/T3 | 断网/轮询失败/任务已结束 | 想理解发生了什么 | stale/错误提示可见 | 旧数据保留 + 可重试 | 恢复后自动/手动刷新 | 白屏、反复弹错、假成功 | INT-06/10 | P1 | stale 标记 + 保留旧数据；错误 role=alert + 重试；动作失败不乐观改状态 | closed（spec R3/R6、design §2.3） |
| W9 | P3/T3 | 键盘/读屏遍历 activity + 删除确认 | 完成 T1 | tablist/按钮/switch 有 role/name | 动作结果 aria-live | Esc 取消焦点回删除按钮 | 只能鼠标/焦点丢失 = 不可用 | INT-09/12 | P1 | 复用 ui-kit tab/list/switch/dialog；axe 0 serious；键盘 e2e | closed（interaction `ACT2-A11Y-01`、design §2.3） |
| W10 | P1/T3 | goal 为空/损坏时看 activity；刷新/切会话 | 想知道目标 | goal 分区空/降级可读 | 只读 goal.get；不写/不派生工具行 | 刷新恢复；不串会话 | 工具行派生与真实 goal 不一致 | INT-07/08 | P1 | 只读 goal.get（不进 snapshot）；空/损坏测试；plan 不展示 | closed（spec R4/design §2.3、tasks #4/#6） |

| W11 | P1/T2 | 首次用户看到 schedule 区，尝试编辑/删除 | N/A：首任务不涉及修改定时任务，频率低、失败代价高但不由首任务触发 | N/A：入口仍可见（W1） | N/A | N/A | 不适用组合，但不得漏格 | INT-01/07 | N/A | 替代验证：W1 覆盖入口发现性，W4/W5 覆盖编辑/删除与非法输入 | N/A（替代验证见左） |
| W12 | P3/T1 | 读屏/键盘打开 activity，停止 running job | 想停止任务 | tab/行/停止按钮有 role/name | 确认区 alertdialog 读出；动作结果 aria-live | Esc 取消回停止按钮 | 只能鼠标/焦点丢失 = 不可用 | INT-02/05/09/10 | P1 | 复用 ui-kit tab/list/button + alertdialog；axe 0 serious；键盘 e2e | closed（interaction `ACT2-A11Y-01`、tasks #9） |

**P0 摩擦结论**
- W2（job 作用域/已结束误停）、W3（job 停止误操作/不可逆）、W4（schedule 删除无确认/未知 id）、W7（轮询旧响应串会话）四项 P0 卡点均已用最小修法关闭，并落到 spec/design/interaction/tasks；**无 OPEN P0 摩擦**；3×3 矩阵已满格（含 N/A + 替代验证），可进入实现（仍待评审复评）。

## 4. 用户测试计划（未执行 → UNVERIFIED）

- 对象：≥5 名真实用户（非实现者，至少 2 名不熟悉本产品）
- 任务：T1/T2/T3；think-aloud；不提示操作路径
- 指标：任务成功率 / 首次点击正确率 / 完成时间 / 错误数 / SEQ-SUS
- 证据：`target/evidence/2026-11-01-client-activity-panels/usability/{script.md,notes/*.md,scores.json}`
- 结论：**UNVERIFIED**（未执行；执行前不得声称“符合直觉/直观”——`docs/11` §8.6、`docs/27` §2.8）

## 5. 评审复核（fresh context，待执行）

- 评审者（非实现者）按本走查复跑 P1–P3 × T1–T3，核对 `intuition.md` 的 P0 映射是否覆盖 interaction.md 全部 P0；
- 结论记录到 `docs/reviews/` 并回填 `acceptance.md`；P0 摩擦未关闭 → 不通过。
