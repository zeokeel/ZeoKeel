/**
 * Pane 80 · enterprise (enterprise edition contribution only).
 *
 * Login server address (channel-locked = read-only, C14/ADR-042), gateway model
 * selection and logout. The pane exists only in enterprise compositions
 * (`editions = ["enterprise"]`); it contains no runtime edition branch.
 */
import type { ReactNode } from "react";
import {
  type SettingsPaneProps,
  Button,
  inputClass,
  labelClass,
  PaneSection,
} from "@zeokeel/ui-kit";

export function EnterprisePane(props: SettingsPaneProps): ReactNode {
  const { t, draft, update, app } = props;
  const locked = app.serverUrlLocked;
  return (
    <>
      <PaneSection title={t("settingsServerUrl")}>
        <label className="flex flex-col gap-1 sm:col-span-2" htmlFor="settings-server-url">
          <span className={labelClass}>{t("settingsServerUrl")}</span>
          <input
            id="settings-server-url"
            aria-label={t("settingsServerUrl")}
            className={inputClass}
            type="url"
            inputMode="url"
            value={draft.serverUrl}
            disabled={locked}
            onChange={(event) => {
              update("serverUrl", event.target.value);
            }}
            placeholder="https://zeokeel.example.com"
            aria-describedby="settings-server-hint"
          />
          <p id="settings-server-hint" className="text-[12px] text-muted-foreground">
            {locked ? t("settingsServerUrlLocked") : t("settingsServerUrlHint")}
          </p>
        </label>
        {app.gatewayModels.length > 0 ? (
          <label className="flex flex-col gap-1 sm:col-span-2" htmlFor="settings-gateway-model">
            <span className={labelClass}>{t("settingsGatewayModel")}</span>
            <select
              id="settings-gateway-model"
              aria-label={t("settingsGatewayModel")}
              className={inputClass}
              value={app.gatewayModel}
              onChange={(event) => {
                app.onSelectGatewayModel?.(event.target.value);
              }}
            >
              {app.gatewayModels.map((model) => (
                <option key={model} value={model}>
                  {model}
                </option>
              ))}
            </select>
          </label>
        ) : null}
      </PaneSection>
      {app.onLogout === undefined ? null : (
        <PaneSection title={t("settingsLogout")}>
          <div className="sm:col-span-2">
            <Button type="button" variant="destructive" size="sm" onClick={app.onLogout}>
              {t("settingsLogout")}
            </Button>
          </div>
        </PaneSection>
      )}
    </>
  );
}
