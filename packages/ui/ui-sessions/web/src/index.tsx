import { useEffect, useMemo, useState, type ReactNode } from "react";
import { createPortal } from "react-dom";
import {
  BrandLogo,
  Button,
  IconChevron,
  IconClose,
  IconCommand,
  IconFolder,
  IconMore,
  IconPlus,
  IconSearch,
  IconSliders,
  IconTask,
  useUi,
  type Conversation,
  type Locale,
  type Workspace,
} from "@zeokeel/ui-kit";

function sessionTime(timestamp: number, locale: Locale): string {
  const date = new Date(timestamp);
  const now = new Date();
  const sameDay =
    date.getFullYear() === now.getFullYear() &&
    date.getMonth() === now.getMonth() &&
    date.getDate() === now.getDate();
  return new Intl.DateTimeFormat(
    locale,
    sameDay ? { hour: "2-digit", minute: "2-digit" } : { month: "numeric", day: "numeric" },
  ).format(date);
}

function Modal(props: {
  labelId: string;
  title: string;
  onClose: () => void;
  children: ReactNode;
}) {
  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        props.onClose();
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => {
      window.removeEventListener("keydown", onKeyDown);
    };
  }, [props]);

  return createPortal(
    <div className="fixed inset-0 z-[60] flex items-center justify-center p-4">
      <button
        type="button"
        aria-label="close"
        className="absolute inset-0 border-0 bg-[var(--zk-overlay)]"
        onClick={props.onClose}
      />
      <div
        role="dialog"
        aria-modal="true"
        aria-labelledby={props.labelId}
        className="relative z-10 w-[min(420px,100%)] rounded-xl bg-popover p-4 shadow-[var(--zk-shadow-lift)]"
      >
        <div className="mb-3 flex items-center justify-between gap-3">
          <h2 id={props.labelId} className="text-[15px] font-semibold text-foreground">
            {props.title}
          </h2>
          <button
            type="button"
            className="inline-flex h-7 w-7 items-center justify-center rounded-control text-muted-foreground hover:bg-foreground/[0.05] hover:text-foreground"
            aria-label="close"
            onClick={props.onClose}
          >
            <IconClose />
          </button>
        </div>
        {props.children}
      </div>
    </div>,
    document.body,
  );
}

/** `sessions.sidebar` slot: DSH-style flat workspaces with nested conversations. */
export function Sidebar() {
  const {
    t,
    locale,
    brand,
    workspaces,
    conversationsByWorkspace,
    activeWorkspaceId,
    activeConversationId,
    streaming,
    sidebarOpen,
    workspaceError,
    mock,
    actions,
  } = useUi();

  const [expanded, setExpanded] = useState<Set<string>>(() => new Set());
  const [query, setQuery] = useState("");
  const [searchOpen, setSearchOpen] = useState(false);
  const [hideEmpty, setHideEmpty] = useState(false);
  const [addOpen, setAddOpen] = useState(false);
  const [pathInput, setPathInput] = useState("");
  const [titleInput, setTitleInput] = useState("");
  const [renameTarget, setRenameTarget] = useState<Workspace | null>(null);
  const [renameValue, setRenameValue] = useState("");
  const [menuId, setMenuId] = useState<string | null>(null);
  const [removeTarget, setRemoveTarget] = useState<Workspace | null>(null);
  const [reassignTo, setReassignTo] = useState("");
  const [paletteOpen, setPaletteOpen] = useState(false);

  useEffect(() => {
    setExpanded((previous) => {
      const next = new Set(previous);
      for (const workspace of workspaces) {
        next.add(workspace.id);
      }
      if (activeWorkspaceId !== null) {
        next.add(activeWorkspaceId);
      }
      return next;
    });
  }, [workspaces, activeWorkspaceId]);

  useEffect(() => {
    if (removeTarget === null) {
      return;
    }
    const candidates = workspaces.filter((workspace) => workspace.id !== removeTarget.id);
    const preferred =
      candidates.find((workspace) => workspace.is_default) ?? candidates[0] ?? null;
    setReassignTo(preferred === null ? "" : preferred.id);
  }, [removeTarget, workspaces]);

  const normalizedQuery = query.trim().toLowerCase();
  const visibleWorkspaces = useMemo(() => {
    return workspaces.filter((workspace) => {
      const sessions = conversationsByWorkspace[workspace.id] ?? [];
      if (hideEmpty && sessions.length === 0) {
        return false;
      }
      if (normalizedQuery === "") {
        return true;
      }
      return (
        workspace.title.toLowerCase().includes(normalizedQuery) ||
        workspace.path.toLowerCase().includes(normalizedQuery) ||
        sessions.some((conversation) => conversation.title.toLowerCase().includes(normalizedQuery))
      );
    });
  }, [workspaces, conversationsByWorkspace, hideEmpty, normalizedQuery]);

  const otherWorkspaces =
    removeTarget === null
      ? []
      : workspaces.filter((workspace) => workspace.id !== removeTarget.id);
  const removeHasSessions =
    removeTarget !== null && (conversationsByWorkspace[removeTarget.id]?.length ?? 0) > 0;
  const removeDisabled =
    removeTarget !== null && (removeTarget.is_default || (removeHasSessions && reassignTo === ""));

  const submitAdd = () => {
    if (pathInput.trim() === "") {
      return;
    }
    actions.createWorkspace(pathInput.trim(), titleInput.trim());
    setPathInput("");
    setTitleInput("");
    setAddOpen(false);
  };

  const submitRename = () => {
    if (renameTarget === null || renameValue.trim() === "") {
      setRenameTarget(null);
      return;
    }
    actions.renameWorkspace(renameTarget.id, renameValue.trim());
    setRenameTarget(null);
  };

  const renderSession = (workspace: Workspace, conversation: Conversation) => {
    const active = conversation.id === activeConversationId && workspace.id === activeWorkspaceId;
    const live = active && streaming;
    return (
      <li key={conversation.id}>
        <button
          type="button"
          className={`relative flex w-full items-center gap-2 rounded-sm py-[5px] pe-2 ps-[22px] text-start text-[12px] transition-colors ${
            active
              ? "bg-muted font-medium text-foreground shadow-[inset_2px_0_0_var(--zk-blue-500)]"
              : "text-muted-foreground hover:bg-foreground/[0.03] hover:text-foreground"
          }`}
          aria-current={active ? "true" : "false"}
          onClick={() => {
            actions.selectConversation(conversation.id);
          }}
        >
          <span
            className={`h-1.5 w-1.5 shrink-0 rounded-capsule ${
              live ? "zk-pulse-dot bg-live" : active ? "bg-live" : "bg-border"
            }`}
            aria-hidden="true"
          />
          <span className="min-w-0 flex-1 truncate">{conversation.title}</span>
          <span className="shrink-0 text-[11px] text-muted-foreground">
            {sessionTime(conversation.updated_at_ms, locale)}
          </span>
        </button>
      </li>
    );
  };

  return (
    <aside
      className="zk-sidebar"
      data-open={sidebarOpen ? "true" : "false"}
      aria-label={t("sidebarTitle")}
    >
      <div className="flex items-center justify-between gap-1.5 px-3 pt-3 pb-2">
        <div className="flex min-w-0 items-center gap-2">
          <BrandLogo className="h-6 w-6 shrink-0 rounded-document" decorative />
          <span className="truncate text-[13px] font-semibold text-foreground">{brand.name}</span>
        </div>
        <div className="flex items-center gap-0.5">
          <button
            type="button"
            className="zk-sidebar-close h-6 w-6 items-center justify-center rounded-sm text-muted-foreground hover:bg-foreground/[0.05] hover:text-foreground"
            aria-label={t("sidebarClose")}
            onClick={actions.toggleSidebar}
          >
            <IconClose />
          </button>
          <button
            type="button"
            className="inline-flex h-6 w-6 items-center justify-center rounded-sm text-muted-foreground hover:bg-foreground/[0.05] hover:text-foreground"
            aria-label={t("workspaceSearch")}
            aria-expanded={searchOpen}
            onClick={() => {
              setSearchOpen((value) => !value);
            }}
          >
            <IconSearch />
          </button>
          <button
            type="button"
            className="inline-flex h-6 w-6 items-center justify-center rounded-sm text-muted-foreground hover:bg-foreground/[0.05] hover:text-foreground"
            aria-label={t("newConversation")}
            onClick={actions.createConversation}
          >
            <IconPlus />
          </button>
        </div>
      </div>

      <div className="px-2">
        <Button
          className="w-full justify-start px-2 py-[5px] text-[12px]"
          onClick={actions.createConversation}
          disabled={activeWorkspaceId === null}
          title={activeWorkspaceId === null ? t("workspaceRequired") : undefined}
        >
          <IconPlus />
          <span>{t("newConversation")}</span>
        </Button>
      </div>

      <div className="mt-2 flex flex-col gap-0.5 px-2">
        <button
          type="button"
          className="flex w-full items-center gap-2 rounded-sm px-2 py-[5px] text-start text-[12px] text-muted-foreground hover:bg-foreground/[0.03] hover:text-foreground"
          onClick={() => {
            setPaletteOpen(true);
          }}
        >
          <IconCommand />
          <span>{t("commandPalette")}</span>
        </button>
        <button
          type="button"
          disabled
          className="flex w-full items-center gap-2 rounded-sm px-2 py-[5px] text-start text-[12px] text-muted-foreground/70"
        >
          <IconTask />
          <span>{t("automationTasks")}</span>
          <span className="ms-auto text-[10px]">{t("comingLater")}</span>
        </button>
        <button
          type="button"
          disabled
          className="flex w-full items-center gap-2 rounded-sm px-2 py-[5px] text-start text-[12px] text-muted-foreground/70"
        >
          <IconSliders />
          <span>{t("plugins")}</span>
          <span className="ms-auto text-[10px]">{t("comingLater")}</span>
        </button>
      </div>

      {searchOpen ? (
        <div className="px-2 pt-2">
          <input
            aria-label={t("workspaceSearch")}
            placeholder={t("workspaceSearch")}
            value={query}
            onChange={(event) => {
              setQuery(event.target.value);
            }}
            className="w-full rounded-control border border-border bg-background px-2 py-1 text-[12px] text-foreground placeholder:text-muted-foreground"
          />
        </div>
      ) : null}

      <div className="mt-3 flex items-center gap-1 px-3 pb-1">
        <span className="text-[11px] font-medium text-muted-foreground">
          {t("workspacesSection")}
        </span>
        <span className="ms-auto flex items-center gap-0.5">
          <button
            type="button"
            className="inline-flex h-5 w-5 items-center justify-center rounded-sm text-muted-foreground hover:bg-foreground/[0.05] hover:text-foreground"
            aria-label={t("workspaceSearch")}
            aria-pressed={searchOpen}
            onClick={() => {
              setSearchOpen((value) => !value);
            }}
          >
            <IconSearch />
          </button>
          <button
            type="button"
            className={`inline-flex h-5 w-5 items-center justify-center rounded-sm hover:bg-foreground/[0.05] ${
              hideEmpty ? "text-live" : "text-muted-foreground hover:text-foreground"
            }`}
            aria-label={t("workspaceFilter")}
            aria-pressed={hideEmpty}
            onClick={() => {
              setHideEmpty((value) => !value);
            }}
          >
            <IconSliders />
          </button>
          <button
            type="button"
            className="inline-flex h-5 w-5 items-center justify-center rounded-sm text-muted-foreground hover:bg-foreground/[0.05] hover:text-foreground"
            aria-label={t("workspaceAdd")}
            aria-expanded={addOpen}
            onClick={() => {
              setAddOpen((value) => !value);
            }}
          >
            <IconPlus />
          </button>
        </span>
      </div>

      {workspaceError !== null ? (
        <p className="mx-2 mb-1 rounded-document bg-muted px-2 py-1 text-[11px] text-destructive" role="alert">
          {workspaceError}
        </p>
      ) : null}

      <nav className="min-h-0 flex-1 overflow-y-auto px-1.5 pb-2" aria-label={t("workspacesSection")}>
        {addOpen ? (
          <form
            className="mb-2 rounded-document bg-muted p-2"
            onSubmit={(event) => {
              event.preventDefault();
              submitAdd();
            }}
          >
            <label className="flex flex-col gap-1 text-[11px] text-muted-foreground">
              {t("workspacePathLabel")}
              <input
                autoFocus
                value={pathInput}
                onChange={(event) => {
                  setPathInput(event.target.value);
                }}
                placeholder={t("workspaceAddHint")}
                className="w-full rounded-control border border-border bg-background px-2 py-1 text-[12px] text-foreground placeholder:text-muted-foreground"
              />
            </label>
            <label className="mt-1.5 flex flex-col gap-1 text-[11px] text-muted-foreground">
              {t("workspaceTitleLabel")}
              <input
                value={titleInput}
                onChange={(event) => {
                  setTitleInput(event.target.value);
                }}
                className="w-full rounded-control border border-border bg-background px-2 py-1 text-[12px] text-foreground"
              />
            </label>
            <div className="mt-2 flex justify-end gap-1.5">
              <Button
                size="sm"
                variant="ghost"
                type="button"
                onClick={() => {
                  setAddOpen(false);
                }}
              >
                {t("settingsCancel")}
              </Button>
              <Button size="sm" variant="primary" type="submit" disabled={pathInput.trim() === ""}>
                {t("workspaceCreate")}
              </Button>
            </div>
          </form>
        ) : null}

        {visibleWorkspaces.length === 0 ? (
          <p className="px-2 py-2 text-[12px] text-muted-foreground">
            {workspaces.length === 0 ? t("workspaceFirstTitle") : t("conversationsEmpty")}
          </p>
        ) : (
          <ul className="flex flex-col gap-0.5">
            {visibleWorkspaces.map((workspace) => {
              const workspaceActive = workspace.id === activeWorkspaceId;
              const sessions = conversationsByWorkspace[workspace.id] ?? [];
              const open = expanded.has(workspace.id);
              const filteredSessions =
                normalizedQuery === ""
                  ? sessions
                  : sessions.filter((conversation) =>
                      conversation.title.toLowerCase().includes(normalizedQuery),
                    );
              return (
                <li key={workspace.id}>
                  <div className="group relative">
                    <div
                      className={`flex items-center gap-1 rounded-sm py-[5px] pe-1 ps-1 ${
                        workspaceActive ? "bg-muted" : "hover:bg-foreground/[0.03]"
                      }`}
                    >
                      <button
                        type="button"
                        className="inline-flex h-5 w-5 shrink-0 items-center justify-center rounded-sm text-muted-foreground hover:bg-foreground/[0.06] hover:text-foreground"
                        aria-label={open ? t("collapseSessions") : t("expandSessions")}
                        aria-expanded={open}
                        onClick={() => {
                          setExpanded((previous) => {
                            const next = new Set(previous);
                            if (next.has(workspace.id)) {
                              next.delete(workspace.id);
                            } else {
                              next.add(workspace.id);
                            }
                            return next;
                          });
                        }}
                      >
                        <IconChevron
                          className={
                            open ? "transition-transform" : "-rotate-90 transition-transform"
                          }
                        />
                      </button>
                      <button
                        type="button"
                        className="flex min-w-0 flex-1 items-center gap-1.5 text-start"
                        onClick={() => {
                          actions.selectWorkspace(workspace.id);
                        }}
                      >
                        <IconFolder className="h-3.5 w-3.5 shrink-0 text-muted-foreground" />
                        <span
                          className={`min-w-0 flex-1 truncate text-[12px] ${
                            workspaceActive
                              ? "font-medium text-foreground"
                              : "text-muted-foreground"
                          }`}
                          title={workspace.path}
                        >
                          {workspace.title}
                        </span>
                        {workspace.is_default ? (
                          <span className="shrink-0 rounded-sm border border-border px-1 text-[10px] text-muted-foreground">
                            {t("workspaceDefault")}
                          </span>
                        ) : null}
                      </button>
                      <button
                        type="button"
                        className="inline-flex h-5 w-5 shrink-0 items-center justify-center rounded-sm text-muted-foreground opacity-0 hover:bg-foreground/[0.06] hover:text-foreground focus-visible:opacity-100 group-hover:opacity-100"
                        aria-label={t("workspaceActions")}
                        aria-expanded={menuId === workspace.id}
                        onClick={() => {
                          setMenuId((value) => (value === workspace.id ? null : workspace.id));
                        }}
                      >
                        <IconMore />
                      </button>
                    </div>

                    {menuId === workspace.id ? (
                      <div
                        role="menu"
                        className="absolute end-1 z-20 mt-0.5 w-[150px] rounded-surface bg-popover p-1 shadow-[var(--zk-shadow-lift)]"
                      >
                        <button
                          type="button"
                          role="menuitem"
                          className="w-full rounded-sm px-2 py-1 text-start text-[12px] text-foreground hover:bg-muted"
                          onClick={() => {
                            setRenameTarget(workspace);
                            setRenameValue(workspace.title);
                            setMenuId(null);
                          }}
                        >
                          {t("workspaceRename")}
                        </button>
                        <button
                          type="button"
                          role="menuitem"
                          disabled={workspace.is_default}
                          className="w-full rounded-sm px-2 py-1 text-start text-[12px] text-foreground hover:bg-muted disabled:opacity-40"
                          onClick={() => {
                            actions.setDefaultWorkspace(workspace.id);
                            setMenuId(null);
                          }}
                        >
                          {t("workspaceSetDefault")}
                        </button>
                        <button
                          type="button"
                          role="menuitem"
                          className="w-full rounded-sm px-2 py-1 text-start text-[12px] text-destructive hover:bg-muted"
                          onClick={() => {
                            setRemoveTarget(workspace);
                            setMenuId(null);
                          }}
                        >
                          {t("workspaceRemove")}
                        </button>
                      </div>
                    ) : null}
                  </div>

                  {open ? (
                    <ul className="ms-[13px] mt-0.5 flex flex-col gap-0.5 border-s border-border ps-1">
                      {filteredSessions.length === 0 ? (
                        <li className="py-1 ps-[22px] text-[11px] text-muted-foreground">
                          {t("workspaceNoSessions")}
                        </li>
                      ) : (
                        filteredSessions.map((conversation) =>
                          renderSession(workspace, conversation),
                        )
                      )}
                    </ul>
                  ) : null}
                </li>
              );
            })}
          </ul>
        )}
      </nav>

      <div className="border-t border-border px-2 py-2">
        <button
          type="button"
          className="flex w-full items-center gap-2 rounded-sm px-2 py-[5px] text-[12px] text-muted-foreground transition-colors hover:bg-foreground/[0.03] hover:text-foreground"
          aria-label={t("openSettings")}
          onClick={actions.openSettings}
        >
          <IconSliders />
          <span>{t("settings")}</span>
        </button>
      </div>

      {mock ? (
        <div className="border-t border-border px-3 py-1.5 text-[10px] text-muted-foreground">
          {t("mockBadge")}
        </div>
      ) : null}

      {renameTarget !== null ? (
        <Modal
          labelId="workspace-rename"
          title={t("workspaceRename")}
          onClose={() => setRenameTarget(null)}
        >
          <form
            onSubmit={(event) => {
              event.preventDefault();
              submitRename();
            }}
          >
            <input
              autoFocus
              value={renameValue}
              onChange={(event) => {
                setRenameValue(event.target.value);
              }}
              className="w-full rounded-control border border-border bg-background px-2 py-1.5 text-[13px] text-foreground"
            />
            <div className="mt-3 flex justify-end gap-2">
              <Button variant="ghost" size="sm" type="button" onClick={() => setRenameTarget(null)}>
                {t("settingsCancel")}
              </Button>
              <Button
                variant="primary"
                size="sm"
                type="submit"
                disabled={renameValue.trim() === ""}
              >
                {t("workspaceRename")}
              </Button>
            </div>
          </form>
        </Modal>
      ) : null}

      {removeTarget !== null ? (
        <Modal
          labelId="workspace-remove"
          title={t("workspaceRemoveTitle")}
          onClose={() => setRemoveTarget(null)}
        >
          <p className="text-[13px] text-muted-foreground">{t("workspaceRemoveBody")}</p>
          {removeTarget.is_default ? (
            <p className="mt-2 rounded-document bg-muted px-2 py-1.5 text-[12px] text-destructive">
              {t("workspaceRemoveDefaultHint")}
            </p>
          ) : (
            <div className="mt-3 flex flex-col gap-2">
              <label className="text-[12px] text-muted-foreground" htmlFor="workspace-reassign">
                {t("workspaceReassignTo")}
              </label>
              <select
                id="workspace-reassign"
                value={reassignTo}
                onChange={(event) => {
                  setReassignTo(event.target.value);
                }}
                className="w-full rounded-control border border-border bg-background px-2 py-1.5 text-[13px] text-foreground"
              >
                {otherWorkspaces.map((workspace) => (
                  <option key={workspace.id} value={workspace.id}>
                    {workspace.title}
                  </option>
                ))}
              </select>
            </div>
          )}
          <div className="mt-3 flex justify-end gap-2">
            <Button variant="ghost" size="sm" type="button" onClick={() => setRemoveTarget(null)}>
              {t("settingsCancel")}
            </Button>
            <Button
              variant="destructive"
              size="sm"
              type="button"
              disabled={removeDisabled}
              onClick={() => {
                if (removeTarget === null) {
                  return;
                }
                actions.removeWorkspace(
                  removeTarget.id,
                  reassignTo === "" ? undefined : reassignTo,
                );
                setRemoveTarget(null);
              }}
            >
              {t("workspaceRemoveConfirm")}
            </Button>
          </div>
        </Modal>
      ) : null}

      {paletteOpen ? (
        <Modal
          labelId="command-palette"
          title={t("commandPaletteTitle")}
          onClose={() => setPaletteOpen(false)}
        >
          <div className="flex flex-col gap-1">
            <button
              type="button"
              autoFocus
              className="flex w-full items-center gap-2 rounded-sm px-2 py-1.5 text-start text-[13px] text-foreground hover:bg-muted"
              onClick={() => {
                setPaletteOpen(false);
                actions.createConversation();
              }}
            >
              <IconPlus />
              <span>{t("newConversation")}</span>
            </button>
            <button
              type="button"
              className="flex w-full items-center gap-2 rounded-sm px-2 py-1.5 text-start text-[13px] text-foreground hover:bg-muted"
              onClick={() => {
                setPaletteOpen(false);
                setAddOpen(true);
              }}
            >
              <IconFolder />
              <span>{t("workspaceAdd")}</span>
            </button>
            <button
              type="button"
              className="flex w-full items-center gap-2 rounded-sm px-2 py-1.5 text-start text-[13px] text-foreground hover:bg-muted"
              onClick={() => {
                setPaletteOpen(false);
                actions.openSettings();
              }}
            >
              <IconSliders />
              <span>{t("settings")}</span>
            </button>
          </div>
        </Modal>
      ) : null}
    </aside>
  );
}

export const components = {
  "sessions.sidebar": Sidebar,
} as const;
