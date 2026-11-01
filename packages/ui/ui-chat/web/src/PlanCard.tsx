/** Plan mode indicator + local review card (`/plan`, turnTail; never auto-executes). */
import { useState } from "react";
import { useUi } from "@zeokeel/ui-kit";

export function PlanCard() {
  const { t, plan, actions, streaming } = useUi();
  const [feedback, setFeedback] = useState("");
  if (plan === null) return null;
  const visible =
    plan.mode === "active" ||
    plan.status === "pending_review" ||
    plan.status === "rejected" ||
    plan.status === "approved";
  if (!visible) return null;
  const statusLabel =
    plan.status === "pending_review"
      ? t("planStatusPending")
      : plan.status === "approved"
        ? t("planStatusApproved")
        : plan.status === "rejected"
          ? t("planStatusRejected")
          : t("planStatusDraft");
  const disabled = streaming;
  return (
    <section aria-label={t("planCardLabel")} className="mx-auto mb-1 w-full max-w-[760px] rounded-control border border-border px-3 py-2 text-[11px]">
      <div className="flex flex-wrap items-center gap-2">
        <span className="font-medium text-foreground">{t("planCardLabel")}</span>
        <span className={`rounded-sm border border-border px-1 ${plan.mode === "active" ? "text-foreground" : "text-muted-foreground"}`}>
          {plan.mode === "active" ? t("planModeActive") : t("planModeOff")}
        </span>
        <span className="text-muted-foreground">{statusLabel}</span>
        {plan.mode === "active" ? (
          <button
            type="button"
            className="rounded-control border border-border px-1.5 py-0.5 text-foreground"
            disabled={disabled}
            onClick={() => { void actions.updatePlan("off"); }}
          >
            {t("planExit")}
          </button>
        ) : null}
      </div>
      {plan.plan !== "" ? (
        <div role="region" aria-label={t("planMarkdownLabel")} className="mt-1 max-h-48 overflow-y-auto whitespace-pre-wrap break-words text-foreground">
          {plan.plan}
        </div>
      ) : (
        <p className="mt-1 text-muted-foreground">{t("planEmpty")}</p>
      )}
      {plan.feedback !== null ? <p className="mt-1 text-muted-foreground">{t("planFeedback", { feedback: plan.feedback })}</p> : null}
      {plan.status === "pending_review" ? (
        <div className="mt-2 flex flex-wrap items-center gap-2">
          <button type="button" disabled={disabled} className="rounded-control border border-border px-2 py-0.5 text-foreground" onClick={() => { void actions.updatePlan("approve"); }}>
            {t("planApprove")}
          </button>
          <button type="button" disabled={disabled} className="rounded-control border border-border px-2 py-0.5 text-foreground" onClick={() => { void actions.updatePlan("reject", feedback.trim() === "" ? undefined : feedback); }}>
            {t("planReject")}
          </button>
          <button type="button" disabled={disabled} className="rounded-control border border-border px-2 py-0.5 text-foreground" onClick={() => { void actions.updatePlan("continue"); }}>
            {t("planContinue")}
          </button>
          <input
            aria-label={t("planFeedbackLabel")}
            className="min-w-40 flex-1 rounded-control border border-border bg-transparent px-2 py-0.5 text-foreground"
            placeholder={t("planFeedbackLabel")}
            value={feedback}
            disabled={disabled}
            onChange={(event) => { setFeedback(event.target.value); }}
          />
        </div>
      ) : null}
      {plan.status === "rejected" ? (
        <div className="mt-2 flex gap-2">
          <button type="button" disabled={disabled} className="rounded-control border border-border px-2 py-0.5 text-foreground" onClick={() => { void actions.updatePlan("continue"); }}>
            {t("planContinue")}
          </button>
        </div>
      ) : null}
    </section>
  );
}
