/** Module: navigation — flat DSH workspaces, sessions nested one level, search/new. */
import type { WorkspaceView } from "../types";
import { Icon, IconButton, Logo } from "./kit";

export interface NavigationProps {
  workspaces: WorkspaceView[];
  activeSessionId: string;
  activeWorkspaceId: string;
  collapsed: Record<string, boolean>;
  onToggleWorkspace: (id: string) => void;
  onSelectSession: (workspaceId: string, sessionId: string) => void;
  onNewSession: () => void;
  onOpenCommands: () => void;
  onOpenSettings: () => void;
  onOpenAddWorkspace: () => void;
  onCloseNarrow: () => void;
  narrow: boolean;
}

export function Navigation(props: NavigationProps) {
  return (
    <aside className="flex h-full w-[248px] shrink-0 flex-col border-e border-border bg-sidebar" aria-label="工作区与会话">
      <div className="flex items-center justify-between px-3 pt-3 pb-2">
        <div className="flex items-center gap-2"><Logo /><span className="text-[12px] font-semibold">ZeoKeel</span></div>
        <div className="flex items-center gap-0.5">
          <IconButton name="search" label="搜索" />
          <IconButton name="plus" label="新建会话" onClick={props.onNewSession} />
          {props.narrow ? <IconButton name="close" label="关闭侧栏" onClick={props.onCloseNarrow} /> : null}
        </div>
      </div>
      <div className="px-2">
        <button type="button" onClick={props.onNewSession} className="flex h-8 w-full items-center justify-center gap-1.5 rounded-control border border-border bg-background text-[12px] font-medium hover:bg-muted">
          <Icon name="plus" size={12} />新会话
        </button>
        <div className="mt-2 flex flex-col gap-0.5 text-[12px] text-muted-foreground">
          <button type="button" onClick={props.onOpenCommands} className="flex items-center gap-2 rounded-sm px-1.5 py-[5px] text-start hover:bg-foreground/[0.035] hover:text-foreground">
            <Icon name="command" size={13} />命令面板<span className="ms-auto font-mono text-[10px]">⌘K</span>
          </button>
          <button type="button" className="flex items-center gap-2 rounded-sm px-1.5 py-[5px] text-start text-muted-foreground/70" disabled title="M3">
            <Icon name="task" size={13} />自动化任务<span className="ms-auto text-[10px]">后续</span>
          </button>
          <button type="button" className="flex items-center gap-2 rounded-sm px-1.5 py-[5px] text-start text-muted-foreground/70" disabled title="M2/M3">
            <Icon name="spark" size={13} />插件<span className="ms-auto text-[10px]">后续</span>
          </button>
        </div>
      </div>
      <div className="mt-3 flex items-center gap-1.5 px-3 pb-1 text-muted-foreground">
        <span className="text-[10px] font-medium uppercase tracking-[0.08em]">工作区</span>
        <span className="ms-auto flex items-center gap-0.5">
          <IconButton name="search" label="搜索工作区" size={11} />
          <IconButton name="sliders" label="工作区选项" size={11} />
          <IconButton name="folder" label="添加工作区" onClick={props.onOpenAddWorkspace} size={11} />
        </span>
      </div>
      <nav className="min-h-0 flex-1 overflow-y-auto px-2 pb-2" aria-label="工作区">
        {props.workspaces.map((workspace) => {
          const collapsed = props.collapsed[workspace.id] === true;
          const isActiveWorkspace = workspace.id === props.activeWorkspaceId;
          return (
            <div key={workspace.id} className="mb-0.5">
              <button
                type="button"
                onClick={() => props.onToggleWorkspace(workspace.id)}
                className={`flex w-full items-center gap-1.5 rounded-sm px-1.5 py-[5px] text-start text-[12px] ${isActiveWorkspace ? "font-medium text-foreground" : "text-muted-foreground hover:bg-foreground/[0.035] hover:text-foreground"}`}
              >
                <Icon name="folder" size={12} />
                <span className="min-w-0 flex-1 truncate">{workspace.name}</span>
                {workspace.isDefault ? <span className="rounded-capsule bg-muted px-1 text-[9px] font-normal text-muted-foreground">默认</span> : null}
                <span className={`text-muted-foreground/70 transition-transform ${collapsed ? "-rotate-90" : ""}`}><Icon name="chevron" size={11} /></span>
              </button>
              {collapsed ? null : (
                <div className="ms-3 border-s border-border/60 ps-1">
                  {workspace.sessions.map((session) => {
                    const active = session.id === props.activeSessionId;
                    return (
                      <button
                        key={session.id}
                        type="button"
                        onClick={() => props.onSelectSession(workspace.id, session.id)}
                        className={`flex w-full items-center gap-1.5 rounded-sm px-1.5 py-[4px] text-start text-[12px] ${active ? "bg-muted font-medium text-foreground shadow-[inset_2px_0_0_var(--zk-blue-500)]" : "text-muted-foreground hover:bg-foreground/[0.035] hover:text-foreground"}`}
                      >
                        <span className={`h-1.5 w-1.5 shrink-0 rounded-capsule ${session.status === "running" ? "bg-live" : session.status === "error" ? "bg-destructive" : "bg-border"}`} aria-hidden="true" />
                        <span className="min-w-0 flex-1 truncate">{session.title}</span>
                        <span className="shrink-0 text-[10px] tabular-nums text-muted-foreground">{session.time}</span>
                      </button>
                    );
                  })}
                </div>
              )}
            </div>
          );
        })}
        <button type="button" onClick={props.onOpenAddWorkspace} className="mt-1 flex w-full items-center gap-1.5 rounded-sm px-1.5 py-[5px] text-start text-[11px] text-muted-foreground hover:bg-foreground/[0.035] hover:text-foreground">
          <Icon name="plus" size={11} />添加工作区
        </button>
      </nav>
      <div className="flex items-center gap-1 border-t border-border px-2 py-1.5">
        <button type="button" onClick={props.onOpenSettings} className="flex h-6 flex-1 items-center gap-1.5 rounded-sm px-1.5 text-[11px] text-muted-foreground hover:bg-foreground/[0.04]">
          <Icon name="sliders" size={12} />设置
        </button>
        <IconButton name="command" label="命令面板" onClick={props.onOpenCommands} />
      </div>
    </aside>
  );
}
