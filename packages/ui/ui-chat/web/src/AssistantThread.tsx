import {
  ActionBarPrimitive,
  AuiIf,
  MessagePrimitive,
  ThreadPrimitive,
  useAuiState,
} from "@assistant-ui/react";
import { CompactDivider } from "./CompactDivider";
import type { TextMessagePartProps } from "@assistant-ui/react";
import {
  IconCopy,
  IconFolder,
  IconRetry,
  isRecord,
  readString,
  useUi,
  type ChatMessage,
} from "@zeokeel/ui-kit";
import { ApprovalDataPart, ApprovalDecideContext } from "./ApprovalPart";
import { QuestionAnswerContext, QuestionDataPart } from "./QuestionPart";
import { AssistantComposer } from "./AssistantComposer";
import { EmptyThread } from "./EmptyThread";
import { MarkdownText } from "./MarkdownText";
import { ToolDataPart } from "./ToolCallPart";
import { UsageDataPart } from "./UsagePart";
import { parseUserContent } from "./composerRefs";

const actionButton =
  "inline-flex items-center gap-1 rounded-sm p-1 text-muted-foreground transition-colors hover:bg-foreground/[0.05] hover:text-foreground";

/**
 * User message body + skill Instructions/reference chips.
 *
 * The persisted text may carry the `--- skill instructions ---` block and/or the
 * normative trailing reference block (`--- referenced files ---`); both are
 * rendered as chips and never shown verbatim (`2026-11-01-client-composer-surface`
 * R7, `2026-11-01-client-skill-commands` R2). Messages without blocks render
 * exactly as before.
 */
function UserText(props: TextMessagePartProps) {
  const { t } = useUi();
  const { body, refs, instruction } = parseUserContent(props.text);
  if (refs.length === 0 && instruction === null) {
    return (
      <p
        data-testid="user-text"
        className="max-w-[78%] rounded-thread bg-muted px-3 py-2 text-[13px] leading-[1.55] whitespace-pre-wrap text-foreground"
      >
        {props.text}
      </p>
    );
  }
  return (
    <div className="flex max-w-[78%] flex-col items-end gap-1.5">
      {body === "" ? null : (
        <p
          data-testid="user-text"
          className="max-w-full rounded-thread bg-muted px-3 py-2 text-[13px] leading-[1.55] whitespace-pre-wrap text-foreground"
        >
          {body}
        </p>
      )}
      {instruction === null ? null : (
        <ul
          data-testid="user-skill"
          aria-label={t("composerRefsLabel")}
          className="flex max-w-full flex-wrap justify-end gap-1"
        >
          <li className="inline-flex max-w-[240px] items-center gap-1 rounded-capsule border border-border bg-background px-2 py-0.5 text-[11px] text-muted-foreground">
            <IconFolder />
            <span className="truncate">{t("skillChipLabel", { name: instruction.name })}</span>
          </li>
        </ul>
      )}
      {refs.length === 0 ? null : (
        <ul
          data-testid="user-refs"
          aria-label={t("composerRefsLabel")}
          className="flex max-w-full flex-wrap justify-end gap-1"
        >
          {refs.map((ref) => (
            <li
              key={ref.path}
              className="inline-flex max-w-[240px] items-center gap-1 rounded-capsule border border-border bg-background px-2 py-0.5 text-[11px] text-muted-foreground"
            >
              <IconFolder />
              <span className="truncate">{ref.path}</span>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

function UserMessage() {
  const { t } = useUi();
  return (
    <MessagePrimitive.Root className="mx-auto w-full max-w-[760px] py-3">
      <div className="flex flex-col items-end gap-1.5">
        <span className="text-[12px] text-muted-foreground">{t("you")}</span>
        <MessagePrimitive.Content components={{ Text: UserText }} />
      </div>
    </MessagePrimitive.Root>
  );
}

function AssistantStatus() {
  const { t } = useUi();
  const status = useAuiState((state) => state.message.status);
  if (status === undefined) {
    return null;
  }
  if (status.type === "running") {
    return (
      <span className="inline-flex items-center gap-1.5 text-[12px] text-muted-foreground">
        <span className="zk-pulse-dot h-1.5 w-1.5 rounded-capsule bg-live" aria-hidden="true" />
        {t("generating")}
      </span>
    );
  }
  if (status.type === "incomplete" && status.reason === "cancelled") {
    return <span className="text-[12px] text-muted-foreground">{t("stopped")}</span>;
  }
  if (status.type === "incomplete" && status.reason === "error") {
    return <span className="text-[12px] text-destructive">{t("agentError")}</span>;
  }
  return null;
}

function CopyAction() {
  const { t } = useUi();
  return (
    <ActionBarPrimitive.Copy aria-label={t("copy")} title={t("copy")} className={actionButton}>
      <IconCopy />
    </ActionBarPrimitive.Copy>
  );
}

function ReloadAction() {
  const { t } = useUi();
  return (
    <ActionBarPrimitive.Reload
      aria-label={t("retrySend")}
      title={t("retrySend")}
      className={actionButton}
    >
      <IconRetry />
    </ActionBarPrimitive.Reload>
  );
}

function ErrorNote() {
  const { t } = useUi();
  const status = useAuiState((state) => state.message.status);
  if (status?.type !== "incomplete" || status.reason !== "error") {
    return null;
  }
  const error = status.error;
  const text = typeof error === "string" ? error : t("agentError");
  return (
    <div role="alert" className="mt-3 rounded-sm bg-muted px-3 py-2 text-[13px] text-destructive">
      {text}
    </div>
  );
}

function TypingDots() {
  const { t } = useUi();
  return (
    <span className="flex items-center gap-1 py-2" aria-label={t("generating")}>
      <span className="zk-typing-dot h-1.5 w-1.5 rounded-capsule bg-muted-foreground" />
      <span className="zk-typing-dot h-1.5 w-1.5 rounded-capsule bg-muted-foreground" />
      <span className="zk-typing-dot h-1.5 w-1.5 rounded-capsule bg-muted-foreground" />
    </span>
  );
}

/** One `present` deliverable derived from this turn's persisted tool rows. */
interface PresentDeliverable {
  path: string;
  name: string;
  description: string;
}

/**
 * Deliverables of one assistant turn: `present` tool results parsed from the
 * persisted tool rows (deduplicated by path, call order preserved; R7).
 *
 * The plan is intentionally NOT shown here (owned by `client-plan-mode`), and no
 * second tool-row state is introduced (C17): the app message list stays the only
 * source and this is a pure read-side derivation.
 */
function presentDeliverables(toolCalls: ChatMessage["toolCalls"]): PresentDeliverable[] {
  const items: PresentDeliverable[] = [];
  for (const call of toolCalls) {
    if (call.name !== "present" || call.result === null || call.result === "") continue;
    let parsed: unknown;
    try {
      parsed = JSON.parse(call.result);
    } catch {
      continue;
    }
    if (!isRecord(parsed)) continue;
    const path = readString(parsed, "presented");
    if (path === null || path === "") continue;
    if (items.some((item) => item.path === path)) continue;
    items.push({
      path,
      name: path.split("/").at(-1) ?? path,
      description: readString(parsed, "description") ?? "",
    });
  }
  return items;
}

/**
 * Turn-tail deliverables card (`2026-11-01-client-activity-panels` R7).
 *
 * Rendered only for a finished turn (done/cancelled/error) that produced at
 * least one `present` result; clicking opens the right-panel files preview.
 */
function TurnTailPresent(props: { messageId: string }) {
  const { t, messages, actions } = useUi();
  const running = useAuiState((state) => state.message.status?.type === "running");
  if (running) return null;
  const message = messages.find((entry) => entry.id === props.messageId);
  if (message === undefined) return null;
  const deliverables = presentDeliverables(message.toolCalls);
  if (deliverables.length === 0) return null;
  return (
    <section
      data-testid="turn-tail-present"
      aria-label={t("turnTailPresentTitle")}
      className="mt-2 flex flex-col gap-1 rounded-document border border-border/60 px-3 py-2"
    >
      <p className="text-[11px] font-medium text-muted-foreground">{t("turnTailPresentTitle")}</p>
      <ul className="flex flex-col gap-1">
        {deliverables.map((deliverable) => (
          <li key={deliverable.path}>
            <button
              type="button"
              aria-label={t("turnTailPresentView", { name: deliverable.name })}
              className="flex w-full items-center gap-2 rounded-control border border-border px-2 py-1 text-left text-foreground hover:bg-muted"
              onClick={() => {
                actions.openInspectorPreview(deliverable.path);
              }}
            >
              <IconFolder />
              <span className="min-w-0 flex-1 truncate font-mono text-[12px]">{deliverable.name}</span>
              {deliverable.description === "" ? null : (
                <span className="truncate text-[11px] text-muted-foreground">
                  {deliverable.description}
                </span>
              )}
            </button>
          </li>
        ))}
      </ul>
    </section>
  );
}

function AssistantMessage() {
  const { brand } = useUi();
  const messageId = useAuiState((state) => state.message.id);
  const running = useAuiState((state) => state.message.status?.type === "running");
  const empty = useAuiState(
    (state) =>
      state.message.parts.length === 0 ||
      state.message.parts.every((part) => part.type === "text" && part.text.trim() === ""),
  );
  return (
    <MessagePrimitive.Root
      className="zk-assistant group mx-auto w-full max-w-[760px] py-3"
      data-running={running ? "true" : "false"}
    >
      <div className="flex min-h-6 items-center gap-2">
        <span className="text-[12px] font-medium text-foreground">{brand.name}</span>
        <AssistantStatus />
        <ActionBarPrimitive.Root
          className="ms-auto flex items-center gap-0.5 opacity-0 transition-opacity group-focus-within:opacity-100 group-hover:opacity-100 focus-within:opacity-100"
          autohide="never"
        >
          <CopyAction />
          <ReloadAction />
        </ActionBarPrimitive.Root>
      </div>
      <div className="mt-1.5">
        {running && empty ? <TypingDots /> : null}
        <MessagePrimitive.Content
          components={{
            Text: MarkdownText,
            data: {
              by_name: {
                approval: ApprovalDataPart,
                tool: ToolDataPart,
                question: QuestionDataPart,
                usage: UsageDataPart,
              },
            },
          }}
        />
        <ErrorNote />
        <TurnTailPresent messageId={messageId} />
      </div>
    </MessagePrimitive.Root>
  );
}

export function AssistantThread() {
  const ui = useUi();
  const { actions, compaction } = ui;

  return (
    <QuestionAnswerContext.Provider value={actions.answerQuestion}>
    <ApprovalDecideContext.Provider value={actions.decideApproval}>
      <ThreadPrimitive.Root className="flex h-full min-h-0 flex-col">
        <ThreadPrimitive.Viewport className="zk-thread-viewport">
          <div className="mx-auto flex min-h-full w-full max-w-[760px] flex-col">
            <AuiIf condition={(state) => state.thread.isEmpty}>
              <EmptyThread />
            </AuiIf>
            <div className="flex-1">
              <ThreadPrimitive.Messages>
                {({ message }) => (
                  <>
                    {message.role === "user" ? <UserMessage /> : <AssistantMessage />}
                    {compaction !== null && message.id === compaction.throughMessageId ? (
                      <CompactDivider marker={compaction} />
                    ) : null}
                  </>
                )}
              </ThreadPrimitive.Messages>
            </div>
            <ThreadPrimitive.ViewportFooter className="zk-thread-footer">
              <AssistantComposer />
            </ThreadPrimitive.ViewportFooter>
          </div>
        </ThreadPrimitive.Viewport>
      </ThreadPrimitive.Root>
    </ApprovalDecideContext.Provider>
    </QuestionAnswerContext.Provider>
  );
}
