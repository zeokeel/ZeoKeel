/** Pane 50 · network & retrieval: web-search endpoint (https only) + timeout. */
import type { ReactNode } from "react";
import {
  type SettingsPaneProps,
  NumericField,
  PaneSection,
  TextField,
} from "@zeokeel/ui-kit";

export function SearchPane(props: SettingsPaneProps): ReactNode {
  const { t, draft, update, errors } = props;
  return (
    <PaneSection title={t("settingsSectionWebSearch")}>
      <TextField
        id="settings-web-search-base-url"
        label={t("settingsWebSearchBaseUrl")}
        value={draft.webSearchBaseUrl}
        onChange={(value) => {
          update("webSearchBaseUrl", value);
        }}
        t={t}
        error={errors.webSearchBaseUrl}
        hint={t("settingsWebSearchBaseUrlHint")}
        type="url"
        placeholder="https://api.duckduckgo.com/"
        span2
      />
      <NumericField
        id="settings-web-search-timeout"
        label={t("settingsWebSearchTimeout")}
        value={draft.webSearchTimeoutMs}
        onChange={(value) => {
          update("webSearchTimeoutMs", value);
        }}
        t={t}
        error={errors.webSearchTimeoutMs}
        min={1000}
        max={120000}
      />
      <p className="text-[12px] text-muted-foreground sm:col-span-2">{t("settingsWebSearchHint")}</p>
    </PaneSection>
  );
}
