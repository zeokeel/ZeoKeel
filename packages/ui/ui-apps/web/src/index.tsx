/** Enterprise app catalog/open panel. */
import { useRef } from "react";
import { isRecord, readString, useDialogFocus, useUi } from "@zeokeel/ui-kit";

export interface AppItem { appId: string; name: string; description: string; version: string; access: string }

export interface AppsPanelProps {
  open: boolean;
  items: AppItem[];
  error: string | null;
  opened: string | null;
  onOpenApp: (item: AppItem) => void;
  onClose: () => void;
}

export function AppsPanel(props: AppsPanelProps) {
  const { t } = useUi();
  const dialogRef = useRef<HTMLDivElement>(null);
  useDialogFocus(dialogRef, props.onClose, props.open);
  if (!props.open) return null;
  return (
    <div ref={dialogRef} className="fixed inset-0 z-50 flex justify-center bg-foreground/20 px-6 pt-[14vh]" role="dialog" aria-modal="true" aria-label={t("appsTitle")}>
      <button type="button" className="absolute inset-0" aria-label={t("appsClose")} onClick={props.onClose} />
      <div className="relative h-fit max-h-[70vh] w-full max-w-[560px] overflow-y-auto rounded-xl border border-border bg-popover p-3 shadow-[var(--zk-shadow-lift)]">
        <div className="flex items-center justify-between">
          <h2 className="text-[14px] font-semibold">{t("appsTitle")}</h2>
          <button type="button" data-autofocus onClick={props.onClose} className="text-[12px] text-muted-foreground">{t("close")}</button>
        </div>
        {props.error !== null ? <p role="alert" className="mt-2 rounded-document bg-muted px-2 py-1.5 text-[12px] text-destructive">{props.error}</p> : null}
        {props.opened !== null ? <p role="status" className="mt-2 text-[12px] text-muted-foreground">{props.opened}</p> : null}
        {props.items.length === 0 ? <p className="mt-4 text-[12px] text-muted-foreground">{t("appsEmpty")}</p> : null}
        <ul className="mt-3 grid gap-2">
          {props.items.map((item) => (
            <li key={item.appId} className="flex items-center gap-3 rounded-document bg-muted px-3 py-2">
              <div className="min-w-0 flex-1">
                <p className="truncate text-[13px] text-foreground">{item.name}</p>
                <p className="text-[11px] text-muted-foreground">v{item.version} · {item.access}</p>
              </div>
              <button type="button" onClick={() => { props.onOpenApp(item); }} className="rounded-control bg-foreground px-2 py-1 text-[11px] text-background">
                {t("appsOpen")}
              </button>
            </li>
          ))}
        </ul>
      </div>
    </div>
  );
}

export interface AppSurfaceProps {
  html: string;
  nonce: string;
  onRequest: (payload: { method: string; path: string; query: string; body_b64: string; request_id: string }) => Promise<{ status: number; body_b64: string }>;
  onClose: () => void;
}

/** Sandboxed static app surface (serve path; desktop uses an isolated Tauri window). */
export function AppSurface(props: AppSurfaceProps) {
  const { t } = useUi();
  const dialogRef = useRef<HTMLDivElement>(null);
  useDialogFocus(dialogRef, props.onClose, true);
  return (
    <div ref={dialogRef} className="fixed inset-0 z-[60] flex flex-col bg-background" role="dialog" aria-modal="true" aria-label={t("appsSurfaceTitle")}>
      <div className="flex h-9 items-center justify-between border-b border-border px-3">
        <span className="text-[12px] text-muted-foreground">{t("appsSurfaceLabel")}</span>
        <button type="button" data-autofocus onClick={props.onClose} className="text-[12px]">{t("appsSurfaceClose")}</button>
      </div>
      <iframe
        title={t("appsTitle")}
        sandbox="allow-scripts"
        className="flex-1 border-0 bg-background"
        srcDoc={`<meta http-equiv="Content-Security-Policy" content="default-src 'none'; script-src 'unsafe-inline'; style-src 'unsafe-inline'; img-src data:; connect-src 'none'; frame-src 'none'"><script>window.__zkNonce=${JSON.stringify(props.nonce)};window.zkAppRequest=function(payload){return new Promise(function(resolve){var id=Math.random().toString(36);function onMessage(event){if(event.data&&event.data.zkResponse===id){window.removeEventListener('message',onMessage);resolve(event.data.response);}}window.addEventListener('message',onMessage);window.parent.postMessage({zkRequest:id,nonce:window.__zkNonce,payload:payload},'*');});};</script>${props.html}`}
        onLoad={(event) => {
          const frame = event.currentTarget;
          const listener = (message: MessageEvent) => {
            const data: unknown = message.data;
            if (frame.contentWindow === null || message.source !== frame.contentWindow) return;
            if (!isRecord(data)) return;
            const nonce = readString(data, "nonce");
            if (nonce !== props.nonce) return;
            const requestId = readString(data, "zkRequest") ?? "";
            const payload = data["payload"];
            if (!isRecord(payload)) return;
            void props.onRequest({
              method: readString(payload, "method") ?? "GET",
              path: readString(payload, "path") ?? "/",
              query: readString(payload, "query") ?? "",
              body_b64: readString(payload, "body_b64") ?? "",
              request_id: readString(payload, "request_id") ?? requestId,
            }).then((response) => { frame.contentWindow?.postMessage({ zkResponse: requestId, response }, "*"); }).catch(() => { frame.contentWindow?.postMessage({ zkResponse: requestId, response: { status: 502, body_b64: "" } }, "*"); });
          };
          window.addEventListener("message", listener);
          frame.addEventListener("load", () => { window.removeEventListener("message", listener); }, { once: true });
        }}
      />
    </div>
  );
}

export const components = { "panel.apps": AppsPanel, "surface.apps": AppSurface } as const;

export const componentsWithSurface = components;
