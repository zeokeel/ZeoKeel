/** Local-only feedback dialog (`/feedback`, command surface R7). */
import { useState } from "react";
import { Modal, useUi, type FeedbackCategory } from "@zeokeel/ui-kit";

const MAX_DESCRIPTION_BYTES = 4096;

export function FeedbackDialog(props: { open: boolean; onClose: () => void }) {
  const { t, actions } = useUi();
  const [category, setCategory] = useState<FeedbackCategory>("bug");
  const [description, setDescription] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const tooLong = new TextEncoder().encode(description).length > MAX_DESCRIPTION_BYTES;
  const submit = (): void => {
    if (tooLong || busy) return;
    setBusy(true);
    setError(null);
    void actions.submitFeedback(category, description).then((ok) => {
      setBusy(false);
      if (ok) {
        setDescription("");
        props.onClose();
      } else {
        setError(t("feedbackFailed"));
      }
    });
  };
  return (
    <Modal open={props.open} title={t("feedbackTitle")} onClose={props.onClose} actions={(
      <>
        <button type="button" className="rounded-control border border-border px-2 py-1" onClick={props.onClose}>{t("feedbackCancel")}</button>
        <button type="button" disabled={busy || tooLong} className="rounded-control border border-border bg-foreground px-2 py-1 text-background disabled:opacity-50" onClick={submit}>{t("feedbackSubmit")}</button>
      </>
    )}>
      <fieldset>
        <legend className="mb-1">{t("feedbackCategory")}</legend>
        <div className="flex gap-2">
          {(["bug", "idea", "other"] as const).map((value) => (
            <label key={value} className="flex items-center gap-1">
              <input type="radio" name="feedback-category" checked={category === value} onChange={() => { setCategory(value); }} />
              {value === "bug" ? t("feedbackBug") : value === "idea" ? t("feedbackIdea") : t("feedbackOther")}
            </label>
          ))}
        </div>
      </fieldset>
      <label className="flex flex-col gap-1">
        {t("feedbackDescription")}
        <textarea aria-label={t("feedbackDescription")} rows={4} value={description} onChange={(event) => { setDescription(event.target.value); }} placeholder={t("feedbackDescriptionPlaceholder")} className="rounded-control border border-border bg-background px-2 py-1 text-foreground" />
      </label>
      {tooLong ? <p role="alert" className="text-destructive">{t("feedbackTooLong")}</p> : null}
      {error !== null ? <p role="alert" className="text-destructive">{error}</p> : null}
      <p className="text-[11px]">{t("feedbackLocalNote")}</p>
    </Modal>
  );
}
