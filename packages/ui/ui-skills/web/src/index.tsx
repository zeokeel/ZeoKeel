/** Enterprise skill catalog/install panel (installation owned by app IPC). */
import { useRef } from "react";
import { useDialogFocus, useUi } from "@zeokeel/ui-kit";

export interface SkillItem {
  id: string;
  name: string;
  version: string;
  sha256: string;
  enforced: boolean;
  min_client_version: string;
  can_install: boolean;
}

export interface SkillsPanelProps {
  open: boolean;
  items: SkillItem[];
  installed: { name: string; version: string }[];
  busy: string | null;
  error: string | null;
  onInstall: (item: SkillItem) => void;
  onClose: () => void;
}

export function SkillsPanel(props: SkillsPanelProps) {
  const { t } = useUi();
  const dialogRef = useRef<HTMLDivElement>(null);
  useDialogFocus(dialogRef, props.onClose, props.open);
  if (!props.open) return null;
  const isInstalled = (item: SkillItem): boolean =>
    props.installed.some((entry) => entry.name === item.name && entry.version === item.version);
  const hasAnyInstalled = (item: SkillItem): boolean =>
    props.installed.some((entry) => entry.name === item.name);
  const actionLabel = (item: SkillItem): string => {
    if (!item.can_install) return t("skillsNotInstallable");
    if (props.busy === item.name) return hasAnyInstalled(item) ? t("skillsUpdating") : t("skillsInstalling");
    return hasAnyInstalled(item) ? t("skillsUpdate") : t("skillsInstall");
  };
  return (
    <div ref={dialogRef} className="fixed inset-0 z-50 flex justify-center bg-foreground/20 px-6 pt-[14vh]" role="dialog" aria-modal="true" aria-label={t("skillsTitle")}>
      <button type="button" className="absolute inset-0" aria-label={t("skillsClose")} onClick={props.onClose} />
      <div className="relative h-fit max-h-[70vh] w-full max-w-[560px] overflow-y-auto rounded-xl border border-border bg-popover p-3 shadow-[var(--zk-shadow-lift)]">
        <div className="flex items-center justify-between">
          <h2 className="text-[14px] font-semibold">{t("skillsTitle")}</h2>
          <button type="button" data-autofocus onClick={props.onClose} className="text-[12px] text-muted-foreground">{t("close")}</button>
        </div>
        {props.error !== null ? <p role="alert" className="mt-2 rounded-document bg-muted px-2 py-1.5 text-[12px] text-destructive">{props.error}</p> : null}
        {props.items.length === 0 ? <p className="mt-4 text-[12px] text-muted-foreground">{t("skillsEmpty")}</p> : null}
        <ul className="mt-3 grid gap-2">
          {props.items.map((item) => (
            <li key={item.id} className="flex items-center gap-3 rounded-document bg-muted px-3 py-2">
              <div className="min-w-0 flex-1">
                <p className="truncate text-[13px] text-foreground">{item.name}</p>
                <p className="text-[11px] text-muted-foreground">
                  {item.version === "" ? t("skillsUnpublished") : item.version} · {item.sha256.slice(0, 12)}
                </p>
              </div>
              {item.enforced ? <span className="text-[10px] text-destructive">{t("skillsEnforced")}</span> : null}
              {isInstalled(item)
                ? <span className="text-[11px] text-muted-foreground">{t("skillsInstalled")}</span>
                : (
                  <button
                    type="button"
                    disabled={!item.can_install || props.busy === item.name}
                    onClick={() => { props.onInstall(item); }}
                    className="rounded-control bg-foreground px-2 py-1 text-[11px] text-background disabled:opacity-40"
                  >
                    {actionLabel(item)}
                  </button>
                )}
            </li>
          ))}
        </ul>
      </div>
    </div>
  );
}

export const components = { "panel.skills": SkillsPanel } as const;
