/**
 * Shared UI DTOs and view types (contract mirrors + chat/stream view state).
 * No app imports: ui-kit only depends on React.
 */

import type { ComponentType } from "react";

export type Locale = "zh-CN" | "en-US";

export type Theme = "auto" | "light" | "dark";

export type ResolvedTheme = "light" | "dark";

export type BootState = "loading" | "ready" | "error";

export type ThreadLoadState = "idle" | "loading" | "error";

/** Mirrors `zk_contract_config::ProviderKind` (`serde(rename_all = "snake_case")`). */
export type ProviderKind = "mock" | "open_ai_compatible" | "ollama";

export interface ProviderConfig {
  kind: ProviderKind;
  base_url: string;
  model: string;
  api_key: string;
}

/** Mirrors `zk_contract_config::ToolChoiceSetting` (serde tag/content). */
export type ToolChoiceSetting =
  | { mode: "auto" }
  | { mode: "none" }
  | { mode: "required" }
  | { mode: "specific"; tools: string[] };

/** Normalized `zk_contract_config::CompletionSettings` (nullable = provider default). */
export interface CompletionSettings {
  temperature: number | null;
  max_tokens: number | null;
  top_p: number | null;
  seed: number | null;
  stop: string[];
  tool_choice: ToolChoiceSetting;
  parallel_tool_calls: boolean | null;
  reasoning_effort: string | null;
  prompt_cache: string | null;
}

/** Mirrors `zk_contract_config::MemorySettings`. */
export interface MemorySettings {
  enabled: boolean;
  dir: string;
}

/** Mirrors `zk_contract_config::HookSettings`. */
export interface HookSettings {
  enabled: boolean;
  log: string;
}

/** Normalized `zk_contract_config::AgentSettings`. */
export interface ScheduleSettings {
  enabled: boolean;
  scan_seconds: number;
}

export interface AgentSettings {
  max_steps: number;
  completion: CompletionSettings;
  memory: MemorySettings;
  hooks: HookSettings;
  schedule: ScheduleSettings;
}

/** Mirrors `zk_contract_agent::AgentUsage`. */
export interface AgentUsage {
  input_tokens: number;
  output_tokens: number;
  total_tokens: number;
  cached_tokens: number;
  reasoning_tokens: number;
  /** Provider-reported cost in micro-USD; absent/`null` when the provider does not report cost. */
  cost_micros?: number | null;
}

/** Mirrors `zk_contract_config::MediaSettings` (image/audio/transcription/rerank). */
export interface MediaSettings {
  base_url: string;
  api_key: string;
  image_model: string;
  audio_model: string;
  transcription_model: string;
  rerank_model: string;
}

/** Mirrors `zk_contract_config::EmbeddingSettings` (vector index provider). */
export interface EmbeddingSettings {
  base_url: string;
  api_key: string;
  model: string;
  dims: number | null;
  dir: string;
}

/** Mirrors `zk_contract_connector::McpServerConfig` (non-secret local MCP config). */
export type McpTransport = "stdio" | "http" | "sse";

/** Mirrors `zk_contract_connector::McpServerConfig`. */
export interface McpServerConfig {
  id: string;
  name: string;
  transport: McpTransport;
  command: string;
  args: string[];
  env: Record<string, string>;
  url: string;
  timeout_ms: number;
  oauth_required: boolean;
}

/** Mirrors `zk_contract_config::McpSettings` (personal local MCP servers). */
export interface McpSettings {
  servers: McpServerConfig[];
}

/** Mirrors `zk_contract_ipc::results::McpTestState` (closed enum). */
export type McpTestState = "pending" | "ok" | "error";

/** Mirrors `zk_contract_ipc::results::McpTestStatusResult`. */
export interface McpTestStatusResult {
  state: McpTestState;
  tools: string[];
  error: string | null;
}

/** One `mcp.test` + polling round trip (app-owned; panes never call IPC directly). */
export type McpTestProbe = (server: McpServerConfig) => Promise<McpTestStatusResult>;

/** Mirrors `zk_contract_config::ApprovalPolicy` (config spec C-1). */
export type ApprovalPolicy = "ask" | "auto_read_only" | "strict";

/** Mirrors `zk_contract_config::ToolSettings` (timeouts in ms). */
export interface ToolSettings {
  tool_timeout_ms: number;
  approval_timeout_ms: number;
  /** Explicit local approval policy; `null` = default `ask`. */
  approval_policy: ApprovalPolicy | null;
}

/** Mirrors `zk_contract_config::WebSearchSettings` (keyless search endpoint). */
export interface WebSearchSettings {
  /** Endpoint base URL (https only); empty = env `ZEOKEEL_SEARCH_URL` > default. */
  base_url: string;
  /** Request timeout in milliseconds (1000..=120000, default 15000). */
  timeout_ms: number;
}

/** Mirrors `zk_contract_config::AppSettings`. */
export interface AppSettings {
  schema: number;
  provider: ProviderConfig;
  theme: Theme;
  locale: Locale;
  auto_approve_read_only: boolean;
  /** Enterprise login server URL (personal edition: empty). */
  server_url: string;
  /** Runtime-configurable local agent settings (serde default for old stores). */
  agent: AgentSettings;
  /** Tool/approval timeout settings (serde default for old stores). */
  tools: ToolSettings;
  /** Embedding/vector-index settings (serde default for old stores). */
  embedding: EmbeddingSettings;
  /** Media capability settings (serde default for old stores). */
  media: MediaSettings;
  /** Web-search endpoint settings (serde default for old stores). */
  web_search: WebSearchSettings;
  /** Local MCP servers (personal edition; serde default for old stores). */
  mcp: McpSettings;
}

/** Mirrors `zk_contract_session::Workspace`. */
export interface Workspace {
  id: string;
  path: string;
  title: string;
  is_default: boolean;
  created_at_ms: number;
  updated_at_ms: number;
}

/** Mirrors `zk_contract_session::Conversation`. */
export interface Conversation {
  id: string;
  workspace_id: string;
  title: string;
  created_at_ms: number;
  updated_at_ms: number;
  /** Pinned to the top of its workspace list (Phase 2b, SQLite v6). */
  pinned?: boolean;
  /** Archived: hidden from the default list, messages kept (Phase 2b). */
  archived?: boolean;
}

/** One content-search hit rendered in the sessions sidebar (Phase 2b R6). */
export interface SessionSearchHit {
  conversationId: string;
  title: string;
  messageId: string;
  role: string;
  snippet: string;
  archived: boolean;
}

/** Content-search result (current workspace only; C18). */
export interface SessionSearchOutcome {
  items: SessionSearchHit[];
  truncated: boolean;
}

/** One preview of `workspace.file_read` (inspector files tab). */
export interface FilePreview {
  path: string;
  state: "loading" | "ready" | "error";
  kind: "text" | "image" | "unsupported";
  text: string;
  dataUrl: string;
  size: number;
  truncated: boolean;
  reason: string;
  error: string;
}

/** Mirrors `zk_contract_session::Message`. */
export interface Message {
  id: string;
  conversation_id: string;
  role: string;
  content: string;
  created_at_ms: number;
  /** `done` | `cancelled` | `error` (M1 status persistence). */
  status?: string;
  /** Persisted provider usage (round-4); absent/null for legacy rows or turns without usage. */
  usage?: AgentUsage | null;
  /** Owning agent turn (tool rows and the closing assistant row share it; legacy rows empty). */
  turn_id?: string;
}

/** Payload subset used by the shell header. */
export interface Meta {
  edition: string;
  version: string;
  packages: string[];
  /** Boot-time agent settings error reported by Rust; invalid values fall back to defaults. */
  agentConfigError?: string | null;
}

/** Terminal status of one persisted tool row. */
export type ToolCallStatus = "success" | "failed" | "denied" | "cancelled";

/** One tool call as rendered inside an assistant message. */
export interface ToolCallView {
  id: string;
  name: string;
  args: string;
  result: string | null;
  /** `pending|running` are streaming-only; persisted rows use the terminal set. */
  status: "pending" | "running" | ToolCallStatus;
  /** True when a persisted tool row could not be parsed (raw text fallback). */
  unparsed?: boolean;
}

/** Typed user decision for one approval request (canonical wire values). */
export type ApprovalDecision = "allow_once" | "allow_session" | "reject" | "cancel";

/** Closed risk level declared by the tool for one approval card. */
export type ApprovalRisk = "low" | "medium" | "high";

/** Tool-declared impact surface for an approval card (tool-side only). */
export interface ApprovalMetadata {
  summary?: string;
  cwd?: string;
  read_paths?: string[];
  write_paths?: string[];
  network_targets?: string[];
  network_undetermined?: boolean;
  risk?: ApprovalRisk;
  diff?: string | null;
}

/** One approval request attached to an assistant message. */
export interface ApprovalView {
  id: string;
  tool: string;
  args: string;
  reason: string;
  /** Always-allow auto pair (`approval_requested.auto`): never waits for input. */
  auto?: boolean;
  /** Absolute deadline (ms since epoch); `0`/absent for legacy events. */
  deadline_ms?: number;
  metadata?: ApprovalMetadata | null;
  decision:
    | "pending"
    | "allow_once"
    | "allow_session"
    | "reject"
    | "cancel"
    | "approved"
    | "rejected";
  /** Fail-closed deadline resolution. */
  timed_out?: boolean;
}

export type MessageStatus = "done" | "streaming" | "cancelled" | "error";

/** One user-question batch attached to an assistant message. */
export interface QuestionView {
  id: string;
  questions: unknown;
  answers: unknown;
}

/** A message as rendered by the chat view (assistant messages may still be streaming). */
export interface ChatMessage {
  id: string;
  role: "user" | "assistant";
  content: string;
  created_at_ms: number;
  status: MessageStatus;
  toolCalls: ToolCallView[];
  approvals: ApprovalView[];
  questions: QuestionView[];
  error: string | null;
  /** Provider token usage for a completed assistant turn, when reported. */
  usage?: AgentUsage | null;
  /** Owning agent turn, used to group persisted tool rows. */
  turnId?: string;
  /** True for the synthetic group of tool rows with no owning assistant. */
  degraded?: boolean;
}

/** Mirrors `zk_contract_agent::AgentEvent` (`serde(tag = "type", rename_all = "snake_case")`). */
export type AgentEvent =
  | { type: "turn_started"; turn_id: string }
  | { type: "text_delta"; delta: string }
  | { type: "tool_call_started"; name: string; args: string }
  | {
      type: "tool_call_finished";
      name: string;
      result: string;
      args?: string;
      status?: ToolCallStatus;
    }
  | {
      type: "approval_requested";
      approval_id: string;
      tool: string;
      args: string;
      reason: string;
      auto?: boolean;
      deadline_ms?: number;
      metadata?: ApprovalMetadata | null;
    }
  | {
      type: "approval_resolved";
      approval_id: string;
      decision: "allow_once" | "allow_session" | "reject" | "cancel";
      timed_out?: boolean;
    }
  | {
      type: "question_asked";
      question_id: string;
      questions: unknown;
    }
  | {
      type: "question_answered";
      question_id: string;
      answers: unknown;
    }
  | { type: "usage"; usage: AgentUsage }
  | {
      type: "turn_completed";
      reply: { text: string; tool_calls: ReplyToolCall[]; usage: AgentUsage | null };
    }
  | { type: "turn_cancelled" }
  | { type: "error"; message: string };

export interface ReplyToolCall {
  name: string;
  args: string;
  result: string;
}

/** Mirrors `zk_contract_ipc::UiEvent`. */
export interface UiEvent {
  stream_id: string;
  seq: number;
  kind: string;
  payload: unknown;
}

/** Mirrors `zk_contract_ipc::CommandResponse`. */
export interface CommandResult {
  ok: boolean;
  data: unknown;
  error: string | null;
}

export function defaultAgentSettings(): AgentSettings {
  return {
    max_steps: 50,
    completion: {
      temperature: null,
      max_tokens: null,
      top_p: null,
      seed: null,
      stop: [],
      tool_choice: { mode: "auto" },
      parallel_tool_calls: null,
      reasoning_effort: null,
      prompt_cache: null,
    },
    memory: { enabled: true, dir: "" },
    hooks: { enabled: true, log: "" },
    schedule: { enabled: false, scan_seconds: 10 },
  };
}

export function defaultMediaSettings(): MediaSettings {
  return {
    base_url: "",
    api_key: "",
    image_model: "",
    audio_model: "",
    transcription_model: "",
    rerank_model: "",
  };
}

export function defaultEmbeddingSettings(): EmbeddingSettings {
  return { base_url: "", api_key: "", model: "", dims: null, dir: "" };
}

export function defaultToolSettings(): ToolSettings {
  return { tool_timeout_ms: 120000, approval_timeout_ms: 300000, approval_policy: null };
}

export function defaultWebSearchSettings(): WebSearchSettings {
  return { base_url: "", timeout_ms: 15000 };
}

export function defaultSettings(): AppSettings {
  return {
    schema: 1,
    provider: { kind: "mock", base_url: "", model: "mock", api_key: "" },
    theme: "auto",
    locale: "zh-CN",
    auto_approve_read_only: true,
    server_url: "",
    agent: defaultAgentSettings(),
    tools: defaultToolSettings(),
    embedding: defaultEmbeddingSettings(),
    media: defaultMediaSettings(),
    web_search: defaultWebSearchSettings(),
    mcp: { servers: [] },
  };
}

// ---------------------------------------------------------------------------
// Settings panes (`ui.settings.pane` list slot; docs/16 §2.4)
//
// The shell owns the dialog draft (`SettingsForm`) and the single bottom save;
// pane packages only render fields from `draft` through `update`. Pane packages
// never branch on `edition` — the editions filter happens at compose time (C2).
// ---------------------------------------------------------------------------

/** Tool-choice form mode (maps to `ToolChoiceSetting`, `specific` uses the tool list). */
export type ToolChoiceMode = "auto" | "none" | "required" | "specific";

/** tri-state `parallel_tool_calls` form mode (`default` = provider default). */
export type ParallelMode = "default" | "true" | "false";

/** `reasoning_effort` form value (empty string = provider default). */
export type ReasoningEffort = "" | "minimal" | "low" | "medium" | "high" | "xhigh" | "max";

/** `prompt_cache` form value (empty string = provider default). */
export type PromptCache = "" | "short" | "long";

/**
 * Settings dialog draft: the whole `AppSettings` document flattened into strings
 * so "provider default" is representable as an empty input. One draft belongs to
 * one settings-dialog session and survives pane switches; the shell merges it
 * back into a complete `AppSettings` on the single bottom save.
 */
export interface SettingsForm {
  providerKind: ProviderKind;
  baseUrl: string;
  model: string;
  apiKey: string;
  theme: Theme;
  locale: Locale;
  autoApproveReadOnly: boolean;
  serverUrl: string;
  // Agent config (strings so the form can represent "provider default" as empty).
  maxSteps: string;
  temperature: string;
  maxTokens: string;
  topP: string;
  seed: string;
  stop: string;
  toolChoiceMode: ToolChoiceMode;
  toolChoiceTools: string;
  parallelToolCalls: ParallelMode;
  reasoningEffort: ReasoningEffort;
  promptCache: PromptCache;
  memoryEnabled: boolean;
  memoryDir: string;
  hooksEnabled: boolean;
  hooksLog: string;
  scheduleEnabled: boolean;
  scanSeconds: string;
  // Tool/approval timeouts (ms, strings for empty/invalid UI states).
  toolTimeoutMs: string;
  approvalTimeoutMs: string;
  // Local approval policy (config spec: ask / auto_read_only / strict).
  approvalPolicy: ApprovalPolicy;
  // Local MCP servers (personal-only pane; empty list = none configured).
  mcpServers: McpServerConfig[];
  // Embedding / vector index (empty dims means "provider default").
  embeddingBaseUrl: string;
  embeddingApiKey: string;
  embeddingModel: string;
  embeddingDims: string;
  embeddingDir: string;
  // Media capabilities (personal only; empty fields fall back to env).
  mediaBaseUrl: string;
  mediaApiKey: string;
  mediaImageModel: string;
  mediaAudioModel: string;
  mediaTranscriptionModel: string;
  mediaRerankModel: string;
  // Web-search tool endpoint (empty base URL falls back to env/default).
  webSearchBaseUrl: string;
  webSearchTimeoutMs: string;
}

/** One inline field error: an i18n key (the shell translates it), or `null`. */
export type SettingsFieldError = string | null;

/** Per-field validation errors for `SettingsForm` (i18n keys). */
export interface SettingsErrors {
  baseUrl: SettingsFieldError;
  model: SettingsFieldError;
  maxSteps: SettingsFieldError;
  temperature: SettingsFieldError;
  maxTokens: SettingsFieldError;
  topP: SettingsFieldError;
  seed: SettingsFieldError;
  toolChoiceTools: SettingsFieldError;
  scanSeconds: SettingsFieldError;
  toolTimeoutMs: SettingsFieldError;
  approvalTimeoutMs: SettingsFieldError;
  embeddingBaseUrl: SettingsFieldError;
  embeddingDims: SettingsFieldError;
  mediaBaseUrl: SettingsFieldError;
  webSearchBaseUrl: SettingsFieldError;
  webSearchTimeoutMs: SettingsFieldError;
  mcpServers: SettingsFieldError;
}

/** Props every `ui.settings.pane` contribution receives from the settings shell. */
export interface SettingsPaneProps {
  /** App translator (keys live in the app dictionary; pane copy is not hardcoded). */
  t: (key: string, vars?: Record<string, string | number>) => string;
  /** Current dialog draft (window-level; switching panes does not reset it). */
  draft: SettingsForm;
  /** Update one draft field (immutable, shell-owned state). */
  update: <Key extends keyof SettingsForm>(key: Key, value: SettingsForm[Key]) => void;
  /** Current validation errors (i18n keys). */
  errors: SettingsErrors;
  /** True while the single bottom save is in flight. */
  saving: boolean;
  /**
   * App-level (not edition-level) facts/actions a pane may need.
   *
   * Which panes exist is decided at build time by `editions` (C2); a pane never
   * branches on the edition. These fields are channel/session facts (locked
   * address, enterprise session, gateway catalog, update check) and may be
   * empty when the app does not provide them.
   */
  app: SettingsPaneAppProps;
}

/** App-level facts/actions injected into every settings pane (see `SettingsPaneProps`). */
export interface SettingsPaneAppProps {
  /** Channel-locked login address (C14/ADR-042): read-only when true. */
  serverUrlLocked: boolean;
  /** Enterprise session logout (absent when there is no login session). */
  onLogout?: (() => void) | undefined;
  /** Enterprise gateway model catalog (empty for personal compositions). */
  gatewayModels: string[];
  /** Currently selected gateway model. */
  gatewayModel: string;
  /** Gateway model selection (absent when the catalog is empty). */
  onSelectGatewayModel?: ((model: string) => void) | undefined;
  /** Update check action (absent when the edition/channel has no updater). */
  onCheckUpdate?: (() => void) | undefined;
  /** Latest update status text (already localized by the app). */
  updateText: string | null;
  /** True while the app is checking/downloading an update. */
  updateBusy: boolean;
  /** No-cache MCP probe (`mcp.test` + `mcp.test.status` polling); absent for panes that do not need it. */
  testMcpServer?: McpTestProbe | undefined;
}

/** Outcome of one settings save attempt (the app owns revision + i18n mapping). */
export type SettingsSaveOutcome =
  | { status: "saved"; settings: AppSettings; revision: number }
  | { status: "conflict" }
  | { status: "error"; message: string };

/** One ordered `ui.settings.pane` contribution emitted into `generated/registry.ts`. */
export interface SettingsPaneContribution {
  /** Stable pane id (last `component` segment; used for nav keys/labels). */
  id: string;
  /** Display order (ascending; ties broken by package id at compose time). */
  order: number;
  /** Pane component; renders fields for the current draft. */
  Component: ComponentType<SettingsPaneProps>;
}

// ---------------------------------------------------------------------------
// Composer input surface (`ui-chat`; `2026-11-01-client-composer-surface`)
// ---------------------------------------------------------------------------

/**
 * One app command descriptor.
 *
 * The app owns exactly one command catalog; the ⌘K palette and the composer `/`
 * menu both render this array and both execute through the same `runCommand`
 * action (C17: no second command list, no second executor).
 */
export interface CommandDescriptor {
  /** Stable command id (e.g. `new-conversation`, `cancel`, `retry`). */
  id: string;
  /** Localized title shown in both surfaces. */
  title: string;
  /** Localized group label (palette column / composer menu tag). */
  group: string;
  /** Optional localized hint (shortcut/summary). */
  hint?: string;
  /** Visible but not executable (e.g. `/cancel` outside a stream). */
  disabled?: boolean;
  /** Surface filter: composer `/` menu vs ⌘K palette (shared catalog). */
  surface: "slash" | "palette" | "both";
  /** Slash command kind (`action`/`popupSelect`/`leadingInput`/`execute`). */
  kind?: "action" | "popupSelect" | "leadingInput" | "execute" | "disabled";
  /** Visible reason for a disabled row (Batch 2 placeholders). */
  disabledReason?: string;
  /** Slash command name without the `/` (e.g. `model`). */
  name?: string;
}

/** One workspace file/dir candidate for `@` references (from `workspace.files`). */
export interface WorkspaceFileRef {
  /** Workspace-relative path with forward slashes. */
  path: string;
  /** `dir` entries are referenced as `@path/`. */
  kind: "file" | "dir";
}

/**
 * One composer draft chip: an `@` workspace reference or a saved attachment.
 *
 * Draft state only — never serialized as a struct on the wire: on send the shell
 * merges the chips into the user message's trailing reference block (plain
 * text), so `chat.send` stays unchanged.
 */
export interface DraftRef {
  /** Client-only draft key (retry/removal); never sent. */
  id?: string;
  /** `workspace` = `@` file/dir reference; `attachment` = host-saved workspace file. */
  kind: "workspace" | "attachment";
  /** Workspace-relative path (directory references end with `/`; attachments only when ready). */
  path: string;
  /** Display name (basename for attachments, last path segment for references). */
  name?: string;
  /** Attachment size in bytes (attachments only). */
  size?: number;
  /** Canonical MIME reported by the host (attachments only). */
  mime?: string;
  /** Image thumbnail as a `data:` URL from `FileReader.readAsDataURL` (never `blob:`). */
  thumbnail?: string;
  /** Upload state for attachments (`workspace` chips are always ready). */
  status?: "uploading" | "ready" | "error";
  /** i18n error key when `status === "error"`. */
  error?: string;
}

/** Mirrors `zk_contract_ipc::results::AttachmentSaveResult` (host-canonical metadata). */
export interface AttachmentSaveResult {
  /** Workspace-relative path written by the host (`.zeokeel/attachments/...`). */
  path: string;
  /** Sanitized display name (basename). */
  name: string;
  /** Decoded byte length. */
  bytes: number;
  /** Canonical MIME. */
  mime: string;
}

/** Conversation goal view (mirrors `zk_contract_ipc::results::GoalView`). */
export interface GoalView {
  objective: string;
  status: string;
  rounds: number;
  max_rounds: number;
  reason?: string | null;
  last_result?: string | null;
  created_ms?: number;
  updated_ms?: number;
}

/** Conversation-scoped model/permission override (Phase 2b command surface). */
export interface ConversationOverride {
  model: string | null;
  reasoning_effort: string | null;
  approval_policy: string | null;
  is_session_override: boolean;
}

/** `session.export` result. */
export interface ConversationExport {
  path: string;
  bytes: number;
  mime: string;
  truncated: boolean;
}

/** Local-only feedback category. */
export type FeedbackCategory = "bug" | "idea" | "other";

/** Persisted conversation compaction marker (`role=system`, `compaction:<uuid>`). */
export interface CompactionMarker {
  markerId: string;
  summary: string;
  throughMessageId: string;
  createdMs: number;
  inputTokens: number;
  outputTokens: number;
  compressedMessages: number;
  keepRecent: number;
}
