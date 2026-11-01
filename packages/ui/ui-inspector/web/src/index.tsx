/** Right inspector (files tree / preview / changes / terminal / activity / plan).
 *
 * The app owns IPC + boundary validation; this package owns view state and the
 * session-scoped layout (tab/width/collapsed in localStorage, `docs/27` §2.6).
 * Tab order is fixed in-component (docs/16 §2.2): files 10 → changes 20 →
 * terminal 30 → activity 40 (this package) → plan 50 (`client-plan-mode`).
 */
import { useCallback, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { Button, StatusDot, Switch, useUi } from "@zeokeel/ui-kit";
import type { ActivityActionKind, ActivitySnapshot } from "@zeokeel/ui-kit";

export type InspectorTab = "files" | "changes" | "terminal" | "activity" | "plan";

/** Fixed right-panel tab order (`docs/16` §2.2; no tab registry/slot). */
const INSPECTOR_TABS: readonly InspectorTab[] = ["files", "changes", "terminal", "activity", "plan"];

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
  /**
   * Cross-surface request: open one workspace-relative file in the files preview
   * (`turnTail`/activity 「在右栏查看」). A changing `nonce` re-opens the path.
   */
  previewRequest?: { path: string; nonce: number } | null;
  /** Called after `previewRequest` was handled so the app can clear it. */
  onPreviewConsumed?: () => void;
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
      tab === "changes" || tab === "terminal" || tab === "files" || tab === "activity" || tab === "plan"
        ? tab
        : DEFAULT_LAYOUT.tab;
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

/** One titled activity section (title + count; per-section empty/error states). */
function ActivitySection(props: { title: string; count: number; children: ReactNode }) {
  return (
    <section aria-label={props.title} className="flex flex-col gap-1">
      <h3 className="text-[11px] font-medium uppercase text-muted-foreground">
        {props.title} · {String(props.count)}
      </h3>
      {props.children}
    </section>
  );
}

/**
 * Inline destructive-action confirmation (C17: no second modal component).
 *
 * `role=alertdialog` with a readable label; the default focus is the cancel
 * button, and Esc cancels without emitting the action (`interaction.md`
 * ACT2-JOB-STOP-01 / ACT2-SUBAGENT-INTERRUPT-01 / ACT2-SCHEDULE-DELETE-01).
 */
function InlineConfirm(props: {
  label: string;
  body: ReactNode;
  confirmLabel: string;
  onConfirm: () => void;
  onCancel: () => void;
}) {
  const { t } = useUi();
  const cancelRef = useRef<HTMLButtonElement | null>(null);
  const cancel = props.onCancel;
  // Keyboard contract: Esc cancels without emitting the action; the default
  // focus is the cancel button (`interaction.md` ACT2-JOB-STOP-01).
  useEffect(() => {
    cancelRef.current?.focus();
    const onKey = (event: KeyboardEvent): void => {
      if (event.key === "Escape") {
        event.preventDefault();
        cancel();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("keydown", onKey);
    };
  }, [cancel]);
  return (
    <div
      role="alertdialog"
      aria-label={props.label}
      className="mt-1 flex flex-col gap-1 rounded-control border border-destructive/40 bg-muted p-2"
    >
      <p className="text-[11px] text-foreground">{props.body}</p>
      <div className="flex items-center gap-1">
        <Button ref={cancelRef} size="sm" onClick={props.onCancel}>
          {t("activityCancel")}
        </Button>
        <Button size="sm" variant="destructive" onClick={props.onConfirm}>
          {props.confirmLabel}
        </Button>
      </div>
    </div>
  );
}

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
  /** Tab button refs so ←/→/Home/End move real focus with the selection. */
  const tabRefs = useRef<Partial<Record<InspectorTab, HTMLButtonElement | null>>>({});

  // ---- Activity tab (`2026-11-01-client-activity-panels`, tab order 40) ----
  // Data/actions come from `UiValue.actions` (app-owned IPC + Ajv validation);
  // polling lives here and only while this tab is open (`docs/16` §2.2, R1/R4).
  const [activity, setActivity] = useState<ActivitySnapshot | null>(null);
  const [activityState, setActivityState] = useState<"idle" | "loading" | "ready" | "error">("idle");
  const [activityError, setActivityError] = useState<string | null>(null);
  const [activityStale, setActivityStale] = useState(false);
  const [activityBusy, setActivityBusy] = useState(false);
  const [actionError, setActionError] = useState<string | null>(null);
  const [actionBusyId, setActionBusyId] = useState<string | null>(null);
  const [expandedJob, setExpandedJob] = useState<string | null>(null);
  const [stopConfirm, setStopConfirm] = useState<{ kind: "job" | "subagent"; id: string } | null>(null);
  const [deleteSchedule, setDeleteSchedule] = useState<string | null>(null);
  const [editSchedule, setEditSchedule] = useState<string | null>(null);
  const [scheduleInterval, setScheduleInterval] = useState("");
  const [schedulePrompt, setSchedulePrompt] = useState("");
  const [scheduleError, setScheduleError] = useState<string | null>(null);
  /** Monotonic request fence: stale polling results are dropped (R1). */
  const activityGeneration = useRef(0);
  const actionsRef = useRef(actions);
  useEffect(() => {
    actionsRef.current = actions;
  }, [actions]);

  /** Map one host `activity/*` code to a readable (i18n) message. */
  const activityErrorText = useCallback(
    (raw: string | null): string => {
      if (raw === null || raw === "") return t("activityErrorGeneric");
      if (raw.startsWith("activity/job-not-found")) return t("activityJobNotFound");
      if (raw.startsWith("activity/job-finished")) return t("activityJobFinished");
      if (raw.startsWith("activity/subagent-not-found")) return t("activitySubagentNotFound");
      if (raw.startsWith("activity/schedule-unreadable")) return t("activityScheduleUnreadable");
      if (raw.startsWith("activity/schedule-invalid")) return t("activityScheduleInvalid");
      if (raw.startsWith("activity/derived-unavailable")) return t("activityDerivedHint");
      return t("activityErrorGeneric");
    },
    [t],
  );

  const loadActivity = useCallback(
    async (mode: "initial" | "poll" | "manual"): Promise<void> => {
      const workspaceId = ui.activeWorkspaceId;
      const conversationId = ui.activeConversationId;
      if (workspaceId === null || conversationId === null) return;
      const generation = activityGeneration.current + 1;
      activityGeneration.current = generation;
      if (mode !== "poll") setActivityError(null);
      try {
        const next = await actionsRef.current.activitySnapshot(workspaceId, conversationId);
        // Drop results of a previous conversation/tab generation (R1).
        if (activityGeneration.current !== generation) return;
        if (ui.activeWorkspaceId !== workspaceId || ui.activeConversationId !== conversationId) return;
        setActivity(next);
        setActivityState("ready");
        setActivityStale(false);
        setActivityError(null);
      } catch (error: unknown) {
        if (activityGeneration.current !== generation) return;
        const message = error instanceof Error ? error.message : "";
        setActivityError(message);
        if (mode === "poll") {
          // Keep the last successful snapshot and show a stale marker (R8).
          setActivityStale(true);
        } else {
          setActivityState((current) => (current === "ready" ? current : "error"));
        }
      }
    },
    [ui.activeConversationId, ui.activeWorkspaceId],
  );

  const refreshActivity = useCallback((): void => {
    setActivityBusy(true);
    void actionsRef.current.refreshGoal().catch(() => undefined);
    void loadActivity("manual").finally(() => {
      setActivityBusy(false);
    });
  }, [loadActivity]);

  const runActivityAction = useCallback(
    async (kind: ActivityActionKind, id: string, updates?: Record<string, unknown>): Promise<void> => {
      const workspaceId = ui.activeWorkspaceId;
      const conversationId = ui.activeConversationId;
      if (workspaceId === null || conversationId === null) return;
      setActionBusyId(id);
      setActionError(null);
      try {
        await actionsRef.current.activityAction({
          workspace_id: workspaceId,
          conversation_id: conversationId,
          action: kind,
          id,
          ...(updates === undefined ? {} : { updates }),
        });
        setStopConfirm(null);
        setDeleteSchedule(null);
        setEditSchedule(null);
        setScheduleError(null);
      } catch (error: unknown) {
        setActionError(error instanceof Error ? error.message : null);
      } finally {
        setActionBusyId(null);
        // Refresh after every action, success or failure (host state is truth).
        await loadActivity("manual");
      }
    },
    [loadActivity, ui.activeConversationId, ui.activeWorkspaceId],
  );

  const saveSchedule = useCallback((): void => {
    const id = editSchedule;
    if (id === null) return;
    const interval = Number.parseInt(scheduleInterval, 10);
    if (!Number.isInteger(interval) || interval < 60) {
      setScheduleError("activityScheduleIntervalError");
      return;
    }
    if (new TextEncoder().encode(schedulePrompt).length > 32 * 1024) {
      setScheduleError("activitySchedulePromptError");
      return;
    }
    setScheduleError(null);
    void runActivityAction("schedule_update", id, { interval_seconds: interval, prompt: schedulePrompt });
  }, [editSchedule, runActivityAction, scheduleInterval, schedulePrompt]);

  const activityJobStatus = useCallback(
    (status: string): string => {
      if (status === "running") return t("activityJobRunning");
      if (status.startsWith("failed")) return t("activityJobFailed");
      return t("activityJobCompleted");
    },
    [t],
  );

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

  // Open one file in the files preview when another surface asks for it
  // (`turnTail`/activity 「在右栏查看」; side-panels design §2.3 alignment).
  const previewRequest = props.previewRequest;
  const onPreviewConsumed = props.onPreviewConsumed;
  useEffect(() => {
    if (previewRequest === undefined || previewRequest === null) return;
    setLayout((current) => ({ ...current, tab: "files", collapsed: false }));
    void openFile(previewRequest.path);
    onPreviewConsumed?.();
  }, [previewRequest, onPreviewConsumed, openFile]);

  // Load on tab open + 2s polling; close/tab-switch stops polling and drops
  // in-flight responses (generation fence). Goal is a separate read (`goal.get`).
  useEffect(() => {
    if (!props.open || layout.tab !== "activity") return;
    if (ui.activeWorkspaceId === null || ui.activeConversationId === null) return;
    setActivity(null);
    setActivityState("loading");
    setActivityError(null);
    setActivityStale(false);
    setActionError(null);
    setStopConfirm(null);
    setDeleteSchedule(null);
    setEditSchedule(null);
    void actionsRef.current.refreshGoal().catch(() => undefined);
    void loadActivity("initial");
    const interval = window.setInterval(() => {
      void loadActivity("poll");
    }, 2000);
    return () => {
      window.clearInterval(interval);
      activityGeneration.current += 1;
    };
  }, [props.open, layout.tab, ui.activeWorkspaceId, ui.activeConversationId, loadActivity]);

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

  const tabLabel = (tab: InspectorTab): string => {
    if (tab === "files") return t("inspectorTabFiles");
    if (tab === "changes") return t("inspectorTabChanges");
    if (tab === "terminal") return t("inspectorTabTerminal");
    if (tab === "activity") return t("inspectorTabActivity");
    return t("inspectorTabPlan");
  };

  const activityStatusLabel = (status: string): string => {
    if (status === "running") return t("activityStatusRunning");
    if (status === "done") return t("activityStatusDone");
    if (status === "failed") return t("activityStatusFailed");
    if (status === "cancelled") return t("activityStatusCancelled");
    return t("activityStatusUnknown");
  };

  const todoStatusLabel = (status: string): string => {
    if (status === "pending") return t("activityTodoPending");
    if (status === "in_progress") return t("activityTodoInProgress");
    if (status === "completed") return t("activityTodoCompleted");
    return status;
  };

  const teamStatusLabel = (status: string): string => {
    if (status === "done") return t("activityStatusDone");
    if (status === "failed") return t("activityStatusFailed");
    if (status === "partial") return t("activityTeamPartial");
    if (status === "timeout") return t("activityTeamTimeout");
    return status;
  };

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
          {INSPECTOR_TABS.map((tab) => (
            <button
              key={tab}
              ref={(element) => { tabRefs.current[tab] = element; }}
              type="button"
              role="tab"
              aria-selected={layout.tab === tab}
              tabIndex={layout.tab === tab ? 0 : -1}
              onClick={() => { setTab(tab); }}
              onKeyDown={(event) => {
                const tabs: readonly InspectorTab[] = INSPECTOR_TABS;
                const index = tabs.indexOf(tab);
                let next: InspectorTab | null = null;
                if (event.key === "ArrowRight") next = tabs[(index + 1) % tabs.length] ?? "files";
                if (event.key === "ArrowLeft") next = tabs[(index + tabs.length - 1) % tabs.length] ?? "files";
                if (event.key === "Home") next = "files";
                if (event.key === "End") next = "plan";
                if (next === null) return;
                event.preventDefault();
                setTab(next);
                tabRefs.current[next]?.focus();
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
        {layout.tab === "activity" ? (
          <div className="flex h-full min-h-0 flex-col gap-2 overflow-y-auto" data-testid="activity-panel">
            <div className="flex items-center gap-2">
              <p className="font-medium text-foreground">{t("activityTitle")}</p>
              <Button
                size="sm"
                variant="ghost"
                aria-label={t("activityRefresh")}
                aria-busy={activityBusy}
                disabled={activityBusy}
                onClick={refreshActivity}
              >
                {t("activityRefresh")}
              </Button>
              {activityStale ? (
                <span role="status" className="text-[11px] text-muted-foreground">
                  {t("activityStale")}
                </span>
              ) : null}
            </div>
            {ui.offline ? (
              <p role="status" className="text-[11px] text-muted-foreground">
                {t("activityOffline")}
              </p>
            ) : null}
            {ui.activeWorkspaceId === null || ui.activeConversationId === null ? (
              <p>{t("activityNoConversation")}</p>
            ) : activityState === "error" ? (
              <div role="alert" className="flex flex-col items-start gap-2">
                <p className="text-destructive">{activityErrorText(activityError)}</p>
                <Button size="sm" onClick={refreshActivity}>
                  {t("activityRetry")}
                </Button>
              </div>
            ) : activity === null ? (
              <p aria-busy="true">{t("activityLoading")}</p>
            ) : (
              <>
                {activity.derived_unavailable ? (
                  <p role="status" className="text-[11px] text-muted-foreground">
                    {t("activityDerivedHint")}
                  </p>
                ) : null}
                {activity.truncated ? (
                  <p role="status" className="text-[11px] text-muted-foreground">
                    {t("activityTruncated")}
                  </p>
                ) : null}
                {actionError !== null ? (
                  <p role="alert" className="text-destructive">
                    {activityErrorText(actionError)}
                  </p>
                ) : null}

                <ActivitySection title={t("activityJobs")} count={activity.jobs.length}>
                  {activity.jobs.length === 0 ? (
                    <p>{t("activityJobsEmpty")}</p>
                  ) : (
                    <ul className="flex flex-col gap-1">
                      {activity.jobs.map((job) => (
                        <li key={job.id} className="rounded-control border border-border px-2 py-1">
                          <div className="flex items-center gap-1">
                            <StatusDot live={job.status === "running"} />
                            <span className="min-w-0 flex-1 truncate font-mono text-foreground">
                              {job.command}
                            </span>
                            <span className="text-[11px] text-muted-foreground">
                              {activityJobStatus(job.status)}
                            </span>
                          </div>
                          <p className="truncate text-[11px]">{job.cwd}</p>
                          <div className="flex items-center gap-1">
                            <Button
                              size="sm"
                              variant="ghost"
                              aria-expanded={expandedJob === job.id}
                              onClick={() => {
                                setExpandedJob(expandedJob === job.id ? null : job.id);
                              }}
                            >
                              {expandedJob === job.id ? t("activityCollapse") : t("activityExpand")}
                            </Button>
                            {job.status === "running" ? (
                              <Button
                                size="sm"
                                variant="destructive"
                                aria-busy={actionBusyId === job.id}
                                disabled={actionBusyId !== null}
                                onClick={() => {
                                  setStopConfirm({ kind: "job", id: job.id });
                                }}
                              >
                                {t("activityStopJob")}
                              </Button>
                            ) : null}
                          </div>
                          {expandedJob === job.id ? (
                            <pre className="max-h-48 overflow-auto whitespace-pre-wrap break-words rounded-control bg-muted p-1 font-mono text-foreground">
                              {job.output_excerpt === "" ? t("activityOutputEmpty") : job.output_excerpt}
                            </pre>
                          ) : null}
                          {stopConfirm !== null && stopConfirm.kind === "job" && stopConfirm.id === job.id ? (
                            <InlineConfirm
                              label={t("activityJobStopConfirmLabel")}
                              body={t("activityJobStopConfirmBody", { id: job.id, command: job.command })}
                              confirmLabel={t("activityConfirm")}
                              onConfirm={() => {
                                void runActivityAction("job_kill", job.id);
                              }}
                              onCancel={() => {
                                setStopConfirm(null);
                              }}
                            />
                          ) : null}
                        </li>
                      ))}
                    </ul>
                  )}
                </ActivitySection>

                <ActivitySection title={t("activitySubagents")} count={activity.subagents.length}>
                  {activity.subagents.length === 0 ? (
                    <p>{t("activitySubagentsEmpty")}</p>
                  ) : (
                    <ul className="flex flex-col gap-1">
                      {activity.subagents.map((subagent) => (
                        <li key={subagent.id} className="rounded-control border border-border px-2 py-1">
                          <div className="flex items-center gap-1">
                            <StatusDot live={subagent.status === "running"} />
                            <span className="min-w-0 flex-1 truncate font-mono text-foreground">
                              {subagent.id}
                            </span>
                            <span className="text-[11px] text-muted-foreground">
                              {activityStatusLabel(subagent.status)}
                            </span>
                          </div>
                          <p className="truncate text-[11px]">{subagent.prompt}</p>
                          {subagent.status === "running" ? (
                            <Button
                              size="sm"
                              variant="destructive"
                              aria-busy={actionBusyId === subagent.id}
                              disabled={actionBusyId !== null}
                              onClick={() => {
                                setStopConfirm({ kind: "subagent", id: subagent.id });
                              }}
                            >
                              {t("activityInterrupt")}
                            </Button>
                          ) : null}
                          {stopConfirm !== null && stopConfirm.kind === "subagent" && stopConfirm.id === subagent.id ? (
                            <InlineConfirm
                              label={t("activitySubagentInterruptConfirmLabel")}
                              body={t("activitySubagentInterruptConfirmBody", {
                                id: subagent.id,
                                status: activityStatusLabel(subagent.status),
                              })}
                              confirmLabel={t("activityConfirm")}
                              onConfirm={() => {
                                void runActivityAction("subagent_interrupt", subagent.id);
                              }}
                              onCancel={() => {
                                setStopConfirm(null);
                              }}
                            />
                          ) : null}
                        </li>
                      ))}
                    </ul>
                  )}
                </ActivitySection>

                <ActivitySection title={t("activityTeams")} count={activity.teams.length}>
                  {activity.teams.length === 0 ? (
                    <p>{t("activityTeamsEmpty")}</p>
                  ) : (
                    (() => {
                      const team = activity.teams[activity.teams.length - 1];
                      if (team === undefined) return <p>{t("activityTeamsEmpty")}</p>;
                      return (
                        <div className="rounded-control border border-border px-2 py-1">
                          <p className="truncate text-foreground">{team.objective}</p>
                          <p className="text-[11px]">
                            {t("activityTeamSummary", {
                              status: teamStatusLabel(team.status),
                              completed: team.completed,
                              failed: team.failed,
                              cancelled: team.cancelled,
                              total: team.total,
                            })}
                          </p>
                          <ul className="flex flex-col gap-0.5">
                            {team.workers.map((worker) => (
                              <li key={worker.id} className="truncate text-[11px]">
                                {worker.id} · {worker.task} · {activityStatusLabel(worker.status)}
                              </li>
                            ))}
                          </ul>
                        </div>
                      );
                    })()
                  )}
                </ActivitySection>

                <ActivitySection title={t("activityTodos")} count={activity.todos.length}>
                  {activity.todos.length === 0 ? (
                    <p>{t("activityTodosEmpty")}</p>
                  ) : (
                    <ul className="flex flex-col gap-0.5">
                      {activity.todos.map((todo) => (
                        <li key={todo.id} className="flex items-center gap-1">
                          <span aria-hidden="true" className="text-[11px]">
                            {todo.status === "completed" ? "✓" : todo.status === "in_progress" ? "◐" : "○"}
                          </span>
                          <span className="min-w-0 flex-1 truncate text-foreground">{todo.text}</span>
                          <span className="text-[11px] text-muted-foreground">
                            {todoStatusLabel(todo.status)}
                          </span>
                        </li>
                      ))}
                    </ul>
                  )}
                </ActivitySection>

                <ActivitySection title={t("activitySchedules")} count={activity.schedules.length}>
                  {activity.schedules_error !== null && activity.schedules_error !== undefined ? (
                    <div role="alert" className="text-destructive">
                      {activityErrorText(activity.schedules_error)}
                    </div>
                  ) : null}
                  {activity.schedules.length === 0 ? (
                    <p>{t("activitySchedulesEmpty")}</p>
                  ) : (
                    <ul className="flex flex-col gap-1">
                      {activity.schedules.map((schedule) => (
                        <li key={schedule.id} className="rounded-control border border-border px-2 py-1">
                          <div className="flex items-center gap-1">
                            <span className="min-w-0 flex-1 truncate text-foreground">
                              {schedule.prompt === "" ? schedule.id : schedule.prompt}
                            </span>
                            <Switch
                              role="switch"
                              checked={schedule.enabled}
                              aria-checked={schedule.enabled}
                              aria-label={t("activityScheduleToggle", { id: schedule.id })}
                              disabled={actionBusyId !== null}
                              onChange={(event) => {
                                void runActivityAction("schedule_update", schedule.id, {
                                  enabled: event.target.checked,
                                });
                              }}
                            />
                          </div>
                          <p className="text-[11px]">
                            {schedule.cron} · {t("activityScheduleIntervalShort", { seconds: schedule.interval_seconds })}
                          </p>
                          <div className="flex items-center gap-1">
                            <Button
                              size="sm"
                              variant="ghost"
                              aria-expanded={editSchedule === schedule.id}
                              onClick={() => {
                                setEditSchedule(schedule.id);
                                setScheduleInterval(String(schedule.interval_seconds));
                                setSchedulePrompt(schedule.prompt);
                                setScheduleError(null);
                              }}
                            >
                              {t("activityScheduleEdit")}
                            </Button>
                            <Button
                              size="sm"
                              variant="destructive"
                              aria-expanded={deleteSchedule === schedule.id}
                              disabled={actionBusyId !== null}
                              onClick={() => {
                                setDeleteSchedule(schedule.id);
                              }}
                            >
                              {t("activityScheduleDelete")}
                            </Button>
                          </div>
                          {editSchedule === schedule.id ? (
                            <div className="mt-1 flex flex-col gap-1">
                              <label className="flex flex-col gap-0.5 text-[11px]">
                                {t("activityScheduleInterval")}
                                <input
                                  className="rounded-control border border-border bg-transparent px-1 py-0.5 font-mono text-foreground"
                                  value={scheduleInterval}
                                  onChange={(event) => {
                                    setScheduleInterval(event.target.value);
                                  }}
                                />
                              </label>
                              <label className="flex flex-col gap-0.5 text-[11px]">
                                {t("activitySchedulePrompt")}
                                <textarea
                                  className="min-h-[48px] rounded-control border border-border bg-transparent px-1 py-0.5 text-foreground"
                                  value={schedulePrompt}
                                  onChange={(event) => {
                                    setSchedulePrompt(event.target.value);
                                  }}
                                />
                              </label>
                              {scheduleError !== null ? (
                                <p role="alert" className="text-destructive">
                                  {t(scheduleError)}
                                </p>
                              ) : null}
                              <div className="flex items-center gap-1">
                                <Button size="sm" disabled={actionBusyId !== null} onClick={saveSchedule}>
                                  {t("activityScheduleSave")}
                                </Button>
                                <Button
                                  size="sm"
                                  variant="ghost"
                                  onClick={() => {
                                    setEditSchedule(null);
                                    setScheduleError(null);
                                  }}
                                >
                                  {t("activityCancel")}
                                </Button>
                              </div>
                            </div>
                          ) : null}
                          {deleteSchedule === schedule.id ? (
                            <InlineConfirm
                              label={t("activityScheduleDeleteConfirmLabel")}
                              body={t("activityScheduleDeleteConfirmBody", { id: schedule.id })}
                              confirmLabel={t("activityConfirm")}
                              onConfirm={() => {
                                void runActivityAction("schedule_delete", schedule.id);
                              }}
                              onCancel={() => {
                                setDeleteSchedule(null);
                              }}
                            />
                          ) : null}
                        </li>
                      ))}
                    </ul>
                  )}
                </ActivitySection>

                <ActivitySection title={t("activityPresentations")} count={activity.presentations.length}>
                  {activity.presentations.length === 0 ? (
                    <p>{t("activityPresentationsEmpty")}</p>
                  ) : (
                    <ul className="flex flex-col gap-0.5">
                      {activity.presentations.map((presentation) => (
                        <li key={presentation.path} className="flex flex-col gap-0.5">
                          <span className="truncate font-mono text-foreground">{presentation.path}</span>
                          {presentation.description === "" ? null : (
                            <span className="truncate text-[11px]">{presentation.description}</span>
                          )}
                          <Button
                            size="sm"
                            variant="ghost"
                            onClick={() => {
                              actions.openInspectorPreview(presentation.path);
                            }}
                          >
                            {t("activityPresentView")}
                          </Button>
                        </li>
                      ))}
                    </ul>
                  )}
                </ActivitySection>

                <ActivitySection title={t("activityGoal")} count={ui.goal === null ? 0 : 1}>
                  {ui.goal === null ? (
                    <p>{t("activityGoalEmpty")}</p>
                  ) : (
                    <div className="rounded-control border border-border px-2 py-1">
                      <p className="break-words text-foreground">{ui.goal.objective}</p>
                      <p className="text-[11px]">
                        {t("activityGoalMeta", {
                          status: ui.goal.status,
                          rounds: ui.goal.rounds,
                          max_rounds: ui.goal.max_rounds,
                        })}
                      </p>
                      {ui.goal.reason === null || ui.goal.reason === undefined || ui.goal.reason === "" ? null : (
                        <p className="text-[11px]">{ui.goal.reason}</p>
                      )}
                    </div>
                  )}
                </ActivitySection>
              </>
            )}
          </div>
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
