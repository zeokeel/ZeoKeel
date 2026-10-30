import { AuiIf, ComposerPrimitive, useAuiState } from "@assistant-ui/react";
import { useT } from "../../i18n";
import { IconSend, IconStop } from "../icons";

const sendButton =
  "inline-flex items-center gap-1.5 rounded-control bg-foreground px-3 py-1.5 text-[13px] font-medium text-background transition-colors hover:bg-foreground/90 disabled:pointer-events-none disabled:opacity-40";
const stopButton =
  "inline-flex items-center gap-1.5 rounded-control border border-destructive/40 px-3 py-1.5 text-[13px] font-medium text-destructive transition-colors hover:bg-destructive/10";

export function AssistantComposer(props: { offline: boolean }) {
  const t = useT();
  const text = useAuiState((state) => state.composer.text);
  const count = Array.from(text).length;

  return (
    <div className="mx-auto w-full max-w-[760px]">
      <ComposerPrimitive.Root className="rounded-thread border border-border bg-background px-3 pt-2.5 pb-2 transition-colors focus-within:border-ring focus-within:ring-2 focus-within:ring-ring/20">
        <label className="sr-only" htmlFor="zk-composer-input">
          {t("composerAria")}
        </label>
        <ComposerPrimitive.Input
          id="zk-composer-input"
          aria-label={t("composerAria")}
          placeholder={t("composerPlaceholder")}
          className="zk-composer-input"
          submitMode="enter"
        />
        <div className="flex items-center gap-3 pt-1">
          <span className="min-w-0 flex-1 truncate text-[12px] text-muted-foreground">
            {props.offline ? t("offlineSendBlocked") : t("keyboardHint")}
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
          <AuiIf condition={(state) => !state.thread.isRunning}>
            <ComposerPrimitive.Send aria-label={t("send")} className={sendButton}>
              <IconSend />
              <span>{t("send")}</span>
            </ComposerPrimitive.Send>
          </AuiIf>
        </div>
      </ComposerPrimitive.Root>
    </div>
  );
}
