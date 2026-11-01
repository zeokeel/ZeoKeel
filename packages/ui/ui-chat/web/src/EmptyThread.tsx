import { ThreadPrimitive } from "@assistant-ui/react";
import { BrandLogo, Button, useUi } from "@zeokeel/ui-kit";

const suggestions = [
  { title: "suggestionReadTitle", prompt: "suggestionReadText" },
  { title: "suggestionWriteTitle", prompt: "suggestionWriteText" },
  { title: "suggestionExplainTitle", prompt: "suggestionExplainText" },
];

export function EmptyThread() {
  const { t, mock, activeConversationId, actions } = useUi();
  const hasConversation = activeConversationId !== null;
  return (
    <div className="flex flex-1 flex-col items-center justify-center px-2 py-10 text-center">
      <BrandLogo className="h-9 w-9 rounded-document" />
      <h2 className="mt-5 max-w-[560px] text-[26px] leading-[1.2] font-medium tracking-[-0.02em] text-foreground">
        {t("chatEmptyTitle")}
      </h2>
      {hasConversation ? (
        <>
          <div className="mt-6 grid w-full max-w-[680px] gap-2 sm:grid-cols-3">
            {suggestions.map((suggestion) => (
              <ThreadPrimitive.Suggestion
                key={suggestion.title}
                prompt={t(suggestion.prompt)}
                send
                className="flex flex-col gap-1 rounded-control bg-muted px-3 py-2.5 text-start transition-colors hover:bg-foreground/[0.06]"
              >
                <span className="text-[12px] font-medium text-foreground">
                  {t(suggestion.title)}
                </span>
                <span className="text-[11.5px] leading-[1.5] text-muted-foreground">
                  {t(suggestion.prompt)}
                </span>
              </ThreadPrimitive.Suggestion>
            ))}
          </div>
          {mock ? (
            <p className="mt-3 text-[11px] text-muted-foreground">{t("chatEmptyMockHint")}</p>
          ) : null}
        </>
      ) : (
        <div className="mt-4 flex flex-col items-center gap-2">
          <p className="text-[12.5px] text-muted-foreground">{t("workspaceNoSessions")}</p>
          <Button variant="primary" size="sm" onClick={() => { void actions.createConversation(); }}>
            {t("newConversation")}
          </Button>
        </div>
      )}
    </div>
  );
}
