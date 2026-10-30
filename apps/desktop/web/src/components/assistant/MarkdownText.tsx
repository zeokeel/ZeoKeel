import { MarkdownTextPrimitive } from "@assistant-ui/react-markdown";

/**
 * Markdown rendering through assistant-ui (react-markdown under the hood;
 * raw HTML is not enabled). `smooth` is off so streaming text updates are
 * deterministic and the accent cursor owns the streaming affordance.
 */
export function MarkdownText() {
  return <MarkdownTextPrimitive className="zk-markdown" smooth={false} />;
}
