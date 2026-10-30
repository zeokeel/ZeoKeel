import * as Dialog from "@radix-ui/react-dialog";
import { useState, type ReactNode } from "react";
import {
  BrandLogo,
  Button,
  IconClose,
  useUi,
  type AppSettings,
  type Locale,
  type ProviderSettingsProps,
  type Theme,
} from "@zeokeel/ui-kit";
import {
  EMPTY_SETTINGS_ERRORS,
  formToSettings,
  hasSettingsErrors,
  toForm,
  validateSettings,
  type SettingsErrors,
  type SettingsFieldError,
  type SettingsForm,
} from "./settings-validation";
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
  /** Enterprise only: render the login server address (docs/19 §3.3). */
  showServerUrl?: boolean;
  /** Channel-locked address (C14/ADR-042): read-only. */
  serverUrlLocked?: boolean;
  /** Enterprise only: sign out (wired by the app; dependency: auth.logout IPC). */
  onLogout?: (() => void) | undefined;
}

/** `settings.dialog` slot: reads settings/open state and actions from useUi. */
export function SettingsDialog(props: SettingsDialogProps = {}) {
  const ui = useUi();
  const models = props.gatewayModels ?? [];
  const { t, settings, settingsOpen, actions } = ui;
  // The app remounts this dialog with a key on open, so the form always reflects persisted settings.
  const [form, setForm] = useState<SettingsForm>(() => toForm(settings));
  const [errors, setErrors] = useState<SettingsErrors>(EMPTY_SETTINGS_ERRORS);
  const [saving, setSaving] = useState(false);
  const [saveError, setSaveError] = useState<string | null>(null);

  const update = <Key extends keyof SettingsForm>(key: Key, value: SettingsForm[Key]): void => {
    setForm((current) => ({ ...current, [key]: value }));
  };

  const handleSubmit = (event: { preventDefault: () => void }): void => {
    event.preventDefault();
    const nextErrors = validateSettings(form);
    setErrors(nextErrors);
    if (hasSettingsErrors(nextErrors)) {
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
  const fieldError = (error: SettingsFieldError): ReactNode =>
    error === null ? null : (
      <span className="text-[12px] text-destructive" role="alert">{t(error)}</span>
    );
  const numeric = (id: string, label: string, value: string, onChange: (next: string) => void, error: SettingsFieldError, extra: { step?: string; min?: number; max?: number } = {}): ReactNode => (
    <label className="flex flex-col gap-1" htmlFor={id}>
      <span className={labelClass}>{label}</span>
      <input
        id={id}
        className={inputClass}
        type="number"
        inputMode="decimal"
        value={value}
        step={extra.step ?? "1"}
        min={extra.min}
        max={extra.max}
        onChange={(event) => { onChange(event.target.value); }}
        aria-invalid={error !== null}
        aria-label={label}
      />
      {fieldError(error)}
    </label>
  );

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
            {props.showServerUrl === true ? t("settingsDescriptionEnterprise") : t("settingsDescription")}
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
            {props.showServerUrl === true ? (
              <fieldset className="rounded-document bg-muted p-4">
                <legend className="px-1.5 text-[12px] font-medium text-muted-foreground">
                  {t("settingsServerUrl")}
                </legend>
                <div className="grid gap-1.5">
                  <label className="flex flex-col gap-1" htmlFor="settings-server-url">
                    <input
                      id="settings-server-url"
                      aria-label={t("settingsServerUrl")}
                      className={inputClass}
                      type="url"
                      inputMode="url"
                      value={form.serverUrl}
                      disabled={props.serverUrlLocked === true}
                      onChange={(event) => { update("serverUrl", event.target.value); }}
                      placeholder="https://zeokeel.example.com"
                      aria-describedby="settings-server-hint"
                    />
                  </label>
                  <p id="settings-server-hint" className="text-[12px] text-muted-foreground">
                    {props.serverUrlLocked === true ? t("settingsServerUrlLocked") : t("settingsServerUrlHint")}
                  </p>
                </div>
              </fieldset>
            ) : null}
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
              </div>
            </fieldset>
            <fieldset className="rounded-document bg-muted p-4">
              <legend className="px-1.5 text-[12px] font-medium text-muted-foreground">
                {t("settingsSectionAgent")}
              </legend>
              <p className="mb-3 text-[12px] text-muted-foreground">{t("settingsAgentDescription")}</p>
              <div className="grid grid-cols-1 gap-3.5 sm:grid-cols-2">
                {numeric("settings-max-steps", t("settingsMaxSteps"), form.maxSteps, (value) => { update("maxSteps", value); }, errors.maxSteps, { min: 1, max: 200 })}
                {numeric("settings-temperature", t("settingsTemperature"), form.temperature, (value) => { update("temperature", value); }, errors.temperature, { step: "0.1", min: 0, max: 2 })}
                {numeric("settings-max-tokens", t("settingsMaxTokens"), form.maxTokens, (value) => { update("maxTokens", value); }, errors.maxTokens, { min: 1 })}
                {numeric("settings-top-p", t("settingsTopP"), form.topP, (value) => { update("topP", value); }, errors.topP, { step: "0.05", min: 0, max: 1 })}
                {numeric("settings-seed", t("settingsSeed"), form.seed, (value) => { update("seed", value); }, errors.seed, { min: 0 })}
                <label className="flex flex-col gap-1" htmlFor="settings-tool-choice">
                  <span className={labelClass}>{t("settingsToolChoice")}</span>
                  <select
                    id="settings-tool-choice"
                    className={inputClass}
                    value={form.toolChoiceMode}
                    onChange={(event) => { update("toolChoiceMode", event.target.value === "none" || event.target.value === "required" || event.target.value === "specific" ? event.target.value : "auto"); }}
                  >
                    <option value="auto">{t("settingsToolChoiceAuto")}</option>
                    <option value="none">{t("settingsToolChoiceNone")}</option>
                    <option value="required">{t("settingsToolChoiceRequired")}</option>
                    <option value="specific">{t("settingsToolChoiceSpecific")}</option>
                  </select>
                </label>
                {form.toolChoiceMode === "specific" ? (
                  <label className="flex flex-col gap-1" htmlFor="settings-tool-choice-tools">
                    <span className={labelClass}>{t("settingsToolChoiceTools")}</span>
                    <input
                      id="settings-tool-choice-tools"
                      className={inputClass}
                      value={form.toolChoiceTools}
                      placeholder="fs.read, fs.write"
                      onChange={(event) => { update("toolChoiceTools", event.target.value); }}
                      aria-invalid={errors.toolChoiceTools !== null}
                    />
                    {fieldError(errors.toolChoiceTools)}
                  </label>
                ) : null}
                <label className="flex flex-col gap-1" htmlFor="settings-parallel-tool-calls">
                  <span className={labelClass}>{t("settingsParallelToolCalls")}</span>
                  <select
                    id="settings-parallel-tool-calls"
                    className={inputClass}
                    value={form.parallelToolCalls}
                    onChange={(event) => { update("parallelToolCalls", event.target.value === "true" ? "true" : event.target.value === "false" ? "false" : "default"); }}
                  >
                    <option value="default">{t("settingsParallelDefault")}</option>
                    <option value="true">{t("settingsParallelOn")}</option>
                    <option value="false">{t("settingsParallelOff")}</option>
                  </select>
                </label>
                <label className="flex flex-col gap-1" htmlFor="settings-reasoning-effort">
                  <span className={labelClass}>{t("settingsReasoningEffort")}</span>
                  <select
                    id="settings-reasoning-effort"
                    className={inputClass}
                    value={form.reasoningEffort}
                    onChange={(event) => { const value = event.target.value; update("reasoningEffort", value === "minimal" || value === "low" || value === "medium" || value === "high" || value === "xhigh" || value === "max" ? value : ""); }}
                  >
                    <option value="">{t("settingsReasoningDefault")}</option>
                    <option value="minimal">minimal</option>
                    <option value="low">low</option>
                    <option value="medium">medium</option>
                    <option value="high">high</option>
                    <option value="xhigh">xhigh</option>
                    <option value="max">max</option>
                  </select>
                </label>
                <label className="flex flex-col gap-1" htmlFor="settings-prompt-cache">
                  <span className={labelClass}>{t("settingsPromptCache")}</span>
                  <select
                    id="settings-prompt-cache"
                    className={inputClass}
                    value={form.promptCache}
                    onChange={(event) => { update("promptCache", event.target.value === "short" || event.target.value === "long" ? event.target.value : ""); }}
                  >
                    <option value="">{t("settingsPromptCacheDefault")}</option>
                    <option value="short">{t("settingsPromptCacheShort")}</option>
                    <option value="long">{t("settingsPromptCacheLong")}</option>
                  </select>
                </label>
                <label className="flex flex-col gap-1 sm:col-span-2" htmlFor="settings-stop">
                  <span className={labelClass}>{t("settingsStop")}</span>
                  <textarea
                    id="settings-stop"
                    className={`${inputClass} min-h-[60px] font-mono`}
                    value={form.stop}
                    onChange={(event) => { update("stop", event.target.value); }}
                    aria-describedby="settings-stop-hint"
                  />
                  <span id="settings-stop-hint" className="text-[12px] text-muted-foreground">{t("settingsStopHint")}</span>
                </label>
                <label className="flex items-center gap-3 rounded-control bg-background px-3 py-2" htmlFor="settings-memory-enabled">
                  <input
                    id="settings-memory-enabled"
                    type="checkbox"
                    checked={form.memoryEnabled}
                    onChange={(event) => { update("memoryEnabled", event.target.checked); }}
                  />
                  <span className="text-[13px] text-foreground">{t("settingsMemoryEnabled")}</span>
                </label>
                <label className="flex flex-col gap-1" htmlFor="settings-memory-dir">
                  <span className={labelClass}>{t("settingsMemoryDir")}</span>
                  <input
                    id="settings-memory-dir"
                    className={inputClass}
                    value={form.memoryDir}
                    onChange={(event) => { update("memoryDir", event.target.value); }}
                    placeholder="$ZEOKEEL_HOME/agent-memory"
                  />
                  <span className="text-[12px] text-muted-foreground">{t("settingsMemoryDirHint")}</span>
                </label>
                <label className="flex items-center gap-3 rounded-control bg-background px-3 py-2" htmlFor="settings-hooks-enabled">
                  <input
                    id="settings-hooks-enabled"
                    type="checkbox"
                    checked={form.hooksEnabled}
                    onChange={(event) => { update("hooksEnabled", event.target.checked); }}
                  />
                  <span className="text-[13px] text-foreground">{t("settingsHooksEnabled")}</span>
                </label>
                <label className="flex flex-col gap-1" htmlFor="settings-hooks-log">
                  <span className={labelClass}>{t("settingsHooksLog")}</span>
                  <input
                    id="settings-hooks-log"
                    className={inputClass}
                    value={form.hooksLog}
                    onChange={(event) => { update("hooksLog", event.target.value); }}
                    placeholder="/tmp/zeokeel-hooks.jsonl"
                  />
                  <span className="text-[12px] text-muted-foreground">{t("settingsHooksLogHint")}</span>
                </label>
              </div>
            </fieldset>
            <fieldset className="rounded-document bg-muted p-4">
              <legend className="px-1.5 text-[12px] font-medium text-muted-foreground">
                {t("settingsSectionTools")}
              </legend>
              <div className="grid grid-cols-1 gap-3.5 sm:grid-cols-2">
                {numeric("settings-tool-timeout", t("settingsToolTimeout"), form.toolTimeoutMs, (value) => { update("toolTimeoutMs", value); }, errors.toolTimeoutMs, { min: 1000, max: 600000 })}
                {numeric("settings-approval-timeout", t("settingsApprovalTimeout"), form.approvalTimeoutMs, (value) => { update("approvalTimeoutMs", value); }, errors.approvalTimeoutMs, { min: 1000, max: 1800000 })}
              </div>
              <p className="mt-2 text-[12px] text-muted-foreground">{t("settingsToolsHint")}</p>
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
          {props.onLogout !== undefined ? (
            <section className="grid gap-2 border-t border-border pt-3">
              <h3 className={labelClass}>{t("settingsLogout")}</h3>
              <div>
                <Button type="button" variant="destructive" size="sm" onClick={props.onLogout}>
                  {t("settingsLogout")}
                </Button>
              </div>
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
