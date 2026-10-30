# @zeokeel/ui-inspector (`ui.slot.inspector`)

**功能说明（骨架，尚未实现业务）**：右侧检查器（文件/变更/终端）。DSH 的右栏是 per-session docking surface（normal/fullscreen、tab 不销毁）；ZeoKeel M1 先固定宽度单栏，“变更”是自有 tab（DSH 无）。

## 职责
- Tabs：文件树、变更（diff 统计）、终端（只读 mock/后续 PTY）。
- 可折叠（`⌘B`）；窄窗自动收起；面板宽度后续可拖拽。
- 文件树：懒加载目录、选中文件、M/A/D badge；预览后续接文档/图片。
- 变更：本次 turn 的文件列表、+/- 行数、点击打开 diff。
- 终端：只读占位，后续接 shell 工具/PTY。

## Props（骨架约定）
```ts
type InspectorTab = "files" | "changes" | "terminal";
interface InspectorPanelProps {
  open: boolean;
  tab: InspectorTab;
  files: FileNode[];
  changes: ChangeView[];
  onTab: (tab: InspectorTab) => void;
  onClose: () => void;
}
```

## 交互
- 点击 tab 切换；关闭按钮/⌘B 折叠；Esc 在窄窗关闭。
- 文件/变更项键盘可达；空/加载/错误态占位。
- 不调用后端；后续由 app 注入数据源。

## 非目标（M1/M5）
- 不实现文件写入/终端执行/浏览器；不做文档预览；不拥有 workspace 数据。
- 不 import `ui-chat`/app；只依赖 `ui-kit`。
