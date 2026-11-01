/** Right inspector (files tree / preview / changes / terminal) — Phase 2b.
 *
 * The app owns IPC + boundary validation; this package owns view state and the
 * session-scoped layout (tab/width/collapsed in localStorage, `docs/27` §2.6).
 */
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useUi } from "@zeokeel/ui-kit";

export type InspectorTab = "files" | "changes" | "terminal" | "plan";

export interface InspectorFileEntry { path: string; kind: "dir" | "file"; depth: number }

export interface InspectorPreview {
  path: string;
  state: "loading" | "ready" | "error";
  kind: "text" | "image" | "unsupported";
  text: string;
  dataUrl: string;
  size: number;
  truncated: boolean;
  reason: string;
  error: string;
}

export interface InspectorChanges {
  items: { path: string; add: number; del: number }[];
  warning: string | null;
}

export interface InspectorTerminal {
  sessions: string[];
  output: string;
  truncated: boolean;
  running: boolean;
  error: string | null;
  unavailable: boolean;
}

export interface InspectorApi {
  loadFiles: (path: string) => Promise<InspectorFileEntry[]>;
  loadPreview: (path: string) => Promise<InspectorPreview>;
  loadChanges: () => Promise<InspectorChanges>;
  terminal: (request: {
    action: "open" | "send" | "close" | "list";
    session?: string;
    command?: string;
  }) => Promise<InspectorTerminal>;
}

export interface InspectorPanelProps {
  open: boolean;
  api: InspectorApi;
  onClose: () => void;
}

interface Layout { tab: InspectorTab; width: number; collapsed: boolean }

const DEFAULT_LAYOUT: Layout = { tab: "files", width: 288, collapsed: false };
const MIN_WIDTH = 240;
const MAX_WIDTH = 720;
const LAYOUT_KEY = "zk.inspector.v1.";

function clampWidth(width: number): number {
  const viewport = typeof window === "undefined" ? MAX_WIDTH : window.innerWidth;
  const max = Math.min(MAX_WIDTH, Math.floor(viewport * 0.5));
  return Math.max(MIN_WIDTH, Math.min(max, Math.round(width)));
}

function readLayout(conversationId: string | null): Layout {
  if (conversationId === null || typeof window === "undefined") return DEFAULT_LAYOUT;
  try {
    const raw = window.localStorage.getItem(`${LAYOUT_KEY}${conversationId}`);
    if (raw === null) return DEFAULT_LAYOUT;
    const parsed: unknown = JSON.parse(raw);
    if (typeof parsed !== "object" || parsed === null) return DEFAULT_LAYOUT;
    const record: Record<string, unknown> = { ...parsed };
    const tab = record["tab"];
    const validTab: InspectorTab =
      tab === "changes" || tab === "terminal" || tab === "files" || tab === "plan" ? tab : DEFAULT_LAYOUT.tab;
    const width = typeof record["width"] === "number" ? record["width"] : DEFAULT_LAYOUT.width;
    const collapsed = record["collapsed"] === true;
    return { tab: validTab, width: clampWidth(width), collapsed };
  } catch {
    return DEFAULT_LAYOUT;
  }
}

function writeLayout(conversationId: string | null, layout: Layout): void {
  if (conversationId === null || typeof window === "undefined") return;
  try {
    window.localStorage.setItem(`${LAYOUT_KEY}${conversationId}`, JSON.stringify(layout));
  } catch {
    // Quota/private mode: keep the in-memory state only (R4).
  }
}

/** Flat visible rows of the expanded tree. */
interface TreeRow { path: string; name: string; kind: "dir" | "file"; depth: number; expanded: boolean }

export function InspectorPanel(props: InspectorPanelProps) {
  const { t, plan, actions, streaming } = useUi();
  const ui = useUi();
  const conversationId = ui.activeConversationId;
  const [layout, setLayout] = useState<Layout>(() => readLayout(conversationId));
  const [rows, setRows] = useState<TreeRow[]>([]);
  const [expanded, setExpanded] = useState<Map<string, "loading" | "ready" | "error">>(() => new Map());
  const children = useRef<Record<string, InspectorFileEntry[]>>({});
  const [filesState, setFilesState] = useState<"idle" | "loading" | "error">("idle");
  const [preview, setPreview] = useState<InspectorPreview | null>(null);
  const [changes, setChanges] = useState<InspectorChanges>({ items: [], warning: null });
  const [changesState, setChangesState] = useState<"idle" | "loading" | "error">("idle");
  const [changesError, setChangesError] = useState<string | null>(null);
  const [terminal, setTerminal] = useState<InspectorTerminal>({
    sessions: [],
    output: "",
    truncated: false,
    running: false,
    error: null,
    unavailable: false,
  });
  const [terminalInput, setTerminalInput] = useState("");
  const [activeSession, setActiveSession] = useState<string | null>(null);
  const [activeIndex, setActiveIndex] = useState(0);
  const [narrow, setNarrow] = useState(false);

  // Session-scoped layout restore (conversation switch) + narrow-window auto collapse.
  useEffect(() => {
    setLayout(readLayout(conversationId));
    setPreview(null);
    setRows([]);
    setExpanded(new Map());
    children.current = {};
    setFilesState("idle");
    setTerminal({ sessions: [], output: "", truncated: false, running: false, error: null, unavailable: false });
    setActiveSession(null);
  }, [conversationId]);
  useEffect(() => {
    const onResize = (): void => { setNarrow(window.innerWidth < 768); };
    onResize();
    window.addEventListener("resize", onResize);
    return () => { window.removeEventListener("resize", onResize); };
  }, []);
  useEffect(() => { writeLayout(conversationId, layout); }, [conversationId, layout]);

  const applyScroll = useCallback((path: string, entries: InspectorFileEntry[]): void => {
    children.current = { ...children.current, [path]: entries };
    const parent = path;
    setRows((current) => {
      const depthOfParent = parent === "" ? -1 : (current.find((row) => row.path === parent)?.depth ?? -1);
      const childRows: TreeRow[] = entries
        .slice()
        .sort((left, right) => {
          if (left.kind !== right.kind) return left.kind === "dir" ? -1 : 1;
          return left.path.localeCompare(right.path);
        })
        .map((entry) => ({
          path: entry.path,
          name: entry.path.split("/").at(-1) ?? entry.path,
          kind: entry.kind,
          depth: depthOfParent + 1,
          expanded: false,
        }));
      const without = current.filter((row) => !row.path.startsWith(parent === "" ? "" : `${parent}/`) || row.path === parent);
      if (parent === "") return childRows;
      const parentIndex = without.findIndex((row) => row.path === parent);
      const next = without.slice();
      next.splice(parentIndex + 1, 0, ...childRows);
      return next;
    });
  }, []);

  const loadRoot = useCallback(async (): Promise<void> => {
    setFilesState("loading");
    try {
      const entries = await props.api.loadFiles("");
      applyScroll("", entries);
      setFilesState("idle");
    } catch {
      setFilesState("error");
    }
  }, [applyScroll, props.api]);

  /** Reload the root and every expanded directory (keeps the expanded paths, R1). */
  const refreshTree = useCallback(async (): Promise<void> => {
    setFilesState("loading");
    try {
      const entries = await props.api.loadFiles("");
      applyScroll("", entries);
      const paths = Array.from(expanded.keys());
      for (const path of paths) {
        try {
          const childrenEntries = await props.api.loadFiles(path);
          applyScroll(path, childrenEntries);
        } catch {
          setExpanded((current) => new Map(current).set(path, "error"));
        }
      }
      setFilesState("idle");
    } catch {
      setFilesState("error");
    }
  }, [applyScroll, expanded, props.api]);

  useEffect(() => {
    if (!props.open || layout.tab !== "files" || ui.activeWorkspaceId === null) return;
    void loadRoot();
  }, [props.open, layout.tab, ui.activeWorkspaceId, loadRoot]);

  useEffect(() => {
    if (!props.open || layout.tab !== "changes" || ui.activeWorkspaceId === null) return;
    let active = true;
    setChangesState("loading");
    setChangesError(null);
    void props.api.loadChanges()
      .then((result) => { if (active) { setChanges(result); setChangesState("idle"); } })
      .catch((error: unknown) => {
        if (!active) return;
        setChangesError(error instanceof Error ? error.message : null);
        setChangesState("error");
      });
    return () => { active = false; };
  }, [props.open, layout.tab, ui.activeWorkspaceId, props.api]);

  const toggleDir = useCallback((row: TreeRow): void => {
    if (row.kind !== "dir") return;
    if (expanded.get(row.path) === "ready") {
      setExpanded((current) => {
        const next = new Map(current);
        next.delete(row.path);
        return next;
      });
      setRows((current) => current.filter((item) => item.path === row.path || !item.path.startsWith(`${row.path}/`)));
      return;
    }
    setExpanded((current) => new Map(current).set(row.path, "loading"));
    setRows((current) => current.map((item) => item.path === row.path ? { ...item, expanded: true } : item));
    void props.api.loadFiles(row.path)
      .then((entries) => {
        applyScroll(row.path, entries);
        setExpanded((current) => new Map(current).set(row.path, "ready"));
        setRows((current) => current.map((item) => item.path === row.path ? { ...item, expanded: true } : item));
      })
      .catch(() => {
        setExpanded((current) => new Map(current).set(row.path, "error"));
        setRows((current) => current.map((item) => item.path === row.path ? { ...item, expanded: true } : item));
      });
  }, [applyScroll, expanded, props.api]);

  const openFile = useCallback(async (path: string): Promise<void> => {
    setPreview({ path, state: "loading", kind: "unsupported", text: "", dataUrl: "", size: 0, truncated: false, reason: "", error: "" });
    try {
      const result = await props.api.loadPreview(path);
      setPreview(result);
    } catch (error: unknown) {
      setPreview({ path, state: "error", kind: "unsupported", text: "", dataUrl: "", size: 0, truncated: false, reason: "", error: error instanceof Error ? error.message : t("inspectorPreviewError") });
    }
  }, [props.api, t]);

  const refreshTerminal = useCallback(async (): Promise<void> => {
    try {
      const result = await props.api.terminal({ action: "list" });
      setTerminal((current) => ({ ...current, sessions: result.sessions, unavailable: result.unavailable }));
      setActiveSession((current) => current ?? result.sessions.at(-1) ?? null);
    } catch {
      // A list failure keeps the existing terminal state (non-blocking).
    }
  }, [props.api]);

  useEffect(() => {
    if (!props.open || layout.tab !== "terminal" || ui.activeWorkspaceId === null) return;
    void refreshTerminal();
  }, [props.open, layout.tab, ui.activeWorkspaceId, refreshTerminal]);

  const sendTerminal = useCallback(async (): Promise<void> => {
    const command = terminalInput;
    if (command.trim() === "") return;
    setTerminalInput("");
    setTerminal((current) => ({ ...current, running: true, error: null }));
    try {
      const opened = await props.api.terminal({ action: "open" });
      const session = activeSession ?? opened.sessions.at(-1) ?? "default";
      setActiveSession(session);
      const result = await props.api.terminal({ action: "send", session, command });
      setTerminal((current) => ({
        ...current,
        sessions: result.sessions,
        output: `${current.output}${result.output}`.slice(-65_536),
        truncated: current.truncated || result.truncated,
        running: false,
        error: null,
      }));
    } catch (error: unknown) {
      setTerminal((current) => ({ ...current, running: false, error: error instanceof Error ? error.message : t("inspectorTerminalError") }));
    }
  }, [activeSession, props.api, t, terminalInput]);

  const closeTerminal = useCallback(async (session: string): Promise<void> => {
    try {
      const result = await props.api.terminal({ action: "close", session });
      setTerminal((current) => ({ ...current, sessions: result.sessions, error: null }));
      setActiveSession(result.sessions.at(-1) ?? null);
    } catch (error: unknown) {
      setTerminal((current) => ({ ...current, error: error instanceof Error ? error.message : t("inspectorTerminalError") }));
    }
  }, [props.api, t]);

  const setTab = (tab: InspectorTab): void => { setLayout((current) => ({ ...current, tab })); };

  const onResizeStart = (event: React.PointerEvent<HTMLElement>): void => {
    event.preventDefault();
    const startX = event.clientX;
    const startWidth = layout.width;
    const onMove = (move: PointerEvent): void => {
      // Dragging the handle left grows the right panel.
      setLayout((current) => ({ ...current, width: clampWidth(startWidth + (startX - move.clientX)) }));
    };
    const onUp = (): void => {
      window.removeEventListener("pointermove", onMove);
      window.removeEventListener("pointerup", onUp);
    };
    window.addEventListener("pointermove", onMove);
    window.addEventListener("pointerup", onUp);
  };

  const onRowKeyDown = (event: React.KeyboardEvent<HTMLDivElement>, row: TreeRow): void => {
    const index = rowsById.get(row.path) ?? 0;
    if (event.key === "ArrowDown") { event.preventDefault(); setActiveIndex(Math.min(rows.length - 1, index + 1)); }
    else if (event.key === "ArrowUp") { event.preventDefault(); setActiveIndex(Math.max(0, index - 1)); }
    else if (event.key === "ArrowRight" && row.kind === "dir" && expanded.get(row.path) !== "ready") { event.preventDefault(); setActiveIndex(index); toggleDir(row); }
    else if (event.key === "ArrowLeft" && row.kind === "dir" && expanded.get(row.path) === "ready") { event.preventDefault(); setActiveIndex(index); toggleDir(row); }
    else if (event.key === "Enter" || event.key === " ") { event.preventDefault(); setActiveIndex(index); if (row.kind === "dir") toggleDir(row); else void openFile(row.path); }
  };

  const tabLabel = (tab: InspectorTab): string =>
    tab === "files"
      ? t("inspectorTabFiles")
      : tab === "changes"
        ? t("inspectorTabChanges")
        : tab === "plan"
          ? t("inspectorTabPlan")
          : t("inspectorTabTerminal");

  const rowsById = useMemo(() => new Map(rows.map((row, index) => [row.path, index])), [rows]);
  if (!props.open || narrow) return null;
  if (layout.collapsed) {
    return (
      <button
        type="button"
        aria-label={t("inspectorExpand")}
        className="flex w-6 shrink-0 items-start justify-center border-s border-border pt-2 text-muted-foreground"
        onClick={() => { setLayout((current) => ({ ...current, collapsed: false })); }}
      >
        ‹
      </button>
    );
  }

  return (
    <aside aria-label={t("inspectorTitle")} className="relative flex shrink-0 flex-col border-s border-border" data-slot="inspector" style={{ width: `${String(layout.width)}px` }}>
      <div
        role="separator"
        aria-label={t("inspectorResize")}
        aria-orientation="vertical"
        aria-valuemin={MIN_WIDTH}
        aria-valuemax={Math.min(MAX_WIDTH, Math.floor((typeof window === "undefined" ? MAX_WIDTH * 2 : window.innerWidth) * 0.5))}
        aria-valuenow={layout.width}
        className="absolute inset-y-0 -start-1 w-2 bg-transparent"
      />
      <button
        type="button"
        aria-label={t("inspectorResize")}
        onPointerDown={onResizeStart}
        onKeyDown={(event) => {
          if (event.key === "ArrowLeft") { event.preventDefault(); setLayout((current) => ({ ...current, width: clampWidth(current.width + 16) })); }
          if (event.key === "ArrowRight") { event.preventDefault(); setLayout((current) => ({ ...current, width: clampWidth(current.width - 16) })); }
        }}
        className="absolute inset-y-0 -start-1 w-2 cursor-col-resize"
      />
      <div className="flex h-9 items-center gap-1 border-b border-border px-2 text-[11px]">
        <div role="tablist" aria-label={t("inspectorTitle")} className="flex items-center gap-1">
          {(["files", "changes", "terminal", "plan"] as const).map((tab) => (
            <button
              key={tab}
              type="button"
              role="tab"
              aria-selected={layout.tab === tab}
              tabIndex={layout.tab === tab ? 0 : -1}
              onClick={() => { setTab(tab); }}
              onKeyDown={(event) => {
                const tabs: InspectorTab[] = ["files", "changes", "terminal", "plan"];
                const index = tabs.indexOf(tab);
                if (event.key === "ArrowRight") { event.preventDefault(); setTab(tabs[(index + 1) % tabs.length] ?? "files"); }
                if (event.key === "ArrowLeft") { event.preventDefault(); setTab(tabs[(index + tabs.length - 1) % tabs.length] ?? "files"); }
                if (event.key === "Home") { event.preventDefault(); setTab("files"); }
                if (event.key === "End") { event.preventDefault(); setTab("plan"); }
              }}
              className={`inline-flex items-center gap-1 rounded-sm px-2 py-1 ${layout.tab === tab ? "bg-muted" : ""}`}
            >
              {tabLabel(tab)}
            </button>
          ))}
        </div>
        <button type="button" className="ms-auto px-1 text-muted-foreground" aria-label={t("inspectorCollapse")} onClick={() => { setLayout((current) => ({ ...current, collapsed: true })); }}>›</button>
        <button type="button" onClick={props.onClose} className="px-1 text-muted-foreground" aria-label={t("inspectorClose")}>×</button>
      </div>
      <div role="tabpanel" className="min-h-0 flex-1 overflow-y-auto p-2 text-[11px] text-muted-foreground">
        {layout.tab === "files" ? (
          preview !== null ? (
            <div className="flex h-full flex-col gap-2">
              <div className="flex items-center gap-2">
                <button type="button" className="rounded-control border border-border px-2 py-0.5 text-foreground" onClick={() => { setPreview(null); }}>{t("inspectorPreviewBack")}</button>
                <span className="truncate text-foreground">{preview.path}</span>
              </div>
              {preview.state === "loading" ? <p aria-busy="true">{t("inspectorPreviewLoading")}</p> : null}
              {preview.state === "error" ? (
                <div role="alert" className="text-destructive">{preview.error}</div>
              ) : null}
              {preview.state === "ready" && preview.kind === "text" ? (
                <>
                  {preview.truncated ? <p role="status">{t("inspectorPreviewTruncated")}</p> : null}
                  <pre className="whitespace-pre-wrap break-words font-mono text-foreground">{preview.text}</pre>
                </>
              ) : null}
              {preview.state === "ready" && preview.kind === "image" ? (
                <img src={preview.dataUrl} alt={t("inspectorPreviewImageAlt", { path: preview.path })} className="max-h-full max-w-full object-contain" />
              ) : null}
              {preview.state === "ready" && preview.kind === "unsupported" ? (
                <div className="flex flex-col gap-1">
                  <p>{t("inspectorPreviewUnsupported")}</p>
                  <p>{preview.reason === "" ? t("inspectorPreviewUnsupportedHint") : preview.reason}</p>
                  <p>{t("inspectorPreviewSize", { size: String(preview.size) })}</p>
                </div>
              ) : null}
            </div>
          ) : filesState === "loading" ? <p aria-busy="true">{t("inspectorFilesLoading")}</p> : filesState === "error" ? (
            <div role="alert" className="flex flex-col items-start gap-2">
              <p className="text-destructive">{t("inspectorFilesError")}</p>
              <button type="button" className="rounded-control border border-border px-2 py-1 text-foreground" onClick={() => { void loadRoot(); }}>{t("inspectorFilesRetry")}</button>
            </div>
          ) : rows.length === 0 ? (
            <div className="flex flex-col items-start gap-2">
              <p>{t("inspectorFilesEmpty")}</p>
              <button type="button" aria-label={t("inspectorFilesRefresh")} className="rounded-control border border-border px-2 py-0.5 text-foreground" onClick={() => { void refreshTree(); }}>{t("inspectorFilesRefresh")}</button>
            </div>
          ) : (
            <div className="flex flex-col gap-1">
            <button
              type="button"
              aria-label={t("inspectorFilesRefresh")}
              className="mb-1 self-start rounded-control border border-border px-2 py-0.5 text-foreground"
              onClick={() => { void refreshTree(); }}
            >
              {t("inspectorFilesRefresh")}
            </button>
            <div role="tree" aria-label={t("inspectorFilesTree")} className="flex flex-col gap-0.5">
              {rows.map((row, index) => {
                const state = expanded.get(row.path);
                const selected = index === Math.min(activeIndex, rows.length - 1);
                return (
                  <div
                    key={row.path}
                    role="treeitem"
                    aria-level={row.depth + 1}
                    aria-expanded={row.kind === "dir" ? state === "ready" : undefined}
                    aria-selected={selected}
                    tabIndex={selected ? 0 : -1}
                    className={`flex items-center gap-1 rounded-sm px-1 py-0.5 text-start ${selected ? "bg-muted text-foreground" : ""}`}
                    style={{ paddingInlineStart: `${String(row.depth * 10 + 2)}px` }}
                    onClick={() => {
                      const known = rowsById.get(row.path);
                      if (known !== undefined) setActiveIndex(known);
                      if (row.kind === "dir") toggleDir(row); else void openFile(row.path);
                    }}
                    onKeyDown={(event) => { onRowKeyDown(event, row); }}
                  >
                    <span aria-hidden="true">{row.kind === "dir" ? (state === "ready" ? "▾" : "▸") : "·"}</span>
                    <span className="truncate">{row.name}</span>
                    {row.kind === "dir" && state === "loading" ? <span aria-hidden="true">{t("inspectorFilesLoadingShort")}</span> : null}
                    {row.kind === "dir" && state === "error" ? (
                      <button
                        type="button"
                        className="rounded-control border border-border px-1 text-destructive"
                        onClick={(event) => { event.stopPropagation(); toggleDir(row); }}
                      >
                        {t("inspectorFilesRetry")}
                      </button>
                    ) : null}
                  </div>
                );
              })}
            </div>
            </div>
          )
        ) : null}
        {layout.tab === "changes" && changesState === "loading" ? <p aria-busy="true">{t("inspectorChangesLoading")}</p> : null}
        {layout.tab === "changes" && changesState === "error" ? (
          <div role="alert" className="flex flex-col items-start gap-2">
            <p className="text-destructive">{t("inspectorChangesError")}</p>
            {changesError !== null ? <p>{changesError}</p> : null}
            <button type="button" className="rounded-control border border-border px-2 py-1 text-foreground" onClick={() => { setChangesState("loading"); setChangesError(null); void props.api.loadChanges().then((result) => { setChanges(result); setChangesState("idle"); }).catch((error: unknown) => { setChangesError(error instanceof Error ? error.message : null); setChangesState("error"); }); }}>{t("inspectorChangesRetry")}</button>
          </div>
        ) : null}
        {layout.tab === "changes" && changesState !== "loading" && changesState !== "error" ? (
          <>
            {changes.warning !== null ? <p role="status" className="mb-1">{t("inspectorChangesWarning", { message: changes.warning })}</p> : null}
            {changes.items.length === 0 ? <p>{t("inspectorChangesEmpty")}</p> : changes.items.map((change) => (
              <p key={change.path} aria-label={t("inspectorChangesItem", { path: change.path, add: change.add, del: change.del })}>
                <span>{t("inspectorChangesFile")} </span>
                {change.path} +{String(change.add)} −{String(change.del)}
              </p>
            ))}
          </>
        ) : null}
        {layout.tab === "plan" ? (
          <div className="flex h-full min-h-0 flex-col gap-2 overflow-y-auto">
            <p className="font-medium text-foreground">{t("planReviewTitle")}</p>
            {plan === null || (plan.mode === "off" && plan.status === "draft") ? (
              <p>{t("inspectorPlanEmpty")}</p>
            ) : (
              <>
                <p>
                  {plan.mode === "active" ? t("planModeActive") : t("planModeOff")} ·{" "}
                  {plan.status === "pending_review"
                    ? t("planStatusPending")
                    : plan.status === "approved"
                      ? t("planStatusApproved")
                      : plan.status === "rejected"
                        ? t("planStatusRejected")
                        : t("planStatusDraft")}
                </p>
                {plan.plan === "" ? null : (
                  <div role="region" aria-label={t("planMarkdownLabel")} className="max-h-64 overflow-y-auto whitespace-pre-wrap break-words text-foreground">
                    {plan.plan}
                  </div>
                )}
                {plan.feedback === null ? null : <p>{t("planFeedback", { feedback: plan.feedback })}</p>}
                {plan.mode === "active" ? (
                  <button type="button" disabled={streaming} className="self-start rounded-control border border-border px-2 py-0.5 text-foreground" onClick={() => { void actions.updatePlan("off"); }}>
                    {t("planExit")}
                  </button>
                ) : null}
                {plan.status === "pending_review" ? (
                  <div className="flex flex-wrap gap-2">
                    <button type="button" disabled={streaming} className="rounded-control border border-border px-2 py-0.5 text-foreground" onClick={() => { void actions.updatePlan("approve"); }}>
                      {t("planApprove")}
                    </button>
                    <button type="button" disabled={streaming} className="rounded-control border border-border px-2 py-0.5 text-foreground" onClick={() => { void actions.updatePlan("reject"); }}>
                      {t("planReject")}
                    </button>
                    <button type="button" disabled={streaming} className="rounded-control border border-border px-2 py-0.5 text-foreground" onClick={() => { void actions.updatePlan("continue"); }}>
                      {t("planContinue")}
                    </button>
                  </div>
                ) : null}
                {plan.status === "rejected" ? (
                  <button type="button" disabled={streaming} className="self-start rounded-control border border-border px-2 py-0.5 text-foreground" onClick={() => { void actions.updatePlan("continue"); }}>
                    {t("planContinue")}
                  </button>
                ) : null}
              </>
            )}
          </div>
        ) : null}
        {layout.tab === "terminal" ? (
          <div className="flex h-full min-h-0 flex-col gap-2">
            <div className="flex items-center gap-1">
              <span>{t("inspectorTerminalSessions")}</span>
              {terminal.sessions.map((session) => (
                <button
                  key={session}
                  type="button"
                  className={`rounded-control border border-border px-1 ${activeSession === session ? "bg-muted text-foreground" : ""}`}
                  onClick={() => { setActiveSession(session); }}
                >
                  {session}
                </button>
              ))}
            </div>
            {terminal.unavailable ? <div role="alert">{t("inspectorTerminalUnavailable")}</div> : null}
            {terminal.error !== null ? <div role="alert" className="text-destructive">{terminal.error}</div> : null}
            {terminal.running ? <p aria-busy="true">{t("inspectorTerminalRunning")}</p> : null}
            <pre data-testid="inspector-terminal-output" className="min-h-[80px] flex-1 overflow-auto whitespace-pre-wrap break-words rounded-control bg-muted p-1 font-mono text-foreground">{terminal.output}</pre>
            {terminal.truncated ? <p role="status">{t("inspectorTerminalTruncated")}</p> : null}
            <div className="flex items-center gap-1">
              <input
                aria-label={t("inspectorTerminalInput")}
                className="min-w-0 flex-1 rounded-control border border-border bg-transparent px-1 py-0.5 font-mono text-foreground"
                value={terminalInput}
                onChange={(event) => { setTerminalInput(event.target.value); }}
                onKeyDown={(event) => { if (event.key === "Enter") { event.preventDefault(); void sendTerminal(); } }}
              />
              <button type="button" className="rounded-control border border-border px-1 text-foreground" onClick={() => { void sendTerminal(); }}>{t("inspectorTerminalSend")}</button>
              <button type="button" className="rounded-control border border-border px-1 text-foreground" onClick={() => { setTerminal((current) => ({ ...current, output: "", truncated: false })); }}>{t("inspectorTerminalClear")}</button>
              {activeSession !== null ? <button type="button" className="rounded-control border border-border px-1 text-foreground" onClick={() => { void closeTerminal(activeSession); }}>{t("inspectorTerminalClose")}</button> : null}
            </div>
          </div>
        ) : null}
      </div>
    </aside>
  );
}

export const components = { "inspector.panel": InspectorPanel } as const;
