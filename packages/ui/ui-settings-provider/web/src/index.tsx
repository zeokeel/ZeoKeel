/** Personal-only provider fields; controlled by the settings dialog. */
import type { ProviderKind, ProviderSettingsProps } from "@zeokeel/ui-kit";

const labelClass = "text-[12px] font-medium text-muted-foreground";
const inputClass =
  "w-full rounded-control border border-border bg-background px-2.5 py-2 text-[13px] text-foreground placeholder:text-muted-foreground disabled:opacity-55";

function providerKindOf(value: string): ProviderKind {
  return value === "open_ai_compatible" || value === "ollama" ? value : "mock";
}

export function ProviderSettings(props: ProviderSettingsProps) {
  const { t, errors, isMock } = props;
  return (
    <fieldset className="rounded-document bg-muted p-4">
      <legend className="px-1.5 text-[12px] font-medium text-muted-foreground">
        {t("settingsSectionProvider")}
      </legend>
      <div className="grid grid-cols-1 gap-3.5 sm:grid-cols-2">
        <label className="flex flex-col gap-1" htmlFor="settings-provider">
          <span className={labelClass}>{t("settingsProvider")}</span>
          <select
            id="settings-provider"
            className={inputClass}
            value={props.providerKind}
            onChange={(event) => { props.onProviderKind(providerKindOf(event.target.value)); }}
          >
            <option value="mock">{t("settingsProviderMock")}</option>
            <option value="open_ai_compatible">{t("settingsProviderOpenai")}</option>
            <option value="ollama">{t("settingsProviderOllama")}</option>
          </select>
        </label>

        <div className="flex flex-col gap-1">
          <label className={labelClass} htmlFor="settings-model">{t("settingsModel")}</label>
          <input id="settings-model" className={inputClass} type="text" value={props.model} aria-invalid={errors.model !== null} onChange={(event) => { props.onModel(event.target.value); }} />
          {errors.model !== null ? <span className="text-[12px] text-destructive" role="alert">{t(errors.model)}</span> : null}
        </div>

        <div className="flex flex-col gap-1 sm:col-span-2">
          <label className={labelClass} htmlFor="settings-base-url">{t("settingsBaseUrl")}</label>
          <input id="settings-base-url" className={inputClass} type="url" inputMode="url" placeholder="https://api.example.com/v1" value={props.baseUrl} disabled={isMock} aria-invalid={errors.baseUrl !== null} onChange={(event) => { props.onBaseUrl(event.target.value); }} />
          {errors.baseUrl !== null ? <span className="text-[12px] text-destructive" role="alert">{t(errors.baseUrl)}</span> : null}
        </div>

        <div className="flex flex-col gap-1 sm:col-span-2">
          <label className={labelClass} htmlFor="settings-api-key">{t("settingsApiKey")}</label>
          <input id="settings-api-key" className={inputClass} type="password" autoComplete="off" spellCheck={false} value={props.apiKey} disabled={isMock} onChange={(event) => { props.onApiKey(event.target.value); }} />
          <span className="text-[12px] text-muted-foreground">{t("settingsApiKeyHint")}</span>
        </div>
      </div>
    </fieldset>
  );
}

export const components = { "settings.provider": ProviderSettings } as const;
