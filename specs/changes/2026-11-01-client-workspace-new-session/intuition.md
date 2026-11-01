# workspace-new-session · 直觉判定（INT）

> 本文件与 `interaction.md` 同目录（第六件，`docs/19` §11.1、`docs/27` §2.7/§2.8、`docs/11` §8.6）。
> 只写本 surface 的直觉判据与映射；原则表/走查方法不在本文件复制。
> 状态：`active`（实现开始时**与 `interaction.md` 同时**改 `active`；planned 只 WARN）。
> 证据根：`target/evidence/2026-11-01-client-workspace-new-session/intuition/`；走查见 `intuition-walkthrough.md`。
> `e2e proxy` 只证明可观察行为，不证明用户觉得直观；最终需 fresh-context 评审 + ≥5 人用户测试（当前 **UNVERIFIED**）。

| ID | 原则 | 场景 / top task | 用户直觉期望（可观察） | 反例 | 验证方式 | 证据 |
| --- | --- | --- | --- | --- | --- | --- |
| INT-01 | 可发现性 | T1 hover Workspace 行 | Workspace 行旁出现「新建会话」按钮；不看文档也知道点它会在该行 Workspace 建 | 只有全局新建；按钮只在 hover 且无焦点/键盘不可达 | e2e proxy + review | `SES2-WSNEW-01`（`target/evidence/2026-11-01-client-workspace-new-session/interaction/SES2-WSNEW-01.json`；`e2e/interaction/sessions-workspace-new.spec.ts`）；review: walkthrough W1 |
| INT-02 | 预期一致 | T1/T2 点击/键盘 | 按钮名「在 <workspace> 新建会话」、点击建到该 Workspace；与全局按钮/行选中语义一致 | 点击建到 active workspace；按钮名只说“新建”；Enter 触发整行切换 | e2e proxy | `SES2-WSNEW-01`/`SES2-WSNEW-02`（同路径） |
| INT-03 | 状态可见 | T1 点击后 | 按钮 busy/禁用；新会话出现在该 Workspace 下并选中；active 行高亮 | 点击无反馈/重复建多个/新会话不在目标 workspace | e2e proxy | `SES2-WSNEW-01`（同路径） |
| INT-04 | 就近反馈 | T2 失败/不可用 | 创建失败/workspace 不可用在行/全局可读提示；选择不变 | 静默失败；选择跳到别的 workspace | e2e proxy | `SES2-WSNEW-03`（同路径） |
| INT-05 | 默认安全 | T2 流式中 | 流式期间 **scoped/global/⌘K 三入口**统一禁用，同一 tooltip/aria/disabledReason「当前正在生成，停止后可新建」；不建/不切/不串消息 | 流式中可点、新建清空/串当前消息 | e2e proxy + review | `SES2-WSNEW-03`（同路径）；review: walkthrough W7 |
| INT-06 | 可恢复 | T2 停止后恢复/失败 | 停止/取消当前流后按钮恢复，可正常建；失败可重试；原会话/流不受影响 | 停止后仍禁用；失败后 active 已变 | e2e proxy | `SES2-WSNEW-03`（同路径） |
| INT-07 | 认知负担 | T1 侧栏 | 每个 Workspace 行只有一个新的 scoped `+`，与折叠/`⋯` 不混淆；active 行常显、其它 hover/focus | 行内按钮过多；需要先记“先点 workspace 再点全局新建” | review（Hick/Miller） | `SES2-WSNEW-02`；review: walkthrough W1 |
| INT-08 | 文案即引导 | T1 无 workspace/流式禁用 | 无 workspace 提示先建 workspace；streaming 三入口同一文案「当前正在生成，停止后可新建」；不直出内部码 | 空白/“cannot create”；无解释的禁用 | review + i18n | `SES2-WSNEW-03`；review: walkthrough W7；`i18n-check` |
| INT-09 | 键盘/焦点 | T2 键盘/读屏 | active 行按钮常显；非 active hover/focus 可见；Tab 顺序合理；按钮 focus ring；Esc 关确认回原按钮 | 只能鼠标；focus 后按钮消失；按钮吞行点击 | e2e proxy | `SES2-WSNEW-02`（同路径）；review: walkthrough W2/W4 |
| INT-10 | 空态/错误态 | T1/T3 六态 | 无 workspace/创建失败/流式中/离线都有可读状态，不白屏 | 空行无提示；错误静默 | e2e proxy | `SES2-WSNEW-03`（同路径） |
| INT-11 | 感知性能 | T1 hover/点击 | hover 显示 ≤100ms；点击 busy ≤100ms；创建/选中无卡顿/闪烁 | 按钮延迟出现；点击后整栏闪 | e2e/perf + review | `SES2-WSNEW-01`；review: walkthrough W1 |
| INT-12 | 单一模式 | T1/T2 按钮 | scoped 新建复用全局 `session.create`/controller；三入口共用同一 streaming guard；无确认 Modal/无第二套建会话路径 | 两套建会话/额外确认 Dialog | review + C17 | `SES2-WSNEW-01`、`SES2-WSNEW-03`；review: walkthrough W4 |

## 覆盖说明

- P0 交互 ID → INT 映射（每个 P0 至少 1 条）：
  - `SES2-WSNEW-01` → INT-01/02/03/07/11/12；
  - `SES2-WSNEW-02` → INT-02/07/09/12。
- P1：`SES2-WSNEW-03` → INT-04/05/06/08/10。
- 用户测试（≥5 人 think-aloud）：**UNVERIFIED**（未执行；发布前按 `intuition-walkthrough.md` §4；未执行前任何 acceptance/发布文案不得声称“符合直觉/直观”）。
- 未解决 P0 摩擦：**无 P0 摩擦**（见 `intuition-walkthrough.md` §3；F1/F2 已关闭）。
- 状态：`active`；实现开始时**与 `interaction.md` 同时**改 `active` 并补 e2e 证据/走查复核。
