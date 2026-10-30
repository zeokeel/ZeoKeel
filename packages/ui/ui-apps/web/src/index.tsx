/** Enterprise app catalog/open panel. */
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
  if (!props.open) return null;
  return (
    <div className="fixed inset-0 z-50 flex justify-center bg-foreground/20 px-6 pt-[14vh]" role="dialog" aria-modal="true" aria-label="应用">
      <button type="button" className="absolute inset-0" aria-label="关闭应用" onClick={props.onClose} />
      <div className="relative h-fit max-h-[70vh] w-full max-w-[560px] overflow-y-auto rounded-xl border border-border bg-popover p-3 shadow-[var(--zk-shadow-lift)]">
        <div className="flex items-center justify-between"><h2 className="text-[14px] font-semibold">应用</h2><button type="button" onClick={props.onClose} className="text-[12px] text-muted-foreground">关闭</button></div>
        {props.error !== null ? <p role="alert" className="mt-2 rounded-document bg-muted px-2 py-1.5 text-[12px] text-destructive">{props.error}</p> : null}
        {props.opened !== null ? <p role="status" className="mt-2 text-[12px] text-muted-foreground">{props.opened}</p> : null}
        {props.items.length === 0 ? <p className="mt-4 text-[12px] text-muted-foreground">暂无可用应用</p> : null}
        <ul className="mt-3 grid gap-2">
          {props.items.map((item) => (
            <li key={item.appId} className="flex items-center gap-3 rounded-document bg-muted px-3 py-2">
              <div className="min-w-0 flex-1"><p className="truncate text-[13px] text-foreground">{item.name}</p><p className="text-[11px] text-muted-foreground">v{item.version} · {item.access}</p></div>
              <button type="button" onClick={() => props.onOpenApp(item)} className="rounded-control bg-foreground px-2 py-1 text-[11px] text-background">打开</button>
            </li>
          ))}
        </ul>
      </div>
    </div>
  );
}

export const components = { "panel.apps": AppsPanel, "surface.apps": AppSurface } as const;

export const componentsWithSurface = components;

export interface AppSurfaceProps {
  html: string;
  nonce: string;
  onRequest: (payload: { method: string; path: string; query: string; body_b64: string; request_id: string }) => Promise<{ status: number; body_b64: string }>;
  onClose: () => void;
}

/** Sandboxed static app surface (serve path; desktop uses an isolated Tauri window). */
export function AppSurface(props: AppSurfaceProps) {
  return (
    <div className="fixed inset-0 z-[60] flex flex-col bg-background" role="dialog" aria-modal="true" aria-label="应用窗口">
      <div className="flex h-9 items-center justify-between border-b border-border px-3">
        <span className="text-[12px] text-muted-foreground">隔离应用窗口</span>
        <button type="button" onClick={props.onClose} className="text-[12px]">关闭</button>
      </div>
      <iframe
        title="应用"
        sandbox="allow-scripts"
        className="flex-1 border-0 bg-background"
        srcDoc={`<meta http-equiv="Content-Security-Policy" content="default-src 'none'; script-src 'unsafe-inline'; style-src 'unsafe-inline'; img-src data:; connect-src 'none'; frame-src 'none'"><script>window.__zkNonce=${JSON.stringify(props.nonce)};window.zkAppRequest=function(payload){return new Promise(function(resolve){var id=Math.random().toString(36);function onMessage(event){if(event.data&&event.data.zkResponse===id){window.removeEventListener('message',onMessage);resolve(event.data.response);}}window.addEventListener('message',onMessage);window.parent.postMessage({zkRequest:id,nonce:window.__zkNonce,payload:payload},'*');});};</script>${props.html}`}
        onLoad={(event) => {
          const frame = event.currentTarget;
          const listener = (message: MessageEvent) => {
            const data: unknown = message.data;
            if (frame.contentWindow === null || message.source !== frame.contentWindow) return;
            if (typeof data !== "object" || data === null) return;
            const value = data as { zkRequest?: unknown; nonce?: unknown; payload?: unknown };
            if (value.nonce !== props.nonce) return;
            const requestId = String(value.zkRequest ?? "");
            const payload = value.payload;
            if (typeof payload !== "object" || payload === null) return;
            const request = payload as { method?: unknown; path?: unknown; query?: unknown; body_b64?: unknown; request_id?: unknown };
            void props.onRequest({
              method: typeof request.method === "string" ? request.method : "GET",
              path: typeof request.path === "string" ? request.path : "/",
              query: typeof request.query === "string" ? request.query : "",
              body_b64: typeof request.body_b64 === "string" ? request.body_b64 : "",
              request_id: typeof request.request_id === "string" ? request.request_id : requestId,
            }).then((response) => { frame.contentWindow?.postMessage({ zkResponse: requestId, response }, "*"); }).catch(() => { frame.contentWindow?.postMessage({ zkResponse: requestId, response: { status: 502, body_b64: "" } }, "*"); });
          };
          window.addEventListener("message", listener);
          frame.addEventListener("load", () => { window.removeEventListener("message", listener); }, { once: true });
        }}
      />
    </div>
  );
}

