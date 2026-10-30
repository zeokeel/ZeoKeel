/**
 * Runtime boundary validation for everything that crosses the IPC seam.
 *
 * The UI never trusts a payload: JSON from Tauri/HTTP/mock is `unknown` and is
 * narrowed with type guards here. Invalid payloads return `null` so callers can
 * ignore them (unknown events) or surface a protocol error (commands).
 */
import {
  defaultSettings,
  type AgentEvent,
  type AppSettings,
  type CommandResult,
  type Conversation,
  type FileEntry,
  type Locale,
  type Message,
  type Meta,
  type ProviderConfig,
  type ProviderKind,
  type ReplyToolCall,
  type Theme,
  type UiEvent,
  type Workspace,
} from "./types";

import { isRecord, readBoolean, readNumber, readString } from "@zeokeel/ui-kit";

export { isRecord, readBoolean, readNumber, readString };

export function readArray(record: Record<string, unknown>, key: string): unknown[] | null {
  const value = record[key];
  return Array.isArray(value) ? value : null;
}

function readArrayOrNull(value: unknown): unknown[] | null {
  return Array.isArray(value) ? value : null;
}

function parseConversationRecord(value: Record<string, unknown>): Conversation | null {
  const id = readString(value, "id");
  const title = readString(value, "title");
  const createdAt = readNumber(value, "created_at_ms");
  if (id === null || title === null || createdAt === null) {
    return null;
  }
  const updatedAt = readNumber(value, "updated_at_ms");
  return {
    id,
    workspace_id: readString(value, "workspace_id") ?? "",
    title,
    created_at_ms: createdAt,
    updated_at_ms: updatedAt === null ? createdAt : updatedAt,
  };
}

export function parseConversation(value: unknown): Conversation | null {
  if (!isRecord(value)) {
    return null;
  }
  const direct = parseConversationRecord(value);
  if (direct !== null) {
    return direct;
  }
  const nested = value["conversation"];
  return isRecord(nested) ? parseConversationRecord(nested) : null;
}

function parseWorkspaceRecord(value: Record<string, unknown>): Workspace | null {
  const id = readString(value, "id");
  const path = readString(value, "path");
  const title = readString(value, "title");
  const isDefault = readBoolean(value, "is_default");
  const createdAt = readNumber(value, "created_at_ms");
  if (id === null || path === null || title === null || isDefault === null || createdAt === null) {
    return null;
  }
  const updatedAt = readNumber(value, "updated_at_ms");
  return {
    id,
    path,
    title,
    is_default: isDefault,
    created_at_ms: createdAt,
    updated_at_ms: updatedAt === null ? createdAt : updatedAt,
  };
}

export function parseWorkspace(value: unknown): Workspace | null {
  if (!isRecord(value)) {
    return null;
  }
  const direct = parseWorkspaceRecord(value);
  if (direct !== null) {
    return direct;
  }
  const nested = value["workspace"];
  return isRecord(nested) ? parseWorkspaceRecord(nested) : null;
}

export function parseFileEntries(value: unknown): FileEntry[] | null {
  if (!isRecord(value)) return null;
  const items = readArray(value, "items");
  if (items === null) return null;
  const out: FileEntry[] = [];
  for (const item of items) {
    if (!isRecord(item)) continue;
    const path = readString(item, "path");
    const depth = readNumber(item, "depth");
    const kind = readString(item, "kind");
    if (path === null || depth === null || (kind !== "dir" && kind !== "file")) continue;
    out.push({ path, depth, kind });
  }
  return out;
}

export function parseWorkspaces(value: unknown): Workspace[] | null {
  const items =
    readArrayOrNull(value) ??
    (isRecord(value) ? (readArray(value, "workspaces") ?? readArray(value, "items")) : null);
  if (items === null) {
    return null;
  }
  const workspaces: Workspace[] = [];
  for (const item of items) {
    const workspace = parseWorkspace(item);
    if (workspace === null) {
      return null;
    }
    workspaces.push(workspace);
  }
  return workspaces;
}

export function parseConversations(value: unknown): Conversation[] | null {
  const items =
    readArrayOrNull(value) ??
    (isRecord(value) ? (readArray(value, "conversations") ?? readArray(value, "items")) : null);
  if (items === null) {
    return null;
  }
  const conversations: Conversation[] = [];
  for (const item of items) {
    const conversation = parseConversation(item);
    if (conversation === null) {
      return null;
    }
    conversations.push(conversation);
  }
  return conversations;
}

function parseMessageRecord(value: Record<string, unknown>): Message | null {
  const id = readString(value, "id");
  const conversationId = readString(value, "conversation_id");
  const role = readString(value, "role");
  const content = readString(value, "content");
  const createdAt = readNumber(value, "created_at_ms");
  if (
    id === null ||
    conversationId === null ||
    role === null ||
    content === null ||
    createdAt === null
  ) {
    return null;
  }
  const status = readString(value, "status") ?? "done";
  return { id, conversation_id: conversationId, role, content, created_at_ms: createdAt, status };
}

export function parseMessages(value: unknown): Message[] | null {
  const items =
    readArrayOrNull(value) ??
    (isRecord(value) ? (readArray(value, "messages") ?? readArray(value, "items")) : null);
  if (items === null) {
    return null;
  }
  const messages: Message[] = [];
  for (const item of items) {
    if (!isRecord(item)) {
      return null;
    }
    const message = parseMessageRecord(item);
    if (message === null) {
      return null;
    }
    messages.push(message);
  }
  return messages;
}

function readLocale(value: unknown): Locale | null {
  return value === "zh-CN" || value === "en-US" ? value : null;
}

function readTheme(value: unknown): Theme | null {
  return value === "auto" || value === "light" || value === "dark" ? value : null;
}

function readProviderKind(value: unknown): ProviderKind | null {
  if (value === "mock" || value === "ollama") {
    return value;
  }
  // Rust serializes `OpenAiCompatible` as `open_ai_compatible`; accept the
  // legacy/hand-written spellings too so old local settings keep working.
  if (value === "open_ai_compatible" || value === "openai-compatible" || value === "openai_compatible") {
    return "open_ai_compatible";
  }
  return null;
}

function parseProvider(value: Record<string, unknown>): ProviderConfig {
  const base = defaultSettings().provider;
  const kind = readProviderKind(value["kind"]) ?? base.kind;
  return {
    kind,
    base_url: readString(value, "base_url") ?? base.base_url,
    model: readString(value, "model") ?? base.model,
    api_key: readString(value, "api_key") ?? base.api_key,
  };
}

export function parseSettings(value: unknown): AppSettings {
  const base = defaultSettings();
  if (!isRecord(value)) {
    return base;
  }
  const provider = isRecord(value["provider"]) ? parseProvider(value["provider"]) : base.provider;
  return {
    schema: readNumber(value, "schema") ?? base.schema,
    provider,
    theme: readTheme(value["theme"]) ?? base.theme,
    locale: readLocale(value["locale"]) ?? base.locale,
    auto_approve_read_only:
      readBoolean(value, "auto_approve_read_only") ?? base.auto_approve_read_only,
    server_url: readString(value, "server_url") ?? base.server_url,
  };
}

export function parseMeta(value: unknown): Meta {
  if (!isRecord(value)) {
    return { edition: "personal", version: "0.0.0", packages: [] };
  }
  const packagesRaw = readArray(value, "packages") ?? [];
  const packages = packagesRaw.filter((item): item is string => typeof item === "string");
  return {
    edition: readString(value, "edition") ?? "personal",
    version: readString(value, "version") ?? "0.0.0",
    packages,
  };
}

export function parseCommandResponse(value: unknown): CommandResult {
  if (!isRecord(value)) {
    return { ok: false, data: null, error: "invalid response" };
  }
  const ok = value["ok"] === true;
  return {
    ok,
    data: ok ? (value["data"] ?? null) : null,
    error: readString(value, "error"),
  };
}

export function parseStreamId(value: unknown): string | null {
  if (typeof value === "string" && value !== "") {
    return value;
  }
  if (isRecord(value)) {
    const streamId = readString(value, "stream_id");
    return streamId === null || streamId === "" ? null : streamId;
  }
  return null;
}

export function parseUiEvent(value: unknown): UiEvent | null {
  if (!isRecord(value)) {
    return null;
  }
  const streamId = readString(value, "stream_id");
  const seq = readNumber(value, "seq");
  const kind = readString(value, "kind");
  if (streamId === null || seq === null || kind === null) {
    return null;
  }
  return { stream_id: streamId, seq, kind, payload: value["payload"] ?? null };
}

type CompletedReply = { text: string; tool_calls: ReplyToolCall[] };

function parseCompletedReply(value: unknown): CompletedReply | null {
  if (!isRecord(value)) {
    return null;
  }
  const text = readString(value, "text");
  if (text === null) {
    return null;
  }
  const rawCalls = readArray(value, "tool_calls") ?? [];
  const toolCalls: ReplyToolCall[] = [];
  for (const item of rawCalls) {
    if (!isRecord(item)) {
      return null;
    }
    const name = readString(item, "name");
    const args = readString(item, "args");
    const result = readString(item, "result");
    if (name === null || args === null || result === null) {
      return null;
    }
    toolCalls.push({ name, args, result });
  }
  return { text, tool_calls: toolCalls };
}

export function parseAgentEvent(value: unknown): AgentEvent | null {
  if (!isRecord(value)) {
    return null;
  }
  const type = readString(value, "type");
  switch (type) {
    case "turn_started": {
      const turnId = readString(value, "turn_id");
      return turnId === null ? null : { type, turn_id: turnId };
    }
    case "text_delta": {
      const delta = readString(value, "delta");
      return delta === null ? null : { type, delta };
    }
    case "tool_call_started": {
      const name = readString(value, "name");
      const args = readString(value, "args");
      return name === null || args === null ? null : { type, name, args };
    }
    case "tool_call_finished": {
      const name = readString(value, "name");
      const result = readString(value, "result");
      return name === null || result === null ? null : { type, name, result };
    }
    case "approval_requested": {
      const approvalId = readString(value, "approval_id");
      const tool = readString(value, "tool");
      const args = readString(value, "args");
      const reason = readString(value, "reason");
      if (approvalId === null || tool === null || args === null || reason === null) {
        return null;
      }
      return { type, approval_id: approvalId, tool, args, reason };
    }
    case "turn_completed": {
      const reply = parseCompletedReply(value["reply"]);
      return reply === null ? null : { type, reply };
    }
    case "turn_cancelled":
      return { type };
    case "error": {
      const message = readString(value, "message");
      return message === null ? null : { type, message };
    }
    default:
      return null;
  }
}
