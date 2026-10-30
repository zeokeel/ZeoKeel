import { useState } from "react";
import { translate } from "./i18n";
import { resolvePortalHref, type UpgradeInfo } from "./upgrade";
import type { Locale, Transport } from "./types";

/** Full-screen forced-upgrade gate (round-5 §2, round-6 §3): no dismiss/business retry. */
export function UpgradeGate(props: {
  info: UpgradeInfo;
  serverUrl: string;
  serverVersion: string | null;
  locale: Locale;
  transport: Transport;
}) {
  const t = (key: string, vars?: Record<string, string | number>): string =>
    translate(props.locale, key, vars);
  const href = resolvePortalHref(props.info.portalUrl, props.serverUrl);
  const unknown = t("upgradeUnknown");
  const [openError, setOpenError] = useState<string | null>(null);
  const tauri = Reflect.get(window, "__TAURI_INTERNALS__") !== undefined;
  const openPortal = (): void => {
    if (href === null) return;
    setOpenError(null);
    if (tauri) {
      void (async () => {
        try {
          await props.transport.call("system.open_url", { url: href });
        } catch (error: unknown) {
          console.warn("[zk] system.open_url failed", String(error));
          setOpenError(t("upgradeOpenFailed"));
        }
      })();
      return;
    }
    try {
      const opened = window.open(href, "_blank", "noopener");
      if (opened === null) setOpenError(t("upgradeOpenFailed"));
    } catch {
      setOpenError(t("upgradeOpenFailed"));
    }
  };
  return (
    <div
      role="alert"
      data-testid="upgrade-gate"
      className="fixed inset-0 z-50 flex items-center justify-center bg-background p-6"
    >
      <div className="w-full max-w-md rounded-document border border-border bg-muted p-6 text-foreground">
        <h1 className="text-lg font-semibold">{t("upgradeTitle")}</h1>
        <p className="mt-2 text-[13px] text-muted-foreground">{t("upgradeBody")}</p>
        <dl className="mt-4 grid grid-cols-[auto_1fr] gap-x-4 gap-y-1 text-[13px]">
          <dt className="text-muted-foreground">{t("upgradeCurrent")}</dt>
          <dd className="font-mono">{props.info.current ?? unknown}</dd>
          <dt className="text-muted-foreground">{t("upgradeMinimum")}</dt>
          <dd className="font-mono">{props.info.minimum ?? unknown}</dd>
        </dl>
        <details className="mt-4 rounded-document border border-border px-3 py-2 text-[12px]">
          <summary className="cursor-pointer text-muted-foreground">{t("upgradeNotes")}</summary>
          <dl className="mt-2 grid grid-cols-[auto_1fr] gap-x-4 gap-y-1">
            <dt className="text-muted-foreground">{t("upgradeCurrent")}</dt>
            <dd className="font-mono">{props.info.current ?? unknown}</dd>
            <dt className="text-muted-foreground">{t("upgradeMinimum")}</dt>
            <dd className="font-mono">{props.info.minimum ?? unknown}</dd>
            {props.serverVersion === null ? null : (
              <>
                <dt className="text-muted-foreground">{t("upgradeServer")}</dt>
                <dd className="font-mono">{props.serverVersion}</dd>
              </>
            )}
            {props.info.requestId === undefined || props.info.requestId === null ? null : (
              <>
                <dt className="text-muted-foreground">{t("upgradeRequestId")}</dt>
                <dd className="font-mono break-all">{props.info.requestId}</dd>
              </>
            )}
          </dl>
        </details>
        {href === null ? null : (
          <a
            href={href}
            target="_blank"
            rel="noreferrer noopener"
            onClick={(event) => { event.preventDefault(); openPortal(); }}
            className="mt-5 inline-block rounded-control bg-foreground px-3 py-2 text-[13px] text-background"
          >
            {t("upgradePortal")}
          </a>
        )}
        {openError === null ? null : <p role="status" className="mt-2 text-[12px] text-destructive">{openError}</p>}
      </div>
    </div>
  );
}
