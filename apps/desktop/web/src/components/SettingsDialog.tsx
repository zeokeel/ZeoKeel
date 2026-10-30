import * as Dialog from "@radix-ui/react-dialog";
import { useEffect, useState, type FormEvent } from "react";
import { useT } from "../i18n";
import {
  formToSettings,
  toForm,
  validateSettings,
  type SettingsErrors,
  type SettingsForm,
} from "../state/settings-validation";
import type { AppSettings, Locale, ProviderKind, Theme } from "../types";
import { BrandLogo } from "./BrandLogo";
import { IconClose } from "./icons";

const EMPTY_ERRORS: SettingsErrors = { baseUrl: null, model: null };
const labelClass = "text-[12px] font-medium text-muted-foreground";
const inputClass =
  "w-full rounded-control border border-border bg-background px-2.5 py-2 text-[13px] text-foreground placeholder:text-muted-foreground disabled:opacity-55";

function providerKindOf(value: string): ProviderKind {
  return value === "open_ai_compatible" || value === "ollama" ? value : "mock";
}

function themeOf(value: string): Theme {
  return value === "light" || value === "dark" ? value : "auto";
}

function localeOf(value: string): Locale {
  return value === "en-US" ? "en-US" : "zh-CN";
}

export function SettingsDialog(props: {
  open: boolean;
  settings: AppSettings;
  onOpenChange: (open: boolean) => void;
  onSave: (settings: AppSettings) => Promise<void>;
}) {
  const t = useT();
  const [form, setForm] = useState<SettingsForm>(() => toForm(props.settings));
  const [errors, setErrors] = useState<SettingsErrors>(EMPTY_ERRORS);
  const [saving, setSaving] = useState(false);
  const [saveError, setSaveError] = useState<string | null>(null);

  useEffect(() => {
    if (!props.open) {
      return;
    }
    setForm(toForm(props.settings));
    setErrors(EMPTY_ERRORS);
    setSaveError(null);
    setSaving(false);
  }, [props.open, props.settings]);

  const update = <Key extends keyof SettingsForm>(key: Key, value: SettingsForm[Key]) => {
    setForm((current) => ({ ...current, [key]: value }));
  };

  const handleSubmit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const nextErrors = validateSettings(form);
    setErrors(nextErrors);
    if (nextErrors.baseUrl !== null || nextErrors.model !== null) {
      return;
    }
    setSaving(true);
    setSaveError(null);
    void props
      .onSave(formToSettings(form, props.settings.schema))
      .catch((error: unknown) => {
        const message = error instanceof Error ? error.message : t("settingsSaveFailed");
        setSaveError(message);
      })
      .finally(() => {
        setSaving(false);
      });
  };

  const isMock = form.providerKind === "mock";

  return (
    <Dialog.Root open={props.open} onOpenChange={props.onOpenChange}>
      <Dialog.Portal>
        <Dialog.Overlay className="fixed inset-0 z-40 bg-[var(--zk-overlay)]" />
        <Dialog.Content
          className="fixed start-1/2 top-1/2 z-50 max-h-[86vh] w-[min(640px,calc(100vw-32px))] -translate-x-1/2 -translate-y-1/2 overflow-y-auto rounded-xl bg-popover p-6 shadow-[var(--zk-shadow-lift)]"
          aria-describedby="settings-description"
        >
          <div className="flex items-start justify-between gap-4">
            <div className="flex items-center gap-2.5">
              <BrandLogo className="h-6 w-6 rounded-document" decorative />
              <Dialog.Title className="text-[20px] font-semibold tracking-[-0.01em] text-foreground">
                {t("settingsTitle")}
              </Dialog.Title>
            </div>
            <Dialog.Close asChild>
              <button
                type="button"
                className="inline-flex h-8 w-8 items-center justify-center rounded-control text-muted-foreground transition-colors hover:bg-foreground/[0.05] hover:text-foreground"
                aria-label={t("close")}
              >
                <IconClose />
              </button>
            </Dialog.Close>
          </div>
          <Dialog.Description id="settings-description" className="mt-1.5 mb-5 text-[13px] text-muted-foreground">
            {t("settingsDescription")}
          </Dialog.Description>

          <form className="flex flex-col gap-4" onSubmit={handleSubmit} noValidate>
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
                    value={form.providerKind}
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
                    value={form.model}
                    aria-invalid={errors.model !== null}
                    onChange={(event) => {
                      update("model", event.target.value);
                    }}
                  />
                  {errors.model !== null ? (
                    <span className="text-[12px] text-destructive" role="alert">
                      {t(errors.model)}
                    </span>
                  ) : null}
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
                    value={form.baseUrl}
                    disabled={isMock}
                    aria-invalid={errors.baseUrl !== null}
                    onChange={(event) => {
                      update("baseUrl", event.target.value);
                    }}
                  />
                  {errors.baseUrl !== null ? (
                    <span className="text-[12px] text-destructive" role="alert">
                      {t(errors.baseUrl)}
                    </span>
                  ) : null}
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
                    value={form.apiKey}
                    disabled={isMock}
                    onChange={(event) => {
                      update("apiKey", event.target.value);
                    }}
                  />
                  <span className="text-[12px] text-muted-foreground">{t("settingsApiKeyHint")}</span>
                </div>
              </div>
            </fieldset>

            <fieldset className="rounded-document bg-muted p-4">
              <legend className="px-1.5 text-[12px] font-medium text-muted-foreground">
                {t("settingsSectionAppearance")}
              </legend>
              <div className="grid grid-cols-1 gap-3.5 sm:grid-cols-2">
                <label className="flex flex-col gap-1" htmlFor="settings-theme">
                  <span className={labelClass}>{t("settingsTheme")}</span>
                  <select
                    id="settings-theme"
                    className={inputClass}
                    value={form.theme}
                    onChange={(event) => {
                      update("theme", themeOf(event.target.value));
                    }}
                  >
                    <option value="auto">{t("settingsThemeAuto")}</option>
                    <option value="light">{t("settingsThemeLight")}</option>
                    <option value="dark">{t("settingsThemeDark")}</option>
                  </select>
                </label>

                <label className="flex flex-col gap-1" htmlFor="settings-locale">
                  <span className={labelClass}>{t("settingsLanguage")}</span>
                  <select
                    id="settings-locale"
                    className={inputClass}
                    value={form.locale}
                    onChange={(event) => {
                      update("locale", localeOf(event.target.value));
                    }}
                  >
                    <option value="zh-CN">中文（简体）</option>
                    <option value="en-US">English (US)</option>
                  </select>
                </label>

                <label
                  className="flex items-center gap-3 rounded-control bg-background px-3 py-2 sm:col-span-2"
                  htmlFor="settings-auto-approve"
                >
                  <input
                    id="settings-auto-approve"
                    className="zk-switch"
                    type="checkbox"
                    checked={form.autoApproveReadOnly}
                    onChange={(event) => {
                      update("autoApproveReadOnly", event.target.checked);
                    }}
                  />
                  <span className="text-[13px] text-foreground">{t("settingsAutoApprove")}</span>
                </label>
              </div>
            </fieldset>

            {saveError !== null ? (
              <p className="rounded-document bg-muted px-3 py-2 text-[13px] text-destructive" role="alert">
                {t("settingsSaveFailed")}: {saveError}
              </p>
            ) : null}

            <div className="flex justify-end gap-2">
              <Dialog.Close asChild>
                <button
                  type="button"
                  className="inline-flex items-center rounded-control border border-border px-3 py-1.5 text-[13px] text-foreground transition-colors hover:bg-muted disabled:opacity-50"
                  disabled={saving}
                >
                  {t("settingsCancel")}
                </button>
              </Dialog.Close>
              <button
                type="submit"
                className="inline-flex items-center rounded-control bg-foreground px-3.5 py-1.5 text-[13px] font-medium text-background transition-colors hover:bg-foreground/90 disabled:pointer-events-none disabled:opacity-50"
                disabled={saving}
              >
                {saving ? t("settingsSaving") : t("settingsSave")}
              </button>
            </div>
          </form>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}
