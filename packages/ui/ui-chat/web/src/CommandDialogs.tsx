/** `/model` and `/permission` popupSelect dialogs (command surface R4/R5). */
import { useEffect, useRef, useState } from "react";
import { Modal, useUi, type ConversationOverride } from "@zeokeel/ui-kit";

const RECENT_KEY = "zk.recentModels.v1";
const MAX_RECENT = 8;

function loadRecent(): string[] {
  try {
    const raw = window.localStorage.getItem(RECENT_KEY);
    if (raw === null) return [];
    const parsed: unknown = JSON.parse(raw);
    return Array.isArray(parsed) ? parsed.filter((item): item is string => typeof item === "string").slice(0, MAX_RECENT) : [];
  } catch {
    return [];
  }
}

function pushRecent(model: string): void {
  if (model.trim() === "") return;
  const next = [model, ...loadRecent().filter((item) => item !== model)].slice(0, MAX_RECENT);
  try {
    window.localStorage.setItem(RECENT_KEY, JSON.stringify(next));
  } catch {
    // Quota/private mode: recents stay in memory only for this dialog session.
  }
}

const EFFORTS = ["", "minimal", "low", "medium", "high", "xhigh", "max"] as const;

export function ModelDialog(props: { open: boolean; onClose: () => void }) {
  const { t, actions, gatewayModels, modelLabel, meta } = useUi();
  const enterprise = meta.edition === "enterprise";
  const [current, setCurrent] = useState<ConversationOverride | null>(null);
  const [model, setModel] = useState("");
  const [effort, setEffort] = useState("");
  const [recent, setRecent] = useState<string[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  useEffect(() => {
    if (!props.open) return undefined;
    const timer = window.setTimeout(() => {
      setError(null);
      setRecent(loadRecent());
      void actions.getConversationOverride().then((value) => {
        setCurrent(value);
        setModel(value?.model ?? "");
        setEffort(value?.reasoning_effort ?? "");
      });
    }, 0);
    return () => { window.clearTimeout(timer); };
  }, [actions, props.open]);
  const apply = (): void => {
    if (busy) return;
    setBusy(true);
    setError(null);
    const patch: { model?: string; reasoning_effort?: string } = {
      model: model.trim(),
      reasoning_effort: effort,
    };
    void actions.setConversationOverride(patch).then((ok) => {
      setBusy(false);
      if (!ok) {
        setError(t("overrideApplyFailed"));
        return;
      }
      if (model.trim() !== "") pushRecent(model.trim());
      props.onClose();
    });
  };
  return (
    <Modal open={props.open} title={t("overrideTitle")} onClose={props.onClose} actions={(
      <>
        <button type="button" className="rounded-control border border-border px-2 py-1" onClick={props.onClose}>{t("overrideCancel")}</button>
        <button type="button" disabled={busy} className="rounded-control border border-border bg-foreground px-2 py-1 text-background disabled:opacity-50" onClick={apply}>{t("overrideSave")}</button>
      </>
    )}>
      <p className="text-[11px]">{t("overrideModelLabel")}：{modelLabel}（{current?.is_session_override === true ? t("overrideSession") : t("overrideDefault")}）</p>
      {enterprise ? (
        <label className="flex flex-col gap-1">
          {t("overrideModelLabel")}
          <select aria-label={t("overrideModelLabel")} value={model} onChange={(event) => { setModel(event.target.value); }} className="rounded-control border border-border bg-background px-2 py-1 text-foreground">
            <option value="">{t("overrideDefault")}</option>
            {gatewayModels.map((id) => <option key={id} value={id}>{id}</option>)}
          </select>
        </label>
      ) : (
        <label className="flex flex-col gap-1">
          {t("overrideModelLabel")}
          <input aria-label={t("overrideModelLabel")} value={model} placeholder={t("overrideModelPlaceholder")} onChange={(event) => { setModel(event.target.value); }} className="rounded-control border border-border bg-background px-2 py-1 text-foreground" />
          <span className="text-[11px] text-muted-foreground">{t("overrideModelHint")}</span>
        </label>
      )}
      {!enterprise && recent.length > 0 ? (
        <div className="flex flex-wrap items-center gap-1">
          <span className="text-[11px]">{t("overrideRecentModels")}:</span>
          {recent.map((id) => (
            <button key={id} type="button" className="rounded-control border border-border px-1.5 py-0.5 text-[11px]" onClick={() => { setModel(id); }}>{id}</button>
          ))}
        </div>
      ) : null}
      <label className="flex flex-col gap-1">
        {t("overrideEffortLabel")}
        <select aria-label={t("overrideEffortLabel")} value={effort} onChange={(event) => { setEffort(event.target.value); }} className="rounded-control border border-border bg-background px-2 py-1 text-foreground">
          {EFFORTS.map((value) => <option key={value} value={value}>{value === "" ? t("overrideEffortDefault") : value}</option>)}
        </select>
      </label>
      {error !== null ? <p role="alert" className="text-destructive">{error}</p> : null}
    </Modal>
  );
}

const PRESETS = [
  { value: "strict", labelKey: "overridePermissionStrict" },
  { value: "auto_read_only", labelKey: "overridePermissionAutoReadOnly" },
  { value: "ask", labelKey: "overridePermissionAsk" },
] as const;

export function PermissionDialog(props: { open: boolean; onClose: () => void }) {
  const { t, actions } = useUi();
  const [current, setCurrent] = useState<ConversationOverride | null>(null);
  const [policy, setPolicy] = useState("");
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    if (!props.open) return undefined;
    const timer = window.setTimeout(() => {
      setError(null);
      void actions.getConversationOverride().then((value) => {
        setCurrent(value);
        setPolicy(value?.approval_policy ?? "");
      });
    }, 0);
    return () => { window.clearTimeout(timer); };
  }, [actions, props.open]);
  const apply = (): void => {
    void actions.setConversationOverride({ approval_policy: policy }).then((ok) => {
      if (!ok) {
        setError(t("overrideApplyFailed"));
        return;
      }
      props.onClose();
    });
  };
  return (
    <Modal open={props.open} title={t("overrideTitle")} onClose={props.onClose} actions={(
      <>
        <button type="button" className="rounded-control border border-border px-2 py-1" onClick={props.onClose}>{t("overrideCancel")}</button>
        <button type="button" className="rounded-control border border-border bg-foreground px-2 py-1 text-background" onClick={apply}>{t("overrideSave")}</button>
      </>
    )}>
      <fieldset>
        <legend className="mb-1">{t("overridePermissionLabel")}（{current?.is_session_override === true ? t("overrideSession") : t("overrideDefault")}）</legend>
        <div className="flex flex-col gap-1">
          <label className="flex items-center gap-1">
            <input type="radio" name="approval-policy" checked={policy === ""} onChange={() => { setPolicy(""); }} />
            {t("overrideDefault")}
          </label>
          {PRESETS.map((preset) => (
            <label key={preset.value} className="flex items-center gap-1">
              <input type="radio" name="approval-policy" checked={policy === preset.value} onChange={() => { setPolicy(preset.value); }} />
              {t(preset.labelKey)}
            </label>
          ))}
        </div>
      </fieldset>
      <p className="text-[11px]">{t("overridePermissionNote")}</p>
      {error !== null ? <p role="alert" className="text-destructive">{error}</p> : null}
    </Modal>
  );
}

/** `/goal` dialog: view current goal, set a new objective (inline confirm), clear. */
export function GoalDialog(props: { open: boolean; initialObjective: string; onClose: () => void }) {
  const { t, actions, goal } = useUi();
  const [objective, setObjective] = useState(props.initialObjective);
  const [confirmClear, setConfirmClear] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const actionsRef = useRef(actions);
  useEffect(() => {
    actionsRef.current = actions;
  }, [actions]);
  useEffect(() => {
    if (!props.open) return undefined;
    const timer = window.setTimeout(() => {
      setObjective(props.initialObjective);
      setConfirmClear(false);
      setError(null);
      void actionsRef.current.refreshGoal();
    }, 0);
    return () => { window.clearTimeout(timer); };
    // Only re-initialize when the dialog opens/objective changes; goal refreshes must
    // not reset the inline clear confirmation.
  }, [props.initialObjective, props.open]);
  const submit = (): void => {
    if (objective.trim() === "") {
      setError(t("goalEmpty"));
      return;
    }
    void actions.setGoal(objective.trim()).then((ok) => {
      if (!ok) {
        setError(t("goalFailed"));
        return;
      }
      props.onClose();
    });
  };
  return (
    <Modal open={props.open} title={t("goalConfirmSet")} onClose={props.onClose} actions={(
      <>
        <button type="button" className="rounded-control border border-border px-2 py-1" onClick={props.onClose}>{t("goalCancel")}</button>
        <button type="button" className="rounded-control border border-border bg-foreground px-2 py-1 text-background" onClick={submit}>{t("goalConfirm")}</button>
      </>
    )}>
      {goal === null ? (
        <p>{t("goalEmpty")}</p>
      ) : (
        <div className="flex flex-col gap-1">
          <p>{goal.objective}</p>
          <p className="text-[11px]">{t("goalRounds", { rounds: goal.rounds, max: goal.max_rounds })}</p>
          {confirmClear ? (
            <div className="flex items-center gap-2">
              <span>{t("goalConfirmClear")}</span>
              <button type="button" className="rounded-control border border-border px-2 py-1" onClick={() => { void actions.updateGoal("cancel").then((ok) => { if (ok) props.onClose(); }); }}>{t("goalConfirm")}</button>
              <button type="button" className="rounded-control border border-border px-2 py-1" onClick={() => { setConfirmClear(false); }}>{t("goalCancel")}</button>
            </div>
          ) : (
            <button type="button" className="self-start rounded-control border border-border px-2 py-1" onClick={() => { setConfirmClear(true); }}>{t("goalClear")}</button>
          )}
        </div>
      )}
      <label className="flex flex-col gap-1">
        {t("goalObjectiveLabel")}
        <textarea aria-label={t("goalObjectiveLabel")} rows={3} value={objective} onChange={(event) => { setObjective(event.target.value); }} className="rounded-control border border-border bg-background px-2 py-1 text-foreground" />
      </label>
      {error !== null ? <p role="alert" className="text-destructive">{error}</p> : null}
    </Modal>
  );
}
