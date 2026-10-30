import { AssistantRuntimeProvider } from "@assistant-ui/react";
import { IconRetry, useUi } from "@zeokeel/ui-kit";
import { AssistantThread } from "./AssistantThread";
import { EmptyThread } from "./EmptyThread";
import { useChatRuntime } from "./runtime";

/** `chat.thread` slot: assistant-ui thread + composer for the app's message state. */
export function ChatView() {
  const ui = useUi();
  const runtime = useChatRuntime(ui);

  if (ui.messagesState === "loading") {
    return (
      <section
        className="flex min-h-0 flex-1 flex-col items-center justify-center gap-4 px-6 py-16"
        aria-label={ui.t("messageListAria")}
        aria-busy="true"
      >
        <span className="zk-spinner" aria-hidden="true" />
        <p className="text-[13px] text-muted-foreground">{ui.t("messagesLoading")}</p>
        <div className="mt-2 flex w-full max-w-[420px] flex-col gap-2" aria-hidden="true">
          <span className="zk-skeleton w-full" />
          <span className="zk-skeleton w-2/3" />
          <span className="zk-skeleton w-full" />
        </div>
      </section>
    );
  }

  if (ui.messagesState === "error") {
    return (
      <section
        className="flex min-h-0 flex-1 flex-col items-center justify-center gap-2 px-6 py-16 text-center"
        aria-label={ui.t("messageListAria")}
        role="alert"
      >
        <h2 className="text-[18px] font-semibold text-foreground">{ui.t("messagesError")}</h2>
        <p className="text-[13px] text-destructive">{ui.messagesError}</p>
        <button
          type="button"
          className="mt-2 inline-flex items-center gap-1.5 rounded-control border border-border px-3 py-1.5 text-[13px] text-foreground transition-colors hover:bg-muted"
          onClick={ui.actions.retryMessages}
        >
          <IconRetry />
          <span>{ui.t("loadMessagesRetry")}</span>
        </button>
      </section>
    );
  }

  if (ui.activeConversationId === null) {
    return (
      <section className="flex min-h-0 flex-1 flex-col" aria-label={ui.t("messageListAria")}>
        <EmptyThread />
      </section>
    );
  }

  return (
    <section className="min-h-0 flex-1" aria-label={ui.t("messageListAria")}>
      <AssistantRuntimeProvider runtime={runtime}>
        <AssistantThread />
      </AssistantRuntimeProvider>
    </section>
  );
}

export const components = {
  "chat.thread": ChatView,
} as const;
