/** Enterprise connector panel. */
export interface ConnectorItem { id: string; name: string; transport: string; enabled: boolean; can_enable: boolean }

export interface ConnectorsPanelProps {
  open: boolean;
  items: ConnectorItem[];
  error: string | null;
  authorized: string[];
  onAuthorize: (item: ConnectorItem) => void;
  onToggle: (item: ConnectorItem) => void;
  onClose: () => void;
}

export function ConnectorsPanel(props: ConnectorsPanelProps) {
  if (!props.open) return null;
  return (
    <div className="fixed inset-0 z-50 flex justify-center bg-foreground/20 px-6 pt-[14vh]" role="dialog" aria-modal="true" aria-label="连接器">
      <button type="button" className="absolute inset-0" aria-label="关闭连接器" onClick={props.onClose} />
      <div className="relative h-fit max-h-[70vh] w-full max-w-[560px] overflow-y-auto rounded-xl border border-border bg-popover p-3 shadow-[var(--zk-shadow-lift)]">
        <div className="flex items-center justify-between"><h2 className="text-[14px] font-semibold">连接器</h2><button type="button" onClick={props.onClose} className="text-[12px] text-muted-foreground">关闭</button></div>
        {props.error !== null ? <p role="alert" className="mt-2 rounded-document bg-muted px-2 py-1.5 text-[12px] text-destructive">{props.error}</p> : null}
        {props.items.length === 0 ? <p className="mt-4 text-[12px] text-muted-foreground">暂无可用连接器</p> : null}
        <ul className="mt-3 grid gap-2">
          {props.items.map((item) => (
            <li key={item.id} className="flex items-center gap-3 rounded-document bg-muted px-3 py-2">
              <div className="min-w-0 flex-1"><p className="truncate text-[13px] text-foreground">{item.name}</p><p className="text-[11px] text-muted-foreground">{item.transport}</p></div>
              <button type="button" disabled={!item.can_enable} onClick={() => props.onToggle(item)} className="rounded-control bg-foreground px-2 py-1 text-[11px] text-background disabled:opacity-40">{item.enabled ? "停用" : "启用"}</button>
            </li>
          ))}
        </ul>
      </div>
    </div>
  );
}

export const components = { "panel.connectors": ConnectorsPanel } as const;
