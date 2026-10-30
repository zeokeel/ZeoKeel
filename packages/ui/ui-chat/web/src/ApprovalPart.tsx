import type { DataMessagePartProps } from "@assistant-ui/react";
import { createContext, useContext } from "react";
import { isRecord, readString, useUi } from "@zeokeel/ui-kit";

export type ApprovalDecision = "approved" | "rejected";

export const ApprovalDecideContext = createContext<
  (approvalId: string, decision: ApprovalDecision) => void
>(() => {});

export function ApprovalDataPart(props: DataMessagePartProps) {
  const { t } = useUi();
  const onDecide = useContext(ApprovalDecideContext);
  const data = props.data;
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
  const decided = decision !== "pending";
  const statusLabel =
    decision === "approved"
      ? t("approvalApproved")
      : decision === "rejected"
        ? t("approvalRejected")
        : t("approvalPending");

  return (
    <div
      role="group"
      aria-label={t("approvalTitle")}
      className="mt-3 rounded-document bg-muted px-3 py-2.5"
    >
      <div className="flex items-center gap-2">
        <strong className="text-[13px] font-medium text-foreground">{t("approvalTitle")}</strong>
        <span className="ms-auto text-[12px] text-muted-foreground">{statusLabel}</span>
      </div>
      <p className="mt-2 font-mono text-[13px] font-medium text-foreground">{tool}</p>
      <pre className="zk-tool-pre mt-1">{args}</pre>
      <p className="mt-2 text-[13px] text-muted-foreground">{reason}</p>
      {decided ? null : (
        <div className="mt-2.5 flex gap-2">
          <button
            type="button"
            className="inline-flex items-center rounded-control bg-foreground px-3 py-1.5 text-[13px] font-medium text-background transition-colors hover:bg-foreground/90"
            onClick={() => {
              onDecide(id, "approved");
            }}
          >
            {t("approvalApprove")}
          </button>
          <button
            type="button"
            className="inline-flex items-center rounded-control border border-border px-3 py-1.5 text-[13px] text-foreground transition-colors hover:bg-foreground/[0.04]"
            onClick={() => {
              onDecide(id, "rejected");
            }}
          >
            {t("approvalReject")}
          </button>
        </div>
      )}
    </div>
  );
}
