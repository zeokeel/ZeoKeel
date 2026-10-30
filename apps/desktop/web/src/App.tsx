import { useEffect, useState } from "react";
import {
  UiProvider,
  type BrandConfig,
  type UiValue,
} from "@zeokeel/ui-kit";
import { BRAND } from "./generated/brand";
import { ChatSlot, CommandsSlot, ConnectorsSlot, InspectorSlot, LoginSlot, SessionsSlot, SettingsProviderSlot, SettingsSlot, ShellSlot, SkillsSlot } from "./generated/registry";
import { translate } from "./i18n";
import { useAppController, type AppController } from "./state/controller";
import { useResolvedTheme } from "./theme";
import type { FileEntry, Transport } from "./types";
import { parseFileEntries, parseUpdateRelease } from "./protocol";
import { downloadSkillArchive, fetchConnectors, fetchSkills, fetchTombstones, sendInstallReceipt, toggleConnector, type ConnectorItem, type SkillItem } from "./store";
import { buildAuthorizeUrl, createPkcePair, exchangeCode, loadConnectorTokens, refreshOAuthToken, saveConnectorTokens, savePendingAuth, takePendingAuth, tokenNeedsRefresh } from "./oauth";
import { clearSession, fetchBootstrapInfo, loadGatewayModel, loadServerUrl, loadSession, login as loginRequest, refreshSession, saveGatewayModel, saveServerUrl, saveSession, type AuthSession } from "./auth";


function isMockMode(): boolean {
  return new URLSearchParams(window.location.search).get("mock") === "1";
}

export function App(props: { transport: Transport }) {
  const controller = useAppController(props.transport);
  const [session, setSession] = useState<AuthSession | null>(loadSession);
  const [serverUrl, setServerUrl] = useState<string>(loadServerUrl);
  const [loginError, setLoginError] = useState<string | null>(null);
  const [loginLoading, setLoginLoading] = useState(false);
  const [gatewayModels, setGatewayModels] = useState<string[]>([]);
  const [gatewayModel, setGatewayModel] = useState<string>(loadGatewayModel);
  const [updateText, setUpdateText] = useState<string | null>(null);
  const [updateBusy, setUpdateBusy] = useState(false);
  const [skills, setSkills] = useState<SkillItem[]>([]);
  const [connectors, setConnectors] = useState<ConnectorItem[]>([]);
  const [installed, setInstalled] = useState<Array<{ name: string; version: string }>>([]);
  const [contentError, setContentError] = useState<string | null>(null);
  const [contentBusy, setContentBusy] = useState<string | null>(null);
  const [authorized, setAuthorized] = useState<string[]>([]);
  const { state } = controller;
  useEffect(() => {
    const params = new URLSearchParams(window.location.search);
    const code = params.get("code");
    const stateValue = params.get("state");
    if (code === null || stateValue === null) return;
    const pending = takePendingAuth();
    if (pending === null || pending.state !== stateValue) return;
    void exchangeCode(pending.metadata, code, pending.verifier)
      .then((tokens) => {
        saveConnectorTokens(pending.connectorId, tokens);
        setAuthorized((values) => values.includes(pending.connectorId) ? values : [...values, pending.connectorId]);
        window.history.replaceState({}, "", window.location.pathname);
      })
      .catch(() => { setContentError("OAUTH_FAILED"); });
  }, []);
  useEffect(() => {
    if (state.meta.edition !== "enterprise" || session === null) return;
    const url = serverUrl === "" ? state.settings.server_url : serverUrl;
    if (url === "") return;
    void props.transport.call("auth.configure", { server_url: url, access_token: session.access_token, model: loadGatewayModel() }).catch(() => {});
    void Promise.all([fetchSkills(session, url), fetchConnectors(session, url), props.transport.call("skills.installed", {}), fetchTombstones(session, url)])
      .then(async ([skillItems, connectorItems, installedValue, tombstones]) => {
        setSkills(skillItems);
        setConnectors(connectorItems);
        const oauthIds: string[] = [];
        for (const connector of connectorItems) {
          if (connector.oauth === null) continue;
          let tokens = loadConnectorTokens(connector.id);
          if (tokens === null) continue;
          if (tokenNeedsRefresh(tokens)) {
            tokens = await refreshOAuthToken({ authorizeUrl: connector.oauth.authorizeUrl, tokenUrl: connector.oauth.tokenUrl, clientId: connector.oauth.clientId, redirectUri: connector.oauth.redirectUri, scopes: connector.oauth.scopes }, tokens).catch(() => null);
            if (tokens !== null) saveConnectorTokens(connector.id, tokens);
          }
          if (tokens !== null) oauthIds.push(connector.id);
        }
        setAuthorized(oauthIds);
        const rows = typeof installedValue === "object" && installedValue !== null ? (installedValue as { items?: unknown }).items : [];
        const local = Array.isArray(rows) ? rows.flatMap((row) => {
          if (typeof row !== "object" || row === null) return [];
          const name = (row as { name?: unknown }).name;
          const version = (row as { version?: unknown }).version;
          return typeof name === "string" && typeof version === "string" ? [{ name, version }] : [];
        }) : [];
        for (const entry of local) {
          if (tombstones.includes(entry.name)) {
            await props.transport.call("skills.remove", { name: entry.name }).catch(() => undefined);
          }
        }
        setInstalled(local.filter((entry) => !tombstones.includes(entry.name)));
      })
      .catch((error: unknown) => { setContentError(error instanceof Error ? error.message : "catalog failed"); });
    const refreshMs = Math.max(session.expires_in, 60) * 800;
    const timer = window.setTimeout(() => {
      void refreshSession(session, url)
        .then((next) => {
          saveSession(next);
          setSession(next);
          void props.transport.call("auth.configure", { server_url: url, access_token: next.access_token, model: loadGatewayModel() }).catch(() => {});
        })
        .catch(() => { clearSession(); setSession(null); });
    }, refreshMs);
    return () => { window.clearTimeout(timer); };
  }, [state.meta.edition, session, serverUrl, state.settings.server_url, props.transport]);
  const theme = useResolvedTheme(state.settings.theme);
  const brand: BrandConfig = BRAND;

  const t = (key: string, vars?: Record<string, string | number>): string =>
    translate(state.settings.locale, key, vars);

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
      saveSettings: controller.saveSettings,
      toggleSidebar: controller.toggleSidebar,
      retryBoot: controller.retryBoot,
      retryMessages: controller.retryMessages,
      decideApproval: controller.decideApproval,
    },
  };

  if (state.meta.edition === "enterprise" && state.boot === "ready" && session === null && LoginSlot !== null) {
    const submit = (input: { serverUrl?: string; username: string; password: string; deviceName: string }): void => {
      const serverUrl = input.serverUrl ?? state.settings.server_url;
      setLoginLoading(true);
      setLoginError(null);
      void loginRequest(serverUrl, input.username, input.password, input.deviceName)
        .then((next) => {
          saveSession(next);
          saveServerUrl(serverUrl);
          setServerUrl(serverUrl);
          void fetchBootstrapInfo(next, serverUrl)
            .then((info) => {
              setGatewayModels(info.models);
              const model = info.defaultModel === "" ? loadGatewayModel() : info.defaultModel;
              setGatewayModel(model);
              if (model !== "") saveGatewayModel(model);
              return props.transport.call("auth.configure", { server_url: serverUrl, access_token: next.access_token, model });
            })
            .catch(() => {});
          setSession(next);
        })
        .catch((error: unknown) => { setLoginError(error instanceof Error ? error.message : "login failed"); })
        .finally(() => { setLoginLoading(false); });
    };
    return (
      <UiProvider value={ui}>
        <LoginSlot serverUrl={state.settings.server_url.trim() === "" ? null : state.settings.server_url} channelLocked={false} loading={loginLoading} error={loginError} onSubmit={submit} />
      </UiProvider>
    );
  }

  return (
    <UiProvider value={ui}>
      <AppShell
        controller={controller}
        ui={ui}
        transport={props.transport}
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
          const url = serverUrl === "" ? state.settings.server_url : serverUrl;
          setContentBusy(item.name);
          setContentError(null);
          void downloadSkillArchive(session, url, item)
            .then((archive) => props.transport.call("skills.install", { name: item.name, version: item.version, archive_base64: archive.archive_base64 }))
            .then((result) => {
              const tree = typeof result === "object" && result !== null && typeof (result as { tree_sha256?: unknown }).tree_sha256 === "string"
                ? (result as { tree_sha256: string }).tree_sha256
                : undefined;
              if (typeof tree !== "string") throw new Error("missing tree_sha256");
              return sendInstallReceipt(session, url, item, tree);
            })
            .then(() => { setInstalled((values) => [...values.filter((entry) => entry.name !== item.name), { name: item.name, version: item.version }]); })
            .catch((error: unknown) => { setContentError(error instanceof Error ? error.message : "install failed"); })
            .finally(() => { setContentBusy(null); });
        }}
        authorized={authorized}
        onAuthorize={(item) => {
          if (item.oauth === null || item.oauth === undefined) return;
          void createPkcePair().then((pair) => {
            const stateValue = crypto.randomUUID();
            const metadata = { authorizeUrl: item.oauth?.authorizeUrl ?? "", tokenUrl: item.oauth?.tokenUrl ?? "", clientId: item.oauth?.clientId ?? "", redirectUri: item.oauth?.redirectUri ?? "", scopes: item.oauth?.scopes ?? [] };
            if (metadata.authorizeUrl === "" || metadata.tokenUrl === "") { setContentError("OAUTH_NOT_CONFIGURED"); return; }
            savePendingAuth({ connectorId: item.id, verifier: pair.verifier, state: stateValue, metadata });
            window.location.assign(buildAuthorizeUrl(metadata, pair.challenge, stateValue));
          }).catch(() => { setContentError("OAUTH_FAILED"); });
        }}
        onToggleConnector={(item) => {
          if (session === null) return;
          const url = serverUrl === "" ? state.settings.server_url : serverUrl;
          void toggleConnector(session, url, item)
            .then(() => fetchConnectors(session, url))
            .then(setConnectors)
            .catch((error: unknown) => { setContentError(error instanceof Error ? error.message : "connector failed"); });
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
            const url = serverUrl === "" ? state.settings.server_url : serverUrl;
            void props.transport.call("auth.configure", { server_url: url, access_token: session.access_token, model }).catch(() => {});
          }
        }}
      />
    </UiProvider>
  );
}

function AppShell(props: { controller: AppController; ui: UiValue; transport: Transport; gatewayModels: string[]; gatewayModel: string; onSelectGatewayModel: (model: string) => void; updateText: string | null; updateBusy: boolean; onCheckUpdate?: (() => void) | undefined;
  skills: SkillItem[]; connectors: ConnectorItem[]; installed: Array<{ name: string; version: string }>; contentError: string | null; contentBusy: string | null;
  onInstall: (item: SkillItem) => void; onAuthorize: (item: ConnectorItem) => void; onToggleConnector: (item: ConnectorItem) => void; authorized: string[] }) {
  const { ui, controller, transport, gatewayModels, gatewayModel, onSelectGatewayModel, updateText, updateBusy, onCheckUpdate, skills, connectors, installed, contentError, contentBusy, onInstall, onAuthorize, onToggleConnector, authorized } = props;
  const t = ui.t;
  const state = controller.state;
  const [commandsOpen, setCommandsOpen] = useState(false);
  const [panel, setPanel] = useState<"skills" | "connectors" | null>(null);
  const [inspectorOpen, setInspectorOpen] = useState(false);
  const [inspectorTab, setInspectorTab] = useState<"files" | "changes" | "terminal">("files");
  const [files, setFiles] = useState<FileEntry[]>([]);
  useEffect(() => {
    if (!inspectorOpen || ui.activeWorkspaceId === null) { setFiles([]); return; }
    let cancelled = false;
    void transport.call("workspace.files", { workspace_id: ui.activeWorkspaceId })
      .then((value) => { if (!cancelled) setFiles(parseFileEntries(value) ?? []); })
      .catch(() => { if (!cancelled) setFiles([]); });
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
        setCommandsOpen((value) => !value);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => { window.removeEventListener("keydown", onKey); };
  }, []);
  const commands = [
    { id: "new-conversation", title: t("newConversation"), group: t("workspaceActions") },
    { id: "toggle-sidebar", title: t("sidebarToggle"), group: t("workspaceActions") },
    { id: "open-settings", title: t("settingsTitle"), group: t("settings") },
    { id: "toggle-inspector", title: t("inspectorToggle"), group: t("workspaceActions") },
    { id: "open-skills", title: "技能", group: t("workspaceActions") },
    { id: "open-connectors", title: "连接器", group: t("workspaceActions") },
  ];
  const runCommand = (id: string): void => {
    if (id === "new-conversation") ui.actions.createConversation();
    if (id === "toggle-sidebar") ui.actions.toggleSidebar();
    if (id === "open-settings") ui.actions.openSettings();
    if (id === "toggle-inspector") setInspectorOpen((value) => !value);
    if (id === "open-skills") setPanel("skills");
    if (id === "open-connectors") setPanel("connectors");
  };
  const activeWorkspace = ui.workspaces.find((workspace) => workspace.id === ui.activeWorkspaceId);
  const active = ui.conversations.find((conversation) => conversation.id === ui.activeConversationId);
  const title = active === undefined ? t("noConversation") : active.title;
  const packageLine =
    ui.meta.packages.length === 0
      ? t("metaPackagesEmpty")
      : t("metaLine", { version: ui.meta.version, packages: ui.meta.packages.length });
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
      navigation={<SessionsSlot />}
      conversation={<ChatSlot key={ui.activeWorkspaceId === null ? "none" : ui.activeWorkspaceId} />}
      settings={<SettingsSlot gatewayModels={gatewayModels} gatewayModel={gatewayModel} onSelectGatewayModel={onSelectGatewayModel} onCheckUpdate={onCheckUpdate} updateText={updateText} updateBusy={updateBusy} renderProvider={SettingsProviderSlot} />}
      overlay={<>
        <SkillsSlot open={panel === "skills"} items={skills} installed={installed} busy={contentBusy} error={contentError} onInstall={onInstall} onClose={() => { setPanel(null); }} />
        <ConnectorsSlot open={panel === "connectors"} items={connectors} error={contentError} authorized={authorized} onAuthorize={onAuthorize} onToggle={onToggleConnector} onClose={() => { setPanel(null); }} />
      </>}
      inspector={<InspectorSlot open={inspectorOpen} tab={inspectorTab} files={files} changes={[]} onTab={setInspectorTab} onClose={() => { setInspectorOpen(false); }} />}
      inspectorOpen={inspectorOpen}
      onToggleInspector={() => { setInspectorOpen((value) => !value); }}
      onRetryBoot={ui.actions.retryBoot}
      onToggleSidebar={ui.actions.toggleSidebar}
      onDismissToast={controller.dismissToast}
      onOpenCommands={() => { setCommandsOpen(true); }}
      onCreateWorkspace={(path, title) => { ui.actions.createWorkspace(path, title); }}
    />
    <CommandsSlot open={commandsOpen} commands={commands} onRun={runCommand} onClose={() => { setCommandsOpen(false); }} />
    </>
  );
}
