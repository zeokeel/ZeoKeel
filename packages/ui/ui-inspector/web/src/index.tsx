/** Module skeleton: right inspector (files/changes/terminal). Data comes from the app. */
export type InspectorTab = "files" | "changes" | "terminal";

export interface FileNode { path: string; depth: number; kind: "dir" | "file"; badge?: string }
export interface ChangeView { path: string; add: number; del: number }

export interface InspectorPanelProps {
  open: boolean;
  tab: InspectorTab;
  files: FileNode[];
  changes: ChangeView[];
  onTab: (tab: InspectorTab) => void;
  onClose: () => void;
}

export function InspectorPanel(props: InspectorPanelProps) {
  if (!props.open) return null;
  return (
    <aside aria-label="右侧检查器" className="flex w-[288px] shrink-0 flex-col border-s border-border" data-slot="inspector">
      <div className="flex h-9 items-center gap-1 border-b border-border px-2 text-[11px]">
        {(["files", "changes", "terminal"] as const).map((tab) => (
          <button key={tab} type="button" onClick={() => props.onTab(tab)} className={`rounded-sm px-2 py-1 ${props.tab === tab ? "bg-muted" : ""}`}>
            {tab}
          </button>
        ))}
        <button type="button" onClick={props.onClose} className="ms-auto px-1 text-muted-foreground" aria-label="收起右栏">×</button>
      </div>
      <div className="min-h-0 flex-1 overflow-y-auto p-2 text-[11px] text-muted-foreground">
        {props.tab === "files" ? props.files.map((file) => <p key={file.path} style={{ paddingInlineStart: `${file.depth * 8}px` }}>{file.path}</p>) : null}
        {props.tab === "changes" ? props.changes.map((change) => <p key={change.path}>{change.path} +{change.add} −{change.del}</p>) : null}
        {props.tab === "terminal" ? <p className="font-mono">terminal skeleton</p> : null}
      </div>
    </aside>
  );
}

export const components = { "inspector.panel": InspectorPanel } as const;
