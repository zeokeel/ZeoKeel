/**
 * assistant-ui adapter: maps the existing reducer state / transport commands
 * onto an ExternalStoreRuntime (no second message store).
 */
import { useExternalStoreRuntime } from "@assistant-ui/react";
import type { AppendMessage, AssistantRuntime, ThreadMessageLike } from "@assistant-ui/react";
import { isRecord } from "../protocol";
import type { AppController } from "../state/controller";
import type { ChatMessage } from "../types";

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

type AssistantPart = TextPart | ToolPart | ApprovalPart;

function buildContent(message: ChatMessage): AssistantPart[] {
  const content: AssistantPart[] = [];
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

export function useChatRuntime(controller: AppController): AssistantRuntime {
  const { state } = controller;
  return useExternalStoreRuntime<ChatMessage>({
    messages: state.messages,
    convertMessage,
    isRunning: state.streaming,
    isDisabled: state.offline,
    isSendDisabled: state.offline,
    onNew: async (message) => {
      const text = appendText(message).trim();
      if (text !== "") {
        controller.send(text);
      }
    },
    onCancel: async () => {
      controller.cancel();
    },
    onReload: async () => {
      controller.retrySend();
    },
  });
}
