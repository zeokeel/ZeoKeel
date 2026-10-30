/**
 * Module skeleton: app frame. Real layout/interaction lands in the UI implementation step;
 * this component only fixes the props contract and composition order.
 */
import type { ReactNode } from "react";

export interface AppShellProps {
  title: string;
  workspaceName: string;
  streaming: boolean;
  inspectorOpen: boolean;
  onToggleInspector: () => void;
  onOpenCommands: () => void;
  navigation: ReactNode;
  conversation: ReactNode;
  composer: ReactNode;
  inspector: ReactNode;
  overlay?: ReactNode;
}

export function AppShell(props: AppShellProps) {
  return (
    <div className="flex h-dvh flex-col bg-background text-foreground" data-slot="shell">
      <div className="flex min-h-0 flex-1">
        {props.navigation}
        <main className="flex min-w-0 flex-1 flex-col">
          <header className="flex h-11 items-center gap-2 border-b border-border px-3">
            <h1 className="min-w-0 flex-1 truncate text-[12px] font-semibold">{props.title}</h1>
            <button type="button" onClick={props.onToggleInspector} aria-label="切换右栏" className="rounded-control px-1.5 py-0.5 text-[11px]">
              右栏
            </button>
            <button type="button" onClick={props.onOpenCommands} aria-label="命令面板" className="rounded-control px-1.5 py-0.5 text-[11px]">
              ⌘K
            </button>
          </header>
          <div className="flex min-h-0 flex-1 flex-col">{props.conversation}{props.composer}</div>
        </main>
        {props.inspectorOpen ? props.inspector : null}
      </div>
      <footer className="flex h-6 items-center gap-2 border-t border-border px-3 text-[10px] text-muted-foreground">
        <span>{props.workspaceName}</span>
        <span className="ms-auto">{props.streaming ? "streaming" : "idle"}</span>
      </footer>
      {props.overlay}
    </div>
  );
}

export const components = { "shell.frame": AppShell } as const;
