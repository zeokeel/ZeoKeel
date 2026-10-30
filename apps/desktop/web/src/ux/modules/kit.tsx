/** Skeleton-shared primitives; production equivalents belong in @zeokeel/ui-kit. */
import { BRAND } from "../../generated/brand";

export type IconName =
  | "search" | "plus" | "sliders" | "panel" | "command" | "attach" | "send" | "stop"
  | "folder" | "file" | "diff" | "terminal" | "chevron" | "more" | "spark" | "branch"
  | "check" | "close" | "closePanel" | "task";

const PATHS: Record<IconName, string> = {
  search: "M7 12a5 5 0 1 0 0-10 5 5 0 0 0 0 10Zm4-1 3 3",
  plus: "M8 3v10M3 8h10",
  sliders: "M3 5h10M3 11h10M6 3v4M10 9v4",
  panel: "M3 3h10v10H3zM9 3v10",
  command: "M6 3a2 2 0 1 0 0 4h4a2 2 0 1 0 0-4 2 2 0 0 0-2 2v6a2 2 0 1 0 2-2H6a2 2 0 1 0 0 4 2 2 0 0 0 2-2V5",
  attach: "M10.5 4.5 5 10a3 3 0 0 0 4 4l5-5a4.5 4.5 0 0 0-6.5-6.5L2.5 8",
  send: "M3 13 14 8 3 3l2 5-2 5Z",
  stop: "M4 4h8v8H4z",
  folder: "M2 4.5c0-.6.5-1 1-1h3l1 1.2h5c.6 0 1 .4 1 1v6.8c0 .6-.4 1-1 1H3c-.5 0-1-.4-1-1V4.5Z",
  file: "M4 2h5l3 3v9H4zM9 2v3h3",
  diff: "M4 5h8M4 11h8M6 2v3M10 9v3",
  terminal: "M3 4l4 4-4 4M8 12h5",
  chevron: "M4 6l4 4 4-4",
  more: "M3 8h.01M8 8h.01M13 8h.01",
  spark: "M8 2l1.3 4.7L14 8l-4.7 1.3L8 14l-1.3-4.7L2 8l4.7-1.3L8 2Z",
  branch: "M5 3v10M5 6c0 3 3 3 6 3v4",
  check: "M3 8l3 3 6-6",
  close: "M4 4l8 8M12 4l-8 8",
  closePanel: "M3 3h10v10H3zM10 3v10M7 6l-2 2 2 2",
  task: "M3 4h10v10H3zM6 8l1.5 1.5L10 7",
};

export function Icon({ name, size = 14 }: { name: IconName; size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 16 16" aria-hidden="true" fill="none" stroke="currentColor" strokeWidth="1.3" strokeLinecap="round" strokeLinejoin="round">
      <path d={PATHS[name]} />
    </svg>
  );
}

export function Logo() {
  return <img src={BRAND.logoLight} alt="" className="h-6 w-6 rounded-document object-contain" />;
}

export function IconButton(props: { name: IconName; label: string; onClick?: () => void; size?: number }) {
  return (
    <button
      type="button"
      aria-label={props.label}
      title={props.label}
      onClick={props.onClick}
      className="flex h-6 w-6 items-center justify-center rounded-sm text-muted-foreground hover:bg-foreground/[0.05] hover:text-foreground"
    >
      <Icon name={props.name} size={props.size ?? 13} />
    </button>
  );
}

export const focusRing = "focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-foreground/50";
