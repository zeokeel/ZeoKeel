# composer（Phase 2a）· 交互契约

> 本文件是 `2026-11-01-client-composer-surface` 的交互契约（`planned`），不是长期规范。
> 长期规则见 `docs/19` §11、`docs/27` §2.5、`docs/11` §8.6；不要在这里复制定义。
> 交付要求：P0/P1 条目必须映射 `@interaction:<ID>` 测试与证据；没有测试/证据的条目不算完成。

- 表面 / 入口：会话页底部输入区（`AssistantComposer`，Phase 2a）
- 归属包：`@zeokeel/ui-chat`（+ `apps/desktop/web` 唯一命令目录/actions）
- 权威文档：`docs/19` §2/§11、`docs/27` §2.5/§2.6、`docs/11` §8.6
- 状态：active（实现阶段；P0/P1 ID 均有 `@interaction:<ID>` 测试与证据）
- 直觉：同目录 `intuition.md` / `intuition-walkthrough.md`（C20；用户测试 UNVERIFIED）
- 证据根：`target/evidence/2026-11-01-client-composer-surface/interaction/`
- 测试：`e2e/interaction/composer-phase2a.spec.ts`（带 `@interaction:<ID>` 标签；实现时新建）

## 1. 元素与语义

| 元素 | role / accessible name | 键盘 | 默认焦点 | 备注 |
| --- | --- | --- | --- | --- |
| 输入框 | `textbox` /「消息输入框」 | Enter 发送、Shift+Enter 换行、Esc 关菜单 | 是 | 原生 textarea，自管 IME/键盘状态机；保持既有 textbox role（基线测试依赖） → `aria-expanded` 不放在 textarea（ARIA 不允许 role=textbox 使用它），菜单展开经 `aria-haspopup="listbox"` + `aria-controls` + `aria-activedescendant` + `role=status` live region 暴露 |
| `/` 候选菜单 | `listbox` + `option`；`aria-expanded`/`aria-controls`/`aria-activedescendant` | ↑/↓/Enter/Esc | 否 | 与 ⌘K 共用唯一命令目录 |
| `@` 文件候选菜单 | `listbox` + `option`（kind 标注） | ↑/↓/Enter/Esc | 否 | 数据来自 `workspace.files`，不新增 IPC |
| 引用/附件 chip | `button`（可删除）/ 图片 | Delete/Backspace 移除 | 否 | 非内联原子 token（design §2.7） |
| 模型标签 | `button` / 当前模型名 | Enter/Space | 否 | 只读展示，点击打开设置并定位模型相关 pane（personal `provider`/enterprise `enterprise`） |
| 用量指示 | `status` /「本轮用量」 | — | 否 | 只读，读屏可读 |

## 2. 状态模型

状态：`empty` / `loading` / `error` / `offline` / `streaming` / `cancelled` + `busy` / `disabled` / `composing` / `menu-open`。

| 状态 | 进入条件 | 可见反馈 | aria（live/expanded/selected…） | 退出 |
| --- | --- | --- | --- | --- |
| composing | IME compositionstart→end | 候选窗由输入法管 | `aria-expanded=false`（菜单不打开） | compositionend |
| menu-open | 词首 `/` 或 `@` | 候选菜单 + 高亮项 | `aria-expanded=true` + `aria-activedescendant` | Enter/Esc/点击外部 |
| streaming | 流式回复中 | 发送禁用、停止可见 | 停止按钮名称「停止」 | turn 完成/取消 |
| error | 上传/发送失败 | 行内错误 + i18n | `role=alert` | 重试/移除/重新输入 |
| offline | 断网 | 发送禁用 + 横幅 | 横幅可读 | 恢复连接 |

## 3. 交互条目（必须 1:1 映射测试）

| ID | 前置 | 人的动作（点击/按键/输入/拖放） | 期望（可见 + aria + 焦点 + 持久化） | 优先级 | 测试落点 | 证据 |
| --- | --- | --- | --- | --- | --- | --- |
| CMP2-CMD-01 | 已有会话 | 词首输入 `/`，输入过滤（如「模」），↑/↓，Esc | `/` 打开 DSH 命令 listbox（添加 file/goal + 指令 model/permission/export 等可见，**不含 UI 动作**）；过滤大小写不敏感；↑↓ 只移高亮、焦点留输入框；Esc 关闭且草稿保留、焦点回输入框（命令执行断言归 `CMD2-*`） | P0 | `composer-phase2a.spec.ts` | `CMP2-CMD-01.json` |
| CMP2-CMD-02 | 菜单已打开 | 输入 `/zzz`；输入 `/plan` 后 Enter；Esc | 无匹配显示「没有匹配的命令」空态、不发送；Batch 2 占位（`/skill`；`/compact`/`/plan` 已真实化）disabled 且 inert（不执行/不发送、显示「未实现（Batch 2）」）；Esc 保留草稿并回输入框 | P0 | `composer-phase2a.spec.ts` | `CMP2-CMD-02.json` |
| CMP2-AT-01 | 会话有 workspace | 词首输入 `@` 或点引用按钮 | 打开文件菜单（数据来自 `workspace.files`）；过滤 path（大小写不敏感）；目录/文件均可见并标注 kind | P0 | `composer-phase2a.spec.ts` | `CMP2-AT-01.json` |
| CMP2-AT-02 | `@` 菜单已打开 | 选择文件/目录；重复选择同一路径；凑满 5 条后再加 | 生成原子引用 chip、从输入框移除 `@query`、去重、按添加顺序；引用+附件合并 ≤5，超出禁用新增并提示 | P0 | `composer-phase2a.spec.ts` | `CMP2-AT-02.json` |
| CMP2-AT-03 | `@` 菜单已打开 | 文件列表空/加载中/错误/离线 | 显示对应状态与可读提示；不阻塞继续输入普通文本 | P1 | `composer-phase2a.spec.ts` | `CMP2-AT-03.json` |
| CMP2-ATT-01 | 已有会话 | 拖放文件 / 文件选择上传合法文件 | 显示上传中 chip（名称/大小/类型）；图片缩略图用 FileReader `data:` URL（非 blob/CSP 变更）；成功后变为可发送 chip；每条消息 ≤5 | P0 | `composer-phase2a.spec.ts` | `CMP2-ATT-01.json` |
| CMP2-ATT-02 | 拖放/选择文件 | 上传不支持类型 / >1 MiB / 第 6 个 / 空文件 / 上传失败 | 可读 i18n 错误，草稿与其它 chip 保留，可移除失败项或重试；不发送半成品附件 | P0 | `composer-phase2a.spec.ts` | `CMP2-ATT-02.json` |
| CMP2-ATT-03 | 附件上传成功 | 刷新页面并经 `workspace.files` 断言 | 文件写入 `<workspace>/.zeokeel/attachments/<conversation_id>/`；chip 刷新后仍可见 | P1 | `composer-phase2a.spec.ts` | `CMP2-ATT-03.json` |
| CMP2-INFO-01 | 个人版已配 provider / 企业版已选网关模型 / 未配置 | 渲染 composer；点击模型标签 | 显示当前模型标签（未配置显示「未配置模型」/「未选择网关模型」）+ 可读 aria-label；点击打开设置并定位模型相关 pane（`provider`/`enterprise`；`openSettings(paneId)` 深链）；设置保存后标签自动更新 | P1 | `composer-phase2a.spec.ts` | `CMP2-INFO-01.json` |
| CMP2-INFO-02 | 本轮有 provider usage | 发送并等待完成；再刷新 | 显示本轮 input/output/cached/total 与会话累计；无 usage 显示「用量未报告」（不显示 0、不估算）；刷新后与会话持久化一致 | P1 | `composer-phase2a.spec.ts` | `CMP2-INFO-02.json` |
| CMP2-KEY-01 | 输入框聚焦、菜单未打开 | Enter / Shift+Enter；菜单打开时 Esc | Enter 发送；Shift+Enter 换行不发送；Esc 关闭菜单且不丢草稿；焦点保持在输入框 | P0 | `composer-phase2a.spec.ts` | `CMP2-KEY-01.json` |
| CMP2-IME-01 | 输入框聚焦 | compositionstart 后按 Enter、输入 `/`/`@`；正文中间输入 `/` | composing 中不发送、不开菜单；`/`/`@` 仅词首触发；焦点不回退/不丢草稿 | P0 | `composer-phase2a.spec.ts` | `CMP2-IME-01.json` |
| CMP2-FILE-01 | 草稿含引用/附件 | 发送；刷新/重载历史 | 用户消息 = 正文 + 规范化引用块（`--- referenced files ---` + 每行 `@<path>`）；正文为空但有引用/附件时允许发送；UI 渲染 chip/列表不显示原始块，刷新后仍识别；去重且顺序稳定 | P0 | `composer-phase2a.spec.ts` | `CMP2-FILE-01.json` |
| CMP2-A11Y-01 | 菜单/附件/模型/用量可见 | axe 扫描 + 键盘遍历 | 0 serious/critical；`aria-expanded`/`aria-controls`/`aria-activedescendant` 正确；候选 `listbox`/`option` 语义；六态不白屏 | P1 | `composer-phase2a.spec.ts` | `CMP2-A11Y-01.json` |
| CMP2-VIS-01 | 新建空会话并打开 `/` 菜单 | 渲染 composer + 菜单 | 截图与基线一致（light/1280；禁动画；阈值 0.002） | P1 | `composer-phase2a.spec.ts` | `CMP2-VIS-01.json` |

- ID 规则：`<SURFACE>-<AREA>-<NN>`（本 surface 前缀 `CMP2-`；AREA 如 `CMD`/`AT`/`ATT`/`INFO`/`KEY`/`IME`/`FILE`/`A11Y`/`VIS`）。
- 优先级：P0 = 主路径/焦点/数据一致性；P1 = 可达性/回归基线；P2 = 可选增强（不进门禁）。
- 一个 ID 至少一个测试；一个测试可覆盖多个 ID（标签可多个），但不得引用未定义 ID。

## 4. 视觉 / ARIA 基线

| ID | 状态 | 主题 | 宽度 | 基线类型 | 基线文件 | 掩码/阈值 |
| --- | --- | --- | --- | --- | --- | --- |
| CMP2-VIS-01 | empty + `/` 菜单打开 | light | 1280 | 人工确认截图（**像素基线 UNVERIFIED**，待 `pnpm e2e:ui:update` 人工确认后接入） | `target/evidence/2026-11-01-client-composer-surface/interaction/CMP2-VIS-01.png` | 结构/布局断言 + 截图证据；agent 不得自动接受基线 |
| CMP2-A11Y-01 | menu-open / chips | — | 1280 | axe + 显式 ARIA 断言（`aria-haspopup`/`aria-controls`/`aria-activedescendant`/listbox/option/live region） | 无快照基线（aria snapshot 需人工确认，**UNVERIFIED**） | axe 0 serious/critical |

## 5. 预算与一致性

- 交互延迟：输入 <16ms；菜单打开 <100ms；附件上传显示 pending。
- 复用：只用 ui-kit 的 Button/Dialog/Listbox/Tooltip/Toast/Empty/Error；禁止第二套 focus/toast/dialog/menu（C17）。
- token：颜色/圆角/间距/动效只用 CSS vars；禁止 raw hex / magic px。
- i18n：所有文案来自字典（zh/en）；禁止硬编码中文/英文。
- 六态：每个交互组件必须覆盖；缺一即不通过评审。
- 视觉更新：只允许人确认后 `pnpm e2e:ui:update`；CI 不得自动接受。
- 门禁：实现开始时把本文件 `状态` 改 `active`，并把 P0/P1 标签/证据接入 `python3 scripts/ui-interaction-gate.py`。
- **工具依赖（P1-5）**：`pnpm e2e:ui` 与 `scripts/ui-interaction-gate.py` 由 `2026-11-01-ui-interaction-verification` 落地；本文件写作时两者尚不存在，当前 **UNVERIFIED**，实现前必须确认可用后再声称门禁绿。
- **演进（`2026-11-01-client-command-surface`，task-42）**：composer `/` 的命令目录已由 command-surface 接管为 DSH 1:1 slash 表面（`/file` `/model` `/permission` `/goal` `/export` `/feedback` + Batch2 占位）；UI 动作（新建会话/侧栏/设置/右栏/cancel/retry）仅保留在 ⌘K palette。`CMP2-CMD-01/02` 仅验证 slash 菜单的打开/过滤/导航/关闭/占位语义，命令执行与会话级行为由 `CMD2-*` 覆盖；两 surface 共用同一 `ui.commands` 目录与唯一 `runCommand` 执行器（C17）。
