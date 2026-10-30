import * as Tooltip from "@radix-ui/react-tooltip";
import { BrandLogo } from "./components/BrandLogo";
import { ChatView } from "./components/ChatView";
import { IconClose, IconMenu, IconRetry, IconSliders, IconWifiOff } from "./components/icons";
import { SettingsDialog } from "./components/SettingsDialog";
import { Sidebar } from "./components/Sidebar";
import { I18nProvider, useT } from "./i18n";
import { useAppController, type AppController } from "./state/controller";
import { ThemeProvider, useResolvedTheme } from "./theme";
import type { Transport } from "./types";

const tooltipClass = "z-50 rounded-surface bg-foreground px-2 py-1 text-[12px] text-background";
const iconButtonClass =
  "inline-flex h-8 w-8 items-center justify-center rounded-control text-muted-foreground transition-colors hover:bg-foreground/[0.05] hover:text-foreground";

function isMockMode(): boolean {
  return new URLSearchParams(window.location.search).get("mock") === "1";
}

export function App(props: { transport: Transport }) {
  const controller = useAppController(props.transport);
  const resolvedTheme = useResolvedTheme(controller.state.settings.theme);
  return (
    <I18nProvider locale={controller.state.settings.locale}>
      <ThemeProvider theme={resolvedTheme}>
        <AppShell controller={controller} />
      </ThemeProvider>
    </I18nProvider>
  );
}

function AppShell(props: { controller: AppController }) {
  const t = useT();
  const { state } = props.controller;
  const mock = isMockMode();

  if (state.boot === "loading") {
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

  if (state.boot === "error") {
    return (
      <div className="flex h-dvh flex-col items-center justify-center gap-3 bg-background px-6 text-center">
        <BrandLogo className="h-10 w-10 rounded-document" />
        <h1 className="text-[20px] font-semibold tracking-[-0.01em] text-foreground">
          {t("bootErrorTitle")}
        </h1>
        <p className="max-w-[480px] text-[13px] text-muted-foreground">{t("bootErrorHint")}</p>
        {state.bootError !== null ? (
          <p className="max-w-[480px] rounded-document bg-muted px-3 py-2 font-mono text-[12px] text-destructive">
            {state.bootError}
          </p>
        ) : null}
        <button
          type="button"
          className="mt-1 inline-flex items-center gap-1.5 rounded-control bg-foreground px-3.5 py-1.5 text-[13px] font-medium text-background transition-colors hover:bg-foreground/90"
          onClick={props.controller.retryBoot}
        >
          <IconRetry />
          <span>{t("retry")}</span>
        </button>
      </div>
    );
  }

  const active = state.conversations.find(
    (conversation) => conversation.id === state.activeConversationId,
  );
  const title = active === undefined ? t("noConversation") : active.title;

  return (
    <Tooltip.Provider delayDuration={300}>
      <a className="zk-skip-link" href="#zk-composer-input">
        {t("skipToInput")}
      </a>
      <div className="flex h-dvh flex-col overflow-hidden bg-background text-foreground">
        {state.offline ? (
          <div
            className="flex items-center justify-center gap-2 border-b border-border bg-muted px-4 py-1.5 text-[12px] text-destructive"
            role="status"
          >
            <IconWifiOff />
            <span>{t("offlineBanner")}</span>
          </div>
        ) : null}

        <div className="flex min-h-0 flex-1">
          <Sidebar
            conversations={state.conversations}
            activeId={state.activeConversationId}
            streaming={state.streaming}
            open={state.sidebarOpen}
            meta={state.meta}
            mock={mock}
            onNew={props.controller.createConversation}
            onSelect={props.controller.selectConversation}
            onClose={props.controller.toggleSidebar}
          />
          {state.sidebarOpen ? (
            <button
              type="button"
              className="zk-sidebar-backdrop"
              aria-label={t("sidebarClose")}
              onClick={props.controller.toggleSidebar}
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
                    onClick={props.controller.toggleSidebar}
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
                  <span
                    className={`h-1.5 w-1.5 rounded-capsule ${
                      state.streaming ? "zk-pulse-dot bg-live" : "bg-muted-foreground"
                    }`}
                    aria-hidden="true"
                  />
                  <span>{state.streaming ? t("statusStreaming") : t("statusReady")}</span>
                  {mock ? (
                    <span className="rounded-sm border border-border px-1.5 py-0.5 text-[12px] text-muted-foreground">
                      {t("mockBadge")}
                    </span>
                  ) : null}
                </p>
              </div>

              <Tooltip.Root>
                <Tooltip.Trigger asChild>
                  <button
                    type="button"
                    className={iconButtonClass}
                    aria-label={t("openSettings")}
                    onClick={() => {
                      props.controller.setSettingsOpen(true);
                    }}
                  >
                    <IconSliders />
                  </button>
                </Tooltip.Trigger>
                <Tooltip.Portal>
                  <Tooltip.Content className={tooltipClass} sideOffset={6}>
                    {t("settings")}
                  </Tooltip.Content>
                </Tooltip.Portal>
              </Tooltip.Root>
            </header>

            <ChatView controller={props.controller} mock={mock} />
          </main>
        </div>

        <SettingsDialog
          open={state.settingsOpen}
          settings={state.settings}
          onOpenChange={props.controller.setSettingsOpen}
          onSave={props.controller.saveSettings}
        />

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
              onClick={props.controller.dismissToast}
            >
              <IconClose />
            </button>
          </div>
        ) : null}
      </div>
    </Tooltip.Provider>
  );
}
