import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  SlotErrorBoundary,
  UiProvider,
  isRecord,
  readNumber,
  readString,
  type AppSettings,
  type AttachmentSaveResult,
  type BrandConfig,
  type CommandDescriptor,
  type SettingsSaveOutcome,
  type CompactionMarker,
  type ConversationExport,
  type ConversationOverride,
  type FeedbackCategory,
  type GoalView,
  type UiValue,
  type WorkspaceFileRef,
  type SessionSearchOutcome,
} from "@zeokeel/ui-kit";
import { BRAND } from "./generated/brand";
import { AppSurfaceSlot, AppsSlot, ChatSlot, CommandsSlot, ConnectorsSlot, InspectorSlot, LoginSlot, SessionsSlot, SettingsPanes, SettingsSlot, ShellSlot, SkillsSlot } from "./generated/registry";
import { translate } from "./i18n";
import { UpgradeGate } from "./UpgradeGate";
import { upgradeInfoOf } from "./upgrade";
import { useAppController, type AppController } from "./state/controller";
import { useResolvedTheme } from "./theme";
import type { Transport } from "./types";
import { parseCompactResult, parseCompactionGetResult, parseConversation, parseConversationOverrideResult, parseFeedbackSubmitResult, parseFileEntries, parseGoalResult, parseSessionExportResult, parseSessionSearchResult, parseTerminalResult, parseUpdateRelease, parseWorkspaceChanges, parseWorkspaceFileReadResult } from "./protocol";
import { validateAttachmentSaveResult } from "./validate";
import { downloadSkillArchive, fetchAppHtml, fetchApps, fetchConnectorCatalog, fetchSkills, fetchTombstones, openApp, sendInstallReceipt, toBase64, toggleConnector, type AppItem, type ConnectorItem, type SkillItem } from "./store";
import { clearConnectorCatalog, reloadConnectorSessions, syncConnectorCatalog } from "./connectors-sync";
import { probeMcpServer } from "./mcp-probe";
import { oauthMetadataPayload, parseOAuthTokens, tokenNeedsRefresh, waitForOAuthFlow } from "./oauth";
import { clearSession, fetchBootstrapInfo, loadGatewayModel, loadServerUrl, loadSession, login as loginRequest, refreshSession, saveGatewayModel, saveServerUrl, saveSession, type AuthSession } from "./auth";

/** Channel default login address (C14/ADR-042); non-empty = locked address. */
const CHANNEL_SERVER_URL: string = BRAND.serverUrl;

function isMockMode(): boolean {
  return new URLSearchParams(window.location.search).get("mock") === "1";
}

function normalizeUrl(value: string): string {
  return value.trim().replace(/\/$/, "");
}

/** Channel default > persisted settings > legacy localStorage default. */
function resolveServerUrl(settingsUrl: string): string {
  if (CHANNEL_SERVER_URL !== "") {
    return normalizeUrl(CHANNEL_SERVER_URL);
  }
  const fromSettings = normalizeUrl(settingsUrl);
  if (fromSettings !== "") {
    return fromSettings;
  }
  return normalizeUrl(loadServerUrl());
}

interface SurfaceState {
  html: string;
  nonce: string;
  appId: string;
}

interface SurfaceRequestPayload {
  app_id?: string;
  method: string;
  path: string;
  query: string;
  body_b64: string;
  request_id: string;
}

interface SurfaceResponse {
  status: number;
  body_b64: string;
}

/** One command catalog input: the app owns the data, both surfaces render it. */
export interface CommandCatalogInput {
  t: (key: string, vars?: Record<string, string | number>) => string;
  enterprise: boolean;
  streaming: boolean;
  /** Last text sent in the active conversation (`null` = nothing to retry). */
  lastSendText: string | null;
}

/**
 * The **single** command catalog (`2026-11-01-client-composer-surface` D1/D6).
 *
 * ⌘K (`ui-commands` palette) and the composer `/` menu both render this array
 * and both execute through `UiActions.runCommand` — no second list/executor.
 * Personal = 6 commands; enterprise adds the skills/connectors/apps panels (9).
 */
/** DSH 1:1 slash commands (Batch 1) + Batch 2 placeholders (shared catalog rows). */
export function buildSlashCommands(
  t: (key: string, vars?: Record<string, string | number>) => string,
  streaming = false,
): CommandDescriptor[] {
  const row = (
    id: string,
    name: string,
    title: string,
    group: string,
    kind: "action" | "popupSelect" | "leadingInput" | "execute" | "disabled",
  ): CommandDescriptor => ({
    id,
    name,
    title,
    group,
    kind,
    surface: "slash",
    ...(kind === "disabled" ? { disabled: true, disabledReason: t("commandUnimplemented") } : {}),
  });
  return [
    row("slash-file", "file", t("slashFile"), t("commandGroupAdd"), "action"),
    row("slash-goal", "goal", t("slashGoal"), t("commandGroupAdd"), "leadingInput"),
    row("slash-plan", "plan", t("slashPlan"), t("commandGroupAdd"), "disabled"),
    row("slash-feedback", "feedback", t("slashFeedback"), t("commandGroupAdd"), "action"),
    { ...row("slash-compact", "compact", t("slashCompact"), t("commandGroupCommand"), "execute"), ...(streaming ? { disabled: true, disabledReason: t("compactStreamingBlocked") } : {}) },
    row("slash-permission", "permission", t("slashPermission"), t("commandGroupCommand"), "popupSelect"),
    row("slash-model", "model", t("slashModel"), t("commandGroupCommand"), "popupSelect"),
    row("slash-export", "export", t("slashExport"), t("commandGroupCommand"), "execute"),
    row("slash-skill", "skill", t("slashSkill"), t("commandGroupSkill"), "disabled"),
  ];
}

/**
 * The **single** command catalog (`2026-11-01-client-composer-surface` D1/D6,
 * `2026-11-01-client-command-surface` R1): ⌘K palette rows carry
 * `surface: "palette"`, composer `/` rows carry `surface: "slash"`; both
 * surfaces render this array and execute through `UiActions.runCommand`.
 */
export function buildCommandCatalog(input: CommandCatalogInput): CommandDescriptor[] {
  const { t, enterprise, streaming, lastSendText } = input;
  const palette: CommandDescriptor[] = [
    {
      id: "new-conversation",
      title: t("newConversation"),
      group: t("workspaceActions"),
      surface: "palette",
      disabled: streaming,
      ...(streaming ? { disabledReason: t("newConversationStreamingBlocked") } : {}),
    },
    { id: "toggle-sidebar", title: t("sidebarToggle"), group: t("workspaceActions"), surface: "palette" },
    { id: "open-settings", title: t("settingsTitle"), group: t("settings"), surface: "palette" },
    { id: "toggle-inspector", title: t("inspectorToggle"), group: t("workspaceActions"), surface: "palette" },
    {
      id: "cancel",
      title: t("commandCancel"),
      group: t("commandGroupTurn"),
      hint: t("commandHintCancel"),
      disabled: !streaming,
      surface: "palette",
    },
    {
      id: "retry",
      title: t("commandRetry"),
      group: t("commandGroupTurn"),
      hint: t("commandHintRetry"),
      disabled: streaming || lastSendText === null,
      surface: "palette",
    },
  ];
  if (enterprise) {
    palette.push(
      { id: "open-skills", title: t("panelSkills"), group: t("workspaceActions"), surface: "palette" },
      { id: "open-connectors", title: t("panelConnectors"), group: t("workspaceActions"), surface: "palette" },
      { id: "open-apps", title: t("panelApps"), group: t("workspaceActions"), surface: "palette" },
    );
  }
  return [...palette, ...buildSlashCommands(t, streaming)];
}

export function App(props: { transport: Transport }) {
  const controller = useAppController(props.transport);
  const reportUpgrade = controller.reportUpgrade;
  const { state } = controller;
  const [session, setSession] = useState<AuthSession | null>(loadSession);
  const [sessionServerUrl, setSessionServerUrl] = useState<string | null>(() =>
    loadSession() === null ? null : resolveServerUrl(controller.state.settings.server_url),
  );
  const [loginError, setLoginError] = useState<string | null>(null);
  const [loginLoading, setLoginLoading] = useState(false);
  const [gatewayModels, setGatewayModels] = useState<string[]>([]);
  const [goal, setGoalState] = useState<GoalView | null>(null);
  const [slashCommand, setSlashCommand] = useState<{ name: string; query: string; token: number } | null>(null);
  const [compaction, setCompaction] = useState<CompactionMarker | null>(null);
  const [gatewayModel, setGatewayModel] = useState<string>(loadGatewayModel);
  const [updateText, setUpdateText] = useState<string | null>(null);
  const [updateBusy, setUpdateBusy] = useState(false);
  const [skills, setSkills] = useState<SkillItem[]>([]);
  const [connectors, setConnectors] = useState<ConnectorItem[]>([]);
  const [apps, setApps] = useState<AppItem[]>([]);
  const [appOpened, setAppOpened] = useState<string | null>(null);
  const [appError, setAppError] = useState<string | null>(null);
  const [appSurface, setAppSurface] = useState<SurfaceState | null>(null);
  const [installed, setInstalled] = useState<{ name: string; version: string }[]>([]);
  const [contentError, setContentError] = useState<string | null>(null);
  const [contentBusy, setContentBusy] = useState<string | null>(null);
  const [authorized, setAuthorized] = useState<string[]>([]);

  const theme = useResolvedTheme(state.settings.theme);
  const brand: BrandConfig = BRAND;
  const t = useCallback(
    (key: string, vars?: Record<string, string | number>): string =>
      translate(state.settings.locale, key, vars),
    [state.settings.locale],
  );

  // Composer surface state (`2026-11-01-client-composer-surface`): the settings
  // pane deep-link and the workspace-file candidate cache live here so the
  // command catalog / actions have exactly one owner (app).
  const [settingsInitialPane, setSettingsInitialPane] = useState<string | undefined>(undefined);
  const [panel, setPanel] = useState<"skills" | "connectors" | "apps" | null>(null);
  const [inspectorOpen, setInspectorOpen] = useState(false);
  const workspaceFilesCache = useRef<{ workspaceId: string; files: WorkspaceFileRef[] } | null>(
    null,
  );

  const openSettings = useCallback(
    (paneId?: string): void => {
      setSettingsInitialPane(paneId);
      controller.setSettingsOpen(true);
    },
    [controller],
  );

  const closeSettings = useCallback((): void => {
    setSettingsInitialPane(undefined);
    controller.setSettingsOpen(false);
  }, [controller]);

  const toggleInspector = useCallback((): void => {
    setInspectorOpen((value) => !value);
  }, []);

  /** `/compact`: summarize older history into a marker (raw history untouched). */
  const compactConversation = useCallback(async (): Promise<boolean> => {
    const workspaceId = state.activeWorkspaceId;
    const conversationId = state.activeConversationId;
    if (workspaceId === null || conversationId === null) {
      controller.showToast(t("workspaceRequired"));
      return false;
    }
    try {
      const parsed = parseCompactResult(
        await props.transport.call("session.compact", {
          workspace_id: workspaceId,
          conversation_id: conversationId,
        }),
      );
      if (parsed === null) {
        controller.showToast(t("compactFailed"));
        return false;
      }
      setCompaction(parsed.marker);
      controller.showToast(parsed.noop ? t("compactNoop") : t("compactDone"));
      if (!parsed.noop) {
        // Reload raw messages so the divider can anchor to the persisted message id.
        controller.retryMessages();
      }
      return true;
    } catch {
      controller.showToast(t("compactFailed"));
      return false;
    }
  }, [controller, props.transport, state.activeWorkspaceId, state.activeConversationId, t]);

  /** Single executor for the single catalog (⌘K palette + composer `/`). */
  const runCommand = useCallback(
    (id: string, query?: string): void => {
      switch (id) {
        case "new-conversation":
          void controller.createConversation();
          return;
        case "toggle-sidebar":
          controller.toggleSidebar();
          return;
        case "open-settings":
          openSettings();
          return;
        case "toggle-inspector":
          setInspectorOpen((value) => !value);
          return;
        case "open-skills":
          setPanel("skills");
          return;
        case "open-connectors":
          setPanel("connectors");
          return;
        case "open-apps":
          setPanel("apps");
          return;
        case "cancel":
          controller.cancel();
          return;
        case "retry":
          controller.retrySend();
          return;
        default:
          break;
      }
      // DSH `/` commands: same catalog, same executor; the composer reacts to the
      // intent (file picker / popups / dialogs) instead of duplicating executors.
      if (id === "slash-compact") {
        void compactConversation();
        return;
      }
      if (id.startsWith("slash-")) {
        const command = buildSlashCommands(t, state.streaming).find((entry) => entry.id === id);
        if (command?.name === undefined || command.disabled === true) {
          controller.showToast(t("commandUnimplemented"));
          return;
        }
        setSlashCommand({ name: command.name, query: query ?? "", token: Date.now() });
      }
    },
    [compactConversation, controller, openSettings, state.streaming, t],
  );

  /** Workspace file candidates for `@` (cached `workspace.files`; never a second IPC). */
  const searchWorkspaceFiles = useCallback(
    async (query: string): Promise<WorkspaceFileRef[]> => {
      const workspaceId = state.activeWorkspaceId;
      if (workspaceId === null) {
        return [];
      }
      const cached = workspaceFilesCache.current;
      let files = cached !== null && cached.workspaceId === workspaceId ? cached.files : null;
      if (files === null) {
        const raw = await props.transport.call("workspace.files", { workspace_id: workspaceId });
        const entries = parseFileEntries(raw) ?? [];
        const next: WorkspaceFileRef[] = [];
        for (const entry of entries) {
          // Candidate-only filter: hidden tool/attachment/VCS dirs stay out of `@`.
          if (
            entry.path === ".git" ||
            entry.path.startsWith(".git/") ||
            entry.path === ".zeokeel" ||
            entry.path.startsWith(".zeokeel/")
          ) {
            continue;
          }
          next.push({ path: entry.path, kind: entry.kind === "dir" ? "dir" : "file" });
        }
        workspaceFilesCache.current = { workspaceId, files: next };
        files = next;
      }
      const needle = query.trim().toLowerCase();
      const matched =
        needle === "" ? files : files.filter((file) => file.path.toLowerCase().includes(needle));
      return matched.slice(0, 50);
    },
    [props.transport, state.activeWorkspaceId],
  );

  /** Persist one attachment via the single host IPC, validating the host result (C10). */
  const saveAttachment = useCallback(
    async (file: File): Promise<AttachmentSaveResult> => {
      const workspaceId = state.activeWorkspaceId;
      const conversationId = state.activeConversationId;
      if (workspaceId === null || conversationId === null) {
        throw new Error("attachment/invalid: no active conversation");
      }
      const bytes = new Uint8Array(await file.arrayBuffer());
      const raw = await props.transport.call("workspace.attachment_save", {
        workspace_id: workspaceId,
        conversation_id: conversationId,
        name: file.name,
        mime: file.type,
        data_base64: toBase64(bytes),
      });
      if (!isRecord(raw) || !validateAttachmentSaveResult(raw)) {
        throw new Error("attachment/invalid: unexpected host response");
      }
      const path = readString(raw, "path");
      const name = readString(raw, "name");
      const mime = readString(raw, "mime");
      const written = readNumber(raw, "bytes");
      if (path === null || name === null || mime === null || written === null) {
        throw new Error("attachment/invalid: incomplete host response");
      }
      return { path, name, mime, bytes: written };
    },
    [props.transport, state.activeWorkspaceId, state.activeConversationId],
  );

  /** Rename/pin/archive one conversation (C18: workspace id never changes). */
  const updateConversation = useCallback(
    async (
      conversationId: string,
      patch: { title?: string; pinned?: boolean; archived?: boolean },
    ): Promise<boolean> => {
      const workspaceId = state.activeWorkspaceId;
      if (workspaceId === null) {
        return false;
      }
      try {
        const raw = await props.transport.call("session.update", {
          workspace_id: workspaceId,
          conversation_id: conversationId,
          ...patch,
        });
        if (parseConversation(raw) === null) {
          controller.showToast(t("sessionOpFailed"));
          return false;
        }
        await controller.refreshActiveConversations();
        return true;
      } catch {
        controller.showToast(t("sessionOpFailed"));
        return false;
      }
    },
    [controller, props.transport, state.activeWorkspaceId, t],
  );

  /** Fork one conversation inside its workspace and select the copy (C18). */
  const forkConversation = useCallback(
    async (conversationId: string): Promise<boolean> => {
      const workspaceId = state.activeWorkspaceId;
      if (workspaceId === null) {
        return false;
      }
      try {
        const raw = await props.transport.call("session.fork", {
          workspace_id: workspaceId,
          conversation_id: conversationId,
        });
        const forked = parseConversation(raw);
        if (forked === null) {
          controller.showToast(t("sessionOpFailed"));
          return false;
        }
        await controller.refreshActiveConversations();
        controller.selectConversation(forked.id);
        return true;
      } catch {
        controller.showToast(t("sessionOpFailed"));
        return false;
      }
    },
    [controller, props.transport, state.activeWorkspaceId, t],
  );

  /** Copy a conversation id to the clipboard (with a success toast). */
  const copyConversationId = useCallback(
    async (conversationId: string): Promise<boolean> => {
      try {
        await navigator.clipboard.writeText(conversationId);
        controller.showToast(t("sessionCopied", { id: conversationId }));
        return true;
      } catch {
        controller.showToast(t("sessionCopyFailed"));
        return false;
      }
    },
    [controller, t],
  );

  /** Content search in the current workspace (`session.search`; snippet hits only). */
  const searchSessions = useCallback(
    async (query: string, limit?: number): Promise<SessionSearchOutcome> => {
      const workspaceId = state.activeWorkspaceId;
      if (workspaceId === null || query.trim() === "") {
        return { items: [], truncated: false };
      }
      try {
        const raw = await props.transport.call("session.search", {
          workspace_id: workspaceId,
          query,
          limit: limit ?? 20,
        });
        const parsed = parseSessionSearchResult(raw);
        if (parsed === null) {
          return { items: [], truncated: false };
        }
        return {
          items: parsed.items.map((hit) => ({
            conversationId: hit.conversation_id,
            title: hit.title,
            messageId: hit.message_id,
            role: hit.role,
            snippet: hit.snippet,
            archived: hit.archived,
          })),
          truncated: parsed.truncated,
        };
      } catch {
        throw new Error(t("sessionSearchFailed"));
      }
    },
    [props.transport, state.activeWorkspaceId, t],
  );

  const commandCatalog = buildCommandCatalog({
    t,
    enterprise: state.meta.edition === "enterprise",
    streaming: state.streaming,
    lastSendText: state.lastSendText,
  });
  const providerModel = state.settings.provider.model.trim();
  const modelLabel =
    state.meta.edition === "personal"
      ? providerModel === ""
        ? t("modelUnconfigured")
        : providerModel
      : gatewayModel === ""
        ? t("gatewayModelUnselected")
        : gatewayModel;
  const modelSettingsPane = state.meta.edition === "personal" ? "provider" : "enterprise";

  // Channel default > persisted settings > legacy localStorage default (C14/ADR-042).
  const activeServerUrl = resolveServerUrl(state.settings.server_url);

  const logout = (): void => {
    const current = session;
    clearSession();
    setSession(null);
    setSessionServerUrl(null);
    setGatewayModels([]);
    setGatewayModel("");
    setLoginError(null);
    if (current !== null) {
      void props.transport.call("auth.logout", { access_token: current.access_token }).catch(() => undefined);
    }
    // Drop any catalog-provided MCP config on logout (config spec R7; idempotent
    // with the host's own auth.logout cleanup).
    void clearConnectorCatalog(props.transport).catch(() => undefined);
  };

  // Enterprise session: configure gateway, load catalog, refresh token (docs/19 §3.2, docs/38).
  useEffect(() => {
    if (state.meta.edition !== "enterprise" || session === null) return;
    const url = activeServerUrl;
    if (url === "") return;
    void props.transport.call("auth.configure", { server_url: url, access_token: session.access_token, model: loadGatewayModel(), models: gatewayModels }).catch(() => undefined);
    void Promise.all([fetchSkills(session, url), fetchConnectorCatalog(session, url), props.transport.call("skills.installed", {}), fetchTombstones(session, url), fetchApps(session, url)])
      .then(async ([skillItems, connectorCatalog, installedValue, tombstones, appItems]) => {
        setSkills(skillItems);
        setConnectors(connectorCatalog.items);
        setApps(appItems);
        // Web -> host catalog sync (config spec R7): the host validates and
        // installs the MCP config; failures stay visible, the old config runs on.
        void syncConnectorCatalog(props.transport, connectorCatalog).catch((error: unknown) => {
          setContentError(error instanceof Error ? error.message : t("connectorsCatalogFailed"));
        });
        const oauthIds: string[] = [];
        for (const connector of connectorCatalog.items) {
          if (connector.oauth === null) continue;
          const stored = await props.transport.call("oauth.load", { connector_id: connector.id }).catch(() => null);
          const raw = isRecord(stored) ? stored["tokens"] : null;
          const tokens = parseOAuthTokens(raw);
          if (tokens === null) continue;
          let authorized = !tokenNeedsRefresh(tokens);
          if (!authorized) {
            // The host rotates the stored token via the loopback flow module
            // (`oauth.refresh`); poll its status instead of doing crypto here.
            await props.transport.call("oauth.refresh", {
              connector_id: connector.id,
              oauth: oauthMetadataPayload(connector.oauth),
            }).catch(() => undefined);
            const status = await waitForOAuthFlow((name, payload) => props.transport.call(name, payload), connector.id, { timeoutMs: 30_000 }).catch(() => null);
            authorized = status?.state === "authorized";
          }
          if (authorized) oauthIds.push(connector.id);
        }
        setAuthorized(oauthIds);
        const rows = isRecord(installedValue) ? installedValue["items"] : null;
        const local = Array.isArray(rows) ? rows.flatMap((row) => {
          if (!isRecord(row)) return [];
          const name = readString(row, "name");
          const version = readString(row, "version");
          return name !== null && version !== null ? [{ name, version }] : [];
        }) : [];
        for (const entry of local) {
          if (tombstones.includes(entry.name)) {
            await props.transport.call("skills.remove", { name: entry.name }).catch(() => undefined);
          }
        }
        setInstalled(local.filter((entry) => !tombstones.includes(entry.name)));
      })
      .catch((error: unknown) => {
        const upgrade = upgradeInfoOf(error);
        if (upgrade !== null) {
          reportUpgrade(upgrade);
          return;
        }
        setContentError(error instanceof Error ? error.message : t("contentLoadFailed"));
      });
    const refreshMs = Math.max(session.expires_in, 60) * 800;
    const timer = window.setTimeout(() => {
      void refreshSession(session, url)
        .then((next) => {
          saveSession(next);
          setSession(next);
          void props.transport.call("auth.configure", { server_url: url, access_token: next.access_token, model: loadGatewayModel(), models: gatewayModels }).catch(() => undefined);
        })
        .catch((error: unknown) => {
          const upgrade = upgradeInfoOf(error);
          if (upgrade !== null) {
            reportUpgrade(upgrade);
            return;
          }
          clearSession();
          setSession(null);
        });
    }, refreshMs);
    return () => { window.clearTimeout(timer); };
  }, [state.meta.edition, session, activeServerUrl, props.transport, t, reportUpgrade, gatewayModels]);

  // Policy/catalog may change server-side; refresh it on every authenticated start (not only login).
  useEffect(() => {
    if (state.meta.edition !== "enterprise" || session === null || activeServerUrl === "") return;
    let cancelled = false;
    const isCancelled = (): boolean => cancelled;
    void fetchBootstrapInfo(session, activeServerUrl)
      .then((info) => {
        if (isCancelled()) return;
        setGatewayModels(info.models);
        const model = info.defaultModel === "" ? loadGatewayModel() : info.defaultModel;
        if (model !== "") {
          setGatewayModel(model);
          saveGatewayModel(model);
        }
      })
      .catch((error: unknown) => {
        const upgrade = upgradeInfoOf(error);
        if (upgrade !== null) {
          reportUpgrade(upgrade);
        }
      });
    return () => { cancelled = true; };
  }, [state.meta.edition, session, activeServerUrl, reportUpgrade]);

  // Latest compaction marker is conversation-scoped (raw messages stay visible).
  useEffect(() => {
    const workspaceId = state.activeWorkspaceId;
    const conversationId = state.activeConversationId;
    let active = true;
    const timer = window.setTimeout(() => {
      if (!active) return;
      if (workspaceId === null || conversationId === null) {
        setCompaction(null);
        return;
      }
      void props.transport
        .call("session.compaction_get", { workspace_id: workspaceId, conversation_id: conversationId })
        .then((raw) => {
          if (!active) return;
          const parsed = parseCompactionGetResult(raw);
          setCompaction(parsed === null ? null : parsed.marker);
        })
        .catch(() => {
          if (active) setCompaction(null);
        });
    }, 0);
    return () => {
      active = false;
      window.clearTimeout(timer);
    };
  }, [props.transport, state.activeWorkspaceId, state.activeConversationId]);

  // Conversation goal is conversation-scoped; reload it when the selection changes.
  useEffect(() => {
    const workspaceId = state.activeWorkspaceId;
    const conversationId = state.activeConversationId;
    let active = true;
    const timer = window.setTimeout(() => {
      if (!active) return;
      if (workspaceId === null || conversationId === null) {
        setGoalState(null);
        return;
      }
      void props.transport
        .call("goal.get", { workspace_id: workspaceId, conversation_id: conversationId })
        .then((raw) => {
          if (!active) return;
          const parsed = parseGoalResult(raw);
          setGoalState(parsed === null ? null : parsed.goal);
        })
        .catch(() => {
          if (active) setGoalState(null);
        });
    }, 0);
    return () => {
      active = false;
      window.clearTimeout(timer);
    };
  }, [props.transport, state.activeWorkspaceId, state.activeConversationId]);

  const conversationArgs = useCallback((): { workspace_id: string; conversation_id: string } | null => {
    const workspaceId = state.activeWorkspaceId;
    const conversationId = state.activeConversationId;
    if (workspaceId === null || conversationId === null) return null;
    return { workspace_id: workspaceId, conversation_id: conversationId };
  }, [state.activeWorkspaceId, state.activeConversationId]);

  const refreshGoal = useCallback(async (): Promise<void> => {
    const args = conversationArgs();
    if (args === null) return;
    const parsed = parseGoalResult(await props.transport.call("goal.get", args));
    setGoalState(parsed === null ? null : parsed.goal);
  }, [conversationArgs, props.transport]);

  const setGoal = useCallback(
    async (objective: string, maxRounds?: number): Promise<boolean> => {
      const args = conversationArgs();
      if (args === null) return false;
      try {
        const raw = await props.transport.call("goal.set", {
          ...args,
          objective,
          ...(maxRounds === undefined ? {} : { max_rounds: maxRounds }),
        });
        const parsed = parseGoalResult(raw);
        if (parsed === null) {
          controller.showToast(t("goalFailed"));
          return false;
        }
        setGoalState(parsed.goal);
        return true;
      } catch {
        controller.showToast(t("goalFailed"));
        return false;
      }
    },
    [controller, conversationArgs, props.transport, t],
  );

  const updateGoal = useCallback(
    async (action: "pause" | "resume" | "cancel"): Promise<boolean> => {
      const args = conversationArgs();
      if (args === null) return false;
      try {
        const raw = await props.transport.call("goal.update", { ...args, action });
        const parsed = parseGoalResult(raw);
        if (parsed === null) {
          controller.showToast(t("goalFailed"));
          return false;
        }
        // `clear` maps to goal.update{cancel}; the UI hides the bar immediately.
        setGoalState(action === "cancel" ? null : parsed.goal);
        return true;
      } catch {
        controller.showToast(t("goalFailed"));
        return false;
      }
    },
    [controller, conversationArgs, props.transport, t],
  );

  const getConversationOverride = useCallback(async (): Promise<ConversationOverride | null> => {
    const args = conversationArgs();
    if (args === null) return null;
    try {
      const parsed = parseConversationOverrideResult(
        await props.transport.call("session.override_get", args),
      );
      return parsed;
    } catch {
      return null;
    }
  }, [conversationArgs, props.transport]);

  const setConversationOverride = useCallback(
    async (patch: {
      model?: string;
      reasoning_effort?: string;
      approval_policy?: string;
    }): Promise<boolean> => {
      const args = conversationArgs();
      if (args === null) return false;
      try {
        const parsed = parseConversationOverrideResult(
          await props.transport.call("session.override_set", { ...args, ...patch }),
        );
        if (parsed === null) {
          controller.showToast(t("overrideFailed"));
          return false;
        }
        return true;
      } catch {
        controller.showToast(t("overrideFailed"));
        return false;
      }
    },
    [controller, conversationArgs, props.transport, t],
  );

  const exportConversation = useCallback(async (): Promise<ConversationExport | null> => {
    const args = conversationArgs();
    if (args === null) return null;
    try {
      const parsed = parseSessionExportResult(
        await props.transport.call("session.export", { ...args, format: "markdown" }),
      );
      if (parsed === null) {
        controller.showToast(t("exportFailed"));
        return null;
      }
      // Serve carrier: fetch the exported file through the same-origin download route.
      // Tauri: display + copy the local path (no automatic open; system.open_url is http(s) only).
      if (window.location.protocol === "http:" || window.location.protocol === "https:") {
        const url = `/download?path=${encodeURIComponent(parsed.path)}`;
        const link = document.createElement("a");
        link.href = url;
        link.download = parsed.path.split("/").at(-1) ?? "conversation.md";
        link.click();
      } else {
        await navigator.clipboard.writeText(parsed.path).catch(() => undefined);
      }
      controller.showToast(t("exportReady", { path: parsed.path }));
      return parsed;
    } catch (error: unknown) {
      const message = error instanceof Error ? error.message : "";
      controller.showToast(
        message.includes("no messages to export") ? t("exportEmpty") : t("exportFailed"),
      );
      return null;
    }
  }, [controller, conversationArgs, props.transport, t]);

  const submitFeedback = useCallback(
    async (category: FeedbackCategory, description: string): Promise<boolean> => {
      const args = conversationArgs();
      if (args === null) return false;
      try {
        const parsed = parseFeedbackSubmitResult(
          await props.transport.call("feedback.submit", { ...args, category, description }),
        );
        if (parsed === null) {
          controller.showToast(t("feedbackFailed"));
          return false;
        }
        controller.showToast(t("feedbackSavedLocal"));
        return true;
      } catch {
        controller.showToast(t("feedbackFailed"));
        return false;
      }
    },
    [controller, conversationArgs, props.transport, t],
  );

  const ui: UiValue = {
    t,
    locale: state.settings.locale,
    theme,
    brand,
    boot: state.boot,
    bootError: state.bootError,
    meta: state.meta,
    workspaces: state.workspaces,
    activeWorkspaceId: state.activeWorkspaceId,
    workspaceError: state.workspaceError,
    conversationsByWorkspace: state.conversationsByWorkspace,
    conversations: state.conversations,
    activeConversationId: state.activeConversationId,
    messages: state.messages,
    messagesState: state.messagesState,
    messagesError: state.messagesError,
    streaming: state.streaming,
    offline: state.offline,
    settings: state.settings,
    settingsOpen: state.settingsOpen,
    sidebarOpen: state.sidebarOpen,
    mock: isMockMode(),
    commands: commandCatalog,
    modelLabel,
    modelSettingsPane,
    gatewayModels,
    goal,
    slashCommand,
    compaction,
    actions: {
      send: controller.send,
      cancel: controller.cancel,
      retry: controller.retrySend,
      createConversation: controller.createConversation,
      createWorkspace: controller.createWorkspace,
      selectWorkspace: controller.selectWorkspace,
      renameWorkspace: controller.renameWorkspace,
      setDefaultWorkspace: controller.setDefaultWorkspace,
      removeWorkspace: controller.removeWorkspace,
      selectConversation: controller.selectConversation,
      openSettings: (): void => {
        openSettings();
      },
      openSettingsPane: (paneId: string): void => {
        openSettings(paneId);
      },
      openModelSettings: (paneId: string): void => {
        openSettings(paneId);
      },
      closeSettings,
      runCommand,
      searchWorkspaceFiles,
      saveAttachment,
      updateConversation,
      forkConversation,
      copyConversationId,
      searchSessions,
      refreshGoal,
      setGoal,
      updateGoal,
      getConversationOverride,
      setConversationOverride,
      exportConversation,
      submitFeedback,
      compactConversation,
      saveSettings: (next: AppSettings): Promise<SettingsSaveOutcome> => {
        const requiresRelogin =
          session !== null &&
          normalizeUrl(next.server_url) !== "" &&
          normalizeUrl(next.server_url) !== normalizeUrl(state.settings.server_url) &&
          normalizeUrl(next.server_url) !== (sessionServerUrl ?? "");
        return controller.saveSettings(next).then((outcome) => {
          if (outcome.status === "saved" && requiresRelogin) {
            clearSession();
            setSession(null);
            setSessionServerUrl(null);
            setGatewayModels([]);
            setLoginError(t("settingsReloginRequired"));
          }
          return outcome;
        });
      },
      reloadSettings: controller.reloadSettings,
      toggleSidebar: controller.toggleSidebar,
      openCommands: () => { controller.setCommandsOpen(true); },
      retryBoot: controller.retryBoot,
      retryMessages: controller.retryMessages,
      decideApproval: controller.decideApproval,
      answerQuestion: controller.answerQuestion,
    },
  };

  if (state.upgrade !== null) {
    return (
      <UiProvider value={ui}>
        <UpgradeGate info={state.upgrade} serverUrl={activeServerUrl} serverVersion={state.meta.version === "" ? null : state.meta.version} locale={state.settings.locale} transport={props.transport} />
      </UiProvider>
    );
  }

  if (state.meta.edition === "enterprise" && state.boot === "ready" && session === null) {
    const submit = (input: { serverUrl?: string; username: string; password: string; deviceName: string }): void => {
      // The first-run address is persisted to settings so it is not asked again (docs/19 §3.3).
      const target = normalizeUrl(input.serverUrl ?? activeServerUrl);
      if (target === "") {
        setLoginError(t("loginServerRequired"));
        return;
      }
      setLoginLoading(true);
      setLoginError(null);
      void loginRequest(target, input.username, input.password, input.deviceName)
        .then((next) => {
          saveSession(next);
          saveServerUrl(target);
          setSessionServerUrl(target);
          controller.applyServerUrl(target);
          void fetchBootstrapInfo(next, target)
            .then((info) => {
              setGatewayModels(info.models);
              const model = info.defaultModel === "" ? loadGatewayModel() : info.defaultModel;
              setGatewayModel(model);
              if (model !== "") saveGatewayModel(model);
              return props.transport.call("auth.configure", { server_url: target, access_token: next.access_token, model });
            })
            .catch((error: unknown) => {
              const upgrade = upgradeInfoOf(error);
              if (upgrade !== null) reportUpgrade(upgrade);
            });
          setSession(next);
        })
        .catch((error: unknown) => {
          const upgrade = upgradeInfoOf(error);
          if (upgrade !== null) {
            reportUpgrade(upgrade);
            return;
          }
          setLoginError(error instanceof Error ? error.message : t("loginFailed"));
        })
        .finally(() => { setLoginLoading(false); });
    };
    return (
      <UiProvider value={ui}>
        <LoginSlot
          key={activeServerUrl}
          serverUrl={activeServerUrl === "" ? null : activeServerUrl}
          channelLocked={CHANNEL_SERVER_URL !== ""}
          loading={loginLoading}
          error={loginError}
          onSubmit={submit}
        />
      </UiProvider>
    );
  }

  const effectiveUrl = activeServerUrl;

  return (
    <UiProvider value={ui}>
      <AppShell
        controller={controller}
        ui={ui}
        transport={props.transport}
        serverUrl={effectiveUrl}
        gatewayModels={gatewayModels}
        gatewayModel={gatewayModel}
        updateText={updateText}
        updateBusy={updateBusy}
        skills={skills}
        connectors={connectors}
        installed={installed}
        contentError={contentError}
        contentBusy={contentBusy}
        onInstall={(item) => {
          if (session === null) return;
          setContentBusy(item.name);
          setContentError(null);
          void downloadSkillArchive(session, effectiveUrl, item)
            .then((archive) => props.transport.call("skills.install", { name: item.name, version: item.version, archive_base64: archive.archive_base64 }))
            .then((result) => {
              const tree = isRecord(result) ? readString(result, "tree_sha256") : null;
              if (tree === null) throw new Error("missing tree_sha256");
              return sendInstallReceipt(session, effectiveUrl, item, tree);
            })
            .then(() => { setInstalled((values) => [...values.filter((entry) => entry.name !== item.name), { name: item.name, version: item.version }]); })
            .catch((error: unknown) => { setContentError(error instanceof Error ? error.message : t("contentInstallFailed")); })
            .finally(() => { setContentBusy(null); });
        }}
        authorized={authorized}
        onAuthorize={(item) => {
          const oauth = item.oauth;
          if (oauth === null) return;
          if (oauth.authorizeUrl === "" || oauth.tokenUrl === "") { setContentError(t("oauthNotConfigured")); return; }
          setContentError(null);
          // The Rust host opens the system browser, catches the loopback redirect,
          // exchanges the code (PKCE S256) and writes the keychain; the UI only polls.
          void props.transport.call("oauth.authorize", {
            connector_id: item.id,
            oauth: oauthMetadataPayload(oauth),
            timeout_seconds: 180,
          })
            .then(() => waitForOAuthFlow((name, payload) => props.transport.call(name, payload), item.id))
            .then((status) => {
              if (status.state === "authorized") {
                setAuthorized((values) => values.includes(item.id) ? values : [...values, item.id]);
                return;
              }
              setContentError(status.error !== null ? `${t("oauthFailed")}: ${status.error}` : t("oauthFailed"));
            })
            .catch((error: unknown) => { setContentError(error instanceof Error ? error.message : t("oauthFailed")); });
        }}
        apps={apps}
        appOpened={appOpened}
        appError={appError}
        surface={appSurface}
        onCloseSurface={() => { setAppSurface(null); }}
        onSurfaceRequest={(request) => props.transport.call("app_request", {
          app_id: request.app_id,
          method: request.method,
          path: request.path,
          query: request.query,
          body_b64: request.body_b64,
          request_id: request.request_id,
        })}
        onOpenApp={(item) => {
          if (session === null) return;
          setAppError(null);
          void openApp(session, effectiveUrl, item)
            .then(async (result) => {
              setAppOpened(t("appsOpened", { name: item.name, version: result.version }));
              const tauri = window.__TAURI__;
              if (tauri !== undefined) {
                await tauri.core.invoke("open_app_window", { appId: item.appId, entry: result.entry });
                return;
              }
              const html = await fetchAppHtml(session, effectiveUrl, item, result.entry);
              setAppSurface({ html, nonce: crypto.randomUUID(), appId: item.appId });
            })
            .catch((error: unknown) => { setAppError(error instanceof Error ? error.message : t("appsOpenFailed")); });
        }}
        onToggleConnector={(item) => {
          if (session === null) return;
          setContentError(null);
          void toggleConnector(session, effectiveUrl, item)
            .then(() => fetchConnectorCatalog(session, effectiveUrl))
            .then((catalog) => {
              setConnectors(catalog.items);
              return syncConnectorCatalog(props.transport, catalog);
            })
            .catch((error: unknown) => { setContentError(error instanceof Error ? error.message : t("connectorFailed")); });
        }}
        onReloadConnectors={() => {
          setContentError(null);
          void reloadConnectorSessions(props.transport)
            .catch((error: unknown) => { setContentError(error instanceof Error ? error.message : t("connectorsReloadFailed")); });
        }}
        onCheckUpdate={state.meta.edition === "personal" ? () => {
          setUpdateBusy(true);
          setUpdateText(null);
          void props.transport.call("update.check", {})
            .then((value) => {
              const release = parseUpdateRelease(value);
              setUpdateText(release === null ? t("settingsUpToDate") : t("settingsUpdateAvailable", { version: release.version }));
            })
            .catch(() => { setUpdateText(t("settingsUpdateCheckFailed")); })
            .finally(() => { setUpdateBusy(false); });
        } : undefined}
        onSelectGatewayModel={(model) => {
          setGatewayModel(model);
          saveGatewayModel(model);
          if (session !== null) {
            void props.transport.call("auth.configure", { server_url: effectiveUrl, access_token: session.access_token, model }).catch(() => undefined);
          }
        }}
        serverUrlLocked={CHANNEL_SERVER_URL !== ""}
        onLogout={logout}
        settingsInitialPane={settingsInitialPane}
        panel={panel}
        onPanelChange={setPanel}
        inspectorOpen={inspectorOpen}
        onInspectorOpenChange={setInspectorOpen}
        onToggleInspector={toggleInspector}
      />
    </UiProvider>
  );
}

interface AppShellProps {
  controller: AppController;
  ui: UiValue;
  transport: Transport;
  serverUrl: string;
  gatewayModels: string[];
  gatewayModel: string;
  onSelectGatewayModel: (model: string) => void;
  updateText: string | null;
  updateBusy: boolean;
  onCheckUpdate?: (() => void) | undefined;
  skills: SkillItem[];
  connectors: ConnectorItem[];
  installed: { name: string; version: string }[];
  contentError: string | null;
  contentBusy: string | null;
  onInstall: (item: SkillItem) => void;
  onAuthorize: (item: ConnectorItem) => void;
  onToggleConnector: (item: ConnectorItem) => void;
  onReloadConnectors: () => void;
  authorized: string[];
  apps: AppItem[];
  appOpened: string | null;
  appError: string | null;
  onOpenApp: (item: AppItem) => void;
  surface: SurfaceState | null;
  onCloseSurface: () => void;
  onSurfaceRequest: (request: SurfaceRequestPayload) => Promise<unknown>;
  serverUrlLocked: boolean;
  onLogout: () => void;
  /** Lifted panel/inspector state so the single command catalog can drive them. */
  /** Deep-link pane for the settings dialog (composer model label). */
  settingsInitialPane?: string | undefined;
  panel: "skills" | "connectors" | "apps" | null;
  onPanelChange: (panel: "skills" | "connectors" | "apps" | null) => void;
  inspectorOpen: boolean;
  onInspectorOpenChange: (open: boolean) => void;
  onToggleInspector: () => void;
}

function AppShell(props: AppShellProps) {
  const { ui, controller, transport, gatewayModels, gatewayModel, onSelectGatewayModel, updateText, updateBusy, onCheckUpdate, skills, connectors, installed, contentError, contentBusy, onInstall, onAuthorize, onToggleConnector, onReloadConnectors, authorized, apps, appOpened, appError, onOpenApp, surface, onCloseSurface, onSurfaceRequest, serverUrlLocked, onLogout, settingsInitialPane, panel, onPanelChange, inspectorOpen, onInspectorOpenChange, onToggleInspector } = props;
  const t = ui.t;
  const state = controller.state;
  const enterprise = state.meta.edition === "enterprise";
  useEffect(() => {
    const onKey = (event: KeyboardEvent): void => {
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "b") {
        event.preventDefault();
        onToggleInspector();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => { window.removeEventListener("keydown", onKey); };
  }, [onToggleInspector]);
  useEffect(() => {
    const onKey = (event: KeyboardEvent): void => {
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "k") {
        event.preventDefault();
        controller.setCommandsOpen(!controller.state.commandsOpen);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => { window.removeEventListener("keydown", onKey); };
  }, [controller]);
  const activeWorkspace = ui.workspaces.find((workspace) => workspace.id === ui.activeWorkspaceId);
  const active = ui.conversations.find((conversation) => conversation.id === ui.activeConversationId);
  const title = active === undefined ? t("noConversation") : active.title;
  const packageLine =
    ui.meta.packages.length === 0
      ? t("metaPackagesEmpty")
      : t("metaLine", { version: ui.meta.version, packages: ui.meta.packages.length });
  /** Inspector data callbacks: the app owns IPC + schema validation; the package owns view state. */
  const inspectorApi = useMemo(() => ({
    loadFiles: async (path: string) => {
      const workspaceId = ui.activeWorkspaceId;
      if (workspaceId === null) return [];
      const raw = await transport.call("workspace.files", {
        workspace_id: workspaceId,
        path,
        depth: 1,
        limit: 200,
      });
      const entries = parseFileEntries(raw) ?? [];
      return entries.map((entry) => ({
        path: entry.path,
        kind: entry.kind === "dir" ? ("dir" as const) : ("file" as const),
        depth: entry.depth,
      }));
    },
    loadPreview: async (path: string) => {
      const workspaceId = ui.activeWorkspaceId;
      if (workspaceId === null) throw new Error("workspace/not-found");
      const raw = await transport.call("workspace.file_read", { workspace_id: workspaceId, path });
      const result = parseWorkspaceFileReadResult(raw);
      if (result === null) throw new Error("workspace/file-read-invalid");
      return {
        path: result.path,
        state: "ready" as const,
        kind: result.kind === "image" ? ("image" as const) : result.kind === "text" ? ("text" as const) : ("unsupported" as const),
        text: result.content,
        dataUrl: result.encoding === "base64" && result.content !== "" ? `data:${result.mime};base64,${result.content}` : "",
        size: result.size,
        truncated: result.truncated,
        reason: result.reason ?? "",
        error: "",
      };
    },
    loadChanges: async () => {
      const workspaceId = ui.activeWorkspaceId;
      if (workspaceId === null) return { items: [], warning: null };
      const raw = await transport.call("workspace.changes", { workspace_id: workspaceId });
      const parsed = parseWorkspaceChanges(raw);
      if (parsed === null) throw new Error("workspace/changes-invalid");
      return { items: parsed.items, warning: parsed.warning };
    },
    terminal: async (request: { action: "open" | "send" | "close" | "list"; session?: string; command?: string }) => {
      const workspaceId = ui.activeWorkspaceId;
      const conversationId = ui.activeConversationId;
      if (workspaceId === null || conversationId === null) throw new Error("session/not-found");
      const raw = await transport.call("terminal", {
        workspace_id: workspaceId,
        conversation_id: conversationId,
        ...request,
      });
      const result = parseTerminalResult(raw);
      if (result === null) throw new Error("terminal/invalid");
      return {
        sessions: result.sessions,
        output: result.output,
        truncated: result.truncated,
        running: false,
        error: null,
        unavailable: false,
      };
    },
  }), [transport, ui.activeWorkspaceId, ui.activeConversationId]);
  const handleSurfaceRequest = (request: SurfaceRequestPayload): Promise<SurfaceResponse> => {
    const appId = surface === null ? "" : surface.appId;
    if (appId === "") {
      return Promise.resolve({ status: 500, body_b64: "" });
    }
    return onSurfaceRequest({ ...request, app_id: appId }).then((value) => {
      const status = isRecord(value) ? readNumber(value, "status") : null;
      const body = isRecord(value) ? readString(value, "body_b64") : null;
      return { status: status ?? 500, body_b64: body ?? "" };
    });
  };
  return (
    <>
    <ShellSlot
      t={t}
      boot={ui.boot}
      bootError={ui.bootError}
      workspaceCount={ui.workspaces.length}
      sidebarOpen={ui.sidebarOpen}
      offline={ui.offline}
      streaming={ui.streaming}
      mock={ui.mock}
      title={title}
      packageLine={packageLine}
      workspacePath={activeWorkspace === undefined ? t("noConversation") : activeWorkspace.path}
      toast={state.toast}
      workspaceError={ui.workspaceError}
      agentConfigError={ui.meta.agentConfigError ?? null}
      navigation={<SlotErrorBoundary name="ui.sessions"><SessionsSlot /></SlotErrorBoundary>}
      conversation={<SlotErrorBoundary name="ui.chat"><ChatSlot key={ui.activeWorkspaceId ?? "none"} /></SlotErrorBoundary>}
      settings={(
        <SlotErrorBoundary name="ui.settings">
          <SettingsSlot
            key={state.settingsOpen ? "open" : "closed"}
            panes={SettingsPanes}
            initialPane={settingsInitialPane}
            app={{
              serverUrlLocked,
              onLogout,
              gatewayModels,
              gatewayModel,
              onSelectGatewayModel,
              onCheckUpdate,
              updateText,
              updateBusy,
              testMcpServer: (server) => probeMcpServer(props.transport, server),
            }}
          />
        </SlotErrorBoundary>
      )}
      surface={surface === null ? null : (
        <SlotErrorBoundary name="ui.app.surface">
          <AppSurfaceSlot
            html={surface.html}
            nonce={surface.nonce}
            onClose={onCloseSurface}
            onRequest={handleSurfaceRequest}
          />
        </SlotErrorBoundary>
      )}
      overlay={enterprise ? (
        <>
          <SlotErrorBoundary name="ui.skills"><SkillsSlot open={panel === "skills"} items={skills} installed={installed} busy={contentBusy} error={contentError} onInstall={onInstall} onClose={() => { onPanelChange(null); }} /></SlotErrorBoundary>
          <SlotErrorBoundary name="ui.connectors"><ConnectorsSlot open={panel === "connectors"} items={connectors} error={contentError} authorized={authorized} onAuthorize={onAuthorize} onToggle={onToggleConnector} onReload={onReloadConnectors} onClose={() => { onPanelChange(null); }} /></SlotErrorBoundary>
          <SlotErrorBoundary name="ui.apps"><AppsSlot open={panel === "apps"} items={apps} error={appError} opened={appOpened} onOpenApp={onOpenApp} onClose={() => { onPanelChange(null); }} /></SlotErrorBoundary>
        </>
      ) : null}
      inspector={<SlotErrorBoundary name="ui.inspector"><InspectorSlot open={inspectorOpen} api={inspectorApi} onClose={() => { onInspectorOpenChange(false); }} /></SlotErrorBoundary>}
      inspectorOpen={inspectorOpen}
      onToggleInspector={onToggleInspector}
      onRetryBoot={ui.actions.retryBoot}
      onToggleSidebar={ui.actions.toggleSidebar}
      onDismissToast={controller.dismissToast}
      onOpenCommands={() => { controller.setCommandsOpen(true); }}
      onCreateWorkspace={(path, title) => { ui.actions.createWorkspace(path, title); }}
    />
    <SlotErrorBoundary name="ui.commands"><CommandsSlot key={state.commandsOpen ? "open" : "closed"} open={state.commandsOpen} commands={ui.commands.filter((command) => command.surface !== "slash")} onRun={ui.actions.runCommand} onClose={() => { controller.setCommandsOpen(false); }} /></SlotErrorBoundary>
    </>
  );
}
