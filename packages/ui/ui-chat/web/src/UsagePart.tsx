import type { DataMessagePartProps } from "@assistant-ui/react";
import { isRecord, readNumber, useUi, type AgentUsage } from "@zeokeel/ui-kit";

/** Token/cost summary entries for one assistant turn; empty when nothing was reported. */
export interface UsageSummary {
  input: number | null;
  cached: number | null;
  output: number | null;
  reasoning: number | null;
  total: number | null;
  costMicros: number | null;
}

function readCount(value: unknown): number | null {
  return typeof value === "number" && Number.isFinite(value) && value >= 0 ? value : null;
}

export function readUsage(value: unknown): UsageSummary {
  if (!isRecord(value)) {
    return { input: null, cached: null, output: null, reasoning: null, total: null, costMicros: null };
  }
  const usage = isRecord(value["usage"]) ? value["usage"] : value;
  return {
    input: readCount(usage["input_tokens"]),
    cached: readCount(usage["cached_tokens"]),
    output: readCount(usage["output_tokens"]),
    reasoning: readCount(usage["reasoning_tokens"]),
    total: readCount(usage["total_tokens"]),
    costMicros: readCount(usage["cost_micros"]),
  };
}

function hasUsage(summary: UsageSummary): boolean {
  return Object.values(summary).some((value) => value !== null);
}

/** Inline token/cost meta under one assistant turn (docs/38 §5). */
export function UsageDataPart(props: DataMessagePartProps) {
  const { t } = useUi();
  const summary = readUsage(props.data);
  if (!hasUsage(summary)) {
    return null;
  }
  const cost = summary.costMicros === null ? null : (summary.costMicros / 1_000_000).toFixed(6);
  return (
    <div className="mt-1 flex flex-wrap items-center gap-x-3 gap-y-1 text-[11px] text-muted-foreground" aria-label={t("usageLabel")}>
      <span>{t("usageLabel")}</span>
      {summary.input !== null ? <span>{t("usageInput", { count: summary.input })}</span> : null}
      {summary.cached !== null ? <span>{t("usageCached", { count: summary.cached })}</span> : null}
      {summary.output !== null ? <span>{t("usageOutput", { count: summary.output })}</span> : null}
      {summary.reasoning !== null ? <span>{t("usageReasoning", { count: summary.reasoning })}</span> : null}
      {summary.total !== null ? <span>{t("usageTotal", { count: summary.total })}</span> : null}
      {cost !== null ? <span>{t("usageCost", { cost })}</span> : null}
    </div>
  );
}

export function usageFromReply(value: unknown): AgentUsage | null {
  if (!isRecord(value)) {
    return null;
  }
  const summary = readUsage(value);
  if (!hasUsage(summary)) {
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

export { readNumber };
