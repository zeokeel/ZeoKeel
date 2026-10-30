import * as Tooltip from "@radix-ui/react-tooltip";
import { useState, type FormEvent } from "react";
import {
  BrandLogo,
  Button,
  Chip,
  IconClose,
  IconFolder,
  IconMenu,
  IconRetry,
  IconWifiOff,
  StatusDot,
  UiProvider,
  type BrandConfig,
  type UiValue,
} from "@zeokeel/ui-kit";
import { BRAND } from "./generated/brand";
import { ChatSlot, SessionsSlot, SettingsSlot } from "./generated/registry";
import { translate } from "./i18n";
import { useAppController, type AppController } from "./state/controller";
import { useResolvedTheme } from "./theme";
import type { Transport } from "./types";

const tooltipClass = "z-50 rounded-surface bg-foreground px-2 py-1 text-[12px] text-background";
const iconButtonClass =
  "inline-flex h-7 w-7 items-center justify-center rounded-sm text-muted-foreground transition-colors hover:bg-foreground/[0.05] hover:text-foreground";

function isMockMode(): boolean {
  return new URLSearchParams(window.location.search).get("mock") === "1";
}

export function App(props: { transport: Transport }) {
  const controller = useAppController(props.transport);
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

  if (ui.boot === "loading") {
    return (
      <div
        className="flex h-dvh flex-col items-center justify-center gap-4 bg-background text-center"
        role="status"
      >
        <BrandLogo className="h-10 w-10 rounded-document" />
        <span className="zk-spinner" aria-hidden="true" />
        <p className="text-[13px] text-muted-foreground">{t("bootLoading")}</p>
      </div>
    );
  }

  if (ui.boot === "error") {
    return (
      <div className="flex h-dvh flex-col items-center justify-center gap-3 bg-background px-6 text-center">
        <BrandLogo className="h-10 w-10 rounded-document" />
        <h1 className="text-[20px] font-semibold tracking-[-0.01em] text-foreground">
          {t("bootErrorTitle")}
        </h1>
        <p className="max-w-[480px] text-[13px] text-muted-foreground">{t("bootErrorHint")}</p>
        {ui.bootError !== null ? (
          <p className="max-w-[480px] rounded-document bg-muted px-3 py-2 font-mono text-[12px] text-destructive">
            {ui.bootError}
          </p>
        ) : null}
        <Button variant="primary" className="mt-1" onClick={ui.actions.retryBoot}>
          <IconRetry />
          <span>{t("retry")}</span>
        </Button>
      </div>
    );
  }

  if (ui.workspaces.length === 0) {
    return <FirstRun ui={ui} />;
  }

  const activeWorkspace = ui.workspaces.find(
    (workspace) => workspace.id === ui.activeWorkspaceId,
  );
  const active = ui.conversations.find(
    (conversation) => conversation.id === ui.activeConversationId,
  );
  const title = active === undefined ? t("noConversation") : active.title;
  const packageLine =
    ui.meta.packages.length === 0
      ? t("metaPackagesEmpty")
      : t("metaLine", { version: ui.meta.version, packages: ui.meta.packages.length });

  return (
    <Tooltip.Provider delayDuration={300}>
      <a className="zk-skip-link" href="#zk-composer-input">
        {t("skipToInput")}
      </a>
      <div className="flex h-dvh flex-col overflow-hidden bg-background text-foreground">
        {ui.offline ? (
          <div
            className="flex items-center justify-center gap-2 border-b border-border bg-muted px-4 py-1 text-[11px] text-destructive"
            role="status"
          >
            <IconWifiOff />
            <span>{t("offlineBanner")}</span>
          </div>
        ) : null}

        <div className="flex min-h-0 flex-1">
          <SessionsSlot />
          {ui.sidebarOpen ? (
            <button
              type="button"
              className="zk-sidebar-backdrop"
              aria-label={t("sidebarClose")}
              onClick={ui.actions.toggleSidebar}
            />
          ) : null}

          <main className="flex min-w-0 flex-1 flex-col">
            <header className="flex min-h-[44px] items-center gap-2 border-b border-border bg-background px-3 py-1.5">
              <Tooltip.Root>
                <Tooltip.Trigger asChild>
                  <button
                    type="button"
                    className={`${iconButtonClass} zk-sidebar-toggle`}
                    aria-label={t("sidebarToggle")}
                    onClick={ui.actions.toggleSidebar}
                  >
                    <IconMenu />
                  </button>
                </Tooltip.Trigger>
                <Tooltip.Portal>
                  <Tooltip.Content className={tooltipClass} sideOffset={6}>
                    {t("sidebarToggle")}
                  </Tooltip.Content>
                </Tooltip.Portal>
              </Tooltip.Root>

              <div className="flex min-w-0 flex-1 items-center gap-2">
                <h1 className="truncate text-[13px] font-semibold tracking-[-0.005em] text-foreground">
                  {title}
                </h1>
                <span className="flex items-center gap-1 text-[11px] text-muted-foreground">
                  <StatusDot live={ui.streaming} />
                  <span>{ui.streaming ? t("statusStreaming") : t("statusReady")}</span>
                </span>
                {ui.mock ? <Chip>{t("mockBadge")}</Chip> : null}
              </div>
            </header>

            <ChatSlot key={ui.activeWorkspaceId === null ? "none" : ui.activeWorkspaceId} />

            <footer className="flex h-6 shrink-0 items-center gap-3 border-t border-border bg-background px-3 text-[11px] text-muted-foreground">
              <span
                className="min-w-0 truncate"
                title={activeWorkspace === undefined ? "" : activeWorkspace.path}
              >
                {activeWorkspace === undefined ? t("noConversation") : activeWorkspace.path}
              </span>
              <span className="ms-auto flex items-center gap-1.5">
                <span className="h-1.5 w-1.5 rounded-capsule bg-live" aria-hidden="true" />
                <span>{ui.offline ? t("offlineBanner") : t("statusReady")}</span>
              </span>
              <span className="truncate">{packageLine}</span>
            </footer>
          </main>
        </div>

        <SettingsSlot />

        {state.toast !== null ? (
          <div
            className="fixed end-3 top-[52px] z-50 flex max-w-[min(420px,calc(100vw-24px))] items-center gap-2 rounded-xl bg-popover px-2.5 py-1.5 text-[12px] text-foreground shadow-[var(--zk-shadow-lift)]"
            role="status"
          >
            <span>{state.toast}</span>
            <button
              type="button"
              className="inline-flex h-5 w-5 items-center justify-center rounded-sm text-muted-foreground transition-colors hover:bg-foreground/[0.05] hover:text-foreground"
              aria-label={t("dismissToast")}
              onClick={controller.dismissToast}
            >
              <IconClose />
            </button>
          </div>
        ) : null}
      </div>
    </Tooltip.Provider>
  );
}
