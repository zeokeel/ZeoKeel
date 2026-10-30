/** Command palette (⌘K): keyboard-first, app owns command execution. */
import { useEffect, useRef, useState } from "react";
import { IconClose } from "@zeokeel/ui-kit";

export interface CommandItem {
  id: string;
  title: string;
  group: string;
  hint?: string;
  disabled?: boolean;
}

export interface CommandPaletteProps {
  open: boolean;
  commands: CommandItem[];
  onRun: (id: string) => void;
  onClose: () => void;
}

export function CommandPalette(props: CommandPaletteProps) {
  const [query, setQuery] = useState("");
  const [active, setActive] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);
  const visible = props.commands.filter((command) =>
    `${command.title} ${command.group} ${command.hint ?? ""}`.toLowerCase().includes(query.trim().toLowerCase()),
  );

  useEffect(() => {
    if (props.open) {
      setQuery("");
      setActive(0);
      window.setTimeout(() => inputRef.current?.focus(), 0);
    }
  }, [props.open]);

  if (!props.open) return null;
  const run = (id: string): void => { props.onRun(id); props.onClose(); };
  const onKeyDown = (event: React.KeyboardEvent<HTMLInputElement>): void => {
    if (event.key === "Escape") { event.preventDefault(); props.onClose(); return; }
    if (event.key === "ArrowDown") { event.preventDefault(); setActive((value) => Math.min(value + 1, Math.max(visible.length - 1, 0))); return; }
    if (event.key === "ArrowUp") { event.preventDefault(); setActive((value) => Math.max(value - 1, 0)); return; }
    if (event.key === "Enter") {
      event.preventDefault();
      const command = visible[active];
      if (command !== undefined && command.disabled !== true) run(command.id);
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex justify-center bg-foreground/20 px-6 pt-[18vh]" role="dialog" aria-modal="true" aria-label="命令面板" data-slot="commands">
      <button type="button" aria-label="关闭命令面板" className="absolute inset-0" onClick={props.onClose} />
      <div className="relative h-fit w-full max-w-[520px] rounded-xl border border-border bg-popover p-2 shadow-[var(--zk-shadow-lift)]">
        <div className="flex items-center gap-2 border-b border-border px-2 pb-2">
          <input
            ref={inputRef}
            value={query}
            onChange={(event) => { setQuery(event.target.value); setActive(0); }}
            onKeyDown={onKeyDown}
            placeholder="搜索命令…"
            aria-label="搜索命令"
            className="min-w-0 flex-1 bg-transparent py-1 text-[13px] text-foreground outline-none placeholder:text-muted-foreground"
          />
          <button type="button" onClick={props.onClose} aria-label="关闭" className="inline-flex h-6 w-6 items-center justify-center rounded-sm text-muted-foreground hover:bg-foreground/[0.05] hover:text-foreground"><IconClose /></button>
        </div>
        <div className="mt-1 max-h-[320px] overflow-y-auto" role="listbox" aria-label="命令列表">
          {visible.length === 0 ? <p className="px-2 py-6 text-center text-[12px] text-muted-foreground">没有匹配的命令</p> : null}
          {visible.map((command, index) => (
            <button
              key={command.id}
              type="button"
              role="option"
              aria-selected={index === active}
              disabled={command.disabled === true}
              onMouseEnter={() => { setActive(index); }}
              onClick={() => { run(command.id); }}
              className={`flex w-full items-center gap-2 rounded-control px-2 py-1.5 text-start text-[12px] ${index === active ? "bg-muted" : ""} hover:bg-muted disabled:opacity-40`}
            >
              <span className="w-[64px] shrink-0 text-[10px] text-muted-foreground">{command.group}</span>
              <span className="flex-1">{command.title}</span>
              {command.hint !== undefined ? <span className="font-mono text-[10px] text-muted-foreground">{command.hint}</span> : null}
            </button>
          ))}
        </div>
      </div>
    </div>
  );
}

export const components = { "commands.palette": CommandPalette } as const;
