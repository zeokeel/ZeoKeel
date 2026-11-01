/**
 * Pane 65 · media capabilities (personal edition contribution only).
 *
 * Image/audio/transcription/rerank model names + write-only credentials; empty
 * fields fall back to the environment (unchanged behaviour from before the split).
 */
import type { ReactNode } from "react";
import {
  type SettingsPaneProps,
  PaneSection,
  TextField,
} from "@zeokeel/ui-kit";

export function MediaPane(props: SettingsPaneProps): ReactNode {
  const { t, draft, update, errors } = props;
  return (
    <PaneSection title={t("settingsSectionMedia")}>
      <TextField
        id="settings-media-base-url"
        label={t("settingsMediaBaseUrl")}
        value={draft.mediaBaseUrl}
        onChange={(value) => {
          update("mediaBaseUrl", value);
        }}
        t={t}
        error={errors.mediaBaseUrl}
        type="url"
        placeholder="https://api.example.com/v1"
        span2
      />
      <TextField
        id="settings-media-api-key"
        label={t("settingsMediaApiKey")}
        value={draft.mediaApiKey}
        onChange={(value) => {
          update("mediaApiKey", value);
        }}
        t={t}
        hint={t("settingsMediaApiKeyHint")}
        type="password"
        placeholder="sk-..."
      />
      <TextField
        id="settings-media-image-model"
        label={t("settingsMediaImageModel")}
        value={draft.mediaImageModel}
        onChange={(value) => {
          update("mediaImageModel", value);
        }}
        t={t}
        placeholder="dall-e-3"
      />
      <TextField
        id="settings-media-audio-model"
        label={t("settingsMediaAudioModel")}
        value={draft.mediaAudioModel}
        onChange={(value) => {
          update("mediaAudioModel", value);
        }}
        t={t}
        placeholder="tts-1"
      />
      <TextField
        id="settings-media-transcription-model"
        label={t("settingsMediaTranscriptionModel")}
        value={draft.mediaTranscriptionModel}
        onChange={(value) => {
          update("mediaTranscriptionModel", value);
        }}
        t={t}
        placeholder="whisper-1"
      />
      <TextField
        id="settings-media-rerank-model"
        label={t("settingsMediaRerankModel")}
        value={draft.mediaRerankModel}
        onChange={(value) => {
          update("mediaRerankModel", value);
        }}
        t={t}
        placeholder="rerank-1"
      />
      <p className="text-[12px] text-muted-foreground sm:col-span-2">{t("settingsMediaHint")}</p>
    </PaneSection>
  );
}
