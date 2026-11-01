/**
 * Pane 60 · embedding / vector index (personal edition contribution only).
 *
 * The pane is compiled into the personal composition only (`editions =
 * ["personal"]` in `zk.toml`); it contains no runtime edition branch.
 */
import type { ReactNode } from "react";
import {
  type SettingsPaneProps,
  NumericField,
  PaneSection,
  TextField,
} from "@zeokeel/ui-kit";

export function EmbeddingPane(props: SettingsPaneProps): ReactNode {
  const { t, draft, update, errors } = props;
  return (
    <PaneSection title={t("settingsSectionEmbedding")}>
      <TextField
        id="settings-embedding-base-url"
        label={t("settingsEmbeddingBaseUrl")}
        value={draft.embeddingBaseUrl}
        onChange={(value) => {
          update("embeddingBaseUrl", value);
        }}
        t={t}
        error={errors.embeddingBaseUrl}
        type="url"
        placeholder="https://api.example.com/v1"
        span2
      />
      <TextField
        id="settings-embedding-api-key"
        label={t("settingsEmbeddingApiKey")}
        value={draft.embeddingApiKey}
        onChange={(value) => {
          update("embeddingApiKey", value);
        }}
        t={t}
        hint={t("settingsEmbeddingApiKeyHint")}
        type="password"
        placeholder="sk-..."
      />
      <TextField
        id="settings-embedding-model"
        label={t("settingsEmbeddingModel")}
        value={draft.embeddingModel}
        onChange={(value) => {
          update("embeddingModel", value);
        }}
        t={t}
        placeholder="text-embedding-3-small"
      />
      <NumericField
        id="settings-embedding-dims"
        label={t("settingsEmbeddingDims")}
        value={draft.embeddingDims}
        onChange={(value) => {
          update("embeddingDims", value);
        }}
        t={t}
        error={errors.embeddingDims}
        min={1}
        max={65536}
      />
      <TextField
        id="settings-embedding-dir"
        label={t("settingsEmbeddingDir")}
        value={draft.embeddingDir}
        onChange={(value) => {
          update("embeddingDir", value);
        }}
        t={t}
        placeholder="$ZEOKEEL_HOME/vector-index"
        span2
      />
      <p className="text-[12px] text-muted-foreground sm:col-span-2">{t("settingsEmbeddingHint")}</p>
    </PaneSection>
  );
}
