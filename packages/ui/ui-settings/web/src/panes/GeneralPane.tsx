/**
 * Pane 10 · general/appearance: theme + language.
 *
 * Renders fields for the shell-owned draft; no app/backend imports (pane props only).
 */
import type { ReactNode } from "react";
import {
  type Locale,
  type SettingsPaneProps,
  type Theme,
  inputClass,
  labelClass,
  PaneSection,
} from "@zeokeel/ui-kit";

function themeOf(value: string): Theme {
  return value === "light" || value === "dark" ? value : "auto";
}

function localeOf(value: string): Locale {
  return value === "en-US" ? "en-US" : "zh-CN";
}

export function GeneralPane(props: SettingsPaneProps): ReactNode {
  const { t, draft, update } = props;
  return (
    <PaneSection title={t("settingsSectionAppearance")}>
      <label className="flex flex-col gap-1" htmlFor="settings-theme">
        <span className={labelClass}>{t("settingsTheme")}</span>
        <select
          id="settings-theme"
          className={inputClass}
          value={draft.theme}
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
          value={draft.locale}
          onChange={(event) => {
            update("locale", localeOf(event.target.value));
          }}
        >
          <option value="zh-CN">{t("settingsLanguageZh")}</option>
          <option value="en-US">{t("settingsLanguageEn")}</option>
        </select>
      </label>
    </PaneSection>
  );
}
