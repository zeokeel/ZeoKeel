import type { DataMessagePartProps } from "@assistant-ui/react";
import { isRecord, useUi, type AgentUsage } from "@zeokeel/ui-kit";

/** Normalized token/cost summary for one assistant turn; `null` when no usage object is present. */
export interface UsageSummary {
  input: number;
  cached: number;
  output: number;
  reasoning: number;
  total: number;
  costMicros: number | null;
}

function readCount(value: unknown): number {
  return typeof value === "number" && Number.isFinite(value) && value >= 0 ? value : 0;
}

export function readUsage(value: unknown): UsageSummary | null {
  if (!isRecord(value)) {
    return null;
  }
  // Both the data part (`data-usage`) and an `AgentReply` wrap the counters in `usage`.
  const usage = value["usage"];
  if (!isRecord(usage)) {
    return null;
  }
  return {
    input: readCount(usage["input_tokens"]),
    cached: readCount(usage["cached_tokens"]),
    output: readCount(usage["output_tokens"]),
    reasoning: readCount(usage["reasoning_tokens"]),
    total: readCount(usage["total_tokens"]),
    costMicros: typeof usage["cost_micros"] === "number" && Number.isFinite(usage["cost_micros"]) ? usage["cost_micros"] : null,
  };
}

/** Inline token/cost meta under one assistant turn (docs/38 §5). */
export function UsageDataPart(props: DataMessagePartProps) {
  const { t } = useUi();
  const summary = readUsage(props.data);
  if (summary === null) {
    return null;
  }
  const cost = summary.costMicros === null ? null : (summary.costMicros / 1_000_000).toFixed(6);
  return (
    <div className="mt-1 flex flex-wrap items-center gap-x-3 gap-y-1 text-[11px] text-muted-foreground" aria-label={t("usageLabel")}>
      <span>{t("usageLabel")}</span>
      <span>{t("usageInput", { count: summary.input })}</span>
      <span>{t("usageCached", { count: summary.cached })}</span>
      <span>{t("usageOutput", { count: summary.output })}</span>
      <span>{t("usageReasoning", { count: summary.reasoning })}</span>
      <span>{t("usageTotal", { count: summary.total })}</span>
      {cost !== null ? <span>{t("usageCost", { cost })}</span> : null}
    </div>
  );
}

export function usageFromReply(value: unknown): AgentUsage | null {
  if (!isRecord(value)) {
    return null;
  }
  const summary = readUsage(value);
  if (summary === null) {
    return null;
  }
  return {
    input_tokens: summary.input,
    output_tokens: summary.output,
    total_tokens: summary.total,
    cached_tokens: summary.cached,
    reasoning_tokens: summary.reasoning,
    cost_micros: summary.costMicros,
  };
}
