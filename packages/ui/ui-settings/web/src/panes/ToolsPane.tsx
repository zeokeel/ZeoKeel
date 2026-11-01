/**
 * Pane 40 · tools & approval timeouts.
 *
 * Approval *policy* (auto-approve read-only) is a separate config-spec pane; the
 * timeout fields stay exactly where they were before the IA split.
 */
import type { ReactNode } from "react";
import {
  type SettingsPaneProps,
  NumericField,
  PaneSection,
} from "@zeokeel/ui-kit";

export function ToolsPane(props: SettingsPaneProps): ReactNode {
  const { t, draft, update, errors } = props;
  return (
    <PaneSection title={t("settingsSectionTools")}>
      <NumericField
        id="settings-tool-timeout"
        label={t("settingsToolTimeout")}
        value={draft.toolTimeoutMs}
        onChange={(value) => {
          update("toolTimeoutMs", value);
        }}
        t={t}
        error={errors.toolTimeoutMs}
        min={1000}
        max={600000}
      />
      <NumericField
        id="settings-approval-timeout"
        label={t("settingsApprovalTimeout")}
        value={draft.approvalTimeoutMs}
        onChange={(value) => {
          update("approvalTimeoutMs", value);
        }}
        t={t}
        error={errors.approvalTimeoutMs}
        min={1000}
        max={1800000}
      />
      <p className="text-[12px] text-muted-foreground sm:col-span-2">{t("settingsToolsHint")}</p>
    </PaneSection>
  );
}
