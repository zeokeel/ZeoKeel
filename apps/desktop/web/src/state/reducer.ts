/**
 * Pure chat/session reducer. All I/O lives in the transport + controller; the
 * reducer only transforms state so it can be replayed/golden-tested.
 */
import {
  defaultSettings,
  type AgentEvent,
  type AppSettings,
  type ApprovalView,
  type ChatMessage,
  type Conversation,
  type Message,
  type Meta,
  type ToolCallView,
} from "../types";

export type BootState = "loading" | "ready" | "error";
export type MessagesState = "idle" | "loading" | "error";
export type RetryAction = "boot" | "messages" | "send";

export interface AppState {
  boot: BootState;
  bootError: string | null;
  meta: Meta;
  settings: AppSettings;
  settingsOpen: boolean;
  conversations: Conversation[];
  activeConversationId: string | null;
  messages: ChatMessage[];
  messagesState: MessagesState;
  messagesError: string | null;
  streaming: boolean;
  streamId: string | null;
  lastSendText: string | null;
  sendError: string | null;
  retryAction: RetryAction | null;
  offline: boolean;
  toast: string | null;
  sidebarOpen: boolean;
}

export type AppAction =
  | { type: "boot_loading" }
  | {
      type: "boot_loaded";
      meta: Meta;
      conversations: Conversation[];
      settings: AppSettings;
      activeId: string | null;
    }
  | { type: "boot_failed"; message: string }
  | { type: "conversations_loaded"; conversations: Conversation[] }
  | { type: "conversation_created"; conversation: Conversation }
  | { type: "conversation_selected"; conversationId: string }
  | { type: "messages_loading"; conversationId: string }
  | { type: "messages_loaded"; conversationId: string; messages: Message[] }
  | { type: "messages_failed"; conversationId: string; message: string }
  | {
      type: "send_started";
      conversationId: string;
      text: string;
      messageId: string;
      assistantId: string;
      now: number;
    }
  | { type: "send_accepted"; streamId: string }
  | { type: "send_failed"; message: string }
  | { type: "retry_started"; now: number }
  | { type: "agent_event"; event: AgentEvent; now: number }
  | { type: "offline_changed"; offline: boolean }
  | { type: "settings_open"; open: boolean }
  | { type: "settings_saved"; settings: AppSettings }
  | { type: "toast"; message: string | null }
  | { type: "sidebar_toggled"; open: boolean }
  | { type: "approval_decided"; approvalId: string; decision: "approved" | "rejected" };

export function createInitialState(): AppState {
  return {
    boot: "loading",
    bootError: null,
    meta: { edition: "personal", version: "0.0.0", packages: [] },
    settings: defaultSettings(),
    settingsOpen: false,
    conversations: [],
    activeConversationId: null,
    messages: [],
    messagesState: "idle",
    messagesError: null,
    streaming: false,
    streamId: null,
    lastSendText: null,
    sendError: null,
    retryAction: null,
    offline: false,
    toast: null,
    sidebarOpen: false,
  };
}

function toChatMessage(message: Message): ChatMessage {
  return {
    id: message.id,
    role: message.role === "user" ? "user" : "assistant",
    content: message.content,
    created_at_ms: message.created_at_ms,
    status: "done",
    toolCalls: [],
    approvals: [],
    error: null,
  };
}

function bumpConversation(
  conversations: Conversation[],
  conversationId: string,
  now: number,
): Conversation[] {
  const updated = conversations.map((conversation) =>
    conversation.id === conversationId
      ? { ...conversation, updated_at_ms: now }
      : conversation,
  );
  const current = updated.find((conversation) => conversation.id === conversationId);
  if (current === undefined) {
    return updated;
  }
  return [current, ...updated.filter((conversation) => conversation.id !== conversationId)];
}

function streamingIndex(messages: ChatMessage[]): number {
  for (let index = messages.length - 1; index >= 0; index -= 1) {
    const message = messages[index];
    if (message !== undefined && message.role === "assistant" && message.status === "streaming") {
      return index;
    }
  }
  return -1;
}

function updateStreamingMessage(
  state: AppState,
  update: (message: ChatMessage) => ChatMessage,
): AppState {
  const index = streamingIndex(state.messages);
  if (index < 0) {
    return state;
  }
  const messages = state.messages.map((message, position) =>
    position === index ? update(message) : message,
  );
  return { ...state, messages };
}

function applyToolFinished(message: ChatMessage, name: string, result: string): ChatMessage {
  let updated = false;
  const toolCalls = message.toolCalls.map((tool) => {
    if (!updated && tool.name === name && tool.status === "running") {
      updated = true;
      return { ...tool, result, status: "done" as const };
    }
    return tool;
  });
  if (updated) {
    return { ...message, toolCalls };
  }
  const fallback: ToolCallView = {
    id: `${message.id}-tool-${message.toolCalls.length}`,
    name,
    args: "",
    result,
    status: "done",
  };
  return { ...message, toolCalls: [...toolCalls, fallback] };
}

function applyAgentEvent(state: AppState, event: AgentEvent): AppState {
  if (event.type === "turn_started") {
    return state;
  }
  if (event.type === "turn_completed") {
    return updateStreamingMessage(state, (message) => ({
      ...message,
      content: event.reply.text === "" ? message.content : event.reply.text,
      toolCalls:
        event.reply.tool_calls.length === 0
          ? message.toolCalls
          : event.reply.tool_calls.map((tool, index) => ({
              id: `${message.id}-tool-${index}`,
              name: tool.name,
              args: tool.args,
              result: tool.result,
              status: "done" as const,
            })),
      status: "done",
      error: null,
    }));
  }
  if (event.type === "turn_cancelled") {
    return updateStreamingMessage(state, (message) => ({
      ...message,
      status: "cancelled",
    }));
  }
  if (event.type === "error") {
    return updateStreamingMessage(state, (message) => ({
      ...message,
      status: "error",
      error: event.message,
    }));
  }
  if (event.type === "text_delta") {
    return updateStreamingMessage(state, (message) => ({
      ...message,
      content: message.content + event.delta,
    }));
  }
  if (event.type === "tool_call_started") {
    return updateStreamingMessage(state, (message) => {
      const tool: ToolCallView = {
        id: `${message.id}-tool-${message.toolCalls.length}`,
        name: event.name,
        args: event.args,
        result: null,
        status: "running",
      };
      return { ...message, toolCalls: [...message.toolCalls, tool] };
    });
  }
  if (event.type === "tool_call_finished") {
    return updateStreamingMessage(state, (message) =>
      applyToolFinished(message, event.name, event.result),
    );
  }
  const approval: ApprovalView = {
    id: event.approval_id,
    tool: event.tool,
    args: event.args,
    reason: event.reason,
    decision: "pending",
  };
  const updated = updateStreamingMessage(state, (message) => ({
    ...message,
    approvals: [...message.approvals, approval],
  }));
  if (updated !== state) {
    return updated;
  }
  return state;
}

function finishStream(state: AppState, message: string | null): AppState {
  const base = updateStreamingMessage(state, (assistant) => ({
    ...assistant,
    status: message === null ? "done" : "error",
    error: message,
  }));
  return {
    ...base,
    streaming: false,
    streamId: null,
    sendError: message,
    retryAction: message === null ? null : "send",
  };
}

export function appReducer(state: AppState, action: AppAction): AppState {
  switch (action.type) {
    case "boot_loading":
      return { ...state, boot: "loading", bootError: null, retryAction: null };
    case "boot_loaded": {
      const activeId = action.activeId;
      return {
        ...state,
        boot: "ready",
        bootError: null,
        meta: action.meta,
        conversations: action.conversations,
        settings: action.settings,
        activeConversationId: activeId,
        messages: [],
        messagesState: activeId === null ? "idle" : "loading",
        messagesError: null,
        retryAction: null,
        sendError: null,
      };
    }
    case "boot_failed":
      return {
        ...state,
        boot: "error",
        bootError: action.message,
        retryAction: "boot",
      };
    case "conversations_loaded":
      return { ...state, conversations: action.conversations };
    case "conversation_created":
      return {
        ...state,
        conversations: [
          action.conversation,
          ...state.conversations.filter((item) => item.id !== action.conversation.id),
        ],
        activeConversationId: action.conversation.id,
        messages: [],
        messagesState: "idle",
        messagesError: null,
        sendError: null,
        retryAction: null,
        sidebarOpen: false,
      };
    case "conversation_selected": {
      if (action.conversationId === state.activeConversationId && state.messagesState !== "error") {
        return { ...state, sidebarOpen: false };
      }
      return {
        ...state,
        activeConversationId: action.conversationId,
        messages: [],
        messagesState: "loading",
        messagesError: null,
        sendError: null,
        retryAction: null,
        sidebarOpen: false,
      };
    }
    case "messages_loading":
      return {
        ...state,
        messagesState: "loading",
        messagesError: null,
      };
    case "messages_loaded":
      if (action.conversationId !== state.activeConversationId) {
        return state;
      }
      return {
        ...state,
        messages: state.streaming
          ? [...action.messages.map(toChatMessage), ...state.messages]
          : action.messages.map(toChatMessage),
        messagesState: "idle",
        messagesError: null,
        retryAction: null,
      };
    case "messages_failed":
      if (action.conversationId !== state.activeConversationId || state.streaming) {
        return state;
      }
      return {
        ...state,
        messagesState: "error",
        messagesError: action.message,
        retryAction: "messages",
      };
    case "send_started": {
      const user: ChatMessage = {
        id: action.messageId,
        role: "user",
        content: action.text,
        created_at_ms: action.now,
        status: "done",
        toolCalls: [],
        approvals: [],
        error: null,
      };
      const assistant: ChatMessage = {
        id: action.assistantId,
        role: "assistant",
        content: "",
        created_at_ms: action.now,
        status: "streaming",
        toolCalls: [],
        approvals: [],
        error: null,
      };
      return {
        ...state,
        messages: [...state.messages, user, assistant],
        conversations: bumpConversation(state.conversations, action.conversationId, action.now),
        streaming: true,
        streamId: null,
        lastSendText: action.text,
        sendError: null,
        retryAction: null,
      };
    }
    case "send_accepted":
      return { ...state, streamId: action.streamId };
    case "send_failed":
      return finishStream(state, action.message);
    case "retry_started": {
      let index = -1;
      for (let position = state.messages.length - 1; position >= 0; position -= 1) {
        const message = state.messages[position];
        if (message !== undefined && message.role === "assistant") {
          index = position;
          break;
        }
      }
      if (index < 0) {
        return state;
      }
      const messages = state.messages.map((message, position) =>
        position === index
          ? {
              ...message,
              content: "",
              status: "streaming" as const,
              toolCalls: [],
              approvals: [],
              error: null,
            }
          : message,
      );
      return {
        ...state,
        messages,
        streaming: true,
        streamId: null,
        sendError: null,
        retryAction: null,
        conversations:
          state.activeConversationId === null
            ? state.conversations
            : bumpConversation(state.conversations, state.activeConversationId, action.now),
      };
    }
    case "agent_event": {
      const updated = applyAgentEvent(state, action.event);
      if (updated === state) {
        return state;
      }
      if (
        action.event.type === "turn_completed" ||
        action.event.type === "turn_cancelled" ||
        action.event.type === "error"
      ) {
        return {
          ...updated,
          streaming: false,
          streamId: null,
          retryAction: action.event.type === "error" ? "send" : null,
          sendError: action.event.type === "error" ? action.event.message : null,
        };
      }
      return updated;
    }
    case "offline_changed":
      return { ...state, offline: action.offline };
    case "settings_open":
      return { ...state, settingsOpen: action.open };
    case "settings_saved":
      return { ...state, settings: action.settings, settingsOpen: false };
    case "toast":
      return { ...state, toast: action.message };
    case "sidebar_toggled":
      return { ...state, sidebarOpen: action.open };
    case "approval_decided": {
      let changed = false;
      const messages = state.messages.map((message) => {
        const approvals = message.approvals.map((approval) => {
          if (approval.id !== action.approvalId) {
            return approval;
          }
          changed = true;
          return { ...approval, decision: action.decision };
        });
        return changed && approvals !== message.approvals
          ? { ...message, approvals }
          : message;
      });
      return changed ? { ...state, messages } : state;
    }
    default:
      return state;
  }
}
