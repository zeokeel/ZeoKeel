/** Enterprise skill catalog/install panel (installation owned by app IPC). */
export interface SkillItem {
  id: string;
  name: string;
  version: string;
  sha256: string;
  enforced: boolean;
  min_client_version: string;
  can_install: boolean;
}

export interface SkillsPanelProps {
  open: boolean;
  items: SkillItem[];
  installed: Array<{ name: string; version: string }>;
  busy: string | null;
  error: string | null;
  onInstall: (item: SkillItem) => void;
  onClose: () => void;
}

export function SkillsPanel(props: SkillsPanelProps) {
  if (!props.open) return null;
  return (
    <div className="fixed inset-0 z-50 flex justify-center bg-foreground/20 px-6 pt-[14vh]" role="dialog" aria-modal="true" aria-label="技能">
      <button type="button" className="absolute inset-0" aria-label="关闭技能" onClick={props.onClose} />
      <div className="relative h-fit max-h-[70vh] w-full max-w-[560px] overflow-y-auto rounded-xl border border-border bg-popover p-3 shadow-[var(--zk-shadow-lift)]">
        <div className="flex items-center justify-between"><h2 className="text-[14px] font-semibold">技能</h2><button type="button" onClick={props.onClose} className="text-[12px] text-muted-foreground">关闭</button></div>
        {props.error !== null ? <p role="alert" className="mt-2 rounded-document bg-muted px-2 py-1.5 text-[12px] text-destructive">{props.error}</p> : null}
        {props.items.length === 0 ? <p className="mt-4 text-[12px] text-muted-foreground">暂无可用技能</p> : null}
        <ul className="mt-3 grid gap-2">
          {props.items.map((item) => (
            <li key={item.id} className="flex items-center gap-3 rounded-document bg-muted px-3 py-2">
              <div className="min-w-0 flex-1"><p className="truncate text-[13px] text-foreground">{item.name}</p><p className="text-[11px] text-muted-foreground">{item.version || "未发布"} · {item.sha256.slice(0, 12)}</p></div>
              {item.enforced ? <span className="text-[10px] text-destructive">强制</span> : null}
              {props.installed.some((installed) => installed.name === item.name && installed.version === item.version) ? <span className="text-[11px] text-muted-foreground">已安装</span> : (
                <button type="button" disabled={!item.can_install || props.busy === item.name} onClick={() => props.onInstall(item)} className="rounded-control bg-foreground px-2 py-1 text-[11px] text-background disabled:opacity-40">{props.busy === item.name ? (props.installed.some((installed) => installed.name === item.name) ? "更新中…" : "安装中…") : item.can_install ? (props.installed.some((installed) => installed.name === item.name) ? "更新" : "安装") : "不可安装"}</button>
              )}
            </li>
          ))}
        </ul>
      </div>
    </div>
  );
}

export const components = { "panel.skills": SkillsPanel } as const;
