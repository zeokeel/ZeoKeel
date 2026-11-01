/**
 * In-memory demo backend (`?mock=1`): local/screenshot/component-test mode.
 *
 * Mirrors the workspace-scoped IPC (docs/26):
 * - workspace.list/create/rename/set_default/remove,
 * - session.create/list/messages all require a workspace_id,
 * - chat.send requires both workspace_id and conversation_id (no auto-create),
 * - 300ms streaming chunks; tool events when the input mentions “读取”.
 *
 * Extra demo-only query flags (handled by transport/index.ts):
 * - `no-workspace=1`: start with zero workspaces (first-run screen),
 * - `seed=1`: add sample workspaces/sessions for screenshots.
 */
import { isRecord, parseSettings } from "../protocol";
import {
  defaultSettings,
  type AgentEvent,
  type AppSettings,
  type Conversation,
  type Message,
  type Transport,
  type UiEvent,
  type Workspace,
} from "../types";

interface MockOptions {
  chunkDelayMs?: number;
  startEmptyWorkspaces?: boolean;
  seed?: boolean;
}

interface StreamState {
  streamId: string;
  conversationId: string;
  assistantId: string;
  steps: AgentEvent[];
  index: number;
  timer: number;
  text: string;
}

const MOCK_PACKAGES = ["tool.fs", "llm.mock", "agent-loop"];
const MOCK_VERSION = "0.1.0";

let idCounter = 0;

function nextId(prefix: string): string {
  idCounter += 1;
  return `${prefix}-${String(idCounter)}`;
}

function splitChunks(text: string, size: number): string[] {
  const characters = Array.from(text);
  const chunks: string[] = [];
  for (let index = 0; index < characters.length; index += size) {
    chunks.push(characters.slice(index, index + size).join(""));
  }
  return chunks;
}

function planTurn(input: string): AgentEvent[] {
  const steps: AgentEvent[] = [];
  let finalText: string;
  if (input.includes("读取")) {
    const args = JSON.stringify({ path: "demo-data/hello.txt" });
    const result = "Hello from ZeoKeel demo data.\n";
    steps.push({ type: "tool_call_started", name: "fs.read", args });
    steps.push({ type: "tool_call_finished", name: "fs.read", result });
    finalText = `已读取 \`demo-data/hello.txt\`：\n\n\`\`\`text\n${result}\`\`\`\n\n还需要我做什么吗？`;
  } else {
    finalText = `收到：「${input}」。\n\n这是 **mock 后端** 的流式回复（不联网、内存会话）。试试输入「读取 demo-data/hello.txt」。`;
  }
  for (const chunk of splitChunks(finalText, 8)) {
    steps.push({ type: "text_delta", delta: chunk });
  }
  steps.push({ type: "turn_completed", reply: { text: finalText, tool_calls: [], usage: null } });
  return steps;
}

function sortedWorkspaces(workspaces: Workspace[]): Workspace[] {
  return [...workspaces].sort((left, right) => {
    if (left.is_default !== right.is_default) {
      return left.is_default ? -1 : 1;
    }
    return left.created_at_ms - right.created_at_ms;
  });
}

export function createMockTransport(options: MockOptions = {}): Transport {
  const chunkDelayMs = options.chunkDelayMs ?? 300;
  const workspaces: Workspace[] = [];
  const conversations: Conversation[] = [];
  const messagesByConversation = new Map<string, Message[]>();
  const streams = new Map<string, StreamState>();
  const listeners = new Set<(event: UiEvent) => void>();
  let settings: AppSettings = defaultSettings();
  // Optimistic-concurrency fence mirror of the real host (`settings.meta.json`).
  let settingsRevision = 0;
  let seq = 0;
  let streamCounter = 0;

  const now = (): number => Date.now();

  const createWorkspace = (path: string, title: string): Workspace => {
    const trimmedPath = path.trim();
    const trimmedTitle = title.trim();
    const workspace: Workspace = {
      id: nextId("ws"),
      path: trimmedPath,
      title:
        trimmedTitle !== ""
          ? trimmedTitle
          : trimmedPath === "~"
            ? "主目录"
            : trimmedPath,
      is_default: false,
      created_at_ms: now(),
      updated_at_ms: now(),
    };
    workspaces.push(workspace);
    return workspace;
  };

  const addConversation = (
    workspaceId: string,
    title: string,
    updatedAt = now(),
  ): Conversation => {
    const conversation: Conversation = {
      id: nextId("conv"),
      workspace_id: workspaceId,
      title,
      created_at_ms: updatedAt,
      updated_at_ms: updatedAt,
    };
    conversations.unshift(conversation);
    messagesByConversation.set(conversation.id, []);
    return conversation;
  };

  if (options.startEmptyWorkspaces !== true) {
    const home = createWorkspace("~", "主目录");
    home.is_default = true;
    if (options.seed === true) {
      const projects = createWorkspace("~/code/zeokeel", "zeokeel");
      const api = createWorkspace("~/work/api-server", "api-server");
      addConversation(home.id, "重构工作区导航");
      addConversation(home.id, "解释 Rust 所有权");
      addConversation(projects.id, "排查 CI 缓存失效");
      addConversation(projects.id, "设计审批卡交互");
      addConversation(api.id, "整理 MCP 连接器清单");
    }
  }

  const emit = (streamId: string, event: AgentEvent): void => {
    seq += 1;
    const uiEvent: UiEvent = { stream_id: streamId, seq, kind: "agent", payload: event };
    for (const listener of listeners) {
      listener(uiEvent);
    }
  };

  const appendMessage = (message: Message): void => {
    const existing = messagesByConversation.get(message.conversation_id) ?? [];
    messagesByConversation.set(message.conversation_id, [...existing, message]);
    const current = conversations.find((item) => item.id === message.conversation_id);
    if (current !== undefined) {
      current.updated_at_ms = message.created_at_ms;
    }
  };

  const endStream = (streamId: string): void => {
    const stream = streams.get(streamId);
    if (stream === undefined) {
      return;
    }
    window.clearInterval(stream.timer);
    streams.delete(streamId);
  };

  const finishTurn = (streamId: string, replyText: string): void => {
    const stream = streams.get(streamId);
    if (stream === undefined) {
      return;
    }
    appendMessage({
      id: stream.assistantId,
      conversation_id: stream.conversationId,
      role: "assistant",
      content: replyText,
      created_at_ms: now(),
    });
    endStream(streamId);
  };

  const advance = (streamId: string): void => {
    const stream = streams.get(streamId);
    if (stream === undefined) {
      return;
    }
    const step = stream.steps[stream.index];
    stream.index += 1;
    if (step === undefined) {
      endStream(streamId);
      return;
    }
    emit(streamId, step);
    if (step.type === "text_delta") {
      stream.text += step.delta;
    }
    if (step.type === "turn_completed") {
      finishTurn(streamId, step.reply.text);
    }
  };

  const startTurn = (conversationId: string, text: string): string => {
    streamCounter += 1;
    const streamId = `mock-stream-${String(streamCounter)}`;
    const steps = planTurn(text);
    const stream: StreamState = {
      streamId,
      conversationId,
      assistantId: nextId("assistant"),
      steps,
      index: 0,
      timer: 0,
      text: "",
    };
    stream.timer = window.setInterval(() => {
      advance(streamId);
    }, chunkDelayMs);
    streams.set(streamId, stream);
    emit(streamId, { type: "turn_started", turn_id: streamId });
    return streamId;
  };

  const findWorkspace = (value: unknown): Workspace | null => {
    if (typeof value !== "string") {
      return null;
    }
    return workspaces.find((workspace) => workspace.id === value) ?? null;
  };

  const findConversation = (value: unknown): Conversation | null => {
    if (typeof value !== "string") {
      return null;
    }
    return conversations.find((conversation) => conversation.id === value) ?? null;
  };

  const requireWorkspace = (payload: Record<string, unknown>): Workspace => {
    const workspace = findWorkspace(payload["workspace_id"]);
    if (workspace === null) {
      throw new Error("workspace/not-found");
    }
    return workspace;
  };

  const requireConversation = (
    payload: Record<string, unknown>,
    workspace: Workspace,
  ): Conversation => {
    const conversation = findConversation(payload["conversation_id"]);
    if (conversation === null) {
      throw new Error("missing conversation_id");
    }
    if (conversation.workspace_id !== workspace.id) {
      throw new Error("session/workspace-mismatch");
    }
    return conversation;
  };

  const call = async (name: string, payload: Record<string, unknown>): Promise<unknown> => {
    await Promise.resolve();
    switch (name) {
      case "meta.get":
        return { edition: "personal", version: MOCK_VERSION, packages: MOCK_PACKAGES };
      case "workspace.list":
        return sortedWorkspaces(workspaces);
      case "workspace.create": {
        const path = payload["path"];
        if (typeof path !== "string" || path.trim() === "") {
          throw new Error("missing path");
        }
        const title = typeof payload["title"] === "string" ? payload["title"] : "";
        if (workspaces.some((workspace) => workspace.path === path.trim())) {
          throw new Error("workspace/duplicate-path");
        }
        return createWorkspace(path, title);
      }
      case "workspace.rename": {
        const workspace = requireWorkspace(payload);
        const title = payload["title"];
        if (typeof title !== "string" || title.trim() === "") {
          throw new Error("workspace/rename-empty");
        }
        workspace.title = title.trim();
        workspace.updated_at_ms = now();
        return workspace;
      }
      case "workspace.set_default": {
        const workspace = requireWorkspace(payload);
        for (const item of workspaces) {
          item.is_default = item.id === workspace.id;
        }
        return sortedWorkspaces(workspaces);
      }
      case "workspace.remove": {
        const workspace = requireWorkspace(payload);
        if (workspace.is_default) {
          throw new Error("workspace/default");
        }
        const reassign = findWorkspace(payload["reassign_to"]);
        const scoped = conversations.filter(
          (conversation) => conversation.workspace_id === workspace.id,
        );
        if (scoped.length > 0 && reassign === null) {
          throw new Error("workspace/has-sessions");
        }
        if (reassign !== null && reassign.id === workspace.id) {
          throw new Error("workspace/reassign-self");
        }
        let moved = 0;
        for (const conversation of scoped) {
          if (reassign !== null) {
            conversation.workspace_id = reassign.id;
            moved += 1;
          }
        }
        const index = workspaces.findIndex((item) => item.id === workspace.id);
        if (index >= 0) {
          workspaces.splice(index, 1);
        }
        return { removed: true, moved_sessions: moved };
      }
      case "session.list": {
        const workspace = requireWorkspace(payload);
        return conversations
          .filter((conversation) => conversation.workspace_id === workspace.id)
          .sort((left, right) => right.updated_at_ms - left.updated_at_ms);
      }
      case "session.create": {
        const workspace = requireWorkspace(payload);
        const titleValue = payload["title"];
        const title =
          typeof titleValue === "string" && titleValue.trim() !== ""
            ? titleValue
            : "New conversation";
        return addConversation(workspace.id, title);
      }
      case "session.messages": {
        const workspace = requireWorkspace(payload);
        const conversation = requireConversation(payload, workspace);
        return messagesByConversation.get(conversation.id) ?? [];
      }
      case "settings.get":
        return { settings: parseSettings(settings), revision: settingsRevision };
      case "settings.set": {
        const value = payload["settings"];
        if (!isRecord(value)) {
          throw new Error("settings.set: invalid settings payload");
        }
        const expected = payload["expected_revision"];
        if (expected !== undefined && expected !== null) {
          if (typeof expected !== "number" || expected !== settingsRevision) {
            throw new Error("settings/revision-conflict: stale expected_revision");
          }
        }
        settings = parseSettings(value);
        settingsRevision += 1;
        return { settings: parseSettings(settings), revision: settingsRevision };
      }
      case "workspace.changes": {
        requireWorkspace(payload);
        return { items: [] };
      }
      case "chat.send": {
        const workspace = requireWorkspace(payload);
        const conversation = requireConversation(payload, workspace);
        const text = payload["text"];
        if (typeof text !== "string" || text.trim() === "") {
          throw new Error("chat.send: text is required");
        }
        appendMessage({
          id: nextId("user"),
          conversation_id: conversation.id,
          role: "user",
          content: text,
          created_at_ms: now(),
        });
        const streamId = startTurn(conversation.id, text);
        return {
          stream_id: streamId,
          conversation_id: conversation.id,
          workspace_id: workspace.id,
        };
      }
      case "chat.cancel": {
        const streamId = payload["stream_id"];
        if (typeof streamId !== "string") {
          throw new Error("chat.cancel: stream_id is required");
        }
        const stream = streams.get(streamId);
        if (stream === undefined) {
          return {};
        }
        emit(streamId, { type: "turn_cancelled" });
        finishTurn(streamId, stream.text);
        return {};
      }
      case "approval.decide":
        return {};
      default:
        throw new Error(`mock: unknown command: ${name}`);
    }
  };

  return {
    call,
    subscribe(handler: (event: UiEvent) => void) {
      listeners.add(handler);
      return () => {
        listeners.delete(handler);
      };
    },
  };
}
