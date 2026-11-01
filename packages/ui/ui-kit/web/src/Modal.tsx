/**
 * Shared modal primitive (single Dialog implementation, C17).
 *
 * Replaces the private modal in `ui-sessions` and is reused by the composer
 * command-surface dialogs (`2026-11-01-client-command-surface` task 3b).
 */
import { useEffect, useRef, type ReactNode } from "react";

export interface ModalProps {
  open: boolean;
  title: string;
  children: ReactNode;
  /** Footer actions (buttons). */
  actions?: ReactNode;
  onClose: () => void;
  /** Optional extra class for the panel (width). */
  className?: string;
}

export function Modal(props: ModalProps) {
  const panel = useRef<HTMLDivElement | null>(null);
  const { open, onClose } = props;
  useEffect(() => {
    if (!open) {
      return;
    }
    const onKey = (event: KeyboardEvent): void => {
      if (event.key === "Escape") {
        event.preventDefault();
        onClose();
      }
    };
    window.addEventListener("keydown", onKey);
    const previous = document.activeElement;
    panel.current?.focus();
    return () => {
      window.removeEventListener("keydown", onKey);
      if (previous instanceof HTMLElement) {
        previous.focus();
      }
    };
  }, [open, onClose]);
  if (!props.open) {
    return null;
  }
  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-label={props.title}
      className="fixed inset-0 z-50 flex items-center justify-center bg-foreground/20 p-4"
    >
      <div
        ref={panel}
        tabIndex={-1}
        className={`flex w-full max-w-md flex-col gap-3 rounded-control border border-border bg-popover p-4 shadow-[var(--zk-shadow-lift)] outline-none ${props.className ?? ""}`}
      >
        <h2 className="text-[14px] font-semibold text-foreground">{props.title}</h2>
        <div className="flex flex-col gap-2 text-[12px] text-muted-foreground">{props.children}</div>
        {props.actions ? <div className="flex items-center justify-end gap-1.5">{props.actions}</div> : null}
      </div>
    </div>
  );
}
