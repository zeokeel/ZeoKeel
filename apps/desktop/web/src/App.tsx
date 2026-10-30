import { useEffect, useState } from "react";
import {
  UiProvider,
  type BrandConfig,
  type UiValue,
} from "@zeokeel/ui-kit";
import { BRAND } from "./generated/brand";
import { ChatSlot, CommandsSlot, LoginSlot, SessionsSlot, SettingsSlot, ShellSlot } from "./generated/registry";
import { translate } from "./i18n";
import { useAppController, type AppController } from "./state/controller";
import { useResolvedTheme } from "./theme";
import type { Transport } from "./types";
import { loadSession, login as loginRequest, saveSession, type AuthSession } from "./auth";


function isMockMode(): boolean {
  return new URLSearchParams(window.location.search).get("mock") === "1";
}

export function App(props: { transport: Transport }) {
  const controller = useAppController(props.transport);
  const [session, setSession] = useState<AuthSession | null>(loadSession);
  const [loginError, setLoginError] = useState<string | null>(null);
  const [loginLoading, setLoginLoading] = useState(false);
  const { state } = controller;
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
        .then((next) => { saveSession(next); setSession(next); })
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
      <AppShell controller={controller} ui={ui} />
    </UiProvider>
  );
}

function AppShell(props: { controller: AppController; ui: UiValue }) {
  const { ui, controller } = props;
  const t = ui.t;
  const state = controller.state;
  const [commandsOpen, setCommandsOpen] = useState(false);
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
  ];
  const runCommand = (id: string): void => {
    if (id === "new-conversation") ui.actions.createConversation();
    if (id === "toggle-sidebar") ui.actions.toggleSidebar();
    if (id === "open-settings") ui.actions.openSettings();
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
      settings={<SettingsSlot />}
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
