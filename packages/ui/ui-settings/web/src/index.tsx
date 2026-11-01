/**
 * Settings dialog shell (`ui.settings` slot).
 *
 * Owns the dialog chrome, the window-level draft, the section navigation and the
 * single bottom save. The sections themselves are `ui.settings.pane` list-slot
 * contributions rendered from `generated/registry.ts` (`SettingsPanes`); this
 * shell hardcodes no business fields and branches on no edition (C2).
 *
 * Concurrency: `settings.get` returns `{settings, revision}`; the save action
 * sends the current revision as `expected_revision`. A conflict keeps the draft
 * and offers "reload" (the app re-reads and returns the fresh document).
 */
import * as Dialog from "@radix-ui/react-dialog";
import { useEffect, useRef, useState } from "react";
import {
  BrandLogo,
  Button,
  defaultSettings,
  IconClose,
  useUi,
  type AppSettings,
  type SettingsErrors,
  type SettingsForm,
  type SettingsPaneContribution,
  type SettingsPaneProps,
} from "@zeokeel/ui-kit";
import {
  EMPTY_SETTINGS_ERRORS,
  formToSettings,
  hasSettingsErrors,
  toForm,
  validateSettings,
} from "./settings-validation";
import { AgentPane } from "./panes/AgentPane";
import { AboutPane } from "./panes/AboutPane";
import { EmbeddingPane } from "./panes/EmbeddingPane";
import { EnterprisePane } from "./panes/EnterprisePane";
import { GeneralPane } from "./panes/GeneralPane";
import { MediaPane } from "./panes/MediaPane";
import { ModelsPane } from "./panes/ModelsPane";
import { SearchPane } from "./panes/SearchPane";
import { ToolsPane } from "./panes/ToolsPane";

/** Nav label for a pane id: `general` → `settingsPaneGeneral` (missing key falls back to the key). */
function paneLabelKey(id: string): string {
  const [first = "", ...rest] = id.split(/[-_]/);
  const camel = [
    first.charAt(0).toUpperCase() + first.slice(1),
    ...rest.map((part) => part.charAt(0).toUpperCase() + part.slice(1)),
  ].join("");
  return `settingsPane${camel}`;
}

/**
 * Error key → pane id, in display order: after a failed submit the shell jumps to
 * the first pane that holds an invalid field, then focuses that field.
 */
const ERROR_PANES: readonly (readonly [keyof SettingsErrors, string])[] = [
  ["baseUrl", "provider"],
  ["model", "provider"],
  ["temperature", "models"],
  ["maxTokens", "models"],
  ["topP", "models"],
  ["seed", "models"],
  ["toolChoiceTools", "models"],
  ["maxSteps", "agent"],
  ["scanSeconds", "agent"],
  ["toolTimeoutMs", "tools"],
  ["approvalTimeoutMs", "tools"],
  ["webSearchBaseUrl", "search"],
  ["webSearchTimeoutMs", "search"],
  ["embeddingBaseUrl", "embedding"],
  ["embeddingDims", "embedding"],
  ["mediaBaseUrl", "media"],
];

export interface SettingsDialogProps {
  /** Ordered `ui.settings.pane` contributions from the generated registry. */
  panes: readonly SettingsPaneContribution[];
  /** App-level (not edition-level) facts/actions the panes may use. */
  app: SettingsPaneProps["app"];
}

/** `settings.dialog` slot: reads settings/open state and actions from useUi. */
export function SettingsDialog(props: SettingsDialogProps) {
  const ui = useUi();
  const { t, settings, settingsOpen, actions } = ui;
  const panes = props.panes;
  // The app remounts this dialog with a key on open, so the draft always starts
  // from the persisted settings (window-level draft session, docs/19 §3.4).
  const [form, setForm] = useState<SettingsForm>(() => toForm(settings));
  const [errors, setErrors] = useState<SettingsErrors>(EMPTY_SETTINGS_ERRORS);
  const [saving, setSaving] = useState(false);
  const [saveError, setSaveError] = useState<string | null>(null);
  const [conflict, setConflict] = useState(false);
  const [selectedId, setSelectedId] = useState<string>(() => panes[0]?.id ?? "");
  const [submitAttempt, setSubmitAttempt] = useState(0);
  // The dialog is opened from app state (no `Dialog.Trigger`), so Radix has no
  // opener to restore focus to. Remember the element that had focus when the
  // dialog opened and put it back on close (WCAG focus order / B12).
  const restoreFocusRef = useRef<HTMLElement | null>(null);
  const contentRef = useRef<HTMLDivElement | null>(null);

  const activeIndex = panes.findIndex((pane) => pane.id === selectedId);
  const active = activeIndex < 0 ? panes[0] : panes[activeIndex];
  const activeId = active?.id ?? "";

  // Focus the first invalid field after a failed submit (pane switch included).
  useEffect(() => {
    if (submitAttempt === 0) {
      return;
    }
    const invalid = contentRef.current?.querySelector<HTMLElement>('[aria-invalid="true"]');
    invalid?.focus();
  }, [submitAttempt, errors, activeId]);

  const update = <Key extends keyof SettingsForm>(key: Key, value: SettingsForm[Key]): void => {
    setForm((current) => ({ ...current, [key]: value }));
  };

  const handleSubmit = (event: { preventDefault: () => void }): void => {
    event.preventDefault();
    const nextErrors = validateSettings(form);
    setErrors(nextErrors);
    setSaveError(null);
    setConflict(false);
    setSubmitAttempt((attempt) => attempt + 1);
    if (hasSettingsErrors(nextErrors)) {
      // Jump to the first pane that contains an invalid field.
      for (const [key, paneId] of ERROR_PANES) {
        if (nextErrors[key] !== null && panes.some((pane) => pane.id === paneId)) {
          setSelectedId(paneId);
          return;
        }
      }
      return;
    }
    setSaving(true);
    const next: AppSettings = formToSettings(form, settings.schema);
    void actions.saveSettings(next).then((outcome) => {
      if (outcome.status === "saved") {
        // The app dispatches `settings_saved` (closes the dialog and resets the
        // baseline from the returned document); nothing left to do here.
        return;
      }
      setSaving(false);
      if (outcome.status === "conflict") {
        setConflict(true);
        setSaveError(t("settingsRevisionConflict"));
        return;
      }
      setSaveError(outcome.message);
    });
  };

  const handleReload = (): void => {
    void actions
      .reloadSettings()
      .then((fresh) => {
        setForm(toForm(fresh));
        setErrors(EMPTY_SETTINGS_ERRORS);
        setSaveError(null);
        setConflict(false);
      })
      .catch((error: unknown) => {
        setSaveError(error instanceof Error ? error.message : t("settingsSaveFailed"));
      });
  };

  const handleRestoreDefaults = (): void => {
    setForm(toForm(defaultSettings()));
    setErrors(EMPTY_SETTINGS_ERRORS);
    setSaveError(null);
    setConflict(false);
  };

  const paneProps: SettingsPaneProps = {
    t,
    draft: form,
    update,
    errors,
    saving,
    app: props.app,
  };

  return (
    <Dialog.Root open={settingsOpen} onOpenChange={actions.closeSettings}>
      <Dialog.Portal>
        <Dialog.Overlay className="fixed inset-0 z-40 bg-[var(--zk-overlay)]" />
        <Dialog.Content
          className="fixed start-1/2 top-1/2 z-50 flex max-h-[86vh] w-[min(880px,calc(100vw-32px))] -translate-x-1/2 -translate-y-1/2 flex-col overflow-hidden rounded-xl bg-popover shadow-[var(--zk-shadow-lift)]"
          aria-describedby="settings-description"
          onOpenAutoFocus={() => {
            restoreFocusRef.current =
              document.activeElement instanceof HTMLElement ? document.activeElement : null;
          }}
          onCloseAutoFocus={(event) => {
            const target = restoreFocusRef.current;
            if (target?.isConnected === true) {
              event.preventDefault();
              target.focus();
            }
          }}
        >
          <div className="flex items-start justify-between gap-4 px-5 pt-5">
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
            className="mt-1.5 mb-3 px-5 text-[13px] text-muted-foreground"
          >
            {t("settingsDescription")}
          </Dialog.Description>

          <form className="flex min-h-0 flex-1 flex-col" onSubmit={handleSubmit} noValidate>
            <div className="flex min-h-0 flex-1 flex-col gap-3 px-5 min-[720px]:flex-row">
              <nav
                aria-label={t("settingsSections")}
                className="flex shrink-0 gap-1 overflow-x-auto pb-1 min-[720px]:w-44 min-[720px]:flex-col min-[720px]:overflow-y-auto min-[720px]:pe-2 min-[720px]:pb-0"
              >
                {panes.map((pane) => {
                  const isActive = pane.id === activeId;
                  return (
                    <button
                      key={pane.id}
                      type="button"
                      aria-current={isActive ? "page" : undefined}
                      className={`shrink-0 whitespace-nowrap rounded-control px-2.5 py-1.5 text-start text-[13px] transition-colors ${
                        isActive
                          ? "bg-muted font-medium text-foreground"
                          : "text-muted-foreground hover:bg-foreground/[0.05] hover:text-foreground"
                      }`}
                      onClick={() => {
                        setSelectedId(pane.id);
                      }}
                    >
                      {t(paneLabelKey(pane.id))}
                    </button>
                  );
                })}
              </nav>

              <div
                ref={contentRef}
                data-zk-settings-content=""
                className="min-h-0 flex-1 overflow-y-auto pe-1"
              >
                {active === undefined ? (
                  <div className="flex flex-col items-start gap-3 rounded-document bg-muted p-4">
                    <p className="text-[13px] text-muted-foreground">{t("settingsEmptyPanes")}</p>
                    <Button type="button" variant="default" size="sm" onClick={handleRestoreDefaults}>
                      {t("settingsRestoreDefaults")}
                    </Button>
                  </div>
                ) : (
                  <active.Component {...paneProps} />
                )}
              </div>
            </div>

            {saveError === null ? null : (
              <div
                className="mx-5 mt-3 flex flex-wrap items-center gap-2 rounded-document bg-muted px-3 py-2"
                role="alert"
              >
                <p className="text-[13px] text-destructive">{saveError}</p>
                {conflict ? (
                  <Button type="button" variant="default" size="sm" onClick={handleReload}>
                    {t("settingsReload")}
                  </Button>
                ) : null}
              </div>
            )}

            <div className="mt-3 flex justify-end gap-2 px-5 pb-5">
              <Dialog.Close asChild>
                <Button variant="ghost" disabled={saving}>
                  {t("settingsCancel")}
                </Button>
              </Dialog.Close>
              <Button type="submit" variant="primary" disabled={saving}>
                {saving ? t("settingsSaving") : t("settingsSave")}
              </Button>
            </div>
          </form>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}

/** `components` map consumed by `zk compose` (zk.toml contribution components). */
export const components = {
  "settings.dialog": SettingsDialog,
  "settings.pane.general": GeneralPane,
  "settings.pane.models": ModelsPane,
  "settings.pane.agent": AgentPane,
  "settings.pane.tools": ToolsPane,
  "settings.pane.search": SearchPane,
  "settings.pane.embedding": EmbeddingPane,
  "settings.pane.media": MediaPane,
  "settings.pane.enterprise": EnterprisePane,
  "settings.pane.about": AboutPane,
} as const;

export { formToSettings, toForm, validateSettings } from "./settings-validation";
export type { SettingsErrors, SettingsForm } from "./settings-validation";
