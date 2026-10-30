import { useCallback, useEffect, useState } from "react";
import {
  SlotErrorBoundary,
  UiProvider,
  isRecord,
  readNumber,
  readString,
  type AppSettings,
  type BrandConfig,
  type UiValue,
} from "@zeokeel/ui-kit";
import { BRAND } from "./generated/brand";
import { AppSurfaceSlot, AppsSlot, ChatSlot, CommandsSlot, ConnectorsSlot, InspectorSlot, LoginSlot, SessionsSlot, SettingsProviderSlot, SettingsSlot, ShellSlot, SkillsSlot } from "./generated/registry";
import { translate } from "./i18n";
import { UpgradeGate } from "./UpgradeGate";
import { upgradeInfoOf } from "./upgrade";
import { useAppController, type AppController } from "./state/controller";
import { useResolvedTheme } from "./theme";
import type { Transport } from "./types";
import { parseFileEntries, parseWorkspaceChanges, parseUpdateRelease, type FileEntry, type WorkspaceChange } from "./protocol";
import { downloadSkillArchive, fetchAppHtml, fetchApps, fetchConnectors, fetchSkills, fetchTombstones, openApp, sendInstallReceipt, toggleConnector, type AppItem, type ConnectorItem, type SkillItem } from "./store";
import { buildAuthorizeUrl, createPkcePair, exchangeCode, parseOAuthTokens, refreshOAuthToken, savePendingAuth, takePendingAuth, tokenNeedsRefresh } from "./oauth";
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
  };

  useEffect(() => {
    const params = new URLSearchParams(window.location.search);
    const code = params.get("code");
    const stateValue = params.get("state");
    if (code === null || stateValue === null) return;
    const pending = takePendingAuth();
    if (pending?.state !== stateValue) return;
    void exchangeCode(pending.metadata, code, pending.verifier)
      .then(async (tokens) => {
        await props.transport.call("oauth.save", { connector_id: pending.connectorId, tokens }).catch((error: unknown) => { console.warn("[zk] oauth-save-failed", String(error)); });
        setAuthorized((values) => values.includes(pending.connectorId) ? values : [...values, pending.connectorId]);
        window.history.replaceState({}, "", window.location.pathname);
      })
      .catch(() => { setContentError(t("oauthFailed")); });
  }, [props.transport, t]);

  // Enterprise session: configure gateway, load catalog, refresh token (docs/19 §3.2, docs/38).
  useEffect(() => {
    if (state.meta.edition !== "enterprise" || session === null) return;
    const url = activeServerUrl;
    if (url === "") return;
    void props.transport.call("auth.configure", { server_url: url, access_token: session.access_token, model: loadGatewayModel() }).catch(() => undefined);
    void Promise.all([fetchSkills(session, url), fetchConnectors(session, url), props.transport.call("skills.installed", {}), fetchTombstones(session, url), fetchApps(session, url)])
      .then(async ([skillItems, connectorItems, installedValue, tombstones, appItems]) => {
        setSkills(skillItems);
        setConnectors(connectorItems);
        setApps(appItems);
        const oauthIds: string[] = [];
        for (const connector of connectorItems) {
          if (connector.oauth === null) continue;
          const stored = await props.transport.call("oauth.load", { connector_id: connector.id }).catch(() => null);
          const raw = isRecord(stored) ? stored["tokens"] : null;
          let tokens = parseOAuthTokens(raw);
          if (tokens === null) continue;
          if (tokenNeedsRefresh(tokens)) {
            tokens = await refreshOAuthToken({ authorizeUrl: connector.oauth.authorizeUrl, tokenUrl: connector.oauth.tokenUrl, clientId: connector.oauth.clientId, redirectUri: connector.oauth.redirectUri, scopes: connector.oauth.scopes }, tokens).catch(() => null);
            if (tokens !== null) await props.transport.call("oauth.save", { connector_id: connector.id, tokens }).catch(() => undefined);
          }
          if (tokens !== null) oauthIds.push(connector.id);
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
          void props.transport.call("auth.configure", { server_url: url, access_token: next.access_token, model: loadGatewayModel() }).catch(() => undefined);
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
  }, [state.meta.edition, session, activeServerUrl, props.transport, t, reportUpgrade]);

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
      openSettings: () => {
        controller.setSettingsOpen(true);
      },
      closeSettings: () => {
        controller.setSettingsOpen(false);
      },
      saveSettings: (next: AppSettings): Promise<void> => {
        const requiresRelogin =
          session !== null &&
          normalizeUrl(next.server_url) !== "" &&
          normalizeUrl(next.server_url) !== normalizeUrl(state.settings.server_url) &&
          normalizeUrl(next.server_url) !== (sessionServerUrl ?? "");
        return controller.saveSettings(next).then(() => {
          if (requiresRelogin) {
            clearSession();
            setSession(null);
            setSessionServerUrl(null);
            setGatewayModels([]);
            setLoginError(t("settingsReloginRequired"));
          }
        });
      },
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
          if (item.oauth === null) return;
          void createPkcePair().then((pair) => {
            const stateValue = crypto.randomUUID();
            const metadata = { authorizeUrl: item.oauth?.authorizeUrl ?? "", tokenUrl: item.oauth?.tokenUrl ?? "", clientId: item.oauth?.clientId ?? "", redirectUri: item.oauth?.redirectUri ?? "", scopes: item.oauth?.scopes ?? [] };
            if (metadata.authorizeUrl === "" || metadata.tokenUrl === "") { setContentError(t("oauthNotConfigured")); return; }
            savePendingAuth({ connectorId: item.id, verifier: pair.verifier, state: stateValue, metadata });
            window.location.assign(buildAuthorizeUrl(metadata, pair.challenge, stateValue));
          }).catch(() => { setContentError(t("oauthFailed")); });
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
          void toggleConnector(session, effectiveUrl, item)
            .then(() => fetchConnectors(session, effectiveUrl))
            .then(setConnectors)
            .catch((error: unknown) => { setContentError(error instanceof Error ? error.message : t("connectorFailed")); });
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
        showServerUrl={state.meta.edition === "enterprise"}
        serverUrlLocked={CHANNEL_SERVER_URL !== ""}
        onLogout={logout}
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
  authorized: string[];
  apps: AppItem[];
  appOpened: string | null;
  appError: string | null;
  onOpenApp: (item: AppItem) => void;
  surface: SurfaceState | null;
  onCloseSurface: () => void;
  onSurfaceRequest: (request: SurfaceRequestPayload) => Promise<unknown>;
  showServerUrl: boolean;
  serverUrlLocked: boolean;
  onLogout: () => void;
}

function AppShell(props: AppShellProps) {
  const { ui, controller, transport, gatewayModels, gatewayModel, onSelectGatewayModel, updateText, updateBusy, onCheckUpdate, skills, connectors, installed, contentError, contentBusy, onInstall, onAuthorize, onToggleConnector, authorized, apps, appOpened, appError, onOpenApp, surface, onCloseSurface, onSurfaceRequest, showServerUrl, serverUrlLocked, onLogout } = props;
  const t = ui.t;
  const state = controller.state;
  const enterprise = state.meta.edition === "enterprise";
  const [panel, setPanel] = useState<"skills" | "connectors" | "apps" | null>(null);
  const [inspectorOpen, setInspectorOpen] = useState(false);
  const [inspectorTab, setInspectorTab] = useState<"files" | "changes" | "terminal">("files");
  const [files, setFiles] = useState<FileEntry[]>([]);
  const [changes, setChanges] = useState<WorkspaceChange[]>([]);
  const [changesState, setChangesState] = useState<"idle" | "loading" | "error">("idle");
  const [changesError, setChangesError] = useState<string | null>(null);
  const [changesWarning, setChangesWarning] = useState<string | null>(null);
  const [changesAttempt, setChangesAttempt] = useState(0);
  useEffect(() => {
    if (!inspectorOpen || inspectorTab !== "changes" || ui.activeWorkspaceId === null) {
      return;
    }
    let active = true;
    // Defer the loading transition out of the render commit (react-hooks/set-state-in-effect).
    void Promise.resolve().then(() => {
      if (!active) return undefined;
      setChangesState("loading");
      setChangesError(null);
      return transport.call("workspace.changes", { workspace_id: ui.activeWorkspaceId });
    })
      .then((value) => {
        if (!active) return;
        const parsed = parseWorkspaceChanges(value);
        if (parsed === null) {
          setChanges([]);
          setChangesWarning(null);
          setChangesState("error");
          setChangesError(t("inspectorChangesError"));
          return;
        }
        setChanges(parsed.items);
        setChangesWarning(parsed.warning);
        setChangesState("idle");
      })
      .catch((error: unknown) => {
        if (!active) return;
        setChanges([]);
        setChangesWarning(null);
        setChangesState("error");
        setChangesError(error instanceof Error ? error.message : t("inspectorChangesError"));
      });
    return () => { active = false; };
  }, [inspectorOpen, inspectorTab, ui.activeWorkspaceId, transport, changesAttempt, t]);
  useEffect(() => {
    if (!inspectorOpen || ui.activeWorkspaceId === null) { return; }
    let cancelled = false;
    const isCancelled = (): boolean => cancelled;
    void transport.call("workspace.files", { workspace_id: ui.activeWorkspaceId })
      .then((value) => { if (!isCancelled()) setFiles(parseFileEntries(value) ?? []); })
      .catch(() => { if (!isCancelled()) setFiles([]); });
    return () => { cancelled = true; };
  }, [inspectorOpen, ui.activeWorkspaceId, transport]);
  useEffect(() => {
    const onKey = (event: KeyboardEvent): void => {
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "b") {
        event.preventDefault();
        setInspectorOpen((value) => !value);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => { window.removeEventListener("keydown", onKey); };
  }, []);
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
  const commands = [
    { id: "new-conversation", title: t("newConversation"), group: t("workspaceActions") },
    { id: "toggle-sidebar", title: t("sidebarToggle"), group: t("workspaceActions") },
    { id: "open-settings", title: t("settingsTitle"), group: t("settings") },
    { id: "toggle-inspector", title: t("inspectorToggle"), group: t("workspaceActions") },
    ...(enterprise
      ? [
          { id: "open-skills", title: t("panelSkills"), group: t("workspaceActions") },
          { id: "open-connectors", title: t("panelConnectors"), group: t("workspaceActions") },
          { id: "open-apps", title: t("panelApps"), group: t("workspaceActions") },
        ]
      : []),
  ];
  const runCommand = (id: string): void => {
    if (id === "new-conversation") ui.actions.createConversation();
    if (id === "toggle-sidebar") ui.actions.toggleSidebar();
    if (id === "open-settings") ui.actions.openSettings();
    if (id === "toggle-inspector") setInspectorOpen((value) => !value);
    if (id === "open-skills") setPanel("skills");
    if (id === "open-connectors") setPanel("connectors");
    if (id === "open-apps") setPanel("apps");
  };
  const activeWorkspace = ui.workspaces.find((workspace) => workspace.id === ui.activeWorkspaceId);
  const active = ui.conversations.find((conversation) => conversation.id === ui.activeConversationId);
  const title = active === undefined ? t("noConversation") : active.title;
  const packageLine =
    ui.meta.packages.length === 0
      ? t("metaPackagesEmpty")
      : t("metaLine", { version: ui.meta.version, packages: ui.meta.packages.length });
  const visibleFiles = inspectorOpen && ui.activeWorkspaceId !== null ? files : [];
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
            gatewayModels={gatewayModels}
            gatewayModel={gatewayModel}
            onSelectGatewayModel={onSelectGatewayModel}
            onCheckUpdate={onCheckUpdate}
            updateText={updateText}
            updateBusy={updateBusy}
            renderProvider={SettingsProviderSlot}
            showServerUrl={showServerUrl}
            serverUrlLocked={serverUrlLocked}
            onLogout={onLogout}
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
          <SlotErrorBoundary name="ui.skills"><SkillsSlot open={panel === "skills"} items={skills} installed={installed} busy={contentBusy} error={contentError} onInstall={onInstall} onClose={() => { setPanel(null); }} /></SlotErrorBoundary>
          <SlotErrorBoundary name="ui.connectors"><ConnectorsSlot open={panel === "connectors"} items={connectors} error={contentError} authorized={authorized} onAuthorize={onAuthorize} onToggle={onToggleConnector} onClose={() => { setPanel(null); }} /></SlotErrorBoundary>
          <SlotErrorBoundary name="ui.apps"><AppsSlot open={panel === "apps"} items={apps} error={appError} opened={appOpened} onOpenApp={onOpenApp} onClose={() => { setPanel(null); }} /></SlotErrorBoundary>
        </>
      ) : null}
      inspector={<SlotErrorBoundary name="ui.inspector"><InspectorSlot open={inspectorOpen} tab={inspectorTab} files={visibleFiles} changes={changes} changesState={changesState} changesError={changesError} changesWarning={changesWarning} onRetryChanges={() => { setChangesAttempt((value) => value + 1); }} onTab={setInspectorTab} onClose={() => { setInspectorOpen(false); }} /></SlotErrorBoundary>}
      inspectorOpen={inspectorOpen}
      onToggleInspector={() => { setInspectorOpen((value) => !value); }}
      onRetryBoot={ui.actions.retryBoot}
      onToggleSidebar={ui.actions.toggleSidebar}
      onDismissToast={controller.dismissToast}
      onOpenCommands={() => { controller.setCommandsOpen(true); }}
      onCreateWorkspace={(path, title) => { ui.actions.createWorkspace(path, title); }}
    />
    <SlotErrorBoundary name="ui.commands"><CommandsSlot key={state.commandsOpen ? "open" : "closed"} open={state.commandsOpen} commands={commands} onRun={runCommand} onClose={() => { controller.setCommandsOpen(false); }} /></SlotErrorBoundary>
    </>
  );
}
