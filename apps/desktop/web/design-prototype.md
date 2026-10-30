# ZeoKeel · 设计原型 v1（待审阅）

> 目的：在继续写功能前，用**真实组件库**（assistant-ui primitives + Tailwind 4 + ZeoKeel UI 包）
> 和产品文档（`docs/19` UX、`docs/26` 工作区、`docs/27` 设计系统）搭出可截图的静态原型，
> 供产品/设计审阅。原型**不连接后端**，所有数据为 mock；不代表功能已完成。

## 运行 / 截图

```bash
pnpm --dir apps/desktop/web build
pnpm --dir apps/desktop/web exec vite preview --host 127.0.0.1 --port 4173
# 浏览器：http://127.0.0.1:4173/prototype.html?screen=chat&theme=dark
# screen: workspace | empty | chat | settings | loading | error | offline | cancelled | approval | narrow
# theme: light | dark；窄窗用 720×820 视口
```

截图在 `apps/desktop/web/screenshots/prototype/`，拼图 `contact-sheet.png`。

## 设计方向（来自 docs/27）

- assistant-ui 官方 design system：中性色单一 hue（`--tint: 250` 冷灰蓝）、blue-500 仅表示 live
  （流式/运行/选中/连接），错误 destructive；无第二强调色、无 UI 渐变。
- 字体：Public Sans + JetBrains Mono，本地打包；全 sans，无衬线。
- 圆角：control 8 / surface 10 / xl 12 / thread 16 / capsule 9999；普通表面零阴影，浮层才 lift。
- 线：只保留 section hairline；行内用间距/hover 填充。
- 品牌：官方渠道 logo 构建期注入，light/dark 切换；不手绘 mark。

## 屏幕与文档映射

| 原型 screen | 对应文档 | 说明 |
| --- | --- | --- |
| `workspace` | `26` §6 首用 | 无工作区时先选/建工作区；不自动建会话 |
| `empty` | `19` 空态 / `27` | 会话空态 + 建议 prompts |
| `chat` | `19` 流式+工具卡 | 用户气泡、助手无气泡、工具日志卡、流式光标、停止 |
| `dark` | `27` 明暗 | Abyss 深蓝灰，blue 只表 live |
| `settings` | `19` §3.1 / `27` | provider/模型/key、主题、语言、自动批准只读 |
| `loading`/`error`/`offline` | `19` 六态 | 加载 skeleton、错误重试、离线条+禁发 |
| `cancelled` | `19` 取消 | 保留已生成内容 + 已取消标签 |
| `approval` | `19` 审批（M1 占位） | 中性卡 + 允许一次/拒绝；后续接 tool-approval |
| `narrow` | `19` 窄窗 | 720px 抽屉侧栏 + 遮罩 |

## 有意保留的原型简化 / 待你拍板

1. **Workspace switcher 位置**：原型放在 header 右侧；`docs/26` 原写“侧栏顶部”。可改回侧栏顶部（会占侧栏高度），或保持 header（更省空间）。
2. **首用文案**：当前“先选择一个工作区 / 使用此工作区 / 选择其他目录”；是否要更短、更技术化或更温和？
3. **侧栏密度**：288px、会话行 36px；是否要更紧凑（32px）或更宽松（40px）？
4. **设置弹窗**：两列 field panel；API key 是否默认折叠/“显示”按钮？M1 本地 JSON 的提示是否够清楚？
5. **审批卡**：当前中性卡（不抢眼）；是否要在危险工具时用 destructive 边框（但 docs/27 只允许错误用 destructive）？
6. **深色默认**：截图是手动切 dark；是否默认跟随系统？默认浅色/深色偏好？
7. **品牌 logo 尺寸**：侧栏 28px、首用 48px；官方 logo 是横向 wordmark，是否要只用 mark（从官方 logo 派生）以适配小尺寸？

## 下一步（审阅通过后）

按 approved spec 施工：workspace 契约/SQLite 迁移/agent cwd、brand 路径校验、C10 T22、六态/axe/Playwright 证据、
把 mock 原型替换为真实 serve 数据。**在你确认设计方向前，不继续加功能。**

---

# 设计原型 v2：高密度三栏（2026-10-30 反馈后）

> 反馈：“整体 DPI 有点大，容纳不下多少内容”。参考 assistant-ui 官方 Claude/Perplexity 示例、DSH Web 客户端
> （`dsh-client-ui-*`）的功能与信息架构，重做一版高密度原型。

## 学到的成熟 Agent 功能（DSH 包清单 + assistant-ui 示例）

- **导航**：Workspace / 会话树（搜索、分组、运行状态点）、会话归档/置顶/多级 tree。
- **会话头**：标题、模型选择、分支、后台 Jobs、上下文占用、打开目录、权限预设、右侧栏开关。
- **对话**：工具调用树（单行折叠、按 tool 定制卡）、计划/GoalBar、审批 takeover、子代理 catalog、
  `@file`/`@session` 引用、附件、消息反馈、deliverables/变更文件、trajectory。
- **输入**：`/` 命令、`@` 引用、附件、模型下拉、四态主按钮（发送/停止/听写/取消）、队列。
- **右侧栏**：文件树、终端、文档/图片预览、浏览器、diff/变更。
- **全局**：命令面板、快捷键设置、主题、设置分页、目录选择、登录/账号。
- assistant-ui 示例：Claude 的扁平 composer/悬停操作条；Perplexity 的 `rows=2` 多行 composer、
  搜索/模型下拉、sticky follow-up footer、四态主按钮。

## v2 密度 token（原型已用）

| 维度 | v1 | v2 |
| --- | --- | --- |
| 基础字号/行高 | 14 / 1.6 | **13 / 1.55** |
| 左栏 | 288px | **248px**（会话树 + 搜索 + workspace） |
| 右栏 | 无 | **288px**（文件/变更/终端，可折叠） |
| 顶栏 | 56px | **44px** |
| 状态栏 | 无 | **24px**（workspace/连接/包数量/版本） |
| 消息列 | 760px | 760px（13px 正文） |
| 工具卡 | 大卡 | **单行折叠**（name/args/time，hover 展开） |
| 计划/审批 | 大卡 | 紧凑卡/行内 takeover |
| composer | 3 行高 | **2 行**（命令/引用 + 模型/附件/发送） |
| 主按钮 | 大号 | 小号 ink 按钮/图标按钮 |

## 新原型 screen

`dense.html?screen=dense|dark|palette&theme=light|dark`

- `dense`：三栏主界面（会话树、工具行、计划卡、右侧文件/变更、状态栏）。
- `dark`：Abyss 深色。
- `palette`：⌘K 命令面板。

截图：`screenshots/prototype/dense-*.png`；拼图 `contact-sheet-v2.png`。

## 待确认

1. 右栏默认展开还是默认收起（当前默认展开，`⌘B` 收起）？
2. 会话树是否要 DSH 式多级分组（Workspace → 会话）还是单列表？
3. 工具调用默认单行折叠，是否需要“展开全部”？
4. 顶栏是否放模型/上下文/分支（DSH 有）；窄窗隐藏哪些？
5. 13px 是否合适，还是 12.5/13.5？
