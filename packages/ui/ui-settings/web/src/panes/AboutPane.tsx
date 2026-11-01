/**
 * Pane 90 · about & updates: personal updater check (GitHub Releases) and
 * edition/build identification.
 *
 * The update action/status come from the app (`SettingsPaneProps.app`); the pane
 * never calls the network itself.
 */
import type { ReactNode } from "react";
import {
  type SettingsPaneProps,
  Button,
  labelClass,
  PaneSection,
} from "@zeokeel/ui-kit";

export function AboutPane(props: SettingsPaneProps): ReactNode {
  const { t, app } = props;
  return (
    <PaneSection title={t("settingsUpdates")}>
      {app.onCheckUpdate === undefined ? (
        <p className="text-[12px] text-muted-foreground sm:col-span-2">
          {t("settingsUpdatesUnavailable")}
        </p>
      ) : (
        <>
          <div className="flex items-center gap-2 sm:col-span-2">
            <Button
              type="button"
              variant="default"
              size="sm"
              disabled={app.updateBusy}
              onClick={app.onCheckUpdate}
            >
              {app.updateBusy ? t("settingsCheckingUpdate") : t("settingsCheckUpdate")}
            </Button>
            {app.updateText === null ? null : (
              <span className="text-[12px] text-muted-foreground">{app.updateText}</span>
            )}
          </div>
          <p className="text-[12px] text-muted-foreground sm:col-span-2">
            {t("settingsUpdatesHint")}
          </p>
        </>
      )}
      <p className={`${labelClass} sm:col-span-2`}>{t("settingsAboutHint")}</p>
    </PaneSection>
  );
}
