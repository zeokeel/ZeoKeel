/**
 * Settings form validation — pure, shared by the dialog and tests.
 * Errors are i18n keys (the dialog translates them via useUi().t).
 */
import type { AppSettings, Locale, ProviderKind, Theme } from "@zeokeel/ui-kit";

export interface SettingsForm {
  providerKind: ProviderKind;
  baseUrl: string;
  model: string;
  apiKey: string;
  theme: Theme;
  locale: Locale;
  autoApproveReadOnly: boolean;
  serverUrl: string;
}

export type SettingsFieldError = string | null;

export interface SettingsErrors {
  baseUrl: SettingsFieldError;
  model: SettingsFieldError;
}

function isHttpUrl(value: string): boolean {
  try {
    const url = new URL(value);
    return url.protocol === "http:" || url.protocol === "https:";
  } catch {
    return false;
  }
}

export function toForm(settings: AppSettings): SettingsForm {
  return {
    providerKind: settings.provider.kind,
    baseUrl: settings.provider.base_url,
    model: settings.provider.model,
    apiKey: settings.provider.api_key,
    theme: settings.theme,
    locale: settings.locale,
    autoApproveReadOnly: settings.auto_approve_read_only,
    serverUrl: settings.server_url,
  };
}

export function validateSettings(form: SettingsForm): SettingsErrors {
  return {
    baseUrl:
      form.providerKind === "mock" || isHttpUrl(form.baseUrl.trim()) ? null : "settingsInvalidUrl",
    model: form.model.trim() === "" ? "settingsInvalidModel" : null,
  };
}

export function formToSettings(form: SettingsForm, schema: number): AppSettings {
  return {
    schema,
    provider: {
      kind: form.providerKind,
      base_url: form.baseUrl.trim(),
      model: form.model.trim(),
      api_key: form.apiKey,
    },
    theme: form.theme,
    locale: form.locale,
    auto_approve_read_only: form.autoApproveReadOnly,
    server_url: form.serverUrl.trim(),
  };
}
