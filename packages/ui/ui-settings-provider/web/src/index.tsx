/**
 * Personal-only provider pane (`ui.settings.pane`, order 21).
 *
 * Rendered by the settings shell with the shared `SettingsPaneProps`; the pane
 * never branches on the edition (the enterprise composition excludes this
 * package at build time). Credentials stay write-only: an empty key keeps the
 * stored secret on save.
 */
import {
  FieldError,
  inputClass,
  labelClass,
  PaneSection,
  type ProviderKind,
  type SettingsPaneProps,
} from "@zeokeel/ui-kit";

function providerKindOf(value: string): ProviderKind {
  return value === "open_ai_compatible" || value === "ollama" ? value : "mock";
}

export function ProviderSettingsPane(props: SettingsPaneProps) {
  const { t, draft, update, errors } = props;
  const isMock = draft.providerKind === "mock";
  return (
    <PaneSection title={t("settingsSectionProvider")}>
      <label className="flex flex-col gap-1" htmlFor="settings-provider">
        <span className={labelClass}>{t("settingsProvider")}</span>
        <select
          id="settings-provider"
          className={inputClass}
          value={draft.providerKind}
          onChange={(event) => {
            update("providerKind", providerKindOf(event.target.value));
          }}
        >
          <option value="mock">{t("settingsProviderMock")}</option>
          <option value="open_ai_compatible">{t("settingsProviderOpenai")}</option>
          <option value="ollama">{t("settingsProviderOllama")}</option>
        </select>
      </label>

      <div className="flex flex-col gap-1">
        <label className={labelClass} htmlFor="settings-model">
          {t("settingsModel")}
        </label>
        <input
          id="settings-model"
          className={inputClass}
          type="text"
          value={draft.model}
          aria-invalid={errors.model !== null}
          onChange={(event) => {
            update("model", event.target.value);
          }}
        />
        <FieldError error={errors.model} t={t} />
      </div>

      <div className="flex flex-col gap-1 sm:col-span-2">
        <label className={labelClass} htmlFor="settings-base-url">
          {t("settingsBaseUrl")}
        </label>
        <input
          id="settings-base-url"
          className={inputClass}
          type="url"
          inputMode="url"
          placeholder="https://api.example.com/v1"
          value={draft.baseUrl}
          disabled={isMock}
          aria-invalid={errors.baseUrl !== null}
          onChange={(event) => {
            update("baseUrl", event.target.value);
          }}
        />
        <FieldError error={errors.baseUrl} t={t} />
      </div>

      <div className="flex flex-col gap-1 sm:col-span-2">
        <label className={labelClass} htmlFor="settings-api-key">
          {t("settingsApiKey")}
        </label>
        <input
          id="settings-api-key"
          className={inputClass}
          type="password"
          autoComplete="off"
          spellCheck={false}
          value={draft.apiKey}
          disabled={isMock}
          onChange={(event) => {
            update("apiKey", event.target.value);
          }}
        />
        <span className="text-[12px] text-muted-foreground">{t("settingsApiKeyHint")}</span>
      </div>
    </PaneSection>
  );
}

export const components = { "settings.pane.provider": ProviderSettingsPane } as const;
