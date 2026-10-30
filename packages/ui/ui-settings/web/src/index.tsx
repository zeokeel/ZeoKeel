import * as Dialog from "@radix-ui/react-dialog";
import { useEffect, useState, type FormEvent, type ReactNode } from "react";
import {
  BrandLogo,
  Button,
  IconClose,
  Switch,
  useUi,
  type AppSettings,
  type Locale,
  type ProviderKind,
  type ProviderSettingsProps,
  type Theme,
} from "@zeokeel/ui-kit";
import {
  formToSettings,
  toForm,
  validateSettings,
  type SettingsErrors,
  type SettingsForm,
} from "./settings-validation";

const EMPTY_ERRORS: SettingsErrors = { baseUrl: null, model: null };
const labelClass = "text-[12px] font-medium text-muted-foreground";
const inputClass =
  "w-full rounded-control border border-border bg-background px-2.5 py-2 text-[13px] text-foreground placeholder:text-muted-foreground disabled:opacity-55";

function themeOf(value: string): Theme {
  return value === "light" || value === "dark" ? value : "auto";
}

function localeOf(value: string): Locale {
  return value === "en-US" ? "en-US" : "zh-CN";
}

export interface SettingsDialogProps {
  gatewayModels?: string[];
  gatewayModel?: string;
  onSelectGatewayModel?: (model: string) => void;
  onCheckUpdate?: (() => void) | undefined;
  renderProvider?: ((props: ProviderSettingsProps) => ReactNode) | undefined;
  updateText?: string | null;
  updateBusy?: boolean;
}

/** `settings.dialog` slot: reads settings/open state and actions from useUi. */
export function SettingsDialog(props: SettingsDialogProps = {}) {
  const ui = useUi();
  const models = props.gatewayModels ?? [];
  const { t, settings, settingsOpen, actions } = ui;
  const [form, setForm] = useState<SettingsForm>(() => toForm(settings));
  const [errors, setErrors] = useState<SettingsErrors>(EMPTY_ERRORS);
  const [saving, setSaving] = useState(false);
  const [saveError, setSaveError] = useState<string | null>(null);

  useEffect(() => {
    if (!settingsOpen) {
      return;
    }
    setForm(toForm(settings));
    setErrors(EMPTY_ERRORS);
    setSaveError(null);
    setSaving(false);
  }, [settingsOpen, settings]);

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
    const next: AppSettings = formToSettings(form, settings.schema);
    void actions
      .saveSettings(next)
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
    <Dialog.Root open={settingsOpen} onOpenChange={actions.closeSettings}>
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
          <Dialog.Description
            id="settings-description"
            className="mt-1.5 mb-5 text-[13px] text-muted-foreground"
          >
            {t("settingsDescription")}
          </Dialog.Description>

          <form className="flex flex-col gap-4" onSubmit={handleSubmit} noValidate>
            {props.renderProvider?.({
              t,
              providerKind: form.providerKind,
              baseUrl: form.baseUrl,
              model: form.model,
              apiKey: form.apiKey,
              errors,
              isMock,
              onProviderKind: (kind) => { update("providerKind", kind); },
              onBaseUrl: (value) => { update("baseUrl", value); },
              onModel: (value) => { update("model", value); },
              onApiKey: (value) => { update("apiKey", value); },
            })}
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
                  <Switch
                    id="settings-auto-approve"
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
              <p
                className="rounded-document bg-muted px-3 py-2 text-[13px] text-destructive"
                role="alert"
              >
                {t("settingsSaveFailed")}: {saveError}
              </p>
            ) : null}

            <div className="flex justify-end gap-2">
              <Dialog.Close asChild>
                <Button variant="ghost" disabled={saving}>
                  {t("settingsCancel")}
                </Button>
              </Dialog.Close>
              <Button type="submit" variant="primary" disabled={saving}>
                {saving ? t("settingsSaving") : t("settingsSave")}
              </Button>
            </div>
            {props.onCheckUpdate !== undefined ? (
            <section className="grid gap-2 border-t border-border pt-3">
              <h3 className={labelClass}>{t("settingsUpdates")}</h3>
              <div className="flex items-center gap-2">
                <Button type="button" variant="default" size="sm" disabled={props.updateBusy === true} onClick={props.onCheckUpdate}>
                  {props.updateBusy === true ? t("settingsCheckingUpdate") : t("settingsCheckUpdate")}
                </Button>
                {props.updateText !== null && props.updateText !== undefined ? <span className="text-[12px] text-muted-foreground">{props.updateText}</span> : null}
              </div>
            </section>
          ) : null}
          {models.length > 0 ? (
            <section className="grid gap-2 border-t border-border pt-3">
              <h3 className={labelClass}>{t("settingsGatewayModel")}</h3>
              <select
                aria-label={t("settingsGatewayModel")}
                className={inputClass}
                value={props.gatewayModel ?? ""}
                onChange={(event) => { props.onSelectGatewayModel?.(event.target.value); }}
              >
                {models.map((model) => <option key={model} value={model}>{model}</option>)}
              </select>
            </section>
          ) : null}
        </form>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}

export const components = {
  "settings.dialog": SettingsDialog,
} as const;

export { formToSettings, toForm, validateSettings } from "./settings-validation";
export type { SettingsErrors, SettingsForm } from "./settings-validation";
