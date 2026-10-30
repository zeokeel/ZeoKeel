import { useState } from "react";
import type { ToolCallMessagePartProps } from "@assistant-ui/react";
import { IconChevron, useUi } from "@zeokeel/ui-kit";

const PREVIEW_LIMIT = 120;
type ToolStatus = "running" | "done" | "error";

function preview(text: string): string {
  const singleLine = text.replace(/\s+/g, " ").trim();
  if (singleLine.length <= PREVIEW_LIMIT) {
    return singleLine;
  }
  return `${singleLine.slice(0, PREVIEW_LIMIT)}…`;
}

function stringifyResult(value: unknown): string | null {
  if (value === undefined) {
    return null;
  }
  if (typeof value === "string") {
    return value;
  }
  try {
    const encoded: unknown = JSON.stringify(value);
    return typeof encoded === "string" ? encoded : null;
  } catch {
    return "unserializable";
  }
}

function statusOf(type: string): ToolStatus {
  if (type === "incomplete") {
    return "error";
  }
  if (type === "complete") {
    return "done";
  }
  return "running";
}

/** Dense single-line tool row; expand to inspect args/result. */
export function ToolCallPart(props: ToolCallMessagePartProps) {
  const { t } = useUi();
  const [expanded, setExpanded] = useState(false);
  const status = statusOf(props.status.type);
  const statusLabel =
    status === "running" ? t("toolRunning") : status === "error" ? t("toolFailed") : t("toolDone");
  const result = stringifyResult(props.result);
  const summary = result ?? props.argsText;
  const dotClass =
    status === "error"
      ? "bg-destructive"
      : status === "running"
        ? "bg-live zk-pulse-dot"
        : "bg-muted-foreground";

  return (
    <div className="zk-tool px-2 py-1">
      <button
        type="button"
        aria-expanded={expanded}
        onClick={() => {
          setExpanded((value) => !value);
        }}
        className="flex w-full items-center gap-2 text-start"
      >
        <span className={`h-1.5 w-1.5 shrink-0 rounded-capsule ${dotClass}`} aria-hidden="true" />
        <span className="shrink-0 font-mono text-[11.5px] font-medium text-foreground">
          {props.toolName}
        </span>
        <span className="min-w-0 flex-1 truncate text-[11.5px] text-muted-foreground">
          {preview(summary)}
        </span>
        <span className="shrink-0 text-[11px] text-muted-foreground">{statusLabel}</span>
        <IconChevron
          className={`shrink-0 text-muted-foreground ${
            expanded ? "rotate-180 transition-transform" : "transition-transform"
          }`}
        />
      </button>
      {expanded ? (
        <div className="mt-1.5 flex flex-col gap-1.5 ps-[22px]">
          <div>
            <div className="text-[11px] text-muted-foreground">{t("toolArgs")}</div>
            <pre className="zk-tool-pre mt-0.5">{props.argsText}</pre>
          </div>
          {result !== null ? (
            <div>
              <div className="text-[11px] text-muted-foreground">{t("toolResult")}</div>
              <pre className="zk-tool-pre mt-0.5">{result}</pre>
            </div>
          ) : null}
        </div>
      ) : null}
    </div>
  );
}
