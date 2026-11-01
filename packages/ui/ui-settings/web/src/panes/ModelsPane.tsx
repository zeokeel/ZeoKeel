/**
 * Pane 20 · model generation parameters (completion options).
 *
 * Provider credentials live in the personal-only provider pane (order 21);
 * these are the per-request model parameters that also apply to the enterprise
 * gateway (the pane itself never branches on the edition — C2).
 */
import type { ReactNode } from "react";
import {
  type ParallelMode,
  type PromptCache,
  type ReasoningEffort,
  type SettingsPaneProps,
  FieldError,
  inputClass,
  labelClass,
  NumericField,
  PaneSection,
} from "@zeokeel/ui-kit";

function parallelOf(value: string): ParallelMode {
  return value === "true" ? "true" : value === "false" ? "false" : "default";
}

function reasoningOf(value: string): ReasoningEffort {
  return value === "minimal" ||
    value === "low" ||
    value === "medium" ||
    value === "high" ||
    value === "xhigh" ||
    value === "max"
    ? value
    : "";
}

function promptCacheOf(value: string): PromptCache {
  return value === "short" || value === "long" ? value : "";
}

export function ModelsPane(props: SettingsPaneProps): ReactNode {
  const { t, draft, update, errors } = props;
  return (
    <PaneSection title={t("settingsSectionModels")}>
      <NumericField
        id="settings-temperature"
        label={t("settingsTemperature")}
        value={draft.temperature}
        onChange={(value) => {
          update("temperature", value);
        }}
        t={t}
        error={errors.temperature}
        step="0.1"
        min={0}
        max={2}
      />
      <NumericField
        id="settings-max-tokens"
        label={t("settingsMaxTokens")}
        value={draft.maxTokens}
        onChange={(value) => {
          update("maxTokens", value);
        }}
        t={t}
        error={errors.maxTokens}
        min={1}
      />
      <NumericField
        id="settings-top-p"
        label={t("settingsTopP")}
        value={draft.topP}
        onChange={(value) => {
          update("topP", value);
        }}
        t={t}
        error={errors.topP}
        step="0.05"
        min={0}
        max={1}
      />
      <NumericField
        id="settings-seed"
        label={t("settingsSeed")}
        value={draft.seed}
        onChange={(value) => {
          update("seed", value);
        }}
        t={t}
        error={errors.seed}
        min={0}
      />
      <label className="flex flex-col gap-1" htmlFor="settings-tool-choice">
        <span className={labelClass}>{t("settingsToolChoice")}</span>
        <select
          id="settings-tool-choice"
          className={inputClass}
          value={draft.toolChoiceMode}
          onChange={(event) => {
            const value = event.target.value;
            update(
              "toolChoiceMode",
              value === "none" || value === "required" || value === "specific" ? value : "auto",
            );
          }}
        >
          <option value="auto">{t("settingsToolChoiceAuto")}</option>
          <option value="none">{t("settingsToolChoiceNone")}</option>
          <option value="required">{t("settingsToolChoiceRequired")}</option>
          <option value="specific">{t("settingsToolChoiceSpecific")}</option>
        </select>
      </label>
      {draft.toolChoiceMode === "specific" ? (
        <label className="flex flex-col gap-1" htmlFor="settings-tool-choice-tools">
          <span className={labelClass}>{t("settingsToolChoiceTools")}</span>
          <input
            id="settings-tool-choice-tools"
            className={inputClass}
            value={draft.toolChoiceTools}
            placeholder="fs.read, fs.write"
            onChange={(event) => {
              update("toolChoiceTools", event.target.value);
            }}
            aria-invalid={errors.toolChoiceTools !== null}
          />
          <FieldError error={errors.toolChoiceTools} t={t} />
        </label>
      ) : null}
      <label className="flex flex-col gap-1" htmlFor="settings-parallel-tool-calls">
        <span className={labelClass}>{t("settingsParallelToolCalls")}</span>
        <select
          id="settings-parallel-tool-calls"
          className={inputClass}
          value={draft.parallelToolCalls}
          onChange={(event) => {
            update("parallelToolCalls", parallelOf(event.target.value));
          }}
        >
          <option value="default">{t("settingsParallelDefault")}</option>
          <option value="true">{t("settingsParallelOn")}</option>
          <option value="false">{t("settingsParallelOff")}</option>
        </select>
      </label>
      <label className="flex flex-col gap-1" htmlFor="settings-reasoning-effort">
        <span className={labelClass}>{t("settingsReasoningEffort")}</span>
        <select
          id="settings-reasoning-effort"
          className={inputClass}
          value={draft.reasoningEffort}
          onChange={(event) => {
            update("reasoningEffort", reasoningOf(event.target.value));
          }}
        >
          <option value="">{t("settingsReasoningDefault")}</option>
          <option value="minimal">minimal</option>
          <option value="low">low</option>
          <option value="medium">medium</option>
          <option value="high">high</option>
          <option value="xhigh">xhigh</option>
          <option value="max">max</option>
        </select>
      </label>
      <label className="flex flex-col gap-1" htmlFor="settings-prompt-cache">
        <span className={labelClass}>{t("settingsPromptCache")}</span>
        <select
          id="settings-prompt-cache"
          className={inputClass}
          value={draft.promptCache}
          onChange={(event) => {
            update("promptCache", promptCacheOf(event.target.value));
          }}
        >
          <option value="">{t("settingsPromptCacheDefault")}</option>
          <option value="short">{t("settingsPromptCacheShort")}</option>
          <option value="long">{t("settingsPromptCacheLong")}</option>
        </select>
      </label>
      <label className="flex flex-col gap-1 sm:col-span-2" htmlFor="settings-stop">
        <span className={labelClass}>{t("settingsStop")}</span>
        <textarea
          id="settings-stop"
          className={`${inputClass} min-h-[60px] font-mono`}
          value={draft.stop}
          onChange={(event) => {
            update("stop", event.target.value);
          }}
          aria-describedby="settings-stop-hint"
        />
        <span id="settings-stop-hint" className="text-[12px] text-muted-foreground">
          {t("settingsStopHint")}
        </span>
      </label>
    </PaneSection>
  );
}
