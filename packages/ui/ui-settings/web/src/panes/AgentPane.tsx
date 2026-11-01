/**
 * Pane 30 · local agent runtime: step budget, memory, lifecycle hooks, scheduler.
 *
 * Model generation parameters live in the models pane (order 20).
 */
import type { ReactNode } from "react";
import {
  type SettingsPaneProps,
  NumericField,
  PaneSection,
  TextField,
} from "@zeokeel/ui-kit";

export function AgentPane(props: SettingsPaneProps): ReactNode {
  const { t, draft, update, errors } = props;
  return (
    <>
      <PaneSection title={t("settingsSectionAgent")}>
        <NumericField
          id="settings-max-steps"
          label={t("settingsMaxSteps")}
          value={draft.maxSteps}
          onChange={(value) => {
            update("maxSteps", value);
          }}
          t={t}
          error={errors.maxSteps}
          min={1}
          max={200}
        />
        <label
          className="flex items-center gap-3 rounded-control bg-background px-3 py-2"
          htmlFor="settings-memory-enabled"
        >
          <input
            id="settings-memory-enabled"
            type="checkbox"
            checked={draft.memoryEnabled}
            onChange={(event) => {
              update("memoryEnabled", event.target.checked);
            }}
          />
          <span className="text-[13px] text-foreground">{t("settingsMemoryEnabled")}</span>
        </label>
        <TextField
          id="settings-memory-dir"
          label={t("settingsMemoryDir")}
          value={draft.memoryDir}
          onChange={(value) => {
            update("memoryDir", value);
          }}
          t={t}
          hint={t("settingsMemoryDirHint")}
          placeholder="$ZEOKEEL_HOME/agent-memory"
        />
        <label
          className="flex items-center gap-3 rounded-control bg-background px-3 py-2"
          htmlFor="settings-hooks-enabled"
        >
          <input
            id="settings-hooks-enabled"
            type="checkbox"
            checked={draft.hooksEnabled}
            onChange={(event) => {
              update("hooksEnabled", event.target.checked);
            }}
          />
          <span className="text-[13px] text-foreground">{t("settingsHooksEnabled")}</span>
        </label>
        <TextField
          id="settings-hooks-log"
          label={t("settingsHooksLog")}
          value={draft.hooksLog}
          onChange={(value) => {
            update("hooksLog", value);
          }}
          t={t}
          hint={t("settingsHooksLogHint")}
          placeholder="/tmp/zeokeel-hooks.jsonl"
        />
      </PaneSection>
      <PaneSection title={t("settingsSectionSchedule")}>
        <label
          className="flex items-center gap-3 rounded-control bg-background px-3 py-2"
          htmlFor="settings-schedule-enabled"
        >
          <input
            id="settings-schedule-enabled"
            type="checkbox"
            checked={draft.scheduleEnabled}
            onChange={(event) => {
              update("scheduleEnabled", event.target.checked);
            }}
          />
          <span className="text-[13px] text-foreground">{t("settingsScheduleEnabled")}</span>
        </label>
        <NumericField
          id="settings-scan-seconds"
          label={t("settingsScanSeconds")}
          value={draft.scanSeconds}
          onChange={(value) => {
            update("scanSeconds", value);
          }}
          t={t}
          error={errors.scanSeconds}
          min={5}
          max={3600}
        />
        <p className="text-[12px] text-muted-foreground sm:col-span-2">{t("settingsScheduleHint")}</p>
      </PaneSection>
    </>
  );
}
