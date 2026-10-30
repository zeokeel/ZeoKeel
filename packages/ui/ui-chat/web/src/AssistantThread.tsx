import {
  ActionBarPrimitive,
  AuiIf,
  MessagePrimitive,
  ThreadPrimitive,
  useAuiState,
} from "@assistant-ui/react";
import type { TextMessagePartProps } from "@assistant-ui/react";
import { IconCopy, IconRetry, useUi } from "@zeokeel/ui-kit";
import { ApprovalDataPart, ApprovalDecideContext } from "./ApprovalPart";
import { AssistantComposer } from "./AssistantComposer";
import { EmptyThread } from "./EmptyThread";
import { MarkdownText } from "./MarkdownText";
import { ToolCallPart } from "./ToolCallPart";

const actionButton =
  "inline-flex items-center gap-1 rounded-sm p-1 text-muted-foreground transition-colors hover:bg-foreground/[0.05] hover:text-foreground";

function UserText(props: TextMessagePartProps) {
  return (
    <p
      data-testid="user-text"
      className="max-w-[78%] rounded-thread bg-muted px-3 py-2 text-[13px] leading-[1.55] whitespace-pre-wrap text-foreground"
    >
      {props.text}
    </p>
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
  if (status === undefined || status.type !== "incomplete" || status.reason !== "error") {
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

function AssistantMessage() {
  const { t, brand } = useUi();
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
            tools: { Fallback: ToolCallPart },
            data: { by_name: { approval: ApprovalDataPart } },
          }}
        />
        <ErrorNote />
      </div>
    </MessagePrimitive.Root>
  );
}

export function AssistantThread() {
  const { actions } = useUi();
  return (
    <ApprovalDecideContext.Provider value={actions.decideApproval}>
      <ThreadPrimitive.Root className="flex h-full min-h-0 flex-col">
        <ThreadPrimitive.Viewport className="zk-thread-viewport">
          <div className="mx-auto flex min-h-full w-full max-w-[760px] flex-col">
            <AuiIf condition={(state) => state.thread.isEmpty}>
              <EmptyThread />
            </AuiIf>
            <div className="flex-1">
              <ThreadPrimitive.Messages>
                {({ message }) => (message.role === "user" ? <UserMessage /> : <AssistantMessage />)}
              </ThreadPrimitive.Messages>
            </div>
            <ThreadPrimitive.ViewportFooter className="zk-thread-footer">
              <AssistantComposer />
            </ThreadPrimitive.ViewportFooter>
          </div>
        </ThreadPrimitive.Viewport>
      </ThreadPrimitive.Root>
    </ApprovalDecideContext.Provider>
  );
}
