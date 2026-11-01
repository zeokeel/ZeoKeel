/** Compacted-history divider above the kept-recent messages (`/compact`). */
import { useState } from "react";
import { useUi, type CompactionMarker } from "@zeokeel/ui-kit";

export function CompactDivider(props: { marker: CompactionMarker }) {
  const { t } = useUi();
  const [open, setOpen] = useState(false);
  return (
    <div className="mx-auto w-full max-w-[760px] py-2">
      <div className="flex flex-col gap-1 rounded-control border border-dashed border-border px-3 py-2 text-[11px] text-muted-foreground">
        <div className="flex flex-wrap items-center gap-2">
          <span role="separator" aria-label={t("compactDividerLabel")} className="h-px min-w-6 flex-1 bg-border" />
          <span className="font-medium text-foreground">{t("compactDividerLabel")}</span>
          <span>{t("compactCount", { count: props.marker.compressedMessages })}</span>
          <button
            type="button"
            aria-expanded={open}
            className="rounded-control border border-border px-1.5 py-0.5 text-foreground focus-visible:outline focus-visible:outline-2"
            onClick={() => { setOpen((value) => !value); }}
          >
            {open ? t("compactToggleHide") : t("compactToggleShow")}
          </button>
          <span role="separator" aria-label={t("compactDividerLabel")} className="h-px min-w-6 flex-1 bg-border" />
        </div>
        {open ? (
          <div role="region" aria-label={t("compactSummaryLabel")} className="flex flex-col gap-1">
            <p className="whitespace-pre-wrap break-words text-foreground">{props.marker.summary}</p>
            <p>{t("compactUsage", { input: props.marker.inputTokens, output: props.marker.outputTokens })}</p>
          </div>
        ) : null}
      </div>
    </div>
  );
}