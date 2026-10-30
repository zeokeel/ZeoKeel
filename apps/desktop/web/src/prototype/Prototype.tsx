/**
 * Design prototype: static mock screens for review. No backend/transport calls.
 * Screens are selected with ?screen=<name>&theme=light|dark.
 */
import { useEffect, useMemo } from "react";
import {
  BrandLogo,
  Button,
  Chip,
  StatusDot,
  UiProvider,
  defaultSettings,
  type ChatMessage,
  type Conversation,
  type UiValue,
} from "@zeokeel/ui-kit";
import { ChatSlot, SessionsSlot, SettingsSlot } from "../generated/registry";
import { BRAND } from "../generated/brand";
import { translate } from "../i18n";

type Screen = "workspace" | "empty" | "chat" | "settings" | "error" | "loading" | "offline" | "narrow" | "cancelled" | "approval";

const NOW = 1_793_350_000_000;
const CONVERSATIONS: Conversation[] = [
  { id: "c1", title: "读取 demo-data/hello.txt", created_at_ms: NOW - 600_000, updated_at_ms: NOW - 60_000 },
  { id: "c2", title: "解释 Rust 所有权", created_at_ms: NOW - 3_600_000, updated_at_ms: NOW - 3_000_000 },
  { id: "c3", title: "起草项目进展说明", created_at_ms: NOW - 86_400_000, updated_at_ms: NOW - 86_000_000 },
];

function msg(partial: Partial<ChatMessage> & Pick<ChatMessage, "id" | "role" | "content">): ChatMessage {
  return {
    created_at_ms: NOW,
    status: "done",
    toolCalls: [],
    approvals: [],
    error: null,
    ...partial,
  };
}

const CHAT_MESSAGES: ChatMessage[] = [
  msg({ id: "m1", role: "user", content: "读取 demo-data/hello.txt，然后告诉我文件里有什么。" }),
  msg({
    id: "m2",
    role: "assistant",
    content: "已读取 `demo-data/hello.txt`。文件内容如下：\n\n> Hello ZeoKeel! 这是一个最小 demo 文件。\n\n需要我基于它继续总结或改写吗？",
    toolCalls: [
      { id: "t1", name: "fs.read", args: '{"path":"demo-data/hello.txt"}', result: "Hello ZeoKeel!\n这是一个最小 demo 文件。\n", status: "done" },
    ],
  }),
  msg({ id: "m3", role: "user", content: "再帮我写一句更适合发给同事的说明。" }),
  msg({
    id: "m4",
    role: "assistant",
    content: "文件已确认可用：它包含一句欢迎语和一句中文说明，适合作为演示数据。",
    status: "streaming",
    toolCalls: [
      { id: "t2", name: "fs.read", args: '{"path":"demo-data/hello.txt"}', result: null, status: "running" },
    ],
  }),
];

const SETTINGS_MESSAGES: ChatMessage[] = [
  msg({ id: "s1", role: "user", content: "你好，先帮我看一下这个工作区的结构。" }),
  msg({ id: "s2", role: "assistant", content: "可以。当前工作区是 `~/Documents/ZeoKeel/default-workspace`，我会先列出顶层文件，再询问你想深入哪一部分。" }),
];

const ERROR_MESSAGES: ChatMessage[] = [];
const CANCELLED_MESSAGES: ChatMessage[] = [
  msg({ id: "x1", role: "user", content: "把这份文件改写成三条要点。" }),
  msg({ id: "x2", role: "assistant", content: "第一条：这是 ZeoKeel 的最小演示文件；第二条：", status: "cancelled", toolCalls: [] }),
];

const APPROVAL_MESSAGE: ChatMessage = msg({
  id: "a1",
  role: "assistant",
  content: "这个操作会写入工作区文件，按当前策略需要你确认。",
  approvals: [{ id: "ap1", tool: "fs.write", args: '{"path":"notes/todo.md","bytes":128}', reason: "写入工作区文件", decision: "pending" }],
});

function t(locale: "zh-CN" | "en-US") {
  return (key: string, vars?: Record<string, string | number>) => translate(locale, key, vars);
}

function noop() {}

function mockUi(screen: Screen, theme: "light" | "dark"): UiValue {
  const locale = "zh-CN";
  const isWorkspace = screen === "workspace";
  const baseConversation = CONVERSATIONS[0]!;
  const streaming = screen === "chat" || screen === "narrow";
  const messages =
    screen === "chat" || screen === "narrow"
      ? CHAT_MESSAGES
      : screen === "settings"
        ? SETTINGS_MESSAGES
        : screen === "cancelled"
          ? CANCELLED_MESSAGES
          : screen === "approval"
            ? [msg({ id: "p1", role: "user", content: "把刚才的结论写进 notes/todo.md。" }), APPROVAL_MESSAGE]
            : screen === "error"
              ? ERROR_MESSAGES
              : [];
  const messagesState = screen === "loading" ? "loading" : screen === "error" ? "error" : "idle";
  return {
    t: t(locale),
    locale,
    theme,
    brand: { ...BRAND, poweredBy: BRAND.poweredBy },
    boot: "ready",
    bootError: null,
    meta: { edition: "personal", version: "0.1.0", packages: ["tool-fs", "llm-mock", "agent-loop", "store-session-sqlite", "store-settings-json", "ui-kit", "ui-chat", "ui-sessions", "ui-settings"] },
    conversations: isWorkspace ? [] : CONVERSATIONS,
    activeConversationId: isWorkspace || screen === "empty" ? null : baseConversation.id,
    messages,
    messagesState,
    messagesError: screen === "error" ? "无法加载该工作区的会话：数据库暂时不可用。" : null,
    streaming,
    offline: screen === "offline",
    settings: { ...defaultSettings(), theme: "auto" },
    settingsOpen: screen === "settings",
    sidebarOpen: screen === "narrow",
    mock: true,
    actions: {
      send: noop,
      cancel: noop,
      retry: noop,
      createConversation: noop,
      selectConversation: noop,
      openSettings: noop,
      closeSettings: noop,
      saveSettings: async () => {},
      toggleSidebar: noop,
      retryBoot: noop,
      retryMessages: noop,
      decideApproval: noop,
    },
  };
}

function ChevronDown() {
  return (
    <svg width="12" height="12" viewBox="0 0 12 12" aria-hidden="true">
      <path d="M2.5 4.5 6 8l3.5-3.5" fill="none" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}

function FolderIcon() {
  return (
    <svg width="14" height="14" viewBox="0 0 16 16" aria-hidden="true">
      <path d="M1.8 4.2c0-.6.5-1.1 1.1-1.1h3l1.1 1.2h6.1c.6 0 1.1.5 1.1 1.1v7.2c0 .6-.5 1.1-1.1 1.1H2.9c-.6 0-1.1-.5-1.1-1.1V4.2Z" fill="none" stroke="currentColor" strokeWidth="1.2" strokeLinejoin="round" />
    </svg>
  );
}

function WorkspaceChip() {
  return (
    <button type="button" className="inline-flex items-center gap-1.5 rounded-control border border-border bg-background/70 px-2 py-1 text-[12px] text-muted-foreground">
      <FolderIcon />
      <span className="max-w-[150px] truncate">默认工作区</span>
      <ChevronDown />
    </button>
  );
}

function FirstRun() {
  return (
    <div className="flex h-dvh items-center justify-center bg-background px-6 text-foreground">
      <div className="w-full max-w-[520px]">
        <BrandLogo className="mb-8 h-12 w-12 rounded-document" />
        <p className="mb-2 font-mono text-[11px] uppercase tracking-[0.14em] text-muted-foreground">Workspace</p>
        <h1 className="text-[32px] font-medium leading-[1.15] tracking-[-0.03em]">先选择一个工作区</h1>
        <p className="mt-3 max-w-[430px] text-[14px] leading-[1.6] text-muted-foreground">
          ZeoKeel 的每个对话都发生在一个工作区目录里。agent 的文件读写、终端命令和检查点都以它为根；
          它只是上下文边界，不是权限沙箱。
        </p>
        <div className="mt-7 rounded-xl border border-border bg-card p-4">
          <div className="flex items-center gap-3">
            <span className="flex h-9 w-9 items-center justify-center rounded-control bg-muted text-muted-foreground"><FolderIcon /></span>
            <div className="min-w-0 flex-1">
              <p className="truncate text-[13px] font-medium">Documents/ZeoKeel/default-workspace</p>
              <p className="mt-0.5 text-[12px] text-muted-foreground">默认工作区 · 可读写</p>
            </div>
            <button type="button" className="text-[12px] text-muted-foreground underline decoration-border underline-offset-4">更改</button>
          </div>
          <div className="mt-4 flex items-center gap-2">
            <Button variant="primary" className="flex-1">使用此工作区</Button>
            <Button>选择其他目录</Button>
          </div>
        </div>
        <p className="mt-4 text-[12px] leading-[1.6] text-muted-foreground">
          创建默认工作区不会自动创建对话；你仍然需要新建会话并发送第一条消息。
        </p>
      </div>
    </div>
  );
}

function Header(props: { ui: UiValue }) {
  const { ui } = props;
  const active = ui.conversations.find((c) => c.id === ui.activeConversationId);
  return (
    <header className="flex min-h-[56px] items-center gap-3 border-b border-border bg-background px-5 py-2.5">
      <div className="flex min-w-0 flex-1 flex-col">
        <h1 className="truncate text-[14px] font-semibold tracking-[-0.005em]">{active?.title ?? ui.t("noConversation")}</h1>
        <p className="mt-0.5 flex items-center gap-1.5 text-[12px] text-muted-foreground">
          <StatusDot live={ui.streaming} />
          <span>{ui.streaming ? ui.t("statusStreaming") : ui.t("statusReady")}</span>
          <Chip>{ui.t("mockBadge")}</Chip>
        </p>
      </div>
      <WorkspaceChip />
    </header>
  );
}

export function Prototype() {
  const params = new URLSearchParams(window.location.search);
  const screen = (params.get("screen") ?? "empty") as Screen;
  const theme = params.get("theme") === "dark" ? "dark" : "light";
  const ui = useMemo(() => mockUi(screen, theme), [screen, theme]);

  useEffect(() => {
    document.documentElement.dataset["theme"] = theme;
    document.documentElement.lang = "zh-CN";
  }, [theme]);

  if (screen === "workspace") {
    return (
      <UiProvider value={ui}>
        <FirstRun />
      </UiProvider>
    );
  }

  return (
    <UiProvider value={ui}>
      <div className="flex h-dvh flex-col overflow-hidden bg-background text-foreground">
        {ui.offline ? (
          <div className="flex items-center justify-center gap-2 border-b border-border bg-muted px-4 py-1.5 text-[12px] text-destructive" role="status">
            <span className="inline-block h-1.5 w-1.5 rounded-capsule bg-destructive" aria-hidden="true" />
            {ui.t("offlineBanner")}
          </div>
        ) : null}
        <div className="flex min-h-0 flex-1">
          <SessionsSlot />
          {ui.sidebarOpen ? <button type="button" className="zk-sidebar-backdrop" aria-label={ui.t("sidebarClose")} /> : null}
          <main className="flex min-w-0 flex-1 flex-col">
            <Header ui={ui} />
            {screen === "settings" ? (
              <div className="min-h-0 flex-1 opacity-60"><ChatSlot /></div>
            ) : screen === "cancelled" ? (
              <div className="min-h-0 flex-1"><ChatSlot /></div>
            ) : (
              <ChatSlot />
            )}
          </main>
        </div>
        <SettingsSlot />
      </div>
    </UiProvider>
  );
}
