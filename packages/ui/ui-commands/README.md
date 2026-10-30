# @zeokeel/ui-commands (`ui.slot.commands`)

**功能说明（骨架，尚未实现业务）**：composer `/` 命令菜单、`Mod+/` 快捷键参考，以及可选的全局 ⌘K 面板（ZeoKeel 增强，DSH 本组包无全局面板）。

## 职责
- ⌘K/Ctrl+K 打开/关闭；搜索过滤；↑↓ 移动；Enter 执行；Esc 关闭。
- 命令来源（后续）：导航（新会话/切 workspace）、动作（切主题/打开设置/重试）、设置页、快捷键参考。
- 命令项类型/分组/快捷键提示；禁用/需要确认的命令。
- 界面：居中 overlay、搜索框、列表、键盘提示；不抢焦点陷阱。

## Props（骨架约定）
```ts
interface CommandItem {
  id: string;
  title: string;
  group: string;
  hint?: string;
  disabled?: boolean;
}
interface CommandPaletteProps {
  open: boolean;
  commands: CommandItem[];
  onRun: (id: string) => void;
  onClose: () => void;
}
```

## 交互
- 打开时自动聚焦搜索；Esc 关闭并回焦触发按钮；空结果提示。
- 与 app 的快捷键注册解耦；只发出 `onRun`。
- 不实现真实命令执行；由 app 绑定。

## 非目标
- 不做设置页/插件市场；不重复快捷键实现（由 shell/app 注册）。
