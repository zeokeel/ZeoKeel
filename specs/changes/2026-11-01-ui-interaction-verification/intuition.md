# 基线直觉判定（5 个已实现 surface）

> 本文件与 `interaction.md` 同目录，属于 `2026-11-01-ui-interaction-verification` 的直觉基线（`active`）。
> 规范真源：`docs/27` §2.7（INT 判定）/§2.8（走查与用户测试）、`docs/19` §11.1（必填/证据）、`docs/11` §8.6（门禁口径）。
> 证据根：`target/evidence/2026-11-01-ui-interaction-verification/intuition/`；走查见 `intuition-walkthrough.md`。
> 注意：INT 不由 `@interaction` 标签门禁判定；本文件的存在性/最低行数由 gate 检查，质量由 reviewer 走查 + 用户测试判定。

| ID | 原则 | surface | 场景 / top task | 用户直觉期望 | 反例 | 验证方式 | 证据 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| INT-01 | 可发现性 | composer | 首次发送一条消息 | 输入框、发送、停止在视线内可见；不 hover 也能发现 | 发送按钮只有 hover 才出现 | e2e + review | `interaction/CMP-*` + 走查记录 |
| INT-02 | 预期一致 | 全部 | Esc/Enter/Tab/⌘ 键 | 与平台/产品既有行为一致；Esc 关最上层 | 弹窗里 Esc 无反应 | e2e | `SET-ESC-01`、`CMD-ESC-01`、`INS-TOGGLE-01` |
| INT-03 | 状态可见 | composer/settings | 发送、保存、加载 | 100ms 内出现 busy/流式/完成/错误 | 点击后无反馈，用户重复点 | e2e | `CMP-FOCUS-01`、`SET-OPEN-01` |
| INT-04 | 就近反馈 | settings | 字段校验 | 错误出现在字段旁，可读且可修 | 只弹全局“保存失败” | e2e + review | `SET-OPEN-01` + 走查 |
| INT-05 | 默认安全 | settings/sessions | 关闭/删除/危险动作 | 危险动作需确认或可撤销；默认焦点不在破坏性按钮 | 回车直接删除会话 | e2e + review | 现有 `SET-ESC-01` + 走查 |
| INT-06 | 可恢复 | composer/settings | 发送失败/关闭设置 | 失败保留草稿；关闭不丢未保存内容 | 失败后输入框被清空 | e2e | `CMP-PERSIST-01` + 走查 |
| INT-07 | 认知负担 | settings | 调整模型/Agent 参数 | 单屏主要选择 ≤7；分组/渐进披露；识别优于回忆 | 40 个字段一屏堆叠 | review（Hick/Miller） | 走查摩擦表 |
| INT-08 | 文案即引导 | 全部 | 空态/错误/toast | 动词开头、说明下一步、无内部术语 | “操作失败（code 500）” | review + i18n | 走查 + `i18n-check` |
| INT-09 | 键盘/焦点 | 全部 | 键盘完成主任务 | 键盘可达、焦点可见、关闭回原位 | 只能用鼠标保存 | e2e | `SET-ESC-01`、`CMD-NAV-01` |
| INT-10 | 空态/错误态 | sessions/inspector | 无会话/无文件/加载失败 | 空态给下一步；错误给原因+修法+重试 | 空白页或只有红叉 | e2e + review | `SES-*`、`INS-*` + 走查 |
| INT-11 | 感知性能 | 全部 | 面板切换/输入 | 400ms 内反馈；骨架避免闪烁 | 输入卡顿、整页白屏 | e2e/perf | 走查 + 现有 e2e |
| INT-12 | 单一模式 | 全部 | Dialog/Menu/Toast/焦点 | 同一交互只有一套实现 | 两个弹窗 Esc 行为不同 | review + C17 | 走查 + `docs/27` §2.5 |

## P0 → INT 映射（11/11，逐条）

| P0 交互 ID | INT | 说明 |
| --- | --- | --- |
| CMP-FOCUS-01 | INT-03、INT-09 | 发送后状态可见、焦点留在输入框 |
| CMP-KEY-01 | INT-02、INT-09 | Enter/Shift+Enter 符合预期，键盘可完成 |
| CMP-IME-01 | INT-02、INT-09 | IME composing 不误触发，焦点/键盘安全 |
| CMP-PERSIST-01 | INT-03、INT-06 | 状态可见 + 刷新后恢复 |
| SET-OPEN-01 | INT-03、INT-10 | 打开有反馈，dialog 可读 |
| SET-ESC-01 | INT-02、INT-09 | Esc 层叠/焦点回位符合预期 |
| INS-TOGGLE-01 | INT-01、INT-12 | Ctrl+B 可发现，右栏单一模式 |
| CMD-OPEN-01 | INT-01、INT-12 | 命令入口可见，面板单一模式 |
| CMD-NAV-01 | INT-02、INT-09 | 方向键/aria 符合预期 |
| CMD-ESC-01 | INT-02、INT-09 | Esc 关闭并回焦点 |
| SES-NEW-01 | INT-01、INT-10 | 新建入口可见，空态给下一步 |

## 覆盖说明

- 上表逐条覆盖 5 个 surface 的 11 个 P0 交互 ID（CMP 4、SET 2、INS 1、CMD 3、SES 1）。
- 用户测试（≥5 人）尚未执行：发布前按 `intuition-walkthrough.md` §5 执行；未执行前相关结论标 **UNVERIFIED**。
- 走查发现（P1/P2）见 `intuition-walkthrough.md` §3/§4 摩擦表；P0 未解决不得进入实现/发布（本基线无 P0 未决）。
