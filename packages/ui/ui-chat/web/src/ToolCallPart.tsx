import type { DataMessagePartProps } from "@assistant-ui/react";
import { useState } from "react";
import { isRecord, readString, useUi } from "@zeokeel/ui-kit";

const PREVIEW_LIMIT = 120;

type ToolStatus = "pending" | "running" | "success" | "failed" | "denied" | "cancelled";

function preview(text: string): string {
  const singleLine = text.replace(/\s+/g, " ").trim();
  if (singleLine.length <= PREVIEW_LIMIT) {
    return singleLine;
  }
  return `${singleLine.slice(0, PREVIEW_LIMIT)}…`;
}

function statusOf(raw: unknown): ToolStatus {
  switch (raw) {
    case "pending":
    case "running":
    case "success":
    case "failed":
    case "denied":
    case "cancelled":
      return raw;
    case "done":
      return "success";
    case "error":
      return "failed";
    default:
      return "success";
  }
}

function statusLabel(t: (key: string) => string, status: ToolStatus): string {
  switch (status) {
    case "pending":
      return t("toolPending");
    case "running":
      return t("toolRunning");
    case "success":
      return t("toolDone");
    case "failed":
      return t("toolFailed");
    case "denied":
      return t("toolDenied");
    case "cancelled":
      return t("toolCancelled");
    default:
      return status;
  }
}

interface ToolData {
  name: string;
  args: string;
  result: string;
  status: ToolStatus;
  unparsed: boolean;
}

function readTool(data: unknown): ToolData | null {
  if (!isRecord(data)) {
    return null;
  }
  return {
    name: readString(data, "name") ?? "tool",
    args: readString(data, "args") ?? "",
    result: readString(data, "result") ?? "",
    status: statusOf(data["status"]),
    unparsed: data["unparsed"] === true,
  };
}

/** Dense tool row; expand to inspect args/result (persisted and live states). */
export function ToolDataPart(props: DataMessagePartProps) {
  const { t } = useUi();
  const [expanded, setExpanded] = useState(false);
  const tool = readTool(props.data);
  if (tool === null) {
    return null;
  }
  const summary = tool.result === "" ? tool.args : tool.result;
  const dotClass =
    tool.status === "failed"
      ? "bg-destructive"
      : tool.status === "cancelled" || tool.status === "denied"
        ? "bg-muted-foreground"
        : tool.status === "running" || tool.status === "pending"
          ? "bg-state-business-primary"
          : "bg-success";
  return (
    <div className="mt-2 rounded-document border border-border/60" data-tool-status={tool.status}>
      <button
        type="button"
        className="flex w-full items-center gap-2 px-3 py-1.5 text-left"
        aria-expanded={expanded}
        onClick={() => {
          setExpanded((open) => !open);
        }}
      >
        <span className={`h-1.5 w-1.5 flex-none rounded-capsule ${dotClass}`} aria-hidden="true" />
        <span className="min-w-0 flex-1 truncate font-mono text-[12px] text-foreground">
          {tool.name}
        </span>
        <span className="text-[11px] text-muted-foreground">{statusLabel(t, tool.status)}</span>
      </button>
      {tool.unparsed ? (
        <p className="px-3 pb-1 text-[12px] text-destructive">{t("toolUnparsed")}</p>
      ) : null}
      {tool.status === "cancelled" && tool.result !== "" ? (
        <p className="px-3 pb-1 text-[12px] text-muted-foreground">{t("toolCancelledSideEffects")}</p>
      ) : null}
      {!expanded && summary !== "" ? (
        <p className="truncate px-3 pb-1.5 text-[12px] text-muted-foreground">{preview(summary)}</p>
      ) : null}
      {expanded ? (
        <div className="px-3 pb-2">
          {tool.args !== "" ? (
            <>
              <p className="mt-1 text-[11px] font-medium text-muted-foreground">{t("toolArgs")}</p>
              <pre className="zk-tool-pre mt-0.5">{tool.args}</pre>
            </>
          ) : null}
          {tool.result !== "" ? (
            <>
              <p className="mt-2 text-[11px] font-medium text-muted-foreground">{t("toolResult")}</p>
              <pre className="zk-tool-pre mt-0.5">{tool.result}</pre>
            </>
          ) : null}
        </div>
      ) : null}
    </div>
  );
}
