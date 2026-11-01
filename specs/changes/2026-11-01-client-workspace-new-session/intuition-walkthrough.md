# workspace-new-session · 认知走查（intuition walkthrough）

> 方法：`docs/27` §2.8（3 persona × 3 top task × 每步 4 问）；本走查是**专家走查**，不等于用户测试。
> 摩擦项引用 `intuition.md` 的 INT ID；P0 未解决不得进入实现/发布。
> 状态：`active`（实现前先走查；实现后由 fresh-context 评审复核，见 §5）。

## 1. Personas

| ID | Persona | 特征 | 关注点 |
| --- | --- | --- | --- |
| P1 | 首次使用者 | 第一次用、多个 workspace；习惯鼠标 | 能看到每个 Workspace 旁的“+”吗；点它会在该 Workspace 建吗；会不会误点行 |
| P2 | 高频键盘用户 | 每天用、多个 workspace、键盘流 | hover/focus 可见；Tab 顺序；Enter 创建到目标；active 行常显 |
| P3 | 出错/辅助技术用户 | 键盘 + 读屏；流式中/失败/离线 | 按钮名含 workspace；确认可读；失败可恢复；不静默取消流 |

## 2. Top tasks（频率 × 失败代价）

| ID | Task | 频率 | 失败代价 | 对应 surface |
| --- | --- | --- | --- | --- |
| T1 | 在指定 Workspace 行旁点“+”，在该 Workspace 新建并选中会话 | 极高 | 高（建错 workspace） | Workspace 行 scoped 按钮 |
| T2 | 用键盘在多个 Workspace 间新建；区分 active 行/全局按钮 | 高 | 高（建错/误触） | Workspace 行 + 全局新建 |
| T3 | 流式中在另一 Workspace 新建；取消/失败/恢复 | 中 | 高（丢流/数据） | scoped 按钮 + 确认 + controller |

### 2.1 3×3 覆盖矩阵（防漏格）

| Persona \ Task | T1 scoped 新建 | T2 键盘/多 workspace | T3 流式/失败恢复 |
| --- | --- | --- | --- |
| P1 首次使用者 | W1 | W4 | W7 |
| P2 高频键盘用户 | W2 | W5 | W8 |
| P3 出错/辅助技术用户 | W3 | W6 | W9 |

> 每个格子至少一行；本矩阵无 N/A（3 个任务对 3 persona 均有可走查步骤）。

## 3. Walkthrough 与摩擦表

每步回答：① 知道目标吗？② 看得到可操作控件吗？③ 能预判反馈吗？④ 出错能恢复吗？

| # | Persona/Task | 步骤 | ①目标 | ②控件 | ③反馈 | ④恢复 | 卡点 | INT | 严重度 | 最小修法 | 状态 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| W1 | P1/T1 | hover「项目 A」行，点右侧 `+` | 想在 A 建会话 | hover 出现按钮，按钮名/提示含 A | 点击 busy→A 下出现新会话并选中 | 失败 toast，选择不变 | 按钮 hover-only（触屏/键盘不可见） | INT-01/09 | **P0**（可访问性） | hover + focus-visible + active 行常显；`aria-label` 含 workspace；不再 hover-only | closed（spec R1/design §2.1、tasks #2） |
| W2 | P2/T1 | Tab 到行内 `+`，Enter 建到 B；再看 active 行 | 不碰鼠标在 B 建 | Tab 顺序折叠→标题→新建→`⋯`；focus ring | busy→B 下新会话 | 失败/取消不切 | 焦点被行吞/顺序乱 | INT-02/09 | P1 | 独立 button/focus-visible；stopPropagation；e2e 键盘序列 | closed（design §2.1/§2.5、tasks #2/#5） |
| W3 | P3/T1 | 读屏找“新建会话”按钮 | 想建到 A | 按钮名「在 A 新建会话」，role button | 创建后选中 A 新会话 | 失败可读 | 名称不含 workspace，无法判断目标 | INT-02/08 | P1 | 动态 aria-label/title；axe 0 serious；e2e | closed（interaction `SES2-WSNEW-02`、tasks #2/#5） |
| W4 | P1/T2 | 在「项目 A」行生成了，却点到相邻「项目 B」行 | 想在 A 建 | 两个行各自按钮 | 误点会建到 B | 可删除/归档错误会话 | 行内按钮目标混淆/点击行触发切换 | INT-02/07 | **P0**（数据一致性） | 按钮显式 `workspaceId`；`stopPropagation`；点击不切行；单测/e2e 多 workspace | closed（spec R1/design §2.1/§2.2、tasks #2/#3/#4） |
| W5 | P2/T2 | 对比 header `+`/全局按钮、⌘K 与行内 `+` | 知道区别 | 全局/⌘K 沿用 active；行内绑定该行；三入口 streaming 同一 guard | 非流式一致可预期；流式都禁用 | 无 workspace 禁用 | 以为行内按钮先要选行/全局可绕过 | INT-07/12 | P1 | 文案/位置区分；三入口统一 guard；docs/19 | closed（spec R2/design §2.6、tasks #3/#4） |
| W6 | P3/T2 | 读屏遍历行内按钮/全局按钮 | 想用键盘 | 两个按钮名称不同（含 workspace vs 全局） | 创建结果可读 | 焦点回行/按钮 | 名称重复/混淆 | INT-08/09 | P1 | 名称含 workspace；aria snapshot；axe | closed（interaction `SES2-WSNEW-02`） |
| W7 | P1/T3 | 当前会话流式中，在任意 Workspace 行/全局/⌘K 尝试新建 | 想新建 | 三入口全部禁用 + 同一 tooltip/aria/disabledReason | 停止/取消流后恢复 | 不建/不切/不串当前消息 | 任一入口绕过导致清空/串当前消息 | INT-05/06 | **P0**（数据一致性） | **streaming 时 scoped/global/⌘K 三入口统一 guard**；停止后恢复；不切换 activeConversationId；单测/e2e | closed（spec R2/R3/design §2.1/§2.2、tasks #3/#4/#5） |
| W8 | P2/T3 | 停止流后立刻点 `+`；再触发失败 | 快速恢复/可重试 | 停止后按钮恢复；失败 toast | 正常建到目标并选中 | 失败可重试 | 停止后仍禁用/失败后选择变 | INT-06 | P1 | 监听 streaming=false 恢复；失败不切；测试 | closed（spec R3/design §2.2、tasks #3/#5） |
| W9 | P3/T3 | 离线/失败时点 `+`；快速连点 | 知道失败/防重复 | busy/disabled 可见 | 可读错误、只建 1 个 | 恢复重试 | 重复会话/静默失败 | INT-03/04/10 | P1 | busy 防重复；toast 可读；失败不切；单测/e2e | closed（spec R4/design §2.1、tasks #3/#4） |

**P0 摩擦结论**
- W1（hover-only 不可访问）、W4（误建到相邻 workspace）、W7（流式中新建清空/串当前消息）三项 P0 卡点均已用最小修法关闭，并落到 spec/design/interaction/tasks；**无 OPEN P0 摩擦**（D3 修订为 streaming 禁用）。
- 3×3 矩阵满格；D1–D4 已定（Lead 2026-11-01，D3 修订为流式禁用）；可进入实现（仍待独立评审）。

## 4. 用户测试计划（未执行 → UNVERIFIED）

- 对象：≥5 名真实用户（非实现者，至少 2 名不熟悉本产品）
- 任务：T1/T2/T3；think-aloud；不提示操作路径
- 指标：任务成功率 / 首次点击正确率 / 完成时间 / 错误数 / SEQ-SUS
- 证据：`target/evidence/2026-11-01-client-workspace-new-session/usability/{script.md,notes/*.md,scores.json}`
- 结论：**UNVERIFIED**（未执行；执行前不得声称“符合直觉/直观”——`docs/11` §8.6、`docs/27` §2.8）

## 5. 评审复核（fresh context，待执行）

- 评审者（非实现者）按本走查复跑 P1–P3 × T1–T3，核对 `intuition.md` 的 P0 映射覆盖 interaction.md 全部 P0；
- 结论记录到 `docs/reviews/` 并回填 `acceptance.md`；P0 摩擦未关闭 → 不通过。
