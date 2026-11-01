# right sidebar / sessions（Phase 2b）· 交互契约

> 本文件是 `2026-11-01-client-side-panels` 的交互契约（`planned`），不是长期规范。
> 长期规则见 `docs/19` §11、`docs/27` §2.5、`docs/11` §8.6；不要在这里复制定义。
> 交付要求：P0/P1 条目必须映射 `@interaction:<ID>` 测试与证据；没有测试/证据的条目不算完成。

- 表面 / 入口：右栏（files/changes/terminal + 宽度/折叠）与会话侧栏（`⋯` 菜单、内容检索）
- 归属包：`@zeokeel/ui-inspector`、`@zeokeel/ui-sessions`
- 权威文档：`docs/19` §2/§11、`docs/26`（C18）、`docs/27` §2.5/§2.6、`docs/11` §8.6
- 状态：planned（实现开始时改 active；planned 不参与门禁强制）
- 证据根：`target/evidence/2026-11-01-client-side-panels/interaction/`
- 测试：`e2e/interaction/inspector-*.spec.ts`、`sessions-ops.spec.ts`、`sessions-search.spec.ts`（带 `@interaction:<ID>` 标签；实现时新建）

## 1. 元素与语义

| 元素 | role / accessible name | 键盘 | 默认焦点 | 备注 |
| --- | --- | --- | --- | --- |
| 右栏 | `complementary` /「右侧检查器」 | Ctrl+B 切换 | 否 | 现有基线 INS-TOGGLE-01 |
| tablist | `tablist` + `tab`（files/changes/terminal） | ←/→ 切换、Home/End | 是（当前 tab） | `aria-selected` 唯一 |
| 文件树 | `tree` + `treeitem`（展开/折叠） | ↑/↓、→/← 展开/折叠 | 是 | 懒加载；隐藏 `.git`/`.zeokeel` |
| 分隔条 | `separator` /「调整右栏宽度」 | ←/→ 调宽 | 否 | 上下限 + `aria-valuenow` |
| 会话 `⋯` 菜单 | `button` /「会话操作」+ `menu`/`menuitem` | Enter/Space、↑/↓、Esc | 否 | rename/fork/copy id/pin/archive |
| 搜索框 / 模式 | `searchbox` /「搜索会话」+ 模式切换 | Enter/Esc | 是 | 默认标题模式；内容模式调 `session.search` |

## 2. 状态模型

状态：`empty` / `loading` / `error` / `offline` / `streaming` / `cancelled` + `busy` / `disabled` / `composing` / `menu-open`。

| 状态 | 进入条件 | 可见反馈 | aria（live/expanded/selected…） | 退出 |
| --- | --- | --- | --- | --- |
| loading | 目录/预览/终端/搜索请求中 | 行内/区块 loading | `aria-busy=true` | 请求完成 |
| error | 请求失败 | 行内错误 + 重试 | `role=alert` | 重试成功/关闭 |
| empty | 空目录/空结果/空终端 | 空态 + CTA | — | 有数据 |
| offline | 断网 | 面板错误态 + 可读提示 | 横幅可读 | 恢复连接 |
| streaming | 会话流式中 | archive/fork 菜单项禁用 | `aria-disabled=true` | turn 完成/取消 |
| menu-open | `⋯` 菜单打开 | 菜单可见、当前项高亮 | `aria-expanded=true` | 选择/Esc/点击外部 |

## 3. 交互条目（必须 1:1 映射测试）

| ID | 前置 | 人的动作（点击/按键/输入/拖放） | 期望（可见 + aria + 焦点 + 持久化） | 优先级 | 测试落点 | 证据 |
| --- | --- | --- | --- | --- | --- | --- |
| INS2-TREE-01 | 右栏 files tab 打开 | 展开/折叠目录；查看根列表 | 根项目录优先/名称排序；展开按需加载该目录直属子项（不遍历全树）；隐藏 `.git`/`.zeokeel`；符号链接不跟随 | P0 | `inspector-files.spec.ts` | `INS2-TREE-01.json` |
| INS2-TREE-02 | files tab 打开 | 目录加载失败后点行内重试；展开若干目录后点刷新 | 该行错误态+重试，其它目录不受影响；刷新保持已展开路径并重载可见项 | P0 | `inspector-files.spec.ts` | `INS2-TREE-02.json` |
| INS2-PREVIEW-01 | files tab 打开 | 点击文本文件；点击图片文件 | 文本显示内容（>256 KiB 截断并标「已截断」）；图片显示（≤8 MiB）；树保持可返回 | P0 | `inspector-files.spec.ts` | `INS2-PREVIEW-01.json` |
| INS2-PREVIEW-02 | 预览一个 PDF/Office/HTML/二进制/超限文件 | 点击文件 | 显示降级说明（文件名/大小/支持类型）；不白屏、不阻塞右栏 | P1 | `inspector-files.spec.ts` | `INS2-PREVIEW-02.json` |
| INS2-PREVIEW-03 | 文件树可见 | 打开工作区外/`..`/符号链接逃逸路径；打开不存在文件 | 拒绝并显示可读错误（不读文件）；不存在/权限错误可关闭预览，树保持可用 | P1 | `inspector-files.spec.ts` | `INS2-PREVIEW-03.json` |
| INS2-TERMINAL-01 | terminal tab 打开 | 新建会话 → 输入命令 → 查看输出 → 清空 → 关闭 | open→send→输出追加（stdout/stderr）→clear→close 从列表移除；可切换会话；不生成工具行/审批 | P0 | `inspector-terminal.spec.ts` | `INS2-TERMINAL-01.json` |
| INS2-TERMINAL-02 | terminal tab 打开 | 执行长命令/超时命令；在无 shell 平台打开 terminal | 执行中显示运行态；超时/截断（≤64 KiB）可读；平台无 shell 显示「终端不可用」空态，不白屏 | P1 | `inspector-terminal.spec.ts` | `INS2-TERMINAL-02.json` |
| INS2-PERSIST-01 | 已有会话 | 切 tab、拖拽调宽、折叠；刷新/重启/切回会话 | 当前 tab/宽度（px）/折叠状态按会话恢复；首次默认 files + 288px + 展开；脏数据回默认 | P0 | `inspector-layout.spec.ts` | `INS2-PERSIST-01.json` |
| INS2-PERSIST-02 | 右栏可见 | 焦点移到分隔条按 ←/→；拖到上下限；缩窄窗口 <768px | 键盘可调宽、分隔条有可读 aria；宽度 240–720px 且 ≤视口 50%；窄窗自动收起、变宽恢复 | P1 | `inspector-layout.spec.ts` | `INS2-PERSIST-02.json` |
| INS2-SES2-OPS-01 | 侧栏会话行可见 | 打开 `⋯` 菜单：rename（非空 ≤200）、fork、copy id、pin、archive | 各操作生效且列表/激活态一致；rename 空/超长拒绝；fork 选中新会话；copy id 有成功提示；pin 置顶、archive 从默认列表隐藏 | P0 | `sessions-ops.spec.ts` | `INS2-SES2-OPS-01.json` |
| INS2-SES2-OPS-02 | 会话操作场景 | 打开「显示已归档」筛选并 unarchive；在另一 workspace 找跨 workspace fork；制造操作失败 | C18：workspace_id 不变、跨 workspace 不可达；archive 可筛选/恢复（不硬删）；失败显示可读错误且列表不变；流式中 archive/fork 禁用 | P1 | `sessions-ops.spec.ts` | `INS2-SES2-OPS-02.json` |
| INS2-SEARCH-01 | 侧栏搜索框可见 | 切到「内容」模式，输入查询，快速改名再输入 | 调 `session.search`（当前 workspace）；防抖 ≥250ms、取消旧请求；结果按会话分组展示；点击结果打开对应会话 | P0 | `sessions-search.spec.ts` | `INS2-SEARCH-01.json` |
| INS2-SEARCH-02 | 内容模式 | 输入空/纯空白、超长（>200）、无命中、断网 | 空查询不请求；超长裁剪/拒绝；无命中/加载/错误有明确状态；归档命中含 `archived` 标记；limit 默认 20/最大 50 | P1 | `sessions-search.spec.ts` | `INS2-SEARCH-02.json` |
| INS2-ERR-01 | 各面板 | 依次触发空/加载/错误/离线/流式中/已取消 | 对应区域显示六态占位/提示，可重试或返回，不白屏、不丢失其它面板状态 | P1 | `inspector-a11y.spec.ts` | `INS2-ERR-01.json` |
| INS2-A11Y-01 | 右栏与会话侧栏可见 | axe 扫描 + 键盘遍历（tab 切换/树展开/菜单/终端输入/搜索） | 0 serious/critical；键盘可达全部操作；文案 i18n 无硬编码 | P1 | `inspector-a11y.spec.ts` | `INS2-A11Y-01.json` |
| INS2-VIS-01 | files tab 默认态 | 渲染右栏（light/1280） | 截图与基线一致（禁动画；阈值 0.002；动态目录项 mask） | P1 | `inspector-a11y.spec.ts` | `INS2-VIS-01.json` |

- ID 规则：`<SURFACE>-<AREA>-<NN>`（本 surface 前缀 `INS2-`/`SES2-`；AREA 如 `TREE`/`PREVIEW`/`TERMINAL`/`PERSIST`/`OPS`/`SEARCH`/`ERR`/`A11Y`/`VIS`）。
- 优先级：P0 = 主路径/安全（C18）/数据一致性；P1 = 高频细节/可达性/回归基线；P2 = 可选增强（不进门禁）。
- 一个 ID 至少一个测试；一个测试可覆盖多个 ID（标签可多个），但不得引用未定义 ID。

## 4. 视觉 / ARIA 基线

| ID | 状态 | 主题 | 宽度 | 基线类型 | 基线文件 | 掩码/阈值 |
| --- | --- | --- | --- | --- | --- | --- |
| INS2-VIS-01 | files tab 默认态 | light | 1280 | `toHaveScreenshot` | `e2e/interaction/__snapshots__/inspector-a11y.spec.ts/inspector-files-light-1280.png` | `maxDiffPixelRatio=0.002`；目录项 mask |
| INS2-A11Y-01 | 右栏 + 会话侧栏 | — | 1280 | `toMatchAriaSnapshot` + axe | `e2e/interaction/__snapshots__/inspector-a11y.spec.ts/inspector.aria.yml` | 目录/会话名用正则；axe 0 serious/critical |

## 5. 预算与一致性

- 交互延迟：树展开 <300ms（懒加载请求）；tab 切换 <100ms；搜索防抖 ≥250ms。
- 复用：只用 ui-kit 的 Button/Dialog/Menu/Tree/Tooltip/Toast/Empty/Error；禁止第二套 focus/toast/dialog/menu（C17）。
- token：颜色/圆角/间距/动效只用 CSS vars；禁止 raw hex / magic px。
- i18n：所有文案来自字典（zh/en）；禁止硬编码中文/英文。
- 六态：每个交互组件必须覆盖；缺一即不通过评审。
- 视觉更新：只允许人确认后 `pnpm e2e:ui:update`；CI 不得自动接受。
- 门禁：实现开始时把本文件 `状态` 改 `active`，并把 P0/P1 标签/证据接入 `python3 scripts/ui-interaction-gate.py`。
- **工具依赖（P1-5）**：`pnpm e2e:ui` 与 `scripts/ui-interaction-gate.py` 由 `2026-11-01-ui-interaction-verification` 落地；本文件写作时两者尚不存在，当前 **UNVERIFIED**，实现前必须确认可用后再声称门禁绿。
