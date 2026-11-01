/**
 * Runtime boundary validation for everything that crosses the IPC seam.
 *
 * The UI never trusts a payload: JSON from Tauri/HTTP/mock is `unknown` and is
 * narrowed with type guards here. Invalid payloads return `null` so callers can
 * ignore them (unknown events) or surface a protocol error (commands).
 */
import {
  defaultAgentSettings,
  defaultEmbeddingSettings,
  defaultMediaSettings,
  defaultSettings,
  defaultToolSettings,
  defaultWebSearchSettings,
  type AgentEvent,
  type AgentSettings,
  type ApprovalMetadata,
  type ApprovalRisk,
  type AgentUsage,
  type AppSettings,
  type McpServerConfig,
  type CommandResult,
  type Conversation,
  type CompactionMarker,
  type PlanView,
  type ConversationExport,
  type ConversationOverride,
  type EmbeddingSettings,
  type GoalView,
  type Locale,
  type MediaSettings,
  type Message,
  type Meta,
  type ProviderConfig,
  type ProviderKind,
  type ReplyToolCall,
  type Theme,
  type ToolChoiceSetting,
  type ToolSettings,
  type UiEvent,
  type WebSearchSettings,
  type Workspace,
} from "./types";

import { isRecord, readBoolean, readNumber, readString } from "@zeokeel/ui-kit";
import type {
  FileEntry,
  QuestionAskedPayload,
  QuestionDef,
  QuestionOption,
  SessionSearchHit,
  SessionSearchResult,
  TerminalResult,
  WorkspaceChange,
  WorkspaceChangesResult,
  WorkspaceFileReadResult,
  WorkspaceFilesResult,
} from "../../../../shared/contracts/ts/contracts";

// Wire DTOs are generated from the Rust contracts (T22); re-export for app callers so the app does
// not hand-mirror boundary shapes (FileEntry/WorkspaceFilesResult/QuestionDef live only in contracts.ts).
export type { FileEntry, QuestionAskedPayload, QuestionDef, QuestionOption, SessionSearchHit, SessionSearchResult, TerminalResult, WorkspaceChange, WorkspaceChangesResult, WorkspaceFileReadResult, WorkspaceFilesResult };
import {
  validateAgentEvent,
  validateAppSettings,
  validateCommandResponse,
  validateConversation,
  validateMessage,
  validateMetaInfo,
  validateQuestionAsked,
  validateUiEvent,
  validateWorkspace,
  validateWorkspaceChangesResult,
  validateWorkspaceFilesResult,
  validateCompactionGetResult,
  validatePlanGetResult,
  validatePlanResult,
  validateCompactResult,
  validateConversationOverrideResult,
  validateFeedbackSubmitResult,
  validateGoalResult,
  validateSessionExportResult,
  validateSessionSearchResult,
  validateTerminalResult,
  validateWorkspaceFileReadResult,
} from "./validate";

export { isRecord, readBoolean, readNumber, readString };

export function readArray(record: Record<string, unknown>, key: string): unknown[] | null {
  const value = record[key];
  return Array.isArray(value) ? value : null;
}

function readArrayOrNull(value: unknown): unknown[] | null {
  return Array.isArray(value) ? value : null;
}

function parseConversationRecord(value: Record<string, unknown>): Conversation | null {
  if (!validateConversation(value)) {
    return null;
  }
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
    updated_at_ms: updatedAt ?? createdAt,
    pinned: readBoolean(value, "pinned") ?? false,
    archived: readBoolean(value, "archived") ?? false,
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
  if (!validateWorkspace(value)) {
    return null;
  }
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
    updated_at_ms: updatedAt ?? createdAt,
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

/** Validates `workspace.files` against the generated schema, then maps the typed DTO. */
export function parseWorkspaceFiles(value: unknown): WorkspaceFilesResult | null {
  if (!isRecord(value) || !validateWorkspaceFilesResult(value)) return null;
  const items = readArray(value, "items");
  if (items === null) return null;
  const entries: FileEntry[] = [];
  for (const item of items) {
    if (!isRecord(item)) continue;
    const path = readString(item, "path");
    const depth = readNumber(item, "depth");
    const kind = readString(item, "kind");
    if (path === null || depth === null || (kind !== "dir" && kind !== "file")) continue;
    const badge = readString(item, "badge");
    entries.push(badge === null ? { path, depth, kind } : { path, depth, kind, badge });
  }
  return { items: entries };
}

export function parseFileEntries(value: unknown): FileEntry[] | null {
  return parseWorkspaceFiles(value)?.items ?? null;
}

export interface ParsedWorkspaceChanges {
  items: WorkspaceChange[];
  /** Non-fatal scan notice (git unavailable/non-repo/truncated); null when clean. */
  warning: string | null;
}

/** Validates `workspace.file_read` against the generated schema. */
export function parseWorkspaceFileReadResult(value: unknown): WorkspaceFileReadResult | null {
  if (!isRecord(value) || !validateWorkspaceFileReadResult(value)) return null;
  return {
    path: readString(value, "path") ?? "",
    kind: value["kind"] === "image" ? "image" : value["kind"] === "text" ? "text" : "unsupported",
    mime: readString(value, "mime") ?? "application/octet-stream",
    size: readNumber(value, "size") ?? 0,
    truncated: readBoolean(value, "truncated") ?? false,
    encoding: readString(value, "encoding") ?? "",
    content: readString(value, "content") ?? "",
    reason: readString(value, "reason") ?? "",
  };
}

/** Validates `terminal` against the generated schema, then maps the typed DTO. */
export function parseTerminalResult(value: unknown): TerminalResult | null {
  if (!isRecord(value) || !validateTerminalResult(value)) return null;
  const sessions = readArray(value, "sessions");
  if (sessions === null) return null;
  return {
    sessions: sessions.filter((entry): entry is string => typeof entry === "string"),
    output: readString(value, "output") ?? "",
    truncated: readBoolean(value, "truncated") ?? false,
    exit_code: readNumber(value, "exit_code"),
    timed_out: readBoolean(value, "timed_out") ?? false,
  };
}

/** Validates `session.search` against the generated schema, then maps the typed DTO. */
export function parseSessionSearchResult(value: unknown): SessionSearchResult | null {
  if (!isRecord(value) || !validateSessionSearchResult(value)) return null;
  const items = readArray(value, "items");
  if (items === null) return null;
  const hits: SessionSearchHit[] = [];
  for (const item of items) {
    if (!isRecord(item)) continue;
    const conversationId = readString(item, "conversation_id");
    const messageId = readString(item, "message_id");
    if (conversationId === null || messageId === null) continue;
    hits.push({
      conversation_id: conversationId,
      title: readString(item, "title") ?? "",
      message_id: messageId,
      role: readString(item, "role") ?? "",
      snippet: readString(item, "snippet") ?? "",
      archived: readBoolean(item, "archived") ?? false,
    });
  }
  return { items: hits, truncated: readBoolean(value, "truncated") ?? false };
}

/** Validates `workspace.changes` against the generated schema, then maps the typed DTO. */
export function parseWorkspaceChanges(value: unknown): ParsedWorkspaceChanges | null {
  if (!isRecord(value) || !validateWorkspaceChangesResult(value)) {
    return null;
  }
  const rawItems = readArray(value, "items");
  if (rawItems === null) {
    return null;
  }
  const items: WorkspaceChange[] = [];
  for (const item of rawItems) {
    if (!isRecord(item)) {
      continue;
    }
    const path = readString(item, "path");
    const add = readNumber(item, "add");
    const del = readNumber(item, "del");
    if (path === null || add === null || del === null) {
      continue;
    }
    items.push({ path, add, del });
  }
  return { items, warning: readString(value, "warning") };
}

export interface UpdateRelease { version: string; notes: string }

export function parseUpdateRelease(value: unknown): UpdateRelease | null {
  if (!isRecord(value)) return null;
  const release = value["release"];
  if (!isRecord(release)) return null;
  const version = readString(release, "version");
  if (version === null || version === "") return null;
  return { version, notes: readString(release, "notes") ?? "" };
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
  let record: Record<string, unknown> = value;
  let usage: AgentUsage | null = null;
  if (validateMessage(value)) {
    usage = parseUsage(value["usage"]);
  } else {
    // Boundary policy: invalid persisted usage degrades to "unknown" (null) without dropping the
    // message; any other schema violation still rejects the record.
    const withoutUsage = { ...value };
    delete withoutUsage["usage"];
    if (!validateMessage(withoutUsage)) {
      return null;
    }
    record = withoutUsage;
  }
  const id = readString(record, "id");
  const conversationId = readString(record, "conversation_id");
  const role = readString(record, "role");
  const content = readString(record, "content");
  const createdAt = readNumber(record, "created_at_ms");
  if (
    id === null ||
    conversationId === null ||
    role === null ||
    content === null ||
    createdAt === null
  ) {
    return null;
  }
  const status = readString(record, "status") ?? "done";
  const turnId = readString(record, "turn_id") ?? "";
  return {
    id,
    conversation_id: conversationId,
    role,
    content,
    created_at_ms: createdAt,
    status,
    usage,
    turn_id: turnId,
  };
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

function readNullableNumber(value: unknown): number | null {
  return typeof value === "number" && Number.isFinite(value) ? value : null;
}

function readToolChoice(value: unknown): ToolChoiceSetting {
  const fallback: ToolChoiceSetting = { mode: "auto" };
  if (!isRecord(value)) {
    return fallback;
  }
  const mode = readString(value, "mode");
  if (mode === "none" || mode === "required") {
    return { mode };
  }
  if (mode === "specific") {
    const tools = (readArray(value, "tools") ?? []).flatMap((tool) =>
      typeof tool === "string" && tool.trim() !== "" ? [tool] : [],
    );
    return tools.length === 0 ? fallback : { mode: "specific", tools };
  }
  return fallback;
}

function parseAgentSettings(value: unknown): AgentSettings {
  const base = defaultAgentSettings();
  if (!isRecord(value)) {
    return base;
  }
  const completion = isRecord(value["completion"]) ? value["completion"] : {};
  const memory = isRecord(value["memory"]) ? value["memory"] : {};
  const hooks = isRecord(value["hooks"]) ? value["hooks"] : {};
  const schedule = isRecord(value["schedule"]) ? value["schedule"] : {};
  const stop = (readArray(completion, "stop") ?? []).flatMap((item) =>
    typeof item === "string" ? [item] : [],
  );
  return {
    max_steps: readNumber(value, "max_steps") ?? base.max_steps,
    completion: {
      temperature: readNullableNumber(completion["temperature"]),
      max_tokens: readNullableNumber(completion["max_tokens"]),
      top_p: readNullableNumber(completion["top_p"]),
      seed: readNullableNumber(completion["seed"]),
      stop,
      tool_choice: readToolChoice(completion["tool_choice"]),
      parallel_tool_calls: readBoolean(completion, "parallel_tool_calls"),
      reasoning_effort: readString(completion, "reasoning_effort"),
      prompt_cache: readString(completion, "prompt_cache"),
    },
    memory: {
      enabled: readBoolean(memory, "enabled") ?? base.memory.enabled,
      dir: readString(memory, "dir") ?? base.memory.dir,
    },
    hooks: {
      enabled: readBoolean(hooks, "enabled") ?? base.hooks.enabled,
      log: readString(hooks, "log") ?? base.hooks.log,
    },
    schedule: {
      enabled: readBoolean(schedule, "enabled") ?? base.schedule.enabled,
      scan_seconds: readNumber(schedule, "scan_seconds") ?? base.schedule.scan_seconds,
    },
  };
}

function parseUsage(value: unknown): AgentUsage | null {
  if (!isRecord(value)) {
    return null;
  }
  // `usage` is only emitted when the provider reported usage; unreported counters are normalized to 0.
  return {
    input_tokens: readNumber(value, "input_tokens") ?? 0,
    output_tokens: readNumber(value, "output_tokens") ?? 0,
    total_tokens: readNumber(value, "total_tokens") ?? 0,
    cached_tokens: readNumber(value, "cached_tokens") ?? 0,
    reasoning_tokens: readNumber(value, "reasoning_tokens") ?? 0,
    cost_micros: readNullableNumber(value["cost_micros"]),
  };
}

export function parseSettings(value: unknown): AppSettings {
  const base = defaultSettings();
  if (!isRecord(value) || !validateAppSettings(value)) {
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
    agent: parseAgentSettings(value["agent"]),
    tools: parseToolSettings(value["tools"]),
    embedding: parseEmbeddingSettings(value["embedding"]),
    media: parseMediaSettings(value["media"]),
    web_search: parseWebSearchSettings(value["web_search"]),
    mcp: parseMcpSettings(value["mcp"]),
  };
}

/** `settings.get` / `settings.set` result: document + revision fence. */
export interface SettingsDocumentResult {
  settings: AppSettings;
  revision: number;
}

/**
 * Parse the `{ settings, revision }` envelope of `settings.get`/`settings.set`.
 *
 * A missing/invalid revision degrades to `0` (legacy host); the settings body is
 * normalized by `parseSettings` (never throws, defaults on garbage).
 */
export function parseSettingsDocumentResult(value: unknown): SettingsDocumentResult {
  if (!isRecord(value)) {
    return { settings: defaultSettings(), revision: 0 };
  }
  const revision = readNumber(value, "revision");
  return {
    settings: parseSettings(value["settings"]),
    revision:
      revision !== null && revision >= 0 && Number.isSafeInteger(revision) ? revision : 0,
  };
}

function parseWebSearchSettings(value: unknown): WebSearchSettings {
  const base = defaultWebSearchSettings();
  if (!isRecord(value)) {
    return base;
  }
  return {
    base_url: readString(value, "base_url") ?? base.base_url,
    timeout_ms: readNumber(value, "timeout_ms") ?? base.timeout_ms,
  };
}

function parseMediaSettings(value: unknown): MediaSettings {
  const base = defaultMediaSettings();
  if (!isRecord(value)) {
    return base;
  }
  return {
    base_url: readString(value, "base_url") ?? base.base_url,
    api_key: readString(value, "api_key") ?? base.api_key,
    image_model: readString(value, "image_model") ?? base.image_model,
    audio_model: readString(value, "audio_model") ?? base.audio_model,
    transcription_model: readString(value, "transcription_model") ?? base.transcription_model,
    rerank_model: readString(value, "rerank_model") ?? base.rerank_model,
  };
}

function parseEmbeddingSettings(value: unknown): EmbeddingSettings {
  const base = defaultEmbeddingSettings();
  if (!isRecord(value)) {
    return base;
  }
  return {
    base_url: readString(value, "base_url") ?? base.base_url,
    api_key: readString(value, "api_key") ?? base.api_key,
    model: readString(value, "model") ?? base.model,
    dims: readNullableNumber(value["dims"]),
    dir: readString(value, "dir") ?? base.dir,
  };
}

/** Lenient policy reader: unknown/missing values mean "no explicit policy". */
function readApprovalPolicy(value: unknown): "ask" | "auto_read_only" | "strict" | null {
  return value === "ask" || value === "auto_read_only" || value === "strict" ? value : null;
}

/** Lenient MCP parser: unknown rows are dropped; the host validates on save. */
function parseMcpSettings(value: unknown): AppSettings["mcp"] {
  const servers = isRecord(value) ? value["servers"] : null;
  if (!Array.isArray(servers)) return { servers: [] };
  const parsed: McpServerConfig[] = [];
  for (const row of servers) {
    if (!isRecord(row)) continue;
    const id = readString(row, "id");
    const name = readString(row, "name");
    const transport = row["transport"];
    if (id === null || name === null) continue;
    const envValue = isRecord(row["env"]) ? row["env"] : null;
    const env: Record<string, string> = {};
    if (envValue !== null) {
      for (const [key, entry] of Object.entries(envValue)) {
        if (typeof entry === "string") env[key] = entry;
      }
    }
    parsed.push({
      id,
      name,
      transport: transport === "http" || transport === "sse" ? transport : "stdio",
      command: readString(row, "command") ?? "",
      args: readArray(row, "args")?.flatMap((arg) => (typeof arg === "string" ? [arg] : [])) ?? [],
      env,
      url: readString(row, "url") ?? "",
      timeout_ms: readNumber(row, "timeout_ms") ?? 0,
      oauth_required: row["oauth_required"] === true,
    });
  }
  return { servers: parsed };
}

function parseToolSettings(value: unknown): ToolSettings {
  const base = defaultToolSettings();
  if (!isRecord(value)) {
    return base;
  }
  return {
    tool_timeout_ms: readNumber(value, "tool_timeout_ms") ?? base.tool_timeout_ms,
    approval_timeout_ms: readNumber(value, "approval_timeout_ms") ?? base.approval_timeout_ms,
    approval_policy: readApprovalPolicy(value["approval_policy"]),
  };
}

export function parseMeta(value: unknown): Meta {
  if (!isRecord(value) || !validateMetaInfo(value)) {
    return { edition: "personal", version: "0.0.0", packages: [], agentConfigError: null };
  }
  const packagesRaw = readArray(value, "packages") ?? [];
  const packages = packagesRaw.filter((item): item is string => typeof item === "string");
  return {
    edition: readString(value, "edition") ?? "personal",
    version: readString(value, "version") ?? "0.0.0",
    packages,
    agentConfigError: readString(value, "agent_config_error"),
  };
}

export function parseCommandResponse(value: unknown): CommandResult {
  if (!isRecord(value) || !validateCommandResponse(value)) {
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
  if (!isRecord(value) || !validateUiEvent(value)) {
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

interface CompletedReply { text: string; tool_calls: ReplyToolCall[]; usage: AgentUsage | null }

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
  return { text, tool_calls: toolCalls, usage: parseUsage(value["usage"]) };
}

function parseQuestions(value: unknown): QuestionDef[] | null {
  if (!Array.isArray(value)) {
    return null;
  }
  const questions: QuestionDef[] = [];
  for (const item of value) {
    if (!isRecord(item)) {
      return null;
    }
    const id = readString(item, "id");
    const question = readString(item, "question");
    if (id === null || question === null) {
      return null;
    }
    const parsed: QuestionDef = { id, question };
    const header = readString(item, "header");
    if (header !== null) parsed.header = header;
    const multiSelect = readBoolean(item, "multi_select");
    if (multiSelect !== null) parsed.multi_select = multiSelect;
    const rawOptions = item["options"];
    if (rawOptions !== undefined) {
      if (!Array.isArray(rawOptions)) {
        return null;
      }
      const options: QuestionOption[] = [];
      for (const option of rawOptions) {
        if (!isRecord(option)) return null;
        const label = readString(option, "label");
        if (label === null) return null;
        const description = readString(option, "description");
        options.push(description === null ? { label } : { label, description });
      }
      parsed.options = options;
    }
    questions.push(parsed);
  }
  return questions;
}

function parseApprovalMetadata(value: unknown): ApprovalMetadata | null {
  if (!isRecord(value)) {
    return null;
  }
  const textArray = (key: string): string[] => {
    const raw = value[key];
    if (!Array.isArray(raw)) {
      return [];
    }
    const items: string[] = [];
    for (const item of raw) {
      if (typeof item === "string") {
        items.push(item);
      }
    }
    return items;
  };
  const riskRaw = readString(value, "risk");
  const risk: ApprovalRisk | undefined =
    riskRaw === "low" || riskRaw === "medium" || riskRaw === "high" ? riskRaw : undefined;
  const metadata: ApprovalMetadata = {
    summary: readString(value, "summary") ?? "",
    cwd: readString(value, "cwd") ?? "",
    read_paths: textArray("read_paths"),
    write_paths: textArray("write_paths"),
    network_targets: textArray("network_targets"),
    network_undetermined: value["network_undetermined"] === true,
  };
  if (risk !== undefined) {
    metadata.risk = risk;
  }
  const diff = readString(value, "diff");
  if (diff !== null) {
    metadata.diff = diff;
  }
  return metadata;
}

export function parseAgentEvent(value: unknown): AgentEvent | null {
  if (!isRecord(value) || !validateAgentEvent(value)) {
    return null;
  }
  const type = readString(value, "type");
  if (type === null) {
    return null;
  }
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
      if (name === null || result === null) {
        return null;
      }
      const args = readString(value, "args") ?? "";
      const rawStatus = readString(value, "status");
      const status =
        rawStatus === "failed" || rawStatus === "denied" || rawStatus === "cancelled"
          ? rawStatus
          : "success";
      return { type, name, result, args, status };
    }
    case "question_asked": {
      const questionId = readString(value, "question_id");
      if (questionId === null) {
        return null;
      }
      const asked: QuestionAskedPayload = { question_id: questionId, questions: [] };
      const questions = parseQuestions(value["questions"]);
      if (questions === null || !validateQuestionAsked({ question_id: questionId, questions })) {
        return null;
      }
      asked.questions = questions;
      return { type, question_id: asked.question_id, questions: asked.questions };
    }
    case "question_answered": {
      const questionId = readString(value, "question_id");
      return questionId === null ? null : { type, question_id: questionId, answers: value["answers"] ?? null };
    }
    case "approval_requested": {
      const approvalId = readString(value, "approval_id");
      const tool = readString(value, "tool");
      const args = readString(value, "args");
      const reason = readString(value, "reason");
      if (approvalId === null || tool === null || args === null || reason === null) {
        return null;
      }
      return {
        type,
        approval_id: approvalId,
        tool,
        args,
        reason,
        auto: value["auto"] === true,
        deadline_ms: readNumber(value, "deadline_ms") ?? 0,
        metadata: parseApprovalMetadata(value["metadata"]),
      };
    }
    case "approval_resolved": {
      const approvalId = readString(value, "approval_id");
      const decision = readString(value, "decision");
      if (
        approvalId === null ||
        (decision !== "allow_once" &&
          decision !== "allow_session" &&
          decision !== "reject" &&
          decision !== "cancel")
      ) {
        return null;
      }
      return { type, approval_id: approvalId, decision, timed_out: value["timed_out"] === true };
    }
    case "usage": {
      const usage = parseUsage(value["usage"]);
      return usage === null ? null : { type, usage };
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

/** `goal.get`/`goal.set`/`goal.update` result → `GoalView | null`. */
export function parseGoalResult(value: unknown): { goal: GoalView | null } | null {
  if (!isRecord(value) || !validateGoalResult(value)) return null;
  const raw = value["goal"];
  if (raw === null || raw === undefined) return { goal: null };
  if (!isRecord(raw)) return null;
  const objective = readString(raw, "objective");
  const status = readString(raw, "status");
  if (objective === null || status === null) return null;
  return {
    goal: {
      objective,
      status,
      rounds: readNumber(raw, "rounds") ?? 0,
      max_rounds: readNumber(raw, "max_rounds") ?? 0,
      reason: readString(raw, "reason"),
      last_result: readString(raw, "last_result"),
      created_ms: readNumber(raw, "created_ms") ?? 0,
      updated_ms: readNumber(raw, "updated_ms") ?? 0,
    },
  };
}

/** `session.override_get`/`override_set` result → conversation override. */
export function parseConversationOverrideResult(value: unknown): ConversationOverride | null {
  if (!isRecord(value) || !validateConversationOverrideResult(value)) return null;
  const raw = value["overrides"];
  if (!isRecord(raw)) return null;
  return {
    model: readString(raw, "model"),
    reasoning_effort: readString(raw, "reasoning_effort"),
    approval_policy: readString(raw, "approval_policy"),
    is_session_override: readBoolean(raw, "is_session_override") ?? false,
  };
}

/** `session.export` result. */
export function parseSessionExportResult(value: unknown): ConversationExport | null {
  if (!isRecord(value) || !validateSessionExportResult(value)) return null;
  const path = readString(value, "path");
  if (path === null) return null;
  return {
    path,
    bytes: readNumber(value, "bytes") ?? 0,
    mime: readString(value, "mime") ?? "text/markdown",
    truncated: readBoolean(value, "truncated") ?? false,
  };
}

/** `feedback.submit` result. */
export function parseFeedbackSubmitResult(value: unknown): { stored: string; path: string } | null {
  if (!isRecord(value) || !validateFeedbackSubmitResult(value)) return null;
  const path = readString(value, "path");
  if (path === null) return null;
  return { stored: readString(value, "stored") ?? "local", path };
}

/** Parse one marker object (camelCase view type; marker_id → markerId). */
export function parseCompactionMarker(value: unknown): CompactionMarker | null {
  if (!isRecord(value)) return null;
  const markerId = readString(value, "marker_id");
  const summary = readString(value, "summary");
  const through = readString(value, "through_message_id");
  if (markerId === null || summary === null || through === null) return null;
  return {
    markerId,
    summary,
    throughMessageId: through,
    createdMs: readNumber(value, "created_ms") ?? 0,
    inputTokens: readNumber(value, "input_tokens") ?? 0,
    outputTokens: readNumber(value, "output_tokens") ?? 0,
    compressedMessages: readNumber(value, "compressed_messages") ?? 0,
    keepRecent: readNumber(value, "keep_recent") ?? 0,
  };
}

/** `session.compaction_get` result → marker or null. */
export function parseCompactionGetResult(value: unknown): { marker: CompactionMarker | null } | null {
  if (!isRecord(value) || !validateCompactionGetResult(value)) return null;
  return { marker: parseCompactionMarker(value["marker"]) };
}

/** `session.compact` result → { marker, noop }. */
export function parseCompactResult(value: unknown): { marker: CompactionMarker | null; noop: boolean } | null {
  if (!isRecord(value) || !validateCompactResult(value)) return null;
  return {
    marker: parseCompactionMarker(value["marker"]),
    noop: readBoolean(value, "noop") ?? false,
  };
}

/** Parse one plan view object (snake_case wire → camelCase view). */
export function parsePlanView(value: unknown): PlanView | null {
  if (!isRecord(value)) return null;
  const mode = readString(value, "mode");
  const status = readString(value, "status");
  const plan = readString(value, "plan");
  if (mode === null || status === null || plan === null) return null;
  if (mode !== "active" && mode !== "off") return null;
  if (status !== "draft" && status !== "pending_review" && status !== "approved" && status !== "rejected") return null;
  const feedback = readString(value, "feedback");
  const throughTurn = readString(value, "through_turn");
  return { mode, status, plan, feedback, throughTurn };
}

/** `plan.get` result. */
export function parsePlanGetResult(value: unknown): PlanView | null {
  if (!isRecord(value) || !validatePlanGetResult(value)) return null;
  return parsePlanView(value["plan"]);
}

/** `plan.update` result. */
export function parsePlanUpdateResult(value: unknown): PlanView | null {
  if (!isRecord(value) || !validatePlanResult(value)) return null;
  return parsePlanView(value["plan"]);
}
