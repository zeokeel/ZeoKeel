import type { ChangeView, FileNode, MessageView, WorkspaceView } from "./types";

export const WORKSPACES: WorkspaceView[] = [
  {
    id: "w-home",
    name: "~",
    detail: "主目录 · 默认",
    isDefault: true,
    sessions: [
      { id: "s1", title: "重构工作区导航", time: "刚刚", status: "running" },
      { id: "s2", title: "解释 Rust 所有权", time: "12:04", status: "idle" },
      { id: "s3", title: "把发布说明写短一点", time: "11:20", status: "unread" },
    ],
  },
  {
    id: "w-zeokeel",
    name: "zeokeel",
    detail: "~/code/zeokeel",
    isDefault: false,
    sessions: [
      { id: "s4", title: "排查 CI 缓存失效", time: "昨天", status: "idle" },
      { id: "s5", title: "设计审批卡交互", time: "昨天", status: "pending" },
    ],
  },
  {
    id: "w-api",
    name: "api-server",
    detail: "~/work/api-server",
    isDefault: false,
    sessions: [{ id: "s6", title: "整理 MCP 连接器清单", time: "周一", status: "idle" }],
  },
];

export const MESSAGES: MessageView[] = [
  { id: "m1", role: "user", text: "帮我把工作区导航重构一下，参考 DSH 的侧栏。", status: "done", tools: [] },
  {
    id: "m2",
    role: "assistant",
    text: "我先确认当前结构，然后给出 DSH 式三栏骨架：左侧平铺 Workspace + 缩进会话，中间对话，右侧文件/变更/终端。",
    status: "done",
    tools: [
      { id: "t1", name: "fs.read", detail: "packages/ui/ui-sessions/src/index.tsx", time: "0.4s", status: "done", args: '{"path":"packages/ui/ui-sessions/src/index.tsx"}', result: "Sidebar(...) { ... }" },
      { id: "t2", name: "grep", detail: "workspace switcher", time: "0.2s", status: "done", args: '{"pattern":"workspace"}', result: "12 matches" },
    ],
    plan: [
      { id: "p1", label: "读取现有 Sidebar / 布局", status: "done" },
      { id: "p2", label: "对比 DSH 模块职责", status: "done" },
      { id: "p3", label: "写模块化 UX 骨架", status: "running" },
      { id: "p4", label: "评审交互与密度", status: "pending" },
    ],
  },
  { id: "m3", role: "user", text: "右侧面板要能折叠，别一直占地方。", status: "done", tools: [] },
  {
    id: "m4",
    role: "assistant",
    text: "可以，默认展开文件，变更/终端按需切换；窄窗自动折叠。接下来确认一下审批卡的交互。",
    status: "streaming",
    tools: [{ id: "t3", name: "fs.write", detail: "apps/desktop/web/src/ux/", time: "运行中", status: "running", args: '{"path":"ux/modules"}', result: "" }],
    approval: { id: "ap1", tool: "fs.write", args: '{"path":"ux/modules/Conversation.tsx"}', reason: "写入工作区文件", decision: "pending" },
  },
];

export const FILES: FileNode[] = [
  { path: "src/", depth: 0, kind: "dir" },
  { path: "ux/", depth: 1, kind: "dir" },
  { path: "UxSkeleton.tsx", depth: 2, kind: "file", badge: "A" },
  { path: "modules/", depth: 2, kind: "dir" },
  { path: "Navigation.tsx", depth: 3, kind: "file", badge: "A" },
  { path: "Conversation.tsx", depth: 3, kind: "file", badge: "A" },
  { path: "Inspector.tsx", depth: 3, kind: "file", badge: "A" },
  { path: "app.css", depth: 1, kind: "file", badge: "M" },
];

export const CHANGES: ChangeView[] = [
  { path: "UxSkeleton.tsx", add: 120, del: 0 },
  { path: "Navigation.tsx", add: 180, del: 0 },
  { path: "Conversation.tsx", add: 160, del: 0 },
  { path: "app.css", add: 42, del: 11 },
];
