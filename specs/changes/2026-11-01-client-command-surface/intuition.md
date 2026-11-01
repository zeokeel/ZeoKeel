# command-surface（Batch 1）· 直觉判定（INT）

> 本文件与 `interaction.md` 同目录（第六件，`docs/19` §11.1、`docs/27` §2.7/§2.8、`docs/11` §8.6）。
> 只写本 surface 的直觉判据与映射；原则表/走查方法不在本文件复制。
> 状态：`active`（实现开始时**与 `interaction.md` 同时**改 `active`；planned 只 WARN）。
> 证据根：`target/evidence/2026-11-01-client-command-surface/intuition/`；走查见 `intuition-walkthrough.md`。
> `e2e proxy` 只证明可观察行为，不证明用户觉得直观；最终需 fresh-context 评审 + ≥5 人用户测试（当前 **UNVERIFIED**）。

| ID | 原则 | 场景 / top task | 用户直觉期望（可观察） | 反例 | 验证方式 | 证据 |
| --- | --- | --- | --- | --- | --- | --- |
| INT-01 | 可发现性 | T1 首次输入 `/` | `/` 菜单可见，分组「添加/指令/技能」；`/goal` 设置后 GoalBar 直接在 composer 上方 | 命令只能靠记忆；goal 状态藏在设置/右栏 | e2e proxy + review | `CMD2-MENU-01`（`target/evidence/2026-11-01-client-command-surface/interaction/CMD2-MENU-01.json`；`e2e/interaction/command-surface.spec.ts`）、`CMD2-GOAL-01`（同路径）；review: walkthrough W1 |
| INT-02 | 预期一致 | T2/T3 键位 | `/`、↑↓、Enter/Tab、Esc/Shift+Tab 与 DSH 一致；IME 中不触发；未知 `/x` 不当提示词 | Esc 不关/焦点丢；未知命令被发送 | e2e proxy | `CMD2-KEY-01`（同路径）、`CMD2-MENU-02`（同路径）；review: walkthrough W2 |
| INT-03 | 状态可见 | T2 执行命令 | 选中后 ≤100ms 出现 busy/成功/失败；export 显示路径/下载（Tauri 路径）；model/permission 显示应用值 | 点击无反馈/重复执行 | e2e proxy + review | `CMD2-MODEL-01`/`CMD2-EXPORT-01`/`CMD2-FEEDBACK-01`（各 `target/evidence/.../interaction/<ID>.json`） |
| INT-04 | 就近反馈 | T2 命令错误 | 未知/占位/附件不支持/超限都在菜单/行内可读，草稿/附件保留 | 只弹无上下文 toast；清空草稿 | e2e proxy + review | `CMD2-MENU-02`、`CMD2-FILE-01`、`CMD2-MODEL-01`（同路径） |
| INT-05 | 默认安全 | T2 不可逆/敏感 | Batch2 占位不可执行；goal set/clear 有内联确认；override 失败回滚；feedback 不假发送 | 占位被执行；清除无确认；显示“已提交”但无网络 | e2e proxy + review | `CMD2-GOAL-01`、`CMD2-FEEDBACK-01`、`CMD2-MENU-02`（同路径） |
| INT-06 | 可恢复 | T2 切会话/重启/失败 | 会话 model/permission 切回恢复；并发 turn 排队不串配置；turn 中保存 settings 不改当前 turn、结束后按 revision 生效；排队 turn 取消后跳过不报错；失败保留旧值；export/feedback 失败可重试 | 切会话沿用上一会话 override；并发 turn 串配置；失败后值已变 | e2e proxy | `CMD2-MODEL-01`、`CMD2-PERM-01`、`CMD2-EXPORT-01`（同路径） |
| INT-07 | 认知负担 | T1/T2 菜单 | 空查询只有 DSH 两节+技能组；非空过滤；不在 `/` 混入 UI 动作（保留 ⌘K） | 一屏 10+ 动作；命令与 UI 动作混排 | review（Hick/Miller） | `CMD2-MENU-01`；review: walkthrough W1/W4 |
| INT-08 | 文案即引导 | T1/T2 空/错误/权限 | 空态给下一步；权限弹窗说明“仅审批、无沙箱、不限制员工”；Batch2 明说未实现；错误不直出内部码 | “command/unknown”直出；把“完全权限”宣传成沙箱绕过 | review + i18n | `CMD2-MENU-02`、`CMD2-PERM-01`；review: walkthrough W3/W5；`i18n-check` |
| INT-09 | 键盘/焦点 | T2/T3 命令面 | 菜单/弹窗/GoalBar/反馈键盘可达；焦点可见并回 composer；`aria-activedescendant` 正确 | 只能鼠标；Esc 后焦点丢 | e2e proxy | `CMD2-KEY-01`、`CMD2-A11Y-01`（同路径） |
| INT-10 | 空态/错误态 | T1/T3 六态 | 无匹配/无模型/无 goal/无导出/离线/流式中都有可读状态，不白屏、不假成功 | 空白菜单；静默失败 | e2e proxy | `CMD2-ERR-01`、`CMD2-MENU-02`（同路径） |
| INT-11 | 感知性能 | T1 `/` 打开/过滤 | `/` 打开 <100ms、过滤每键 <16ms；大量命令不卡；busy 不闪烁 | 菜单延迟、输入卡顿 | e2e/perf + review | `CMD2-MENU-01`；review: walkthrough W1/W2 |
| INT-12 | 单一模式 | T1/T2 菜单/弹窗 | `/` 与 ⌘K 共用一个 catalog/执行器；弹窗复用 ui-kit Modal；无第二套 menu/Dialog/审批 | 两套命令/两套 Esc/两套 Dialog | review + C17 | `CMD2-MENU-01`、`CMD2-A11Y-01`；review: walkthrough W4 |

## 覆盖说明

- P0 交互 ID → INT 映射（每个 P0 至少 1 条）：
  - `CMD2-MENU-01` → INT-01/02/07/09/11/12；
  - `CMD2-MENU-02` → INT-02/04/05/08/10；
  - `CMD2-FILE-01` → INT-03/04/05；
  - `CMD2-MODEL-01` → INT-03/04/05/06；
  - `CMD2-PERM-01` → INT-04/05/06/08；
  - `CMD2-GOAL-01` → INT-01/03/05/06/10；
  - `CMD2-EXPORT-01` → INT-03/05/06/08；
  - `CMD2-FEEDBACK-01` → INT-03/04/05/08。
- P1：`CMD2-KEY-01` → INT-02/09；`CMD2-A11Y-01` → INT-08/09/10/12；`CMD2-ERR-01` → INT-06/10。
- 用户测试（≥5 人 think-aloud）：**UNVERIFIED**（未执行；发布前按 `intuition-walkthrough.md` §4；未执行前任何 acceptance/发布文案不得声称“符合直觉/直观”）。
- 未解决 P0 摩擦：**无 P0 摩擦**（见 `intuition-walkthrough.md` §3；F1–F4 已关闭）。
- 状态：`active`；实现开始时**与 `interaction.md` 同时**改 `active` 并补 e2e 证据/走查复核。
