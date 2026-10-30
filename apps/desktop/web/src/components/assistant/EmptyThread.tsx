import { ThreadPrimitive } from "@assistant-ui/react";
import { useT, type MessageKey } from "../../i18n";
import { BrandLogo } from "../BrandLogo";

interface Suggestion {
  title: MessageKey;
  prompt: MessageKey;
}

const SUGGESTIONS: Suggestion[] = [
  { title: "suggestionReadTitle", prompt: "suggestionReadText" },
  { title: "suggestionWriteTitle", prompt: "suggestionWriteText" },
  { title: "suggestionExplainTitle", prompt: "suggestionExplainText" },
];

export function EmptyThread(props: { mock: boolean }) {
  const t = useT();
  return (
    <div className="flex flex-1 flex-col items-center justify-center px-2 py-14 text-center">
      <BrandLogo className="h-10 w-10 rounded-document" />
      <h2 className="mt-6 max-w-[560px] text-[32px] leading-[1.2] font-medium tracking-[-0.02em] text-foreground">
        {t("chatEmptyTitle")}
      </h2>
      <div className="mt-8 grid w-full max-w-[680px] gap-2 sm:grid-cols-3">
        {SUGGESTIONS.map((suggestion) => (
          <ThreadPrimitive.Suggestion
            key={suggestion.title}
            prompt={t(suggestion.prompt)}
            send
            className="flex flex-col gap-1 rounded-control bg-muted px-4 py-3 text-start transition-colors hover:bg-foreground/[0.06]"
          >
            <span className="text-[13px] font-medium text-foreground">{t(suggestion.title)}</span>
            <span className="text-[12px] leading-[1.5] text-muted-foreground">
              {t(suggestion.prompt)}
            </span>
          </ThreadPrimitive.Suggestion>
        ))}
      </div>
      {props.mock ? (
        <p className="mt-4 text-[12px] text-muted-foreground">{t("chatEmptyMockHint")}</p>
      ) : null}
    </div>
  );
}
