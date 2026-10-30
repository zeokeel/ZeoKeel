/** Module skeleton: right inspector (files/changes/terminal). Data comes from the app. */
import { useUi } from "@zeokeel/ui-kit";

export type InspectorTab = "files" | "changes" | "terminal";

export interface FileNode { path: string; depth: number; kind: "dir" | "file"; badge?: string }
export interface ChangeView { path: string; add: number; del: number }

export interface InspectorPanelProps {
  open: boolean;
  tab: InspectorTab;
  files: FileNode[];
  /** `workspace.changes` state for the active workspace. */
  changes: ChangeView[];
  changesState: "idle" | "loading" | "error";
  changesError: string | null;
  changesWarning: string | null;
  onRetryChanges: () => void;
  onTab: (tab: InspectorTab) => void;
  onClose: () => void;
}

const TABS: InspectorTab[] = ["files", "changes", "terminal"];

export function InspectorPanel(props: InspectorPanelProps) {
  const { t } = useUi();
  if (!props.open) return null;
  const labelOf = (tab: InspectorTab): string =>
    tab === "files" ? t("inspectorTabFiles") : tab === "changes" ? t("inspectorTabChanges") : t("inspectorTabTerminal");
  return (
    <aside aria-label={t("inspectorTitle")} className="flex w-[288px] shrink-0 flex-col border-s border-border" data-slot="inspector">
      <div className="flex h-9 items-center gap-1 border-b border-border px-2 text-[11px]">
        <div role="tablist" aria-label={t("inspectorTitle")} className="flex items-center gap-1">
          {TABS.map((tab) => (
            <button
              key={tab}
              type="button"
              role="tab"
              aria-selected={props.tab === tab}
              aria-disabled={tab === "changes" && props.changesState === "loading"}
              onClick={() => { props.onTab(tab); }}
              className={`inline-flex items-center gap-1 rounded-sm px-2 py-1 ${props.tab === tab ? "bg-muted" : ""}`}
            >
              {labelOf(tab)}
            </button>
          ))}
        </div>
        <button type="button" onClick={props.onClose} className="ms-auto px-1 text-muted-foreground" aria-label={t("inspectorClose")}>×</button>
      </div>
      <div role="tabpanel" className="min-h-0 flex-1 overflow-y-auto p-2 text-[11px] text-muted-foreground">
        {props.tab === "files" ? (
          props.files.length === 0
            ? <p>{t("inspectorFilesEmpty")}</p>
            : props.files.map((file) => <p key={file.path} style={{ paddingInlineStart: `${String(file.depth * 8)}px` }}>{file.path}</p>)
        ) : null}
        {props.tab === "changes" && props.changesState === "loading" ? <p aria-busy="true">{t("inspectorChangesLoading")}</p> : null}
        {props.tab === "changes" && props.changesState === "error" ? (
          <div role="alert" className="flex flex-col items-start gap-2">
            <p className="text-destructive">{t("inspectorChangesError")}</p>
            {props.changesError !== null ? <p>{props.changesError}</p> : null}
            <button type="button" className="rounded-control border border-border px-2 py-1 text-foreground hover:bg-muted" onClick={props.onRetryChanges}>{t("inspectorChangesRetry")}</button>
          </div>
        ) : null}
        {props.tab === "changes" && props.changesState !== "loading" && props.changesState !== "error" ? (
          <>
            {props.changesWarning !== null ? <p role="status" className="mb-1 text-muted-foreground">{t("inspectorChangesWarning", { message: props.changesWarning })}</p> : null}
            {props.changes.length === 0
              ? <p>{t("inspectorChangesEmpty")}</p>
              : props.changes.map((change) => (
                <p key={change.path} aria-label={t("inspectorChangesItem", { path: change.path, add: change.add, del: change.del })}>
                  <span className="text-muted-foreground">{t("inspectorChangesFile")} </span>
                  {change.path} +{String(change.add)} −{String(change.del)}
                </p>
              ))}
          </>
        ) : null}
        {props.tab === "terminal" ? <p className="font-mono">{t("inspectorTerminalSkeleton")}</p> : null}
      </div>
    </aside>
  );
}

export const components = { "inspector.panel": InspectorPanel } as const;
