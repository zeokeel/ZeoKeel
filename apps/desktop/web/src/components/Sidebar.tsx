import { BRAND } from "../generated/brand";
import { useT } from "../i18n";
import type { Conversation, Meta } from "../types";
import { BrandLogo } from "./BrandLogo";
import { IconClose, IconPlus } from "./icons";

export function Sidebar(props: {
  conversations: Conversation[];
  activeId: string | null;
  streaming: boolean;
  open: boolean;
  meta: Meta;
  mock: boolean;
  onNew: () => void;
  onSelect: (conversationId: string) => void;
  onClose: () => void;
}) {
  const t = useT();
  const metaLine =
    props.meta.packages.length === 0
      ? t("metaPackagesEmpty")
      : t("metaLine", { version: props.meta.version, packages: props.meta.packages.length });

  return (
    <aside
      className="zk-sidebar"
      data-open={props.open ? "true" : "false"}
      aria-label={t("sidebarTitle")}
    >
      <div className="flex items-center justify-between gap-2 px-4 pt-5 pb-4">
        <div className="flex min-w-0 items-center gap-2.5">
          <BrandLogo className="h-7 w-7 shrink-0 rounded-document" decorative />
          <span className="truncate text-[14px] font-semibold text-foreground">{BRAND.name}</span>
        </div>
        <button
          type="button"
          className="zk-sidebar-close h-8 w-8 items-center justify-center rounded-control text-muted-foreground transition-colors hover:bg-foreground/[0.04] hover:text-foreground"
          aria-label={t("sidebarClose")}
          onClick={props.onClose}
        >
          <IconClose />
        </button>
      </div>

      <button
        type="button"
        className="mx-4 mb-4 inline-flex items-center justify-center gap-2 rounded-control border border-border bg-background px-3 py-2 text-[13px] font-medium text-foreground transition-colors hover:bg-muted disabled:pointer-events-none disabled:opacity-40"
        onClick={props.onNew}
        disabled={props.streaming}
      >
        <IconPlus />
        <span>{t("newConversation")}</span>
      </button>

      <nav className="min-h-0 flex-1 overflow-y-auto px-2 pb-3" aria-label={t("sessionListAria")}>
        {props.conversations.length === 0 ? (
          <p className="px-2.5 py-3 text-[13px] text-muted-foreground">
            {t("conversationsEmpty")}
          </p>
        ) : (
          <ul className="flex flex-col gap-0.5">
            {props.conversations.map((conversation) => {
              const active = conversation.id === props.activeId;
              return (
                <li key={conversation.id}>
                  <button
                    type="button"
                    className={`w-full truncate rounded-sm px-2.5 py-2 text-start text-[13px] transition-colors ${
                      active
                        ? "bg-muted font-medium text-foreground shadow-[inset_2px_0_0_var(--zk-blue-500)]"
                        : "text-muted-foreground hover:bg-foreground/[0.03] hover:text-foreground"
                    } disabled:pointer-events-none disabled:opacity-40`}
                    aria-current={active ? "true" : "false"}
                    title={props.streaming ? t("statusStreaming") : ""}
                    disabled={props.streaming && !active}
                    onClick={() => {
                      props.onSelect(conversation.id);
                    }}
                  >
                    {conversation.title}
                  </button>
                </li>
              );
            })}
          </ul>
        )}
      </nav>

      <div className="flex items-center gap-2 border-t border-border px-4 py-3 text-[12px] text-muted-foreground">
        {props.mock ? (
          <span className="rounded-sm border border-border px-1.5 py-0.5 text-[12px] text-muted-foreground">
            {t("mockBadge")}
          </span>
        ) : null}
        <span className="min-w-0 truncate">{metaLine}</span>
      </div>
    </aside>
  );
}
