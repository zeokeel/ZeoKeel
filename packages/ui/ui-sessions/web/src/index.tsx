import {
  BrandLogo,
  Button,
  IconClose,
  IconPlus,
  IconSliders,
  useUi,
} from "@zeokeel/ui-kit";

/** `sessions.sidebar` slot: conversation list, new chat, settings entry, narrow drawer. */
export function Sidebar() {
  const {
    t,
    brand,
    meta,
    conversations,
    activeConversationId,
    streaming,
    sidebarOpen,
    mock,
    actions,
  } = useUi();

  const metaLine =
    meta.packages.length === 0
      ? t("metaPackagesEmpty")
      : t("metaLine", { version: meta.version, packages: meta.packages.length });

  return (
    <aside
      className="zk-sidebar"
      data-open={sidebarOpen ? "true" : "false"}
      aria-label={t("sidebarTitle")}
    >
      <div className="flex items-center justify-between gap-2 px-4 pt-5 pb-4">
        <div className="flex min-w-0 items-center gap-2.5">
          <BrandLogo className="h-7 w-7 shrink-0 rounded-document" decorative />
          <span className="truncate text-[14px] font-semibold text-foreground">{brand.name}</span>
        </div>
        <button
          type="button"
          className="zk-sidebar-close h-8 w-8 items-center justify-center rounded-control text-muted-foreground transition-colors hover:bg-foreground/[0.04] hover:text-foreground"
          aria-label={t("sidebarClose")}
          onClick={actions.toggleSidebar}
        >
          <IconClose />
        </button>
      </div>

      <Button
        className="mx-4 mb-4"
        onClick={actions.createConversation}
        disabled={streaming}
      >
        <IconPlus />
        <span>{t("newConversation")}</span>
      </Button>

      <nav className="min-h-0 flex-1 overflow-y-auto px-2 pb-3" aria-label={t("sessionListAria")}>
        {conversations.length === 0 ? (
          <p className="px-2.5 py-3 text-[13px] text-muted-foreground">{t("conversationsEmpty")}</p>
        ) : (
          <ul className="flex flex-col gap-0.5">
            {conversations.map((conversation) => {
              const active = conversation.id === activeConversationId;
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
                    title={streaming ? t("statusStreaming") : ""}
                    disabled={streaming && !active}
                    onClick={() => {
                      actions.selectConversation(conversation.id);
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

      <div className="border-t border-border px-2.5 py-2">
        <button
          type="button"
          className="flex w-full items-center gap-2 rounded-sm px-2.5 py-2 text-[13px] text-muted-foreground transition-colors hover:bg-foreground/[0.03] hover:text-foreground"
          aria-label={t("openSettings")}
          onClick={actions.openSettings}
        >
          <IconSliders />
          <span>{t("settings")}</span>
        </button>
      </div>

      <div className="flex items-center gap-2 border-t border-border px-4 py-3 text-[12px] text-muted-foreground">
        {mock ? (
          <span className="rounded-sm border border-border px-1.5 py-0.5 text-[12px]">
            {t("mockBadge")}
          </span>
        ) : null}
        <span className="min-w-0 truncate">{metaLine}</span>
      </div>
    </aside>
  );
}

export const components = {
  "sessions.sidebar": Sidebar,
} as const;
