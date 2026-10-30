/** Module skeleton: command palette. Execution is owned by the app via onRun. */
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
  if (!props.open) return null;
  return (
    <div className="fixed inset-0 z-50 flex justify-center bg-foreground/20 px-6 pt-[18vh]" role="dialog" aria-modal="true" aria-label="命令面板" data-slot="commands">
      <button type="button" aria-label="关闭命令面板" className="absolute inset-0" onClick={props.onClose} />
      <div className="relative h-fit w-full max-w-[520px] rounded-xl border border-border bg-popover p-2 shadow-[var(--zk-shadow-lift)]">
        {props.commands.map((command) => (
          <button key={command.id} type="button" disabled={command.disabled === true} onClick={() => props.onRun(command.id)} className="flex w-full items-center gap-2 rounded-control px-2 py-1.5 text-start text-[12px] hover:bg-muted disabled:opacity-40">
            <span className="flex-1">{command.title}</span>
            {command.hint !== undefined ? <span className="font-mono text-[10px] text-muted-foreground">{command.hint}</span> : null}
          </button>
        ))}
      </div>
    </div>
  );
}

export const components = { "commands.palette": CommandPalette } as const;
