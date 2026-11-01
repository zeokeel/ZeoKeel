import type { DataMessagePartProps } from "@assistant-ui/react";
import { createContext, useContext, useEffect, useRef, useState } from "react";
import { isRecord, readString, useUi } from "@zeokeel/ui-kit";

export type ApprovalDecision = "allow_once" | "allow_session" | "reject" | "cancel";

export const ApprovalDecideContext = createContext<
  (approvalId: string, decision: ApprovalDecision) => void
>(() => undefined);

interface ApprovalData {
  id: string;
  tool: string;
  args: string;
  reason: string;
  auto: boolean;
  deadlineMs: number;
  timedOut: boolean;
  decision: string;
  summary: string;
  cwd: string;
  readPaths: string[];
  writePaths: string[];
  networkTargets: string[];
  networkUndetermined: boolean;
  risk: string;
  diff: string | null;
}

function stringArray(data: Record<string, unknown>, key: string): string[] {
  const value = data[key];
  if (!Array.isArray(value)) {
    return [];
  }
  const items: string[] = [];
  for (const item of value) {
    if (typeof item === "string" && item.trim() !== "") {
      items.push(item);
    }
  }
  return items;
}

function readApproval(data: unknown): ApprovalData | null {
  if (!isRecord(data)) {
    return null;
  }
  const id = readString(data, "id");
  const tool = readString(data, "tool");
  const args = readString(data, "args");
  const reason = readString(data, "reason");
  const decision = readString(data, "decision");
  if (id === null || tool === null || args === null || reason === null || decision === null) {
    return null;
  }
  const metadata = isRecord(data["metadata"]) ? data["metadata"] : {};
  return {
    id,
    tool,
    args,
    reason,
    auto: data["auto"] === true,
    deadlineMs: typeof data["deadline_ms"] === "number" ? data["deadline_ms"] : 0,
    timedOut: data["timed_out"] === true,
    decision,
    summary: readString(metadata, "summary") ?? "",
    cwd: readString(metadata, "cwd") ?? "",
    readPaths: stringArray(metadata, "read_paths"),
    writePaths: stringArray(metadata, "write_paths"),
    networkTargets: stringArray(metadata, "network_targets"),
    networkUndetermined: metadata["network_undetermined"] === true,
    risk: readString(metadata, "risk") ?? "",
    diff: readString(metadata, "diff"),
  };
}

function statusLabel(t: (key: string, vars?: Record<string, string | number>) => string, data: ApprovalData): string {
  if (data.auto && data.decision === "allow_session") {
    return t("approvalAutoApproved");
  }
  switch (data.decision) {
    case "approved":
    case "allow_once":
      return t("approvalApproved");
    case "allow_session":
      return t("approvalApprovedSession");
    case "rejected":
    case "reject":
      return data.timedOut ? t("approvalTimedOut") : t("approvalRejected");
    case "cancel":
      return t("approvalCancelled");
    default:
      return t("approvalPending");
  }
}

function riskLabel(t: (key: string) => string, risk: string): string {
  switch (risk) {
    case "low":
      return t("approvalRiskLow");
    case "medium":
      return t("approvalRiskMedium");
    case "high":
      return t("approvalRiskHigh");
    default:
      return risk;
  }
}

function PathList({ label, paths }: { label: string; paths: string[] }) {
  if (paths.length === 0) {
    return null;
  }
  return (
    <div className="mt-1 text-[12px] text-muted-foreground">
      <span className="font-medium text-foreground">{label}: </span>
      <span className="font-mono">{paths.join(", ")}</span>
    </div>
  );
}

export function ApprovalDataPart(props: DataMessagePartProps) {
  const { t } = useUi();
  const onDecide = useContext(ApprovalDecideContext);
  const data = readApproval(props.data);
  const rejectRef = useRef<HTMLButtonElement>(null);
  const [diffOpen, setDiffOpen] = useState(false);
  const [now, setNow] = useState(() => Date.now());
  const pending = data !== null && data.decision === "pending" && !data.auto;
  useEffect(() => {
    if (!pending) {
      return;
    }
    const timer = setInterval(() => {
      setNow(Date.now());
    }, 1000);
    return () => {
      clearInterval(timer);
    };
  }, [pending]);
  useEffect(() => {
    if (pending) {
      rejectRef.current?.focus();
    }
  }, [pending]);
  const approvalId = data?.id ?? "";
  useEffect(() => {
    if (!pending || approvalId === "") {
      return;
    }
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        onDecide(approvalId, "reject");
      }
    };
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("keydown", onKey);
    };
  }, [pending, onDecide, approvalId]);
  if (data === null) {
    return null;
  }
  const remainingSeconds =
    data.deadlineMs > 0 ? Math.max(0, Math.ceil((data.deadlineMs - now) / 1000)) : null;
  const outsideWorkspace =
    data.cwd !== "" && data.writePaths.some((path) => path.startsWith("/"));
  const decided = !pending;
  return (
    <div
      role="group"
      aria-label={t("approvalTitle")}
      className="mt-3 rounded-document bg-muted px-3 py-2.5"
    >
      <div className="flex items-center gap-2">
        <strong className="text-[13px] font-medium text-foreground">{t("approvalTitle")}</strong>
        <span className="ms-auto text-[12px] text-muted-foreground">{statusLabel(t, data)}</span>
      </div>
      {data.auto ? null : (
        <p className="mt-2 font-mono text-[13px] font-medium text-foreground">{data.tool}</p>
      )}
      {data.summary !== "" ? (
        <p className="mt-1 text-[13px] text-foreground">{data.summary}</p>
      ) : null}
      <pre className="zk-tool-pre mt-1">{data.args}</pre>
      <p className="mt-2 text-[13px] text-muted-foreground">{data.reason}</p>
      {data.cwd !== "" ? (
        <div className="mt-1 text-[12px] text-muted-foreground">
          <span className="font-medium text-foreground">{t("approvalCwd")}: </span>
          <span className="font-mono">{data.cwd}</span>
          {outsideWorkspace ? (
            <span className="ms-2 text-destructive">{t("approvalOutsideWorkspace")}</span>
          ) : null}
        </div>
      ) : null}
      <PathList label={t("approvalReadPaths")} paths={data.readPaths} />
      <PathList label={t("approvalWritePaths")} paths={data.writePaths} />
      {data.networkTargets.length > 0 ? (
        <PathList label={t("approvalNetworkTargets")} paths={data.networkTargets} />
      ) : null}
      {data.networkUndetermined ? (
        <p className="mt-1 text-[12px] text-muted-foreground">{t("approvalNetworkUndetermined")}</p>
      ) : null}
      {data.diff !== null ? (
        <div className="mt-2">
          <button
            type="button"
            className="text-[12px] text-foreground underline"
            aria-expanded={diffOpen}
            onClick={() => {
              setDiffOpen((open) => !open);
            }}
          >
            {diffOpen ? t("toolCollapse") : t("approvalDiffExpand")}
          </button>
          {diffOpen ? <pre className="zk-tool-pre mt-1">{data.diff}</pre> : null}
        </div>
      ) : null}
      {data.summary === "" && !data.auto ? (
        <p className="mt-2 text-[12px] text-muted-foreground">{t("approvalMetadataMissing")}</p>
      ) : null}
      <div className="mt-2 flex items-center gap-2 text-[12px] text-muted-foreground">
        {data.risk !== "" ? (
          <span>
            {t("approvalRisk")}: {riskLabel(t, data.risk)}
          </span>
        ) : null}
        {pending && remainingSeconds !== null ? (
          <span className="ms-auto">
            {t("approvalRemaining", { seconds: remainingSeconds })}
          </span>
        ) : null}
      </div>
      {decided ? null : (
        <div className="mt-2.5 flex flex-wrap gap-2">
          <button
            type="button"
            data-decision="allow_once"
            className="inline-flex items-center rounded-control bg-foreground px-3 py-1.5 text-[13px] font-medium text-background transition-colors hover:bg-foreground/90"
            onClick={() => {
              onDecide(data.id, "allow_once");
            }}
          >
            {t("approvalApprove")}
          </button>
          <button
            type="button"
            data-decision="allow_session"
            className="inline-flex items-center rounded-control border border-border px-3 py-1.5 text-[13px] text-foreground transition-colors hover:bg-foreground/[0.04]"
            onClick={() => {
              onDecide(data.id, "allow_session");
            }}
          >
            {t("approvalAllowSession")}
          </button>
          <button
            ref={rejectRef}
            type="button"
            className="inline-flex items-center rounded-control border border-border px-3 py-1.5 text-[13px] text-foreground transition-colors hover:bg-foreground/[0.04]"
            onClick={() => {
              onDecide(data.id, "reject");
            }}
          >
            {t("approvalReject")}
          </button>
          <button
            type="button"
            className="inline-flex items-center rounded-control px-3 py-1.5 text-[13px] text-muted-foreground transition-colors hover:bg-foreground/[0.04]"
            onClick={() => {
              onDecide(data.id, "cancel");
            }}
          >
            {t("approvalCancel")}
          </button>
        </div>
      )}
    </div>
  );
}
