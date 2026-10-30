/**
 * In-memory demo backend (`?mock=1`): local/screenshot/component-test mode.
 *
 * Simulates the same command/event contract as the real bridge:
 * - 300ms streaming chunks (configurable for tests),
 * - tool events when the input mentions “读取”,
 * - sessions and settings live in memory only.
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
} from "../types";

interface MockOptions {
  chunkDelayMs?: number;
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

function nextId(prefix: string): string {
  idCounter += 1;
  return `${prefix}-${idCounter}`;
}
let idCounter = 0;

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
  steps.push({ type: "turn_completed", reply: { text: finalText, tool_calls: [] } });
  return steps;
}

export function createMockTransport(options: MockOptions = {}): Transport {
  const chunkDelayMs = options.chunkDelayMs ?? 300;
  const conversations: Conversation[] = [];
  const messagesByConversation = new Map<string, Message[]>();
  const streams = new Map<string, StreamState>();
  const listeners = new Set<(event: UiEvent) => void>();
  let settings: AppSettings = defaultSettings();
  let seq = 0;
  let streamCounter = 0;

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

  const createConversation = (title: string): Conversation => {
    const now = Date.now();
    const conversation: Conversation = {
      id: nextId("conv"),
      title,
      created_at_ms: now,
      updated_at_ms: now,
    };
    conversations.unshift(conversation);
    messagesByConversation.set(conversation.id, []);
    return conversation;
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
      created_at_ms: Date.now(),
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
    const streamId = `mock-stream-${streamCounter}`;
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

  const call = async (name: string, payload: Record<string, unknown>): Promise<unknown> => {
    switch (name) {
      case "meta.get":
        return { edition: "personal", version: MOCK_VERSION, packages: MOCK_PACKAGES };
      case "session.list":
        return [...conversations].sort((left, right) => right.updated_at_ms - left.updated_at_ms);
      case "session.create": {
        const titleValue = payload["title"];
        const title = typeof titleValue === "string" && titleValue.trim() !== ""
          ? titleValue
          : "New conversation";
        return createConversation(title);
      }
      case "session.messages": {
        const conversationId = payload["conversation_id"];
        if (typeof conversationId !== "string") {
          throw new Error("session.messages: conversation_id is required");
        }
        return messagesByConversation.get(conversationId) ?? [];
      }
      case "settings.get":
        return parseSettings(settings);
      case "settings.set": {
        const value = payload["settings"];
        if (!isRecord(value)) {
          throw new Error("settings.set: invalid settings payload");
        }
        settings = parseSettings(value);
        return parseSettings(settings);
      }
      case "chat.send": {
        const conversationId = payload["conversation_id"];
        const text = payload["text"];
        if (typeof conversationId !== "string" || typeof text !== "string" || text.trim() === "") {
          throw new Error("chat.send: conversation_id and text are required");
        }
        const conversation = conversations.find((item) => item.id === conversationId);
        if (conversation === undefined) {
          throw new Error(`chat.send: conversation not found: ${conversationId}`);
        }
        appendMessage({
          id: nextId("user"),
          conversation_id: conversationId,
          role: "user",
          content: text,
          created_at_ms: Date.now(),
        });
        const streamId = startTurn(conversationId, text);
        return { stream_id: streamId };
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
