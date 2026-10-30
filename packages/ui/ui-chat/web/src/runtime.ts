/**
 * assistant-ui adapter for the app's existing message state and actions.
 * The app reducer/transport remains the only message store.
 */
import { useExternalStoreRuntime } from "@assistant-ui/react";
import type { AppendMessage, AssistantRuntime, ThreadMessageLike } from "@assistant-ui/react";
import { isRecord, type ChatMessage, type UiValue } from "@zeokeel/ui-kit";

type TextPart = { type: "text"; text: string };

type ToolPart = {
  type: "tool-call";
  toolCallId: string;
  toolName: string;
  argsText: string;
  result?: string;
  isError?: boolean;
};

type ApprovalData = {
  id: string;
  tool: string;
  args: string;
  reason: string;
  decision: "pending" | "approved" | "rejected";
};

type ApprovalPart = { type: "data-approval"; id?: string; data: ApprovalData };
type QuestionData = { id: string; questions: unknown; answers: unknown };
type QuestionPart = { type: "data-question"; id?: string; data: QuestionData };

type ContentPart = TextPart | ToolPart | ApprovalPart | QuestionPart;

function buildContent(message: ChatMessage): ContentPart[] {
  const content: ContentPart[] = [];
  if (message.content.trim() !== "") {
    content.push({ type: "text", text: message.content });
  }
  for (const tool of message.toolCalls) {
    const part: ToolPart = {
      type: "tool-call",
      toolCallId: tool.id,
      toolName: tool.name,
      argsText: tool.args,
      isError: tool.status === "error",
    };
    if (tool.result !== null) {
      part.result = tool.result;
    }
    content.push(part);
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
        decision: approval.decision,
      },
    });
  }
  for (const question of message.questions) {
    content.push({ type: "data-question", id: question.id, data: { id: question.id, questions: question.questions, answers: question.answers } });
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

function convertMessage(message: ChatMessage): ThreadMessageLike {
  const base: ThreadMessageLike = {
    id: message.id,
    role: message.role,
    createdAt: new Date(message.created_at_ms),
    content: buildContent(message),
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
    convertMessage,
    isRunning: ui.streaming,
    isDisabled: ui.offline,
    isSendDisabled: ui.offline,
    onNew: async (message) => {
      const text = appendText(message).trim();
      if (text !== "") {
        ui.actions.send(text);
      }
    },
    onCancel: async () => {
      ui.actions.cancel();
    },
    onReload: async () => {
      ui.actions.retry();
    },
  });
}
