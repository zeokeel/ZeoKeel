/**
 * assistant-ui adapter for the app's existing message state and actions.
 * The app reducer/transport remains the only message store.
 */
import { useExternalStoreRuntime } from "@assistant-ui/react";
import type { AppendMessage, AssistantRuntime, ThreadMessageLike } from "@assistant-ui/react";
import { isRecord, type ChatMessage, type UiValue } from "@zeokeel/ui-kit";

interface TextPart { type: "text"; text: string }

interface ToolData {
  id: string;
  name: string;
  args: string;
  result: string;
  status: ChatMessage["toolCalls"][number]["status"];
  unparsed?: boolean;
}

interface ApprovalData {
  id: string;
  tool: string;
  args: string;
  reason: string;
  auto: boolean;
  deadline_ms: number;
  timed_out: boolean;
  decision: string;
  metadata: NonNullable<ChatMessage["approvals"][number]["metadata"]> | null;
}

interface ApprovalPart { type: "data-approval"; id?: string; data: ApprovalData }
interface QuestionData { id: string; questions: unknown; answers: unknown }
interface QuestionPart { type: "data-question"; id?: string; data: QuestionData }
interface UsagePart { type: "data-usage"; data: { usage: NonNullable<ChatMessage["usage"]> } }

interface ToolPart { type: "data-tool"; id?: string; data: ToolData }
type ContentPart = TextPart | ToolPart | ApprovalPart | QuestionPart | UsagePart;

function buildContent(message: ChatMessage, t: (key: string) => string): ContentPart[] {
  const content: ContentPart[] = [];
  if (message.degraded === true) {
    content.push({ type: "text", text: t("toolUnassigned") });
  }
  if (message.content.trim() !== "") {
    content.push({ type: "text", text: message.content });
  }
  for (const tool of message.toolCalls) {
    const data: ToolData = {
      id: tool.id,
      name: tool.name,
      args: tool.args,
      result: tool.result ?? "",
      status: tool.status,
    };
    if (tool.unparsed === true) {
      data.unparsed = true;
    }
    content.push({ type: "data-tool", id: tool.id, data });
  }
  for (const approval of message.approvals) {
    content.push({
      type: "data-approval",
      id: approval.id,
      data: {
        id: approval.id,
        tool: approval.tool,
        args: approval.args,
        reason: approval.reason,
        auto: approval.auto === true,
        deadline_ms: approval.deadline_ms ?? 0,
        timed_out: approval.timed_out === true,
        decision: approval.decision,
        metadata: approval.metadata ?? null,
      },
    });
  }
  for (const question of message.questions) {
    content.push({ type: "data-question", id: question.id, data: { id: question.id, questions: question.questions, answers: question.answers } });
  }
  if (message.usage !== undefined && message.usage !== null) {
    content.push({ type: "data-usage", data: { usage: message.usage } });
  }
  return content;
}

function statusOf(message: ChatMessage): ThreadMessageLike["status"] {
  switch (message.status) {
    case "streaming":
      return { type: "running" };
    case "cancelled":
      return { type: "incomplete", reason: "cancelled" };
    case "error":
      return { type: "incomplete", reason: "error", error: message.error ?? "error" };
    case "done":
      return { type: "complete", reason: "stop" };
    default:
      return { type: "complete", reason: "unknown" };
  }
}

function convertMessage(message: ChatMessage, t: (key: string) => string): ThreadMessageLike {
  const base: ThreadMessageLike = {
    id: message.id,
    role: message.role,
    createdAt: new Date(message.created_at_ms),
    content: buildContent(message, t),
  };
  if (message.role !== "assistant") {
    return base;
  }
  return { ...base, status: statusOf(message) };
}

function appendText(message: AppendMessage): string {
  const content: unknown = message.content;
  if (typeof content === "string") {
    return content;
  }
  if (!Array.isArray(content)) {
    return "";
  }
  const pieces: string[] = [];
  for (const part of content) {
    if (isRecord(part) && part["type"] === "text" && typeof part["text"] === "string") {
      pieces.push(part["text"]);
    }
  }
  return pieces.join("\n");
}

export function useChatRuntime(ui: UiValue): AssistantRuntime {
  return useExternalStoreRuntime<ChatMessage>({
    messages: ui.messages,
    convertMessage: (message) => convertMessage(message, ui.t),
    isRunning: ui.streaming,
    isDisabled: ui.offline,
    isSendDisabled: ui.offline,
    onNew: (message) => {
      const text = appendText(message).trim();
      if (text !== "") {
        ui.actions.send(text);
      }
      return Promise.resolve();
    },
    onCancel: () => {
      ui.actions.cancel();
      return Promise.resolve();
    },
    onReload: () => {
      ui.actions.retry();
      return Promise.resolve();
    },
  });
}
