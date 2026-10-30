/**
 * Complete UX skeleton: modules are composed here; all data is mock.
 * Not part of the production build (dev-only `ux.html`).
 */
import { useEffect, useMemo, useState } from "react";
import { MESSAGES, WORKSPACES } from "./data";
import type { MessageView } from "./types";
import { Composer } from "./modules/Composer";
import { Conversation } from "./modules/Conversation";
import { Inspector, type InspectorTab } from "./modules/Inspector";
import { Navigation } from "./modules/Navigation";
import { CommandPalette, SettingsSkeleton } from "./modules/Overlays";
import { Shell } from "./modules/Shell";

type Screen = "chat" | "empty" | "loading" | "error" | "cancelled" | "narrow";

function parseScreen(value: string | null): Screen {
  switch (value) {
    case "empty": case "loading": case "error": case "cancelled": case "narrow": return value;
    default: return "chat";
  }
}

export function UxSkeleton() {
  const params = new URLSearchParams(window.location.search);
  const screen = parseScreen(params.get("screen"));
  const dark = params.get("theme") === "dark";
  const [collapsed, setCollapsed] = useState<Record<string, boolean>>({});
  const [activeWorkspace, setActiveWorkspace] = useState("w-home");
  const [activeSession, setActiveSession] = useState("s1");
  const [inspectorOpen, setInspectorOpen] = useState(screen !== "narrow");
  const [inspectorTab, setInspectorTab] = useState<InspectorTab>("files");
  const [palette, setPalette] = useState(false);
  const [settings, setSettings] = useState(false);
  const [composerText, setComposerText] = useState("");
  const [expandedTools, setExpandedTools] = useState<Record<string, boolean>>({});
  const [decision, setDecision] = useState<"pending" | "approved" | "rejected">("pending");
  const [messages, setMessages] = useState<MessageView[]>(() => {
    if (screen === "empty") return [];
    if (screen === "cancelled") return MESSAGES.map((m) => (m.id === "m2" ? { ...m, status: "cancelled" as const } : m));
    return MESSAGES;
  });

  useEffect(() => {
    document.documentElement.dataset["theme"] = dark ? "dark" : "light";
    document.documentElement.lang = "zh-CN";
  }, [dark]);

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "k") {
        event.preventDefault();
        setPalette((v) => !v);
      }
      if ((event.metaKey || event.ctrlKey) && event.key === ",") {
        event.preventDefault();
        setSettings((v) => !v);
      }
      if (event.key === "Escape") {
        setPalette(false);
        setSettings(false);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  const workspace = WORKSPACES.find((w) => w.id === activeWorkspace) ?? WORKSPACES[0];
  const session = useMemo(() => WORKSPACES.flatMap((w) => w.sessions).find((s) => s.id === activeSession), [activeSession]);
  const messagesForScreen = useMemo(() => {
    if (screen === "cancelled") return messages;
    if (screen === "error") return messages.map((m) => (m.id === "m4" ? { ...m, status: "error" as const } : m));
    return messages;
  }, [messages, screen]);

  const streaming = screen === "chat" || screen === "narrow" || screen === "cancelled";

  const send = () => {
    if (composerText.trim() === "") return;
    const user: MessageView = { id: `u-${Date.now()}`, role: "user", text: composerText.trim(), status: "done", tools: [] };
    const assistant: MessageView = { id: `a-${Date.now()}`, role: "assistant", text: "（skeleton）这里是流式回复占位，不调用后端。", status: "streaming", tools: [] };
    setMessages([...messages, user, assistant]);
    setComposerText("");
  };

  return (
    <Shell
      title={session?.title ?? "新会话"}
      workspaceName={workspace?.name ?? "~"}
      streaming={streaming}
      inspectorOpen={inspectorOpen}
      onToggleInspector={() => setInspectorOpen((v) => !v)}
      onOpenCommands={() => setPalette(true)}
      navigation={
        <Navigation
          workspaces={WORKSPACES}
          activeSessionId={activeSession}
          activeWorkspaceId={activeWorkspace}
          collapsed={collapsed}
          narrow={screen === "narrow"}
          onToggleWorkspace={(id) => { setCollapsed((c) => ({ ...c, [id]: !c[id] })); setActiveWorkspace(id); }}
          onSelectSession={(workspaceId, sessionId) => { setActiveWorkspace(workspaceId); setActiveSession(sessionId); }}
          onNewSession={() => setMessages([])}
          onOpenCommands={() => setPalette(true)}
          onOpenSettings={() => setSettings(true)}
          onOpenAddWorkspace={() => setPalette(false)}
          onCloseNarrow={() => undefined}
        />
      }
      conversation={
        screen === "loading" ? (
          <div className="flex min-h-0 flex-1 flex-col items-center justify-center gap-3 px-6"><span className="h-5 w-5 animate-spin rounded-capsule border-2 border-border border-t-live" /><p className="text-[12px] text-muted-foreground">正在加载消息…</p><div className="mt-1 grid w-full max-w-[420px] gap-2"><span className="h-3 w-full animate-pulse rounded bg-muted" /><span className="h-3 w-2/3 animate-pulse rounded bg-muted" /><span className="h-3 w-full animate-pulse rounded bg-muted" /></div></div>
        ) : screen === "error" ? (
          <div className="flex min-h-0 flex-1 flex-col items-center justify-center gap-2 px-6 text-center"><h2 className="text-[16px] font-semibold">消息加载失败</h2><p className="text-[12px] text-destructive">工作区暂时不可用，请重试。</p><button type="button" className="mt-2 rounded-control border border-border px-3 py-1.5 text-[12px]">重新加载</button></div>
        ) : (
          <Conversation
            messages={messagesForScreen}
            expandedTools={expandedTools}
            onToggleTool={(id) => setExpandedTools((e) => ({ ...e, [id]: !e[id] }))}
            onDecision={(id, d) => { setDecision(d); setMessages((ms) => ms.map((m) => m.approval?.id === id ? { ...m, approval: { ...m.approval, decision: d } } : m)); }}
            onSuggestion={(text) => setComposerText(text)}
          />
        )
      }
      composer={<Composer value={composerText} onChange={setComposerText} onSend={send} onStop={() => setMessages((ms) => ms.map((m) => (m.status === "streaming" ? { ...m, status: "cancelled" } : m)))} streaming={streaming} disabled={screen === "error"} />}
      inspector={<Inspector open={inspectorOpen} tab={inspectorTab} onTab={setInspectorTab} onClose={() => setInspectorOpen(false)} />}
      overlay={<><CommandPalette open={palette} onClose={() => setPalette(false)} /><SettingsSkeleton open={settings} onClose={() => setSettings(false)} /></>}
    />
  );
}
