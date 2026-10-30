import { translate } from "./i18n";
import { portalHref, type UpgradeInfo } from "./upgrade";
import type { Locale } from "./types";

/** Full-screen forced-upgrade gate (round-5 §2): no dismiss, no retry, portal link only. */
export function UpgradeGate(props: { info: UpgradeInfo; serverUrl: string; locale: Locale }) {
  const t = (key: string, vars?: Record<string, string | number>): string =>
    translate(props.locale, key, vars);
  const href = props.info.portalUrl ?? portalHref(props.serverUrl);
  const unknown = t("upgradeUnknown");
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
        {href === null ? null : (
          <a
            href={href}
            target="_blank"
            rel="noreferrer noopener"
            className="mt-5 inline-block rounded-control bg-foreground px-3 py-2 text-[13px] text-background"
          >
            {t("upgradePortal")}
          </a>
        )}
      </div>
    </div>
  );
}
