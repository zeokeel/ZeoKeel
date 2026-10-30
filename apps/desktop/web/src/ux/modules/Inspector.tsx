/** Module: inspector — files / changes / terminal in a collapsible right rail. */
import { useState } from "react";
import { CHANGES, FILES } from "../data";
import { Icon } from "./kit";

export type InspectorTab = "files" | "changes" | "terminal";

export function Inspector(props: { open: boolean; tab: InspectorTab; onTab: (tab: InspectorTab) => void; onClose: () => void }) {
  const [terminal] = useState(["$ pnpm --dir apps/desktop/web test", "8 files / 34 tests passed", "$ pnpm exec playwright test", "12 passed (28.3s)"]);
  if (!props.open) {
    return null;
  }
  return (
    <aside className="hidden w-[288px] shrink-0 flex-col border-s border-border bg-sidebar/60 lg:flex" aria-label="文件与变更">
      <div className="flex h-9 items-center gap-0.5 border-b border-border px-2 text-[11px]">
        {(["files", "changes", "terminal"] as const).map((tab) => (
          <button key={tab} type="button" onClick={() => props.onTab(tab)} className={`rounded-sm px-2 py-1 ${props.tab === tab ? "bg-muted font-medium text-foreground" : "text-muted-foreground hover:bg-foreground/[0.04]"}`}>
            {tab === "files" ? "文件" : tab === "changes" ? <>变更 <span className="rounded-capsule bg-foreground/10 px-1 text-[10px]">{CHANGES.length}</span></> : "终端"}
          </button>
        ))}
        <button type="button" onClick={props.onClose} aria-label="收起右栏" className="ms-auto flex h-6 w-6 items-center justify-center rounded-sm text-muted-foreground hover:bg-foreground/[0.04]"><Icon name="closePanel" size={12} /></button>
      </div>
      <div className="min-h-0 flex-1 overflow-y-auto px-1.5 py-1">
        {props.tab === "files" ? (
          <div className="font-mono text-[11px]">
            {FILES.map((file) => (
              <div key={file.path} className={`flex items-center gap-1.5 rounded-sm px-1 py-[3px] ${file.kind === "dir" ? "text-muted-foreground" : "text-foreground hover:bg-foreground/[0.04]"}`} style={{ paddingInlineStart: `${6 + file.depth * 10}px` }}>
                <span className="text-muted-foreground/70">{file.kind === "dir" ? "▾" : <Icon name="file" size={11} />}</span>
                <span className="min-w-0 flex-1 truncate">{file.path.split("/").filter(Boolean).at(-1) ?? file.path}</span>
                {file.badge !== undefined ? <span className="text-[10px] text-live">{file.badge}</span> : null}
              </div>
            ))}
          </div>
        ) : props.tab === "changes" ? (
          <div className="px-1 text-[11px]">
            <p className="px-1 py-1 text-[10px] uppercase tracking-[0.08em] text-muted-foreground">本次变更</p>
            {CHANGES.map((change) => (
              <div key={change.path} className="flex items-center gap-1.5 rounded-sm px-1 py-1 font-mono hover:bg-foreground/[0.04]"><Icon name="diff" size={11} /><span className="truncate">{change.path}</span><span className="ms-auto text-[10px] text-muted-foreground">+{change.add} −{change.del}</span></div>
            ))}
          </div>
        ) : (
          <div className="rounded-document bg-[oklch(0.18_0.008_250)] p-2 font-mono text-[11px] text-[oklch(0.92_0.004_250)]">
            {terminal.map((line) => <p key={line} className="whitespace-pre-wrap">{line}</p>)}
          </div>
        )}
      </div>
      <div className="border-t border-border px-2 py-1.5 text-[10px] text-muted-foreground">⌘B 收起 · 文件变更实时同步（skeleton）</div>
    </aside>
  );
}
