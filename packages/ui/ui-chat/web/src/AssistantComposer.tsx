/**
 * Composer input surface (`ui-chat`; `2026-11-01-client-composer-surface`).
 *
 * Controlled native `<textarea>` (assistant-ui `ComposerPrimitive.Root` still
 * provides the container styling and the stop action) with:
 * - `/` command popup fed by the app's **single** command catalog (`ui.commands`)
 *   and executed through `ui.actions.runCommand` (same executor as ⌘K);
 * - `@` workspace file popup fed by `ui.actions.searchWorkspaceFiles`
 *   (cached `workspace.files`; no second file IPC);
 * - atomic reference/attachment chips above the input; attachments use
 *   `FileReader.readAsDataURL` thumbnails (`data:` URLs — never `blob:`);
 * - read-only model label (deep-links to the model settings pane) and usage
 *   status from the existing `AgentUsage` data.
 *
 * Sending composes the normative trailing reference block (plain text) so the
 * `chat.send` contract and the runtime stay unchanged.
 */
import { AuiIf, ComposerPrimitive } from "@assistant-ui/react";
import { useEffect, useRef, useState, type KeyboardEvent as ReactKeyboardEvent } from "react";
import {
  IconClose,
  IconFolder,
  IconPlus,
  IconRetry,
  IconSend,
  IconStop,
  useUi,
  type DraftRef,
  type WorkspaceFileRef,
} from "@zeokeel/ui-kit";
import { GoalDialog, ModelDialog, PermissionDialog } from "./CommandDialogs";
import { FeedbackDialog } from "./FeedbackDialog";
import { GoalBar } from "./GoalBar";
import {
  ATTACHMENT_EXTENSIONS,
  ATTACHMENT_MAX_COUNT,
  attachmentSizeLabel,
  composeOutgoingText,
  composerUsage,
  detectComposerTrigger,
  filterCommands,
  isImageAttachment,
  removeComposerTrigger,
  validateAttachment,
  type ComposerTrigger,
} from "./composerRefs";

const sendButton =
  "inline-flex items-center gap-1.5 rounded-control bg-foreground px-3 py-1.5 text-[13px] font-medium text-background transition-colors hover:bg-foreground/90 disabled:pointer-events-none disabled:opacity-40";
const stopButton =
  "inline-flex items-center gap-1.5 rounded-control border border-destructive/40 px-3 py-1.5 text-[13px] font-medium text-destructive transition-colors hover:bg-destructive/10";
const iconButton =
  "inline-flex h-7 w-7 items-center justify-center rounded-control text-muted-foreground transition-colors hover:bg-foreground/[0.05] hover:text-foreground disabled:pointer-events-none disabled:opacity-40";

interface ComposerMenuState {
  kind: "command" | "file";
  query: string;
  trigger: ComposerTrigger;
}

interface FileMenuState {
  status: "loading" | "ready" | "error" | "offline";
  files: WorkspaceFileRef[];
}

interface ComposerMenuItem {
  id: string;
  label: string;
  hint: string | null;
  disabled: boolean;
}

function readFileAsDataUrl(file: File): Promise<string | undefined> {
  return new Promise((resolve) => {
    const reader = new FileReader();
    reader.onload = () => {
      resolve(typeof reader.result === "string" ? reader.result : undefined);
    };
    reader.onerror = () => {
      resolve(undefined);
    };
    reader.readAsDataURL(file);
  });
}

function referenceKey(path: string): string {
  return path.endsWith("/") ? path.slice(0, -1) : path;
}

export function AssistantComposer() {
  const ui = useUi();
  const { t, offline, streaming, actions, commands, modelLabel, modelSettingsPane, messages, slashCommand, goal } = ui;
  const searchWorkspaceFiles = actions.searchWorkspaceFiles;
  const saveAttachment = actions.saveAttachment;
  const runCommand = actions.runCommand;

  const [text, setText] = useState("");
  const [refs, setRefs] = useState<DraftRef[]>([]);
  const [menu, setMenu] = useState<ComposerMenuState | null>(null);
  const [activeIndex, setActiveIndex] = useState(0);
  const [fileMenu, setFileMenu] = useState<FileMenuState>({ status: "loading", files: [] });
  const [composing, setComposing] = useState(false);
  const [attachmentError, setAttachmentError] = useState<string | null>(null);
  const [commandError, setCommandError] = useState<string | null>(null);
  const [modelOpen, setModelOpen] = useState(false);
  const [permissionOpen, setPermissionOpen] = useState(false);
  const [feedbackOpen, setFeedbackOpen] = useState(false);
  const [goalOpen, setGoalOpen] = useState(false);
  const [goalObjective, setGoalObjective] = useState("");
  const [uploads, setUploads] = useState(0);
  const [dragActive, setDragActive] = useState(false);
  const inputRef = useRef<HTMLTextAreaElement | null>(null);
  const fileInputRef = useRef<HTMLInputElement | null>(null);
  const composingRef = useRef(false);
  const pendingFiles = useRef<Map<string, File>>(new Map());
  const uploadCounter = useRef(0);

  const focusInput = (): void => {
    inputRef.current?.focus();
  };

  // Single executor intent from the app catalog: open the matching surface here
  // (no second command list / executor; C17).
  const handledSlashToken = useRef<number | null>(null);
  useEffect(() => {
    if (slashCommand === null) return undefined;
    if (handledSlashToken.current === slashCommand.token) return undefined;
    handledSlashToken.current = slashCommand.token;
    const { name, query } = slashCommand;
    const timer = window.setTimeout(() => {
      if (name === "file") {
        fileInputRef.current?.click();
      } else if (name === "model") {
        setModelOpen(true);
      } else if (name === "permission") {
        setPermissionOpen(true);
      } else if (name === "feedback") {
        setFeedbackOpen(true);
      } else if (name === "export") {
        void actions.exportConversation();
      } else if (name === "goal") {
        setGoalObjective(query);
        setGoalOpen(true);
      }
    }, 0);
    return () => { window.clearTimeout(timer); };
  }, [actions, slashCommand]);
  const setCaret = (caret: number): void => {
    const input = inputRef.current;
    if (input === null) {
      return;
    }
    input.setSelectionRange(caret, caret);
  };

  const count = Array.from(text).length;
  const readyRefs = refs.filter((ref) => ref.kind === "workspace" || ref.status === "ready");
  const hasBody = text.trim() !== "";
  const hasFailedAttachment = refs.some((ref) => ref.status === "error");
  const uploading = uploads > 0;
  const canSend =
    !streaming &&
    !offline &&
    !uploading &&
    !hasFailedAttachment &&
    (hasBody || readyRefs.length > 0);

  const menuItems: readonly ComposerMenuItem[] =
    menu === null
      ? []
      : menu.kind === "command"
        ? filterCommands(
            commands.filter((command) => command.surface !== "palette"),
            menu.query.split(" ")[0] ?? menu.query,
          ).map((command) => ({
            id: `zk-composer-command-${command.name ?? command.id}`,
            label: command.title,
            hint: command.group,
            disabled: command.disabled === true || command.kind === "disabled",
          }))
        : fileMenu.files.map((file, index) => ({
            id: `zk-composer-file-${String(index)}`,
            label: file.path,
            hint: file.kind === "dir" ? t("fileMenuDirTag") : t("fileMenuFileTag"),
            disabled: false,
          }));
  const itemCount = menuItems.length;
  const safeActive = itemCount === 0 ? 0 : Math.min(activeIndex, itemCount - 1);
  const menuId =
    menu === null
      ? undefined
      : menu.kind === "command"
        ? "zk-composer-command-list"
        : "zk-composer-file-list";
  const menuLabel = menu?.kind === "command" ? t("commandMenuLabel") : t("fileMenuLabel");
  const activeItem = itemCount === 0 ? undefined : menuItems[safeActive];
  const menuEmptyText =
    menu?.kind === "command"
      ? t("commandMenuEmpty")
      : offline
        ? t("fileMenuOffline")
        : fileMenu.status === "loading"
          ? t("fileMenuLoading")
          : fileMenu.status === "error"
            ? t("fileMenuError")
            : t("fileMenuEmpty");

  // `@` candidates are fetched through the app action (cached workspace.files).
  useEffect(() => {
    if (menu?.kind !== "file" || offline) {
      return;
    }
    let cancelled = false;
    void searchWorkspaceFiles(menu.query)
      .then((files) => {
        if (!cancelled) {
          setFileMenu({ status: "ready", files });
        }
      })
      .catch(() => {
        if (!cancelled) {
          setFileMenu({ status: "error", files: [] });
        }
      });
    return () => {
      cancelled = true;
    };
  }, [menu, offline, searchWorkspaceFiles]);

  const syncMenu = (nextText: string, caret: number): void => {
    if (composingRef.current) {
      setMenu(null);
      return;
    }
    const trigger = detectComposerTrigger(nextText, caret);
    if (trigger === null) {
      setMenu(null);
      return;
    }
    setActiveIndex(0);
    if (trigger.kind === "file") {
      setFileMenu({ status: "loading", files: [] });
    }
    setMenu({ kind: trigger.kind, query: trigger.query, trigger });
  };

  const applyTriggerRemoval = (trigger: ComposerTrigger): void => {
    const next = removeComposerTrigger(text, trigger);
    setText(next.text);
    setMenu(null);
    focusInput();
    setCaret(next.caret);
  };

  const addReference = (file: WorkspaceFileRef, trigger: ComposerTrigger): void => {
    if (refs.length >= ATTACHMENT_MAX_COUNT) {
      setAttachmentError("attachmentLimit");
      return;
    }
    const path = file.kind === "dir" ? `${file.path}/` : file.path;
    setRefs((current) => {
      if (current.some((ref) => referenceKey(ref.path) === referenceKey(path))) {
        return current;
      }
      const name = referenceKey(path).split("/").pop() ?? path;
      return [...current, { kind: "workspace", path, name }];
    });
    setAttachmentError(null);
    applyTriggerRemoval(trigger);
  };

  const runActiveItem = (): void => {
    if (menu === null) {
      return;
    }
    if (menu.kind === "command") {
      const slashCommands = commands.filter((command) => command.surface !== "palette");
      const head = menu.query.split(" ")[0] ?? menu.query;
      // `leadingInput` commands accept `/goal <objective>` (name token + free text).
      const textAfterHead = menu.query.slice(head.length).trim();
      const command =
        slashCommands.find((entry) => entry.name === head && entry.kind === "leadingInput") ??
        filterCommands(slashCommands, menu.query)[safeActive];
      if (command === undefined) {
        // Unknown command: never silently forward `/text` as a prompt (DSH semantics).
        setMenu(null);
        setCommandError(t("commandUnknown"));
        focusInput();
        return;
      }
      if (command.disabled === true || command.kind === "disabled") {
        setMenu(null);
        setCommandError(t("commandUnimplemented"));
        focusInput();
        return;
      }
      if (command.name !== "file" && refs.some((ref) => ref.kind === "attachment")) {
        setMenu(null);
        setCommandError(t("commandAttachmentsOnlyFile"));
        focusInput();
        return;
      }
      setCommandError(null);
      applyTriggerRemoval(menu.trigger);
      if (command.kind === "leadingInput") {
        runCommand(command.id, textAfterHead);
      } else {
        runCommand(command.id);
      }
      return;
    }
    const file = fileMenu.files[safeActive];
    if (file !== undefined) {
      addReference(file, menu.trigger);
    }
  };

  const uploadOne = (file: File, existingId?: string, thumbnail?: string): void => {
    uploadCounter.current += 1;
    const id = existingId ?? `att-${String(uploadCounter.current)}`;
    pendingFiles.current.set(id, file);
    setRefs((current) => {
      const entry: DraftRef = {
        id,
        kind: "attachment",
        path: "",
        name: file.name,
        size: file.size,
        mime: file.type,
        status: "uploading",
        ...(thumbnail === undefined ? {} : { thumbnail }),
      };
      if (existingId === undefined) {
        return [...current, entry];
      }
      return current.map((ref) => (ref.id === existingId ? entry : ref));
    });
    setUploads((current) => current + 1);
    void saveAttachment(file)
      .then((saved) => {
        setRefs((current) =>
          current.map((ref) =>
            ref.id === id
              ? {
                  ...ref,
                  path: saved.path,
                  name: saved.name,
                  size: saved.bytes,
                  mime: saved.mime,
                  status: "ready",
                }
              : ref,
          ),
        );
      })
      .catch(() => {
        setRefs((current) =>
          current.map((ref) =>
            ref.id === id ? { ...ref, status: "error", error: "attachmentFailed" } : ref,
          ),
        );
      })
      .finally(() => {
        setUploads((current) => Math.max(0, current - 1));
      });
  };

  const addFiles = (files: readonly File[]): void => {
    setAttachmentError(null);
    let accepted = refs.length;
    for (const file of files) {
      const error = validateAttachment(file, accepted);
      if (error !== null) {
        setAttachmentError(error);
        continue;
      }
      accepted += 1;
      if (isImageAttachment(file.name)) {
        void readFileAsDataUrl(file).then((thumbnail) => {
          uploadOne(file, undefined, thumbnail);
        });
        continue;
      }
      uploadOne(file);
    }
  };

  const retryAttachment = (ref: DraftRef): void => {
    if (ref.id === undefined) {
      return;
    }
    const file = pendingFiles.current.get(ref.id);
    if (file === undefined) {
      return;
    }
    uploadOne(file, ref.id, ref.thumbnail);
  };

  const removeRef = (id: string | undefined, path: string, kind: DraftRef["kind"]): void => {
    setRefs((current) =>
      current.filter((ref) => {
        if (id !== undefined) {
          return ref.id !== id;
        }
        if (ref.id !== undefined) {
          return true;
        }
        return !(ref.kind === kind && ref.path === path);
      }),
    );
  };

  const submit = (): void => {
    // `/goal <objective>` (DSH leadingInput): never send command text as a prompt.
    const goalMatch = /^\s*\/goal(?:\s+(.*))?$/s.exec(text);
    if (goalMatch !== null) {
      const objective = (goalMatch[1] ?? "").trim();
      setText("");
      setMenu(null);
      setGoalObjective(objective === "clear" ? "" : objective);
      setGoalOpen(true);
      return;
    }
    if (!canSend) {
      return;
    }
    const outgoing = composeOutgoingText(text, refs);
    actions.send(outgoing);
    setText("");
    setRefs([]);
    setMenu(null);
    setAttachmentError(null);
    focusInput();
  };

  const onKeyDown = (event: ReactKeyboardEvent<HTMLTextAreaElement>): void => {
    if (event.nativeEvent.isComposing || composingRef.current) {
      return;
    }
    if (menu !== null) {
      if (event.key === "ArrowDown") {
        event.preventDefault();
        setActiveIndex(itemCount === 0 ? 0 : Math.min(safeActive + 1, itemCount - 1));
        return;
      }
      if (event.key === "ArrowUp") {
        event.preventDefault();
        setActiveIndex(Math.max(safeActive - 1, 0));
        return;
      }
      if (event.key === "Escape") {
        event.preventDefault();
        setMenu(null);
        return;
      }
      if (event.key === "Enter") {
        event.preventDefault();
        runActiveItem();
        return;
      }
      if (event.key === "Tab") {
        event.preventDefault();
        if (event.shiftKey) {
          setMenu(null);
          focusInput();
        } else {
          runActiveItem();
        }
        return;
      }
    }
    if (event.key === "Enter" && !event.shiftKey) {
      event.preventDefault();
      submit();
    }
  };

  const openReferenceMenu = (): void => {
    const input = inputRef.current;
    const caret = input === null ? text.length : input.selectionStart;
    const before = text.slice(0, caret);
    const prefix = before === "" || /\s$/.test(before) ? "" : " ";
    const triggerStart = caret + prefix.length;
    const next = `${before}${prefix}@${text.slice(caret)}`;
    setText(next);
    setActiveIndex(0);
    setFileMenu({ status: "loading", files: [] });
    setMenu({
      kind: "file",
      query: "",
      trigger: { kind: "file", query: "", start: triggerStart, end: triggerStart + 1 },
    });
    focusInput();
    setCaret(triggerStart + 1);
  };

  const usage = composerUsage(messages);
  // Visible text stays compact; the full localized sentence is the accessible
  // name so the readout cannot be confused with the per-message usage part.
  const usageText =
    usage.turn === null || usage.session === null
      ? t("usageUnreported")
      : `${t("usageTurnCompact", {
          input: usage.turn.input_tokens,
          output: usage.turn.output_tokens,
          cached: usage.turn.cached_tokens,
          total: usage.turn.total_tokens,
        })} · ${t("usageSessionCompact", {
          input: usage.session.input_tokens,
          output: usage.session.output_tokens,
          cached: usage.session.cached_tokens,
          total: usage.session.total_tokens,
        })}`;
  const usageAria =
    usage.turn === null || usage.session === null
      ? t("usageUnreported")
      : `${t("usageLabel")}: ${t("usageTurnLine", {
          input: usage.turn.input_tokens,
          output: usage.turn.output_tokens,
          cached: usage.turn.cached_tokens,
          total: usage.turn.total_tokens,
        })} · ${t("usageSessionLine", {
          input: usage.session.input_tokens,
          output: usage.session.output_tokens,
          cached: usage.session.cached_tokens,
          total: usage.session.total_tokens,
        })}`;

  return (
    <div className="mx-auto w-full max-w-[760px]">
      <ComposerPrimitive.Root
        className={`rounded-thread border bg-background px-3 pt-2.5 pb-2 transition-colors focus-within:border-foreground/35 ${
          dragActive ? "border-foreground/60" : "border-border"
        }`}
        onDragOver={(event) => {
          event.preventDefault();
          setDragActive(true);
        }}
        onDragLeave={() => {
          setDragActive(false);
        }}
        onDrop={(event) => {
          event.preventDefault();
          setDragActive(false);
          addFiles(Array.from(event.dataTransfer.files));
        }}
      >
        <span className="sr-only" role="status">
          {menu === null ? "" : menuLabel}
        </span>
        {goal === null || goal.status === "cancelled" ? null : (
          <GoalBar
            goal={goal}
            onConfirmClear={() => {
              setGoalObjective("");
              setGoalOpen(true);
            }}
          />
        )}
        {commandError === null ? null : (
          <p role="alert" className="mb-2 rounded-control border border-destructive/40 px-2 py-1 text-[12px] text-destructive">
            {commandError}
          </p>
        )}
        {menu === null ? null : (
          <div className="mb-2 rounded-control border border-border bg-popover p-1 shadow-[var(--zk-shadow-lift)]">
            <div
              id={menuId}
              role="listbox"
              aria-label={menuLabel}
              aria-busy={menu.kind === "file" && fileMenu.status === "loading"}
              // Scrollable region must be focusable (axe `scrollable-region-focusable`);
              // focus is never moved here programmatically — ↑↓/Enter stay on the input.
              tabIndex={0}
              onMouseDown={(event) => { event.preventDefault(); }}
              className="max-h-[220px] overflow-y-auto"
            >
              {itemCount === 0 ? (
                <p className="px-2 py-2 text-[12px] text-muted-foreground">{menuEmptyText}</p>
              ) : (
                menuItems.map((item, index) => (
                  <button
                    key={item.id}
                    id={item.id}
                    type="button"
                    role="option"
                    tabIndex={-1}
                    aria-selected={index === safeActive}
                    disabled={item.disabled}
                    onMouseDown={(event) => {
                      event.preventDefault();
                    }}
                    onMouseEnter={() => {
                      setActiveIndex(index);
                    }}
                    onClick={() => {
                      setActiveIndex(index);
                      runActiveItem();
                    }}
                    className={`flex w-full items-center gap-2 rounded-control px-2 py-1.5 text-start text-[12px] ${
                      index === safeActive ? "bg-muted" : ""
                    } hover:bg-muted disabled:opacity-40`}
                  >
                    <span className="flex-1 truncate">{item.label}</span>
                    {item.hint === null ? null : (
                      <span className="font-mono text-[10px] text-muted-foreground">{item.hint}</span>
                    )}
                  </button>
                ))
              )}
            </div>
            {menu.kind === "file" && fileMenu.status === "error" ? (
              <button
                type="button"
                className="mt-1 flex w-full items-center gap-1 rounded-control px-2 py-1 text-[12px] text-foreground hover:bg-muted"
                onClick={() => {
                  setFileMenu({ status: "loading", files: [] });
                  void searchWorkspaceFiles(menu.query).then(
                    (files) => {
                      setFileMenu({ status: "ready", files });
                    },
                    () => {
                      setFileMenu({ status: "error", files: [] });
                    },
                  );
                }}
              >
                <IconRetry />
                <span>{t("fileMenuRetry")}</span>
              </button>
            ) : null}
          </div>
        )}

        {refs.length > 0 ? (
          <ul aria-label={t("composerRefsLabel")} className="mb-2 flex flex-wrap gap-1.5">
            {refs.map((ref) => {
              const isAttachment = ref.kind === "attachment";
              const label = ref.name ?? ref.path;
              return (
                <li
                  key={ref.id ?? `${ref.kind}:${ref.path}`}
                  className="flex max-w-full items-center gap-1.5 rounded-capsule border border-border bg-muted px-2 py-1 text-[12px] text-foreground"
                >
                  {ref.thumbnail === undefined ? null : (
                    <img
                      src={ref.thumbnail}
                      alt={t("attachmentImageAlt", { name: label })}
                      className="h-5 w-5 rounded-sm object-cover"
                    />
                  )}
                  <span className="max-w-[220px] truncate">{label}</span>
                  {isAttachment && ref.size !== undefined ? (
                    <span className="text-[10px] text-muted-foreground">
                      {attachmentSizeLabel(ref.size)}
                    </span>
                  ) : null}
                  {ref.status === "uploading" ? (
                    <span className="text-[10px] text-muted-foreground">
                      {t("attachmentUploading")}
                    </span>
                  ) : null}
                  {ref.status === "error" ? (
                    <>
                      <span role="alert" className="text-[10px] text-destructive">
                        {t(ref.error ?? "attachmentFailed")}
                      </span>
                      <button
                        type="button"
                        aria-label={t("attachmentRetry")}
                        className={iconButton}
                        onClick={() => {
                          retryAttachment(ref);
                        }}
                      >
                        <IconRetry />
                      </button>
                    </>
                  ) : null}
                  <button
                    type="button"
                    aria-label={
                      isAttachment
                        ? t("attachmentRemove", { name: label })
                        : t("composerRefRemove", { name: label })
                    }
                    className="inline-flex h-4 w-4 items-center justify-center rounded-capsule text-muted-foreground hover:bg-foreground/[0.08] hover:text-foreground"
                    onClick={() => {
                      removeRef(ref.id, ref.path, ref.kind);
                    }}
                  >
                    <IconClose />
                  </button>
                </li>
              );
            })}
          </ul>
        ) : null}

        {attachmentError === null ? null : (
          <p role="alert" className="mb-2 text-[12px] text-destructive">
            {t(attachmentError)}
          </p>
        )}

        <label className="sr-only" htmlFor="zk-composer-input">
          {t("composerAria")}
        </label>
        <textarea
          id="zk-composer-input"
          ref={inputRef}
          aria-label={t("composerAria")}
          placeholder={t("composerPlaceholder")}
          className="zk-composer-input min-h-[52px] w-full resize-none bg-transparent text-[13px] text-foreground outline-none placeholder:text-muted-foreground"
          rows={2}
          value={text}
          disabled={offline}
          aria-haspopup="listbox"
          aria-controls={menuId}
          aria-autocomplete="list"
          aria-activedescendant={activeItem?.id}
          onChange={(event) => {
            const value = event.target.value;
            setText(value);
            syncMenu(value, event.target.selectionStart);
          }}
          onKeyDown={onKeyDown}
          onClick={(event) => {
            syncMenu(text, event.currentTarget.selectionStart);
          }}
          onSelect={(event) => {
            if (menu !== null) {
              return;
            }
            syncMenu(text, event.currentTarget.selectionStart);
          }}
          onCompositionStart={() => {
            composingRef.current = true;
            setComposing(true);
            setMenu(null);
          }}
          onCompositionEnd={(event) => {
            composingRef.current = false;
            setComposing(false);
            const value = event.currentTarget.value;
            syncMenu(value, event.currentTarget.selectionStart);
          }}
        />

        <div className="flex flex-wrap items-center gap-2 pt-1">
          <button
            type="button"
            aria-label={t("composerReferenceAria")}
            title={t("composerReference")}
            className={iconButton}
            onClick={openReferenceMenu}
          >
            <IconFolder />
          </button>
          <button
            type="button"
            aria-label={t("composerAttachAria")}
            title={t("composerAttach")}
            className={iconButton}
            disabled={refs.length >= ATTACHMENT_MAX_COUNT}
            onClick={() => {
              fileInputRef.current?.click();
            }}
          >
            <IconPlus />
          </button>
          <input
            ref={fileInputRef}
            data-testid="composer-attach-input"
            type="file"
            multiple
            hidden
            tabIndex={-1}
            aria-hidden="true"
            accept={ATTACHMENT_EXTENSIONS.map((extension) => `.${extension}`).join(",")}
            onChange={(event) => {
              addFiles(Array.from(event.target.files ?? []));
              event.target.value = "";
            }}
          />
          <span className="min-w-0 flex-1 truncate text-[12px] text-muted-foreground">
            {offline ? t("offlineSendBlocked") : composing ? null : t("keyboardHint")}
          </span>
          <button
            type="button"
            className="inline-flex items-center gap-1 rounded-control px-2 py-1 text-[12px] text-muted-foreground transition-colors hover:bg-foreground/[0.05] hover:text-foreground"
            aria-label={t("modelLabelAria", { model: modelLabel })}
            title={modelLabel}
            onClick={() => {
              actions.openModelSettings(modelSettingsPane);
            }}
          >
            <span className="max-w-[160px] truncate">{modelLabel}</span>
          </button>
          <span
            role="status"
            aria-label={usageAria}
            className="max-w-[320px] truncate text-[12px] tabular-nums text-muted-foreground"
          >
            {usageText}
          </span>
          <span className="text-[12px] tabular-nums text-muted-foreground" aria-hidden="true">
            {t("charCount", { count })}
          </span>
          <AuiIf condition={(state) => state.thread.isRunning}>
            <ComposerPrimitive.Cancel aria-label={t("stopAria")} className={stopButton}>
              <IconStop />
              <span>{t("stop")}</span>
            </ComposerPrimitive.Cancel>
          </AuiIf>
          <button
            type="button"
            aria-label={t("send")}
            className={sendButton}
            disabled={!canSend}
            onClick={submit}
          >
            <IconSend />
            <span>{t("send")}</span>
          </button>
        </div>
      </ComposerPrimitive.Root>
      <ModelDialog open={modelOpen} onClose={() => { setModelOpen(false); }} />
      <PermissionDialog open={permissionOpen} onClose={() => { setPermissionOpen(false); }} />
      <FeedbackDialog open={feedbackOpen} onClose={() => { setFeedbackOpen(false); }} />
      <GoalDialog
        open={goalOpen}
        initialObjective={goalObjective}
        onClose={() => { setGoalOpen(false); }}
      />
    </div>
  );
}
