/** Conversation goal bar above the composer (Phase 2b command surface R6). */
import { useUi, type GoalView } from "@zeokeel/ui-kit";

function statusLabel(t: (key: string) => string, status: string): string {
  if (status === "blocked") return t("goalStatusBlocked");
  if (status === "cancelled") return t("goalStatusCancelled");
  if (status === "achieved") return t("goalStatusAchieved");
  return t("goalStatusActive");
}

export function GoalBar(props: { goal: GoalView; onConfirmClear: () => void }) {
  const { t, actions } = useUi();
  return (
    <div role="region" aria-label={t("goalBarTitle")} className="mb-2 flex items-center gap-2 rounded-control border border-border bg-muted px-2 py-1 text-[11px] text-foreground">
      <span className="shrink-0 font-medium">{t("goalBarTitle")}</span>
      <span className="min-w-0 flex-1 truncate" title={props.goal.objective}>{props.goal.objective}</span>
      <span className="shrink-0 text-muted-foreground">{statusLabel(t, props.goal.status)}</span>
      <span className="shrink-0 text-muted-foreground">{t("goalRounds", { rounds: props.goal.rounds, max: props.goal.max_rounds })}</span>
      {props.goal.status === "blocked" ? (
        <button type="button" className="rounded-control border border-border px-1.5 py-0.5" onClick={() => { void actions.updateGoal("resume"); }}>
          {t("goalResume")}
        </button>
      ) : props.goal.status === "active" ? (
        <button type="button" className="rounded-control border border-border px-1.5 py-0.5" onClick={() => { void actions.updateGoal("pause"); }}>
          {t("goalPause")}
        </button>
      ) : null}
      <button type="button" className="rounded-control border border-border px-1.5 py-0.5" onClick={props.onConfirmClear}>
        {t("goalClear")}
      </button>
    </div>
  );
}
