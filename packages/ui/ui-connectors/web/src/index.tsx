/** Enterprise connector panel. */
import { useRef } from "react";
import { useDialogFocus, useUi } from "@zeokeel/ui-kit";

export interface ConnectorItem { id: string; name: string; transport: string; enabled: boolean; can_enable: boolean; oauth: { clientId: string; scopes: string[]; redirectUri: string; authorizeUrl: string; tokenUrl: string } | null }

export interface ConnectorsPanelProps {
  open: boolean;
  items: ConnectorItem[];
  error: string | null;
  authorized: string[];
  onAuthorize: (item: ConnectorItem) => void;
  onToggle: (item: ConnectorItem) => void;
  /** Drop/reload the host MCP sessions from the current catalog (config spec R7). */
  onReload?: (() => void) | undefined;
  onClose: () => void;
}

export function ConnectorsPanel(props: ConnectorsPanelProps) {
  const { t } = useUi();
  const dialogRef = useRef<HTMLDivElement>(null);
  useDialogFocus(dialogRef, props.onClose, props.open);
  if (!props.open) return null;
  return (
    <div ref={dialogRef} className="fixed inset-0 z-50 flex justify-center bg-foreground/20 px-6 pt-[14vh]" role="dialog" aria-modal="true" aria-label={t("connectorsTitle")}>
      <button type="button" className="absolute inset-0" aria-label={t("connectorsClose")} onClick={props.onClose} />
      <div className="relative h-fit max-h-[70vh] w-full max-w-[560px] overflow-y-auto rounded-xl border border-border bg-popover p-3 shadow-[var(--zk-shadow-lift)]">
        <div className="flex items-center justify-between gap-2">
          <h2 className="text-[14px] font-semibold">{t("connectorsTitle")}</h2>
          <div className="ms-auto flex items-center gap-2">
            {props.onReload !== undefined ? (
              <button
                type="button"
                onClick={() => {
                  if (props.onReload !== undefined) props.onReload();
                }}
                className="rounded-control border border-border px-2 py-1 text-[11px] text-foreground"
              >
                {t("connectorsReload")}
              </button>
            ) : null}
            <button type="button" data-autofocus onClick={props.onClose} className="text-[12px] text-muted-foreground">{t("close")}</button>
          </div>
        </div>
        {props.error !== null ? <p role="alert" className="mt-2 rounded-document bg-muted px-2 py-1.5 text-[12px] text-destructive">{props.error}</p> : null}
        {props.items.length === 0 ? <p className="mt-4 text-[12px] text-muted-foreground">{t("connectorsEmpty")}</p> : null}
        <ul className="mt-3 grid gap-2">
          {props.items.map((item) => (
            <li key={item.id} className="flex items-center gap-3 rounded-document bg-muted px-3 py-2">
              <div className="min-w-0 flex-1">
                <p className="truncate text-[13px] text-foreground">{item.name}</p>
                <p className="text-[11px] text-muted-foreground">{item.transport}</p>
              </div>
              {item.oauth ? (
                <button type="button" onClick={() => { props.onAuthorize(item); }} className="rounded-control border border-border px-2 py-1 text-[11px] text-foreground">
                  {props.authorized.includes(item.id) ? t("connectorsAuthorized") : t("connectorsAuth")}
                </button>
              ) : null}
              <button
                type="button"
                disabled={!item.can_enable}
                onClick={() => { props.onToggle(item); }}
                className="rounded-control bg-foreground px-2 py-1 text-[11px] text-background disabled:opacity-40"
              >
                {item.enabled ? t("connectorsDisable") : t("connectorsEnable")}
              </button>
            </li>
          ))}
        </ul>
      </div>
    </div>
  );
}

export const components = { "panel.connectors": ConnectorsPanel } as const;
