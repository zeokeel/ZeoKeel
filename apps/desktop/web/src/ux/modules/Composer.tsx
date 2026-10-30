/** Module: composer — slash commands, @refs, model, send/stop. */
import { useRef, useState } from "react";
import { Icon } from "./kit";

export interface ComposerProps {
  value: string;
  onChange: (value: string) => void;
  onSend: () => void;
  onStop: () => void;
  streaming: boolean;
  disabled: boolean;
}

export function Composer(props: ComposerProps) {
  const [slashOpen, setSlashOpen] = useState(false);
  const input = useRef<HTMLTextAreaElement | null>(null);
  const commands = [
    { name: "/new", desc: "新建会话" },
    { name: "/model", desc: "切换模型" },
    { name: "/permission", desc: "审批预设" },
    { name: "/clear", desc: "清空当前上下文" },
  ];
  return (
    <div className="mx-auto w-full max-w-[760px] px-5 pb-2">
      {slashOpen ? (
        <div role="listbox" aria-label="命令" className="mb-1.5 w-[260px] overflow-hidden rounded-document border border-border bg-popover shadow-[var(--zk-shadow-lift)]">
          {commands.map((c, i) => (
            <button key={c.name} type="button" role="option" aria-selected={i === 0} onClick={() => { props.onChange(`${c.name} `); setSlashOpen(false); input.current?.focus(); }} className={`flex w-full items-center gap-2 px-2 py-1.5 text-start text-[11px] ${i === 0 ? "bg-muted" : "hover:bg-foreground/[0.04]"}`}>
              <span className="font-mono">{c.name}</span><span className="ms-auto text-muted-foreground">{c.desc}</span>
            </button>
          ))}
        </div>
      ) : null}
      <div className="rounded-thread border border-border bg-background px-2.5 pt-1.5 pb-1.5 focus-within:border-foreground/35">
        <div className="flex items-center gap-1.5 text-[13px]">
          <button type="button" aria-label="/ 命令" onClick={() => setSlashOpen((v) => !v)} className="rounded-control px-1 font-mono text-[12px] text-muted-foreground hover:bg-foreground/[0.04]">/</button>
          <textarea
            ref={input}
            rows={1}
            value={props.value}
            disabled={props.disabled}
            placeholder="输入消息，Enter 发送，Shift+Enter 换行；/ 命令，@ 引用文件"
            aria-label="消息输入框"
            onChange={(e) => { props.onChange(e.target.value); setSlashOpen(e.target.value.startsWith("/")); }}
            onKeyDown={(e) => { if (e.key === "Enter" && !e.shiftKey && !e.nativeEvent.isComposing) { e.preventDefault(); props.onSend(); } if (e.key === "Escape") setSlashOpen(false); }}
            className="max-h-32 min-h-[24px] flex-1 resize-none bg-transparent py-0.5 text-[13px] leading-[1.5] outline-none placeholder:text-muted-foreground"
          />
          <span className="shrink-0 font-mono text-[10px] tabular-nums text-muted-foreground">3.1k / 32k</span>
        </div>
        <div className="mt-1 flex items-center gap-1 text-muted-foreground">
          <button type="button" aria-label="附件" className="flex h-6 w-6 items-center justify-center rounded-sm hover:bg-foreground/[0.05]"><Icon name="attach" size={13} /></button>
          <button type="button" className="flex items-center gap-1 rounded-control border border-border px-1.5 py-0.5 text-[10px] hover:bg-foreground/[0.04]"><Icon name="spark" size={11} />DeepSeek V3 <Icon name="chevron" size={10} /></button>
          <span className="ms-auto hidden text-[10px] text-muted-foreground sm:inline">Enter 发送 · Shift+Enter 换行</span>
          {props.streaming ? (
            <button type="button" onClick={props.onStop} className="flex h-6 items-center gap-1 rounded-control border border-destructive/40 px-2 text-[11px] font-medium text-destructive"><Icon name="stop" size={11} />停止</button>
          ) : (
            <button type="button" onClick={props.onSend} disabled={props.disabled || props.value.trim() === ""} className="flex h-6 items-center gap-1 rounded-control bg-foreground px-2 text-[11px] font-medium text-background disabled:opacity-40"><Icon name="send" size={11} />发送</button>
          )}
        </div>
      </div>
    </div>
  );
}
