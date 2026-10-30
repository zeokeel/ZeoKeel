import { MarkdownTextPrimitive } from "@assistant-ui/react-markdown";

/** Markdown through assistant-ui (react-markdown; raw HTML disabled). */
export function MarkdownText() {
  return <MarkdownTextPrimitive className="zk-markdown" smooth={false} />;
}
