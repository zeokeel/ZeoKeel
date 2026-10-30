import { AssistantRuntimeProvider } from "@assistant-ui/react";
import { useChatRuntime } from "../assistant/runtime";
import { useT } from "../i18n";
import type { AppController } from "../state/controller";
import { AssistantThread } from "./assistant/AssistantThread";
import { IconRetry } from "./icons";

export function ChatView(props: { controller: AppController; mock: boolean }) {
  const t = useT();
  const { state } = props.controller;
  const runtime = useChatRuntime(props.controller);

  if (state.messagesState === "loading") {
    return (
      <section
        className="flex min-h-0 flex-1 flex-col items-center justify-center gap-4 px-6 py-16"
        aria-label={t("messageListAria")}
        aria-busy="true"
      >
        <span className="zk-spinner" aria-hidden="true" />
        <p className="text-[13px] text-muted-foreground">{t("messagesLoading")}</p>
        <div className="mt-2 flex w-full max-w-[420px] flex-col gap-2" aria-hidden="true">
          <span className="zk-skeleton w-full" />
          <span className="zk-skeleton w-2/3" />
          <span className="zk-skeleton w-full" />
        </div>
      </section>
    );
  }

  if (state.messagesState === "error") {
    return (
      <section
        className="flex min-h-0 flex-1 flex-col items-center justify-center gap-2 px-6 py-16 text-center"
        aria-label={t("messageListAria")}
        role="alert"
      >
        <h2 className="text-[18px] font-semibold text-foreground">{t("messagesError")}</h2>
        <p className="text-[13px] text-destructive">{state.messagesError}</p>
        <button
          type="button"
          className="mt-2 inline-flex items-center gap-1.5 rounded-control border border-border px-3 py-1.5 text-[13px] text-foreground transition-colors hover:bg-muted"
          onClick={props.controller.retryMessages}
        >
          <IconRetry />
          <span>{t("loadMessagesRetry")}</span>
        </button>
      </section>
    );
  }

  return (
    <section className="min-h-0 flex-1" aria-label={t("messageListAria")}>
      <AssistantRuntimeProvider runtime={runtime}>
        <AssistantThread
          mock={props.mock}
          offline={state.offline}
          onDecide={props.controller.decideApproval}
        />
      </AssistantRuntimeProvider>
    </section>
  );
}
