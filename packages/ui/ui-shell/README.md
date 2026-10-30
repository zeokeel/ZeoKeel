# @zeokeel/ui-shell (`ui.shell`)

**功能说明（骨架，尚未实现业务）**：应用框架与全局 chrome。

## 职责
- 首用页（FirstRun）：无 workspace 时提供 `~` 主目录/添加路径。
- 三栏 AppFrame：`navigation` / `conversation + composer` / `inspector`；窄窗收起左栏/右栏。
- 顶栏：会话标题、状态、mock 标识、侧栏开关。
- 底部状态栏：连接状态、workspace 路径、package 数、版本、⌘K 提示。
- 主题 token/`data-theme` 由 app 提供；shell 只使用 token。
- Overlay 挂载点：命令面板、设置、工作区弹窗、toast。

## Props（当前实际，见 `web/src/index.tsx`）
```ts
interface AppShellProps {
  title: string;
  workspaceName: string;
  streaming: boolean;
  inspectorOpen: boolean;
  onToggleInspector: () => void;
  onOpenCommands: () => void;
  navigation: ReactNode;
  conversation: ReactNode;
  composer: ReactNode;
  inspector: ReactNode;
  overlay?: ReactNode;
}
```

## 交互 / 状态
- 顶栏信息按宽度渐进披露：<1024 隐藏 branch/context，保留模型/右栏。
- 右栏开关 `⌘B`；窄窗自动收起。
- 六态由子模块负责；shell 只负责布局和状态栏。
- a11y：landmark（aside/main/header）、skip link、焦点可见；所有 icon button 有 aria-label。

## 非目标（M1）
- 不做多窗口/分屏；不做 mac 红绿灯自定义；不实现真实 Jobs/模型切换（显示占位）。
- 不拥有主题状态（app 提供）。

## 依赖
`@zeokeel/ui-kit`；React peer。不得 import app 或其它业务 UI 包。
