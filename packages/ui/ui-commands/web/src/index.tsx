/** Command palette (⌘K): keyboard-first, app owns command execution. */
import { useRef, useState } from "react";
import { IconClose, useDialogFocus, useUi } from "@zeokeel/ui-kit";

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
  const { t } = useUi();
  const [query, setQuery] = useState("");
  const [active, setActive] = useState(0);
  const dialogRef = useRef<HTMLDivElement>(null);
  useDialogFocus(dialogRef, props.onClose, props.open);
  const visible = props.commands.filter((command) =>
    `${command.title} ${command.group} ${command.hint ?? ""}`.toLowerCase().includes(query.trim().toLowerCase()),
  );

  if (!props.open) return null;
  const activeCommand = visible[active];
  const run = (id: string): void => { props.onRun(id); props.onClose(); };
  const onKeyDown = (event: React.KeyboardEvent<HTMLInputElement>): void => {
    if (event.key === "ArrowDown") {
      event.preventDefault();
      setActive((value) => Math.min(value + 1, Math.max(visible.length - 1, 0)));
      return;
    }
    if (event.key === "ArrowUp") {
      event.preventDefault();
      setActive((value) => Math.max(value - 1, 0));
      return;
    }
    if (event.key === "Enter") {
      event.preventDefault();
      if (activeCommand !== undefined && activeCommand.disabled !== true) run(activeCommand.id);
    }
  };

  return (
    <div
      ref={dialogRef}
      className="fixed inset-0 z-50 flex justify-center bg-foreground/20 px-6 pt-[18vh]"
      role="dialog"
      aria-modal="true"
      aria-label={t("commandPaletteLabel")}
      data-slot="commands"
    >
      <button type="button" aria-label={t("commandClose")} className="absolute inset-0" onClick={props.onClose} />
      <div className="relative h-fit w-full max-w-[520px] rounded-xl border border-border bg-popover p-2 shadow-[var(--zk-shadow-lift)]">
        <div className="flex items-center gap-2 border-b border-border px-2 pb-2">
          <input
            data-autofocus
            value={query}
            onChange={(event) => { setQuery(event.target.value); setActive(0); }}
            onKeyDown={onKeyDown}
            placeholder={t("commandSearchPlaceholder")}
            aria-label={t("commandSearchAria")}
            aria-controls="zk-command-list"
            aria-activedescendant={activeCommand === undefined ? undefined : `zk-command-${activeCommand.id}`}
            className="min-w-0 flex-1 bg-transparent py-1 text-[13px] text-foreground outline-none placeholder:text-muted-foreground"
          />
          <button type="button" onClick={props.onClose} aria-label={t("close")} className="inline-flex h-6 w-6 items-center justify-center rounded-sm text-muted-foreground hover:bg-foreground/[0.05] hover:text-foreground">
            <IconClose />
          </button>
        </div>
        <div id="zk-command-list" className="mt-1 max-h-[320px] overflow-y-auto" role="listbox" aria-label={t("commandList")}>
          {visible.length === 0 ? <p className="px-2 py-6 text-center text-[12px] text-muted-foreground">{t("commandNoMatches")}</p> : null}
          {visible.map((command, index) => (
            <button
              key={command.id}
              id={`zk-command-${command.id}`}
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
