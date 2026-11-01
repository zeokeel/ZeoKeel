/**
 * Pane 40 · tools & approval timeouts.
 *
 * Approval *policy* (auto-approve read-only) is a separate config-spec pane; the
 * timeout fields stay exactly where they were before the IA split.
 */
import type { ReactNode } from "react";
import {
  type ApprovalPolicy,
  type SettingsPaneProps,
  NumericField,
  PaneSection,
} from "@zeokeel/ui-kit";

function policyOf(value: string): ApprovalPolicy {
  if (value === "auto_read_only" || value === "strict") {
    return value;
  }
  return "ask";
}

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
      <label className="flex flex-col gap-1 sm:col-span-2" htmlFor="settings-approval-policy">
        <span className="text-[12px] font-medium text-muted-foreground">{t("settingsApprovalPolicy")}</span>
        <select
          id="settings-approval-policy"
          className="w-full rounded-control border border-border bg-background px-2.5 py-2 text-[13px] text-foreground"
          value={draft.approvalPolicy}
          onChange={(event) => {
            update("approvalPolicy", policyOf(event.target.value));
          }}
        >
          <option value="ask">{t("settingsApprovalPolicyAsk")}</option>
          <option value="auto_read_only">{t("settingsApprovalPolicyAutoReadOnly")}</option>
          <option value="strict">{t("settingsApprovalPolicyStrict")}</option>
        </select>
        <span className="text-[12px] text-muted-foreground">{t("settingsApprovalPolicyHint")}</span>
      </label>
      <p className="text-[12px] text-muted-foreground sm:col-span-2">{t("settingsToolsHint")}</p>
    </PaneSection>
  );
}
