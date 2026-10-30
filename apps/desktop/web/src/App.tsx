import * as Tooltip from "@radix-ui/react-tooltip";
import {
  BrandLogo,
  Button,
  Chip,
  IconClose,
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
  "inline-flex h-8 w-8 items-center justify-center rounded-control text-muted-foreground transition-colors hover:bg-foreground/[0.05] hover:text-foreground";

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

  const active = ui.conversations.find(
    (conversation) => conversation.id === ui.activeConversationId,
  );
  const title = active === undefined ? t("noConversation") : active.title;

  return (
    <Tooltip.Provider delayDuration={300}>
      <a className="zk-skip-link" href="#zk-composer-input">
        {t("skipToInput")}
      </a>
      <div className="flex h-dvh flex-col overflow-hidden bg-background text-foreground">
        {ui.offline ? (
          <div
            className="flex items-center justify-center gap-2 border-b border-border bg-muted px-4 py-1.5 text-[12px] text-destructive"
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
            <header className="flex min-h-[56px] items-center gap-2 border-b border-border bg-background px-4 py-2.5 sm:px-6">
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

              <div className="flex min-w-0 flex-1 flex-col">
                <h1 className="truncate text-[14px] font-semibold tracking-[-0.005em] text-foreground">
                  {title}
                </h1>
                <p className="mt-0.5 flex items-center gap-1.5 text-[12px] text-muted-foreground">
                  <StatusDot live={ui.streaming} />
                  <span>{ui.streaming ? t("statusStreaming") : t("statusReady")}</span>
                  {ui.mock ? <Chip>{t("mockBadge")}</Chip> : null}
                </p>
              </div>
            </header>

            <ChatSlot />
          </main>
        </div>

        <SettingsSlot />

        {state.toast !== null ? (
          <div
            className="fixed end-5 top-[68px] z-50 flex max-w-[min(420px,calc(100vw-40px))] items-center gap-2 rounded-xl bg-popover px-3 py-2 text-[13px] text-foreground shadow-[var(--zk-shadow-lift)]"
            role="status"
          >
            <span>{state.toast}</span>
            <button
              type="button"
              className="inline-flex h-6 w-6 items-center justify-center rounded-sm text-muted-foreground transition-colors hover:bg-foreground/[0.05] hover:text-foreground"
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
