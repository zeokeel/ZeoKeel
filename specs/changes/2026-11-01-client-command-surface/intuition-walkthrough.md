# command-surface（Batch 1）· 认知走查（intuition walkthrough）

> 方法：`docs/27` §2.8（3 persona × 3 top task × 每步 4 问）；本走查是**专家走查**，不等于用户测试。
> 摩擦项引用 `intuition.md` 的 INT ID；P0 未解决不得进入实现/发布。
> 状态：`planned`（实现前先走查；实现后由 fresh-context 评审复核，见 §5）。

## 1. Personas

| ID | Persona | 特征 | 关注点 |
| --- | --- | --- | --- |
| P1 | 首次使用者 | 不懂代码、第一次用 ZeoKeel；知道 DSH 的 `/` 但不熟我们的 UI | `/` 里能否找到 DSH 命令；UI 动作去 ⌘K 后是否迷路；批量 2 占位是否误导 |
| P2 | 高频键盘用户 | 每天用、键盘流 | `/`、↑↓、Enter/Tab、Esc/Shift+Tab 是否与 DSH 一致；model/permission 切会话是否稳定；GoalBar 是否可键盘操作 |
| P3 | 出错/辅助技术用户 | 键盘 + 读屏；离线/权限失败/后端失败 | 未知命令/占位是否可读；override/导出/反馈失败是否可恢复；aria/焦点是否正确 |

## 2. Top tasks（频率 × 失败代价）

| ID | Task | 频率 | 失败代价 | 对应 surface |
| --- | --- | --- | --- | --- |
| T1 | 输入 `/`，用 `/file` 添加附件、用 `/model` 选会话模型 | 极高 | 高（附件/模型错用） | `/` 菜单 + popup + composer |
| T2 | `/permission` 切审批、`/goal` 设/查看/清除目标、`/export` 导出、`/feedback` 反馈 | 高 | 高（审批/目标/数据） | popup + GoalBar + dialog/结果 |
| T3 | 键盘/读屏完成 T1，并在切会话/重启/失败后恢复 | 高 | 中 | 全命令面 + 恢复 |

### 2.1 3×3 覆盖矩阵（防漏格）

| Persona \ Task | T1 `/file`+`/model` | T2 `/permission`+`/goal`+`/export`+`/feedback` | T3 键盘/恢复 |
| --- | --- | --- | --- |
| P1 首次使用者 | W1 | W2/W3 | W8 |
| P2 高频键盘用户 | W4 | W5/W6 | W7 |
| P3 出错/辅助技术用户 | W8 | W9 | W10 |

> 每个格子至少一个 W 行；不适用组合写 N/A + 理由 + 替代验证（本矩阵无 N/A）。

## 3. Walkthrough 与摩擦表

每步回答：① 知道目标吗？② 看得到可操作控件吗？③ 能预判反馈吗？④ 出错能恢复吗？

| # | Persona/Task | 步骤 | ①目标 | ②控件 | ③反馈 | ④恢复 | 卡点 | INT | 严重度 | 最小修法 | 状态 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| W1 | P1/T1 | 首次输入 `/`，找 `/file`/`/model` | 知道 DSH 有 `/` 命令 | composer 有 `/` 提示；菜单分两节+技能；UI 动作不在 `/` | 选中 `/file` 打开选择器；`/model` 显示当前值 | Esc 回 composer；取消保留草稿 | 找不到 UI 动作（已去 ⌘K）；Batch2 占位误以为可用 | INT-01/07/08 | P1 | composer hint 同时提示 `/` 与 ⌘K；Batch2 文案「未实现（Batch 2）」；docs/19 更新 | closed（design §2.1/§2.2、tasks #4/#5） |
| W2 | P1/T2 | `/goal` 输入目标→确认；看 GoalBar | 想建目标 | `/goal ` 留在输入框；确认区可见；GoalBar 在 composer 上方 | 确认后 GoalBar 显示 objective/rounds | 取消/清空可回退 | 目标设置/清除不可逆无确认 | INT-01/03/05 | **P0** | goal set/clear 内联确认（文本「设置/清除目标」）；取消不发 IPC；GoalBar 刷新 | closed（spec R6/design §2.5、interaction `CMD2-GOAL-01`、tasks #7） |
| W3 | P1/T2 | `/feedback` 分类+描述→提交；再问“发到哪” | 想反馈 | 弹窗分类/文本域/提交 | 明示「本地保存」；失败 inline | 取消/失败保留表单可重试 | 企业版可能误以为已发送 | INT-04/05/08 | **P0** | 只本地写 JSON；企业版文案「服务端未实现（UNVERIFIED）」；不显示已发送 | closed（spec R8/design §2.7、tasks #9） |
| W4 | P2/T1 | `/model`：personal 手动输入/recent、enterprise 列表；切会话再回来 | 不碰鼠标 | 菜单/弹窗可 Tab/↓；当前值高亮 | 选择后 label 更新；切回恢复；并发 turn 排队；turn 中 settings.set 不改当前 override、结束后按 revision 生效 | Esc 关闭不改变 | 切会话/并发 turn 沿用别会话模型；settings.save 污染当前 turn = 数据串台 | INT-02/03/06/09 | **P0** | 会话 override + 全局 config-mutex turn 前应用/turn 后恢复；失败回滚；`CMD2-MODEL-01` | closed（spec R4/design §2.3、tasks #6） |
| W5 | P2/T2 | `/permission` 选择「工作区写」；读文案；切会话 | 想切审批 | popup 三档可见；说明「仅审批、无沙箱、不限制员工」 | 当前值高亮，切会话恢复 | 失败回滚 | 把「完全权限」理解成沙箱绕过 | INT-04/05/06/08 | **P0**（语义误导） | UI/文档明确映射与差异；不写全局；失败回滚 | closed（spec R5/design §2.4、`CMD2-PERM-01`、tasks #6） |
| W6 | P2/T2 | `/export`；`/goal clear` 确认；看文件/GoalBar | 想保存/清理 | export 立即执行；clear 确认区；GoalBar 按钮 | export 成功路径/失败；Tauri 展示路径；clear 后 GoalBar 消失 | 失败重试；goal 原值保留 | 导出路径逃逸/大文件；clear 误删 | INT-03/05/06/08 | **P0**（路径/不可逆） | export `safe_join_under`+8 MiB；Tauri 不做自动打开；goal clear 内联确认；失败保留原值 | closed（spec R6/R7/design §2.6、tasks #7/#8） |
| W7 | P2/T3 | 键盘完成 W4/W5；Esc/Shift+Tab；重启后检查 override | 想键盘流 | 所有控件可 Tab；aria-activedescendant | 回 composer；重启恢复 | 失败/取消保留原值 | 焦点丢/重启丢失 | INT-02/09 | P1 | 菜单焦点留 textarea；override 持久化；e2e 键盘+重启 | closed（interaction `CMD2-KEY-01`、tasks #6/#10） |
| W8 | P3/T1 | 读屏打开 `/`，选 `/file`；未知 `/xx` | 想用命令 | listbox/option 有 name；错误 role=alert | 选择器/错误可读；草稿保留 | Esc 回 composer；失败可重试 | 未知命令被当提示词发送 | INT-02/04/08/09/10 | **P0**（数据/安全） | 解析器拒绝未知名；`command/unknown` i18n 错误；不调用发送；`CMD2-MENU-02` | closed（spec R2/design §2.2、tasks #5） |
| W9 | P3/T2 | 离线/后端失败时 `/model`/`/export`/`/feedback` | 想知道失败原因 | 错误可见 | 旧值/表单/草稿保留；可重试 | 恢复重试 | 假成功/清空 | INT-06/08/10 | P1 | 六态；错误 role=alert；失败保留原值；export/feedback 可重试 | closed（interaction `CMD2-ERR-01`、tasks #10） |
| W10 | P3/T3 | 读屏遍历菜单/popup/GoalBar/反馈；axe | 完成 T1/T2 | 角色/名称/aria 正确 | busy/错误可读 | 焦点回 composer/入口 | 焦点丢/axe 违规 = 不可用 | INT-09/10/12 | P1 | ui-kit 原语 + 单一 Modal；axe 0 serious；`CMD2-A11Y-01` | closed（interaction `CMD2-A11Y-01`、design §2.2） |

| W11 | P3/T3 | 两会话同时发送，B 排队等 mutex；B 被 chat.cancel | 想知道取消是否生效 | B 的取消提示 | B 获锁后检查 cancelled 并跳过，不执行/不报错；A 不受影响 | 后续 turn 可正常执行 | 排队 turn 仍执行/报错/占用锁 | INT-06/10 | P1 | `chat.cancel` 只作用当前运行 turn；排队 turn 获锁后检查 cancelled 跳过；单测/e2e | closed（spec R4/design §2.3、tasks #6/#10） |

**P0 摩擦结论**
- W2（goal set/clear 无确认）、W3（feedback 假发送）、W4（model 跨会话串）、W5（permission 语义误导）、W6（export 路径/大小、goal clear 不可逆）、W8（未知命令被发送）六项 P0 卡点均已用最小修法关闭，并落到 spec/design/interaction/tasks；**无 OPEN P0 摩擦**。
- 3×3 矩阵满格；可进入实现（仍待用户拍板 D1–D6 + 独立评审）。

## 4. 用户测试计划（未执行 → UNVERIFIED）

- 对象：≥5 名真实用户（非实现者，至少 2 名不熟悉本产品）
- 任务：T1/T2/T3；think-aloud；不提示操作路径
- 指标：任务成功率 / 首次点击正确率 / 完成时间 / 错误数 / SEQ-SUS
- 证据：`target/evidence/2026-11-01-client-command-surface/usability/{script.md,notes/*.md,scores.json}`
- 结论：**UNVERIFIED**（未执行；执行前不得声称“符合直觉/直观”——`docs/11` §8.6、`docs/27` §2.8）

## 5. 评审复核（fresh context，待执行）

- 评审者（非实现者）按本走查复跑 P1–P3 × T1–T3，核对 `intuition.md` 的 P0 映射覆盖 interaction.md 全部 P0；
- 结论记录到 `docs/reviews/` 并回填 `acceptance.md`；P0 摩擦未关闭 → 不通过。
