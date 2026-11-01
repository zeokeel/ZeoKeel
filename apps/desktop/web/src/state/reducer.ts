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
  type QuestionView,
  type Conversation,
  type Message,
  type Meta,
  type ToolCallView,
  type Workspace,
} from "../types";
import { groupToolRows } from "@zeokeel/ui-kit";
import type { UpgradeInfo } from "../upgrade";

export type BootState = "loading" | "ready" | "error";
export type MessagesState = "idle" | "loading" | "error";
export type RetryAction = "boot" | "messages" | "send";

export interface AppState {
  boot: BootState;
  bootError: string | null;
  meta: Meta;
  settings: AppSettings;
  /** Settings revision fence (from `settings.get`; increments on each save). */
  settingsRevision: number;
  settingsOpen: boolean;
  workspaces: Workspace[];
  activeWorkspaceId: string | null;
  workspaceError: string | null;
  conversationsByWorkspace: Record<string, Conversation[]>;
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
  commandsOpen: boolean;
  /** Forced-upgrade gate (HTTP 426 CLIENT_TOO_OLD); highest-priority UI. */
  upgrade: UpgradeInfo | null;
}

export type AppAction =
  | { type: "boot_loading" }
  | {
      type: "boot_loaded";
      meta: Meta;
      workspaces: Workspace[];
      activeWorkspaceId: string | null;
      conversationsByWorkspace: Record<string, Conversation[]>;
      conversations: Conversation[];
      settings: AppSettings;
      settingsRevision: number;
      activeId: string | null;
    }
  | { type: "boot_failed"; message: string }
  | { type: "workspaces_loaded"; workspaces: Workspace[] }
  | { type: "workspace_created"; workspace: Workspace }
  | { type: "workspace_selected"; workspaceId: string }
  | { type: "workspace_error"; message: string | null }
  | { type: "conversations_map_loaded"; conversationsByWorkspace: Record<string, Conversation[]> }
  | {
      type: "conversations_loaded";
      workspaceId: string;
      conversations: Conversation[];
      activeId: string | null;
    }
  | { type: "conversation_activated"; workspaceId: string; conversationId: string }
  | { type: "conversations_refreshed"; workspaceId: string; conversations: Conversation[] }
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
  | { type: "settings_saved"; settings: AppSettings; revision: number }
  | { type: "settings_loaded"; settings: AppSettings; revision: number }
  | { type: "toast"; message: string | null }
  | { type: "sidebar_toggled"; open: boolean }
  | { type: "commands_open"; open: boolean }
  | { type: "upgrade_required"; info: UpgradeInfo }
  | { type: "approval_decided"; approvalId: string; decision: ApprovalView["decision"] }
  | { type: "question_answered"; questionId: string; answers: unknown };

export function createInitialState(): AppState {
  return {
    boot: "loading",
    bootError: null,
    meta: { edition: "personal", version: "0.0.0", packages: [], agentConfigError: null },
    settings: defaultSettings(),
    settingsRevision: 0,
    settingsOpen: false,
    workspaces: [],
    activeWorkspaceId: null,
    workspaceError: null,
    conversationsByWorkspace: {},
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
    commandsOpen: false,
    upgrade: null,
  };
}

function statusOfMessage(message: Message): ChatMessage["status"] {
  if (message.status === "cancelled" || message.status === "error" || message.status === "streaming") {
    return message.status;
  }
  return "done";
}

function toChatMessage(message: Message, toolCalls: ToolCallView[] = []): ChatMessage {
  return {
    id: message.id,
    role: message.role === "user" ? "user" : "assistant",
    content: message.content,
    created_at_ms: message.created_at_ms,
    status: statusOfMessage(message),
    usage: message.usage ?? null,
    toolCalls,
    approvals: [],
    questions: [],
    error: null,
    turnId: message.turn_id ?? "",
  };
}

function storeMessagesToChat(messages: Message[]): ChatMessage[] {
  const grouped = groupToolRows(messages);
  const chat = grouped.visible.map((message) =>
    toChatMessage(message, grouped.toolsByAssistant.get(message.id) ?? []),
  );
  if (grouped.unassigned.length > 0) {
    chat.push({
      id: "unassigned-tools",
      role: "assistant",
      content: "",
      created_at_ms: messages[messages.length - 1]?.created_at_ms ?? 0,
      status: "done",
      toolCalls: grouped.unassigned,
      approvals: [],
      questions: [],
      error: null,
      degraded: true,
    });
  }
  return chat;
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

function bumpConversationMap(
  map: Record<string, Conversation[]>,
  conversationId: string,
  now: number,
): Record<string, Conversation[]> {
  const next: Record<string, Conversation[]> = {};
  for (const [workspaceId, list] of Object.entries(map)) {
    next[workspaceId] = list.some((conversation) => conversation.id === conversationId)
      ? bumpConversation(list, conversationId, now)
      : list;
  }
  return next;
}

function streamingIndex(messages: ChatMessage[]): number {
  for (let index = messages.length - 1; index >= 0; index -= 1) {
    const message = messages[index];
    if (message?.role === "assistant" && message.status === "streaming") {
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

function applyToolFinished(
  message: ChatMessage,
  name: string,
  result: string,
  args: string,
  status: ToolCallView["status"],
): ChatMessage {
  const runningIndex = message.toolCalls.findIndex((tool) => tool.name === name && tool.status === "running");
  if (runningIndex >= 0) {
    const toolCalls = message.toolCalls.map((tool, index) =>
      index === runningIndex ? { ...tool, result, args: args === "" ? tool.args : args, status } : tool,
    );
    return { ...message, toolCalls };
  }
  const fallback: ToolCallView = {
    id: `${message.id}-tool-${String(message.toolCalls.length)}`,
    name,
    args,
    result,
    status,
  };
  return { ...message, toolCalls: [...message.toolCalls, fallback] };
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
              id: `${message.id}-tool-${String(index)}`,
              name: tool.name,
              args: tool.args,
              result: tool.result,
              status: "success" as const,
            })),
      status: "done",
      error: null,
      usage: event.reply.usage,
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
        id: `${message.id}-tool-${String(message.toolCalls.length)}`,
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
      applyToolFinished(message, event.name, event.result, event.args ?? "", event.status ?? "success"),
    );
  }
  if (event.type === "usage") {
    return updateStreamingMessage(state, (message) => ({ ...message, usage: event.usage }));
  }
  if (event.type === "question_asked") {
    const question: QuestionView = { id: event.question_id, questions: event.questions, answers: null };
    return updateStreamingMessage(state, (message) => ({ ...message, questions: [...message.questions, question] }));
  }
  if (event.type === "question_answered") {
    return updateStreamingMessage(state, (message) => ({
      ...message,
      questions: message.questions.map((question) =>
        question.id === event.question_id ? { ...question, answers: event.answers } : question,
      ),
    }));
  }
  if (event.type === "approval_resolved") {
    return updateStreamingMessage(state, (message) => ({
      ...message,
      approvals: message.approvals.map((approval) =>
        approval.id === event.approval_id
          ? { ...approval, decision: event.decision, timed_out: event.timed_out === true }
          : approval,
      ),
    }));
  }
  // Exhaustive switch above leaves only `approval_requested` here.
  const approval: ApprovalView = {
    id: event.approval_id,
    tool: event.tool,
    args: event.args,
    reason: event.reason,
    auto: event.auto === true,
    deadline_ms: event.deadline_ms ?? 0,
    metadata: event.metadata ?? null,
    decision: "pending",
  };
  return updateStreamingMessage(state, (message) => ({
    ...message,
    approvals: [...message.approvals, approval],
  }));
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
        workspaces: action.workspaces,
        activeWorkspaceId: action.activeWorkspaceId,
        workspaceError: null,
        conversationsByWorkspace: action.conversationsByWorkspace,
        conversations: action.conversations,
        settings: action.settings,
        settingsRevision: action.settingsRevision,
        activeConversationId: activeId,
        messages: [],
        messagesState: activeId === null ? "idle" : "loading",
        messagesError: null,
        retryAction: null,
        sendError: null,
        streaming: false,
        streamId: null,
      };
    }
    case "boot_failed":
      return {
        ...state,
        boot: "error",
        bootError: action.message,
        retryAction: "boot",
      };
    case "workspaces_loaded":
      return { ...state, workspaces: action.workspaces, workspaceError: null };
    case "workspace_created":
      return {
        ...state,
        workspaces: [...state.workspaces, action.workspace].sort((left, right) => {
          if (left.is_default !== right.is_default) {
            return left.is_default ? -1 : 1;
          }
          return left.created_at_ms - right.created_at_ms;
        }),
        workspaceError: null,
        sidebarOpen: false,
      };
    case "workspace_selected": {
      const conversations = state.conversationsByWorkspace[action.workspaceId] ?? [];
      const first = conversations[0];
      return {
        ...state,
        activeWorkspaceId: action.workspaceId,
        workspaceError: null,
        conversations,
        activeConversationId: first === undefined ? null : first.id,
        messages: [],
        messagesState: first === undefined ? "idle" : "loading",
        messagesError: null,
        sendError: null,
        retryAction: null,
        streaming: false,
        streamId: null,
        sidebarOpen: false,
      };
    }
    case "workspace_error":
      return { ...state, workspaceError: action.message };
    case "conversations_refreshed": {
      const map = {
        ...state.conversationsByWorkspace,
        [action.workspaceId]: action.conversations,
      };
      const conversations =
        action.workspaceId === state.activeWorkspaceId
          ? action.conversations
          : state.conversations;
      const activeStillExists = conversations.some(
        (conversation) => conversation.id === state.activeConversationId,
      );
      return {
        ...state,
        conversationsByWorkspace: map,
        conversations,
        activeConversationId: activeStillExists ? state.activeConversationId : null,
      };
    }
    case "conversations_map_loaded":
      return {
        ...state,
        conversationsByWorkspace: action.conversationsByWorkspace,
        conversations:
          state.activeWorkspaceId === null
            ? state.conversations
            : (action.conversationsByWorkspace[state.activeWorkspaceId] ?? []),
      };
    case "conversations_loaded": {
      const conversationsByWorkspace = {
        ...state.conversationsByWorkspace,
        [action.workspaceId]: action.conversations,
      };
      if (action.workspaceId !== state.activeWorkspaceId) {
        return { ...state, conversationsByWorkspace };
      }
      const activeId =
        action.activeId !== null &&
        action.conversations.some((conversation) => conversation.id === action.activeId)
          ? action.activeId
          : (action.conversations[0]?.id ?? null);
      const keepLoaded = activeId !== null && activeId === state.activeConversationId;
      return {
        ...state,
        conversationsByWorkspace,
        conversations: action.conversations,
        activeConversationId: activeId,
        messages: keepLoaded ? state.messages : [],
        messagesState: keepLoaded ? state.messagesState : activeId === null ? "idle" : "loading",
        messagesError: keepLoaded ? state.messagesError : null,
        sendError: keepLoaded ? state.sendError : null,
        retryAction: keepLoaded ? state.retryAction : null,
      };
    }
    case "conversation_activated": {
      const conversations = state.conversationsByWorkspace[action.workspaceId] ?? [];
      return {
        ...state,
        activeWorkspaceId: action.workspaceId,
        workspaceError: null,
        conversations,
        activeConversationId: action.conversationId,
        messages: [],
        messagesState: "loading",
        messagesError: null,
        sendError: null,
        retryAction: null,
        streaming: false,
        streamId: null,
        sidebarOpen: false,
      };
    }
    case "conversation_created": {
      const workspaceId = action.conversation.workspace_id !== "" ? action.conversation.workspace_id : (state.activeWorkspaceId ?? "");
      const existing = state.conversationsByWorkspace[workspaceId] ?? [];
      const conversationsByWorkspace = {
        ...state.conversationsByWorkspace,
        [workspaceId]: [
          action.conversation,
          ...existing.filter((item) => item.id !== action.conversation.id),
        ],
      };
      return {
        ...state,
        conversationsByWorkspace,
        conversations:
          workspaceId === state.activeWorkspaceId
            ? (conversationsByWorkspace[workspaceId] ?? [])
            : state.conversations,
        activeConversationId: action.conversation.id,
        messages: [],
        messagesState: "idle",
        messagesError: null,
        sendError: null,
        retryAction: null,
        sidebarOpen: false,
      };
    }
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
          ? [...storeMessagesToChat(action.messages), ...state.messages]
          : storeMessagesToChat(action.messages),
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
        questions: [],
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
        questions: [],
        error: null,
      };
      return {
        ...state,
        messages: [...state.messages, user, assistant],
        conversations: bumpConversation(state.conversations, action.conversationId, action.now),
        conversationsByWorkspace: bumpConversationMap(
          state.conversationsByWorkspace,
          action.conversationId,
          action.now,
        ),
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
        if (message?.role === "assistant") {
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
              questions: [],
              error: null,
              usage: null,
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
        conversationsByWorkspace:
          state.activeConversationId === null
            ? state.conversationsByWorkspace
            : bumpConversationMap(
                state.conversationsByWorkspace,
                state.activeConversationId,
                action.now,
              ),
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
      return {
        ...state,
        settings: action.settings,
        settingsRevision: action.revision,
        settingsOpen: false,
      };
    case "settings_loaded":
      return { ...state, settings: action.settings, settingsRevision: action.revision };
    case "toast":
      return { ...state, toast: action.message };
    case "sidebar_toggled":
      return { ...state, sidebarOpen: action.open };
    case "commands_open":
      return { ...state, commandsOpen: action.open };
    case "upgrade_required": {
      const previous = state.upgrade;
      // Multiple server surfaces can 426 (middleware envelope + legacy flat bootstrap body);
      // merge so known versions/portal from any response win over a later bare 426.
      const info = previous === null
        ? action.info
        : {
            current: action.info.current ?? previous.current,
            minimum: action.info.minimum ?? previous.minimum,
            portalUrl: action.info.portalUrl ?? previous.portalUrl,
            requestId: action.info.requestId ?? previous.requestId ?? null,
          };
      return { ...state, upgrade: info };
    }
    case "question_answered": {
      const messages = state.messages.map((message) => {
        const questions = message.questions.map((question) =>
          question.id === action.questionId ? { ...question, answers: action.answers } : question,
        );
        return questions === message.questions ? message : { ...message, questions };
      });
      const changed = messages.some((message, index) => message !== state.messages[index]);
      return changed ? { ...state, messages } : state;
    }
    case "approval_decided": {
      const messages = state.messages.map((message) => {
        const approvals = message.approvals.map((approval) =>
          approval.id === action.approvalId ? { ...approval, decision: action.decision } : approval,
        );
        return approvals === message.approvals ? message : { ...message, approvals };
      });
      const changed = messages.some((message, index) => message !== state.messages[index]);
      return changed ? { ...state, messages } : state;
    }
    default:
      return state;
  }
}
