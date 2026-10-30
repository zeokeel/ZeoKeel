import { useState } from "react";
import type { ToolCallMessagePartProps } from "@assistant-ui/react";
import { IconChevron, useUi } from "@zeokeel/ui-kit";

const PREVIEW_LIMIT = 240;
type ToolStatus = "running" | "done" | "error";

function preview(text: string, expanded: boolean): string {
  if (expanded || text.length <= PREVIEW_LIMIT) {
    return text;
  }
  return `${text.slice(0, PREVIEW_LIMIT)}…`;
}

function stringifyResult(value: unknown): string | null {
  if (value === undefined) {
    return null;
  }
  if (typeof value === "string") {
    return value;
  }
  try {
    return JSON.stringify(value, null, 2);
  } catch {
    return String(value);
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

export function ToolCallPart(props: ToolCallMessagePartProps) {
  const { t } = useUi();
  const [expanded, setExpanded] = useState(false);
  const status = statusOf(props.status.type);
  const statusLabel =
    status === "running" ? t("toolRunning") : status === "error" ? t("toolFailed") : t("toolDone");
  const result = stringifyResult(props.result);
  const expandable =
    props.argsText.length > PREVIEW_LIMIT || (result?.length ?? 0) > PREVIEW_LIMIT;
  const dotClass =
    status === "error"
      ? "bg-destructive"
      : status === "running"
        ? "bg-live zk-pulse-dot"
        : "bg-muted-foreground";

  return (
    <div className="zk-tool" data-status={status}>
      <div className="flex items-center gap-2">
        <span className={`h-1.5 w-1.5 shrink-0 rounded-capsule ${dotClass}`} aria-hidden="true" />
        <span className="font-mono text-[13px] font-medium text-foreground">{props.toolName}</span>
        <span className="text-[12px] text-muted-foreground">{statusLabel}</span>
        {expandable ? (
          <button
            type="button"
            aria-expanded={expanded}
            onClick={() => {
              setExpanded((value) => !value);
            }}
            className="ms-auto inline-flex items-center gap-1 rounded-sm px-1.5 py-0.5 text-[12px] text-muted-foreground transition-colors hover:bg-foreground/[0.05] hover:text-foreground"
          >
            <IconChevron
              className={expanded ? "rotate-180 transition-transform" : "transition-transform"}
            />
            {expanded ? t("toolCollapse") : t("toolExpand")}
          </button>
        ) : null}
      </div>
      <div className="mt-2.5">
        <div className="text-[12px] text-muted-foreground">{t("toolArgs")}</div>
        <pre className="zk-tool-pre mt-1">{preview(props.argsText, expanded)}</pre>
      </div>
      {result !== null ? (
        <div className="mt-2.5">
          <div className="text-[12px] text-muted-foreground">{t("toolResult")}</div>
          <pre className="zk-tool-pre mt-1">{preview(result, expanded)}</pre>
        </div>
      ) : null}
    </div>
  );
}
