/** Module: shell — header, 3-column frame, status bar. */
import type { ReactNode } from "react";
import { Icon, IconButton } from "./kit";

export function Shell(props: {
  navigation: ReactNode;
  conversation: ReactNode;
  composer: ReactNode;
  inspector: ReactNode;
  overlay?: ReactNode;
  title: string;
  workspaceName: string;
  streaming: boolean;
  inspectorOpen: boolean;
  onToggleInspector: () => void;
  onOpenCommands: () => void;
  children?: ReactNode;
}) {
  return (
    <div className="relative flex h-dvh flex-col overflow-hidden bg-background text-foreground">
      <div className="flex min-h-0 flex-1">
        {props.navigation}
        <main className="flex min-w-0 flex-1 flex-col">
          <header className="flex h-11 shrink-0 items-center gap-2 border-b border-border px-3">
            <div className="min-w-0 flex-1"><h1 className="truncate text-[12px] font-semibold tracking-[-0.005em]">{props.title}</h1></div>
            <div className="flex items-center gap-1.5 text-muted-foreground">
              <span className="hidden items-center gap-1 rounded-control border border-border px-1.5 py-0.5 text-[10px] xl:flex"><Icon name="branch" size={11} />main</span>
              <span className="flex items-center gap-1 rounded-control border border-border px-1.5 py-0.5 text-[10px]"><span className={`h-1.5 w-1.5 rounded-capsule ${props.streaming ? "bg-live" : "bg-border"}`} />DeepSeek V3</span>
              <span className="hidden font-mono text-[10px] lg:flex">42% · 18.2k</span>
              <button type="button" aria-label="任务" className="relative flex h-6 w-6 items-center justify-center rounded-sm hover:bg-foreground/[0.05]"><Icon name="terminal" size={13} /><span className="absolute -right-0.5 -top-0.5 h-2 w-2 rounded-capsule bg-live ring-2 ring-background" /></button>
              <IconButton name="panel" label={props.inspectorOpen ? "收起右栏" : "打开右栏"} onClick={props.onToggleInspector} />
              <IconButton name="more" label="更多" />
            </div>
          </header>
          <div className="flex min-h-0 flex-1 flex-col">{props.conversation}{props.composer}</div>
        </main>
        {props.inspector}
      </div>
      <div className="flex h-6 shrink-0 items-center gap-2 border-t border-border px-3 text-[10px] text-muted-foreground">
        <span className="flex items-center gap-1"><span className="h-1.5 w-1.5 rounded-capsule bg-live" />skeleton · mock data</span><span className="text-border">|</span>
        <span className="font-mono">{props.workspaceName}</span><span className="text-border">|</span><span>9 packages</span>
        <span className="ms-auto flex items-center gap-2 font-mono"><button type="button" onClick={props.onOpenCommands} className="hover:text-foreground">⌘K 命令</button><span className="text-border">|</span><span>v0.1.0</span></span>
      </div>
      {props.overlay}
    </div>
  );
}
