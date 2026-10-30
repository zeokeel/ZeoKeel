import { useState, type FormEvent } from "react";
import {
  BrandLogo,
  Button,
  IconFolder,
  UiProvider,
  type BrandConfig,
  type UiValue,
} from "@zeokeel/ui-kit";
import { BRAND } from "./generated/brand";
import { ChatSlot, LoginSlot, SessionsSlot, SettingsSlot, ShellSlot } from "./generated/registry";
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

function FirstRun(props: { ui: UiValue }) {
  const { t, workspaceError, actions } = props.ui;
  const [path, setPath] = useState("");
  const [title, setTitle] = useState("");
  const [showPicker, setShowPicker] = useState(false);

  const submit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    actions.createWorkspace(path, title);
  };

  return (
    <div className="flex h-dvh flex-col items-center justify-center bg-background px-6 text-center">
      <BrandLogo className="h-10 w-10 rounded-document" />
      <h1 className="mt-5 text-[20px] font-semibold tracking-[-0.01em] text-foreground">
        {t("workspaceFirstTitle")}
      </h1>
      <p className="mt-2 max-w-[460px] text-[13px] text-muted-foreground">
        {t("workspaceFirstBody")}
      </p>
      {workspaceError !== null ? (
        <p className="mt-3 max-w-[460px] rounded-document bg-muted px-3 py-1.5 text-[12px] text-destructive">
          {workspaceError === "workspace/path-required" ? t("workspacePathInvalid") : workspaceError}
        </p>
      ) : null}
      <div className="mt-5 flex flex-wrap items-center justify-center gap-2">
        <Button
          variant="primary"
          onClick={() => {
            actions.createWorkspace("~", "主目录");
          }}
          title={t("workspaceUseHome")}
        >
          <IconFolder />
          <span>{t("workspaceUseHome")}</span>
        </Button>
        <Button
          variant="default"
          aria-expanded={showPicker}
          onClick={() => {
            setShowPicker((value) => !value);
          }}
        >
          <IconFolder />
          <span>{t("workspaceAdd")}</span>
        </Button>
      </div>
      {showPicker ? (
        <form
          className="mt-4 flex w-full max-w-[460px] flex-col gap-2 rounded-document bg-muted p-3 text-start"
          onSubmit={submit}
        >
          <label className="flex flex-col gap-1 text-[12px] text-muted-foreground">
            {t("workspacePathLabel")}
            <input
              autoFocus
              value={path}
              onChange={(event) => {
                setPath(event.target.value);
              }}
              placeholder={t("workspaceAddHint")}
              className="w-full rounded-control border border-border bg-background px-2 py-1.5 text-[13px] text-foreground placeholder:text-muted-foreground"
            />
          </label>
          <label className="flex flex-col gap-1 text-[12px] text-muted-foreground">
            {t("workspaceTitleLabel")}
            <input
              value={title}
              onChange={(event) => {
                setTitle(event.target.value);
              }}
              className="w-full rounded-control border border-border bg-background px-2 py-1.5 text-[13px] text-foreground"
            />
          </label>
          <div className="mt-1 flex justify-end">
            <Button variant="primary" size="sm" type="submit" disabled={path.trim() === ""}>
              {t("workspaceCreate")}
            </Button>
          </div>
        </form>
      ) : null}
    </div>
  );
}

function AppShell(props: { controller: AppController; ui: UiValue }) {
  const { ui, controller } = props;
  const t = ui.t;
  const state = controller.state;
  const activeWorkspace = ui.workspaces.find((workspace) => workspace.id === ui.activeWorkspaceId);
  const active = ui.conversations.find((conversation) => conversation.id === ui.activeConversationId);
  const title = active === undefined ? t("noConversation") : active.title;
  const packageLine =
    ui.meta.packages.length === 0
      ? t("metaPackagesEmpty")
      : t("metaLine", { version: ui.meta.version, packages: ui.meta.packages.length });
  return (
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
      firstRun={<FirstRun ui={ui} />}
      navigation={<SessionsSlot />}
      conversation={<ChatSlot key={ui.activeWorkspaceId === null ? "none" : ui.activeWorkspaceId} />}
      settings={<SettingsSlot />}
      onRetryBoot={ui.actions.retryBoot}
      onToggleSidebar={ui.actions.toggleSidebar}
      onDismissToast={controller.dismissToast}
    />
  );
}
