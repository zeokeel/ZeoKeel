/** App shell frame: layout, header, status bar, overlays. Business slots are injected. */
import * as Tooltip from "@radix-ui/react-tooltip";
import { useState, type ReactNode } from "react";
import { BrandLogo, Button, Chip, IconClose, IconFolder, IconMenu, IconRetry, IconWifiOff, StatusDot } from "@zeokeel/ui-kit";

const tooltipClass = "z-50 rounded-surface bg-foreground px-2 py-1 text-[12px] text-background";
const iconButtonClass =
  "inline-flex h-7 w-7 items-center justify-center rounded-sm text-muted-foreground transition-colors hover:bg-foreground/[0.05] hover:text-foreground";

export interface ShellFrameProps {
  t: (key: string, vars?: Record<string, string | number>) => string;
  boot: "loading" | "ready" | "error";
  bootError: string | null;
  workspaceCount: number;
  sidebarOpen: boolean;
  offline: boolean;
  streaming: boolean;
  mock: boolean;
  title: string;
  packageLine: string;
  workspacePath: string;
  toast: string | null;
  workspaceError: string | null;
  navigation: ReactNode;
  conversation: ReactNode;
  settings: ReactNode;
  inspector: ReactNode;
  overlay?: ReactNode;
  surface?: ReactNode;
  inspectorOpen: boolean;
  onRetryBoot: () => void;
  onToggleSidebar: () => void;
  onDismissToast: () => void;
  onOpenCommands: () => void;
  onToggleInspector: () => void;
  onCreateWorkspace: (path: string, title: string) => void;
}

function FirstRun(props: { t: ShellFrameProps["t"]; error: string | null; onCreateWorkspace: (path: string, title: string) => void }) {
  const { t } = props;
  const [path, setPath] = useState("");
  const [title, setTitle] = useState("");
  const [showPicker, setShowPicker] = useState(false);
  const submit = (event: { preventDefault: () => void }): void => {
    event.preventDefault();
    props.onCreateWorkspace(path, title);
  };
  return (
    <div className="flex h-dvh flex-col items-center justify-center bg-background px-6 text-center">
      <BrandLogo className="h-10 w-10 rounded-document" />
      <h1 className="mt-5 text-[20px] font-semibold tracking-[-0.01em] text-foreground">{t("workspaceFirstTitle")}</h1>
      <p className="mt-2 max-w-[460px] text-[13px] text-muted-foreground">{t("workspaceFirstBody")}</p>
      {props.error !== null ? (
        <p className="mt-3 max-w-[460px] rounded-document bg-muted px-3 py-1.5 text-[12px] text-destructive">
          {props.error === "workspace/path-required" ? t("workspacePathInvalid") : props.error}
        </p>
      ) : null}
      <div className="mt-5 flex flex-wrap items-center justify-center gap-2">
        <Button variant="primary" onClick={() => { props.onCreateWorkspace("~", t("workspaceHomeTitle")); }} title={t("workspaceUseHome")}>
          <IconFolder /><span>{t("workspaceUseHome")}</span>
        </Button>
        <Button variant="default" aria-expanded={showPicker} onClick={() => { setShowPicker((value) => !value); }}>
          <IconFolder /><span>{t("workspaceAdd")}</span>
        </Button>
      </div>
      {showPicker ? (
        <form className="mt-4 flex w-full max-w-[460px] flex-col gap-2 rounded-document bg-muted p-3 text-start" onSubmit={submit}>
          <label className="flex flex-col gap-1 text-[12px] text-muted-foreground">{t("workspacePathLabel")}
            <input data-autofocus value={path} onChange={(event) => { setPath(event.target.value); }} placeholder={t("workspaceAddHint")} className="w-full rounded-control border border-border bg-background px-2 py-1.5 text-[13px] text-foreground placeholder:text-muted-foreground" />
          </label>
          <label className="flex flex-col gap-1 text-[12px] text-muted-foreground">{t("workspaceTitleLabel")}
            <input value={title} onChange={(event) => { setTitle(event.target.value); }} className="w-full rounded-control border border-border bg-background px-2 py-1.5 text-[13px] text-foreground" />
          </label>
          <div className="mt-1 flex justify-end"><Button variant="primary" size="sm" type="submit" disabled={path.trim() === ""}>{t("workspaceCreate")}</Button></div>
        </form>
      ) : null}
    </div>
  );
}

export function ShellFrame(props: ShellFrameProps) {
  const { t } = props;
  if (props.boot === "loading") {
    return (
      <div className="flex h-dvh flex-col items-center justify-center gap-4 bg-background text-center" role="status">
        <BrandLogo className="h-10 w-10 rounded-document" />
        <span className="zk-spinner" aria-hidden="true" />
        <p className="text-[13px] text-muted-foreground">{t("bootLoading")}</p>
      </div>
    );
  }
  if (props.boot === "error") {
    return (
      <div className="flex h-dvh flex-col items-center justify-center gap-3 bg-background px-6 text-center">
        <BrandLogo className="h-10 w-10 rounded-document" />
        <h1 className="text-[20px] font-semibold tracking-[-0.01em] text-foreground">{t("bootErrorTitle")}</h1>
        <p className="max-w-[480px] text-[13px] text-muted-foreground">{t("bootErrorHint")}</p>
        {props.bootError !== null ? (
          <p className="max-w-[480px] rounded-document bg-muted px-3 py-2 font-mono text-[12px] text-destructive">{props.bootError}</p>
        ) : null}
        <Button variant="primary" className="mt-1" onClick={props.onRetryBoot}><IconRetry /><span>{t("retry")}</span></Button>
      </div>
    );
  }
  if (props.workspaceCount === 0) { return <FirstRun t={props.t} error={props.workspaceError} onCreateWorkspace={props.onCreateWorkspace} />; }
  return (
    <Tooltip.Provider delayDuration={300}>
      <a className="zk-skip-link" href="#zk-composer-input">{t("skipToInput")}</a>
      <div className="flex h-dvh flex-col overflow-hidden bg-background text-foreground">
        {props.offline ? (
          <div className="flex items-center justify-center gap-2 border-b border-border bg-muted px-4 py-1 text-[11px] text-destructive" role="status">
            <IconWifiOff /><span>{t("offlineBanner")}</span>
          </div>
        ) : null}
        <div className="flex min-h-0 flex-1">
          {props.navigation}
          {props.sidebarOpen ? (
            <button type="button" className="zk-sidebar-backdrop" aria-label={t("sidebarClose")} onClick={props.onToggleSidebar} />
          ) : null}
          <main className="flex min-w-0 flex-1 flex-col">
            <header className="flex min-h-[44px] items-center gap-2 border-b border-border bg-background px-3 py-1.5">
              <Tooltip.Root>
                <Tooltip.Trigger asChild>
                  <button type="button" className={`${iconButtonClass} zk-sidebar-toggle`} aria-label={t("sidebarToggle")} onClick={props.onToggleSidebar}><IconMenu /></button>
                </Tooltip.Trigger>
                <Tooltip.Portal><Tooltip.Content className={tooltipClass} sideOffset={6}>{t("sidebarToggle")}</Tooltip.Content></Tooltip.Portal>
              </Tooltip.Root>
              <div className="flex min-w-0 flex-1 items-center gap-2">
                <h1 className="truncate text-[13px] font-semibold tracking-[-0.005em] text-foreground">{props.title}</h1>
                <span className="flex items-center gap-1 text-[11px] text-muted-foreground">
                  <StatusDot live={props.streaming} />
                  <span>{props.streaming ? t("statusStreaming") : t("statusReady")}</span>
                </span>
                {props.mock ? <Chip>{t("mockBadge")}</Chip> : null}
              </div>
              <button type="button" className={`${iconButtonClass} hidden sm:inline-flex`} aria-label={t("commandPalette")} title={t("commandPalette")} onClick={props.onOpenCommands}>
                <span className="font-mono text-[11px]">⌘K</span>
              </button>
              <button type="button" className={iconButtonClass} aria-label={t("inspectorToggle")} title={t("inspectorToggle")} aria-expanded={props.inspectorOpen} onClick={props.onToggleInspector}>
                <span className="font-mono text-[11px]">⌘B</span>
              </button>
            </header>
            {props.conversation}
            <footer className="flex h-6 shrink-0 items-center gap-3 border-t border-border bg-background px-3 text-[11px] text-muted-foreground">
              <span className="min-w-0 truncate" title={props.workspacePath}>{props.workspacePath}</span>
              <span className="ms-auto flex items-center gap-1.5">
                <span className="h-1.5 w-1.5 rounded-capsule bg-live" aria-hidden="true" />
                <span>{props.offline ? t("offlineBanner") : t("statusReady")}</span>
              </span>
              <span className="truncate">{props.packageLine}</span>
            </footer>
          </main>
          {props.inspectorOpen ? props.inspector : null}
        </div>
        {props.settings}
        {props.overlay ?? null}
        {props.surface ?? null}
        {props.toast !== null ? (
          <div className="fixed end-3 top-[52px] z-50 flex max-w-[min(420px,calc(100vw-24px))] items-center gap-2 rounded-xl bg-popover px-2.5 py-1.5 text-[12px] text-foreground shadow-[var(--zk-shadow-lift)]" role="status">
            <span>{props.toast}</span>
            <button type="button" className="inline-flex h-5 w-5 items-center justify-center rounded-sm text-muted-foreground transition-colors hover:bg-foreground/[0.05] hover:text-foreground" aria-label={t("dismissToast")} onClick={props.onDismissToast}><IconClose /></button>
          </div>
        ) : null}
      </div>
    </Tooltip.Provider>
  );
}

export const components = { "shell.frame": ShellFrame } as const;
