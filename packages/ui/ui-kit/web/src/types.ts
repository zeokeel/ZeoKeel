/**
 * Shared UI DTOs and view types (contract mirrors + chat/stream view state).
 * No app imports: ui-kit only depends on React.
 */

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
export interface AgentSettings {
  max_steps: number;
  completion: CompletionSettings;
  memory: MemorySettings;
  hooks: HookSettings;
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

/** Mirrors `zk_contract_config::ToolSettings` (timeouts in ms). */
export interface ToolSettings {
  tool_timeout_ms: number;
  approval_timeout_ms: number;
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
}

/** Payload subset used by the shell header. */
export interface Meta {
  edition: string;
  version: string;
  packages: string[];
  /** Boot-time agent settings error reported by Rust; invalid values fall back to defaults. */
  agentConfigError?: string | null;
}

/** One tool call as rendered inside an assistant message. */
export interface ToolCallView {
  id: string;
  name: string;
  args: string;
  result: string | null;
  status: "running" | "done" | "error";
}

/** One approval request attached to an assistant message (M1 placeholder). */
export interface ApprovalView {
  id: string;
  tool: string;
  args: string;
  reason: string;
  decision: "pending" | "approved" | "rejected";
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
}

/** Mirrors `zk_contract_agent::AgentEvent` (`serde(tag = "type", rename_all = "snake_case")`). */
export type AgentEvent =
  | { type: "turn_started"; turn_id: string }
  | { type: "text_delta"; delta: string }
  | { type: "tool_call_started"; name: string; args: string }
  | { type: "tool_call_finished"; name: string; result: string }
  | {
      type: "approval_requested";
      approval_id: string;
      tool: string;
      args: string;
      reason: string;
    }
  | {
      type: "approval_resolved";
      approval_id: string;
      decision: "allow_once" | "reject" | "cancel";
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
  };
}

export function defaultToolSettings(): ToolSettings {
  return { tool_timeout_ms: 120000, approval_timeout_ms: 300000 };
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
  };
}

/** Controlled props for the personal-only provider settings package. */
export interface ProviderSettingsProps {
  t: (key: string, vars?: Record<string, string | number>) => string;
  providerKind: ProviderKind;
  baseUrl: string;
  model: string;
  apiKey: string;
  errors: { baseUrl: string | null; model: string | null };
  isMock: boolean;
  onProviderKind: (kind: ProviderKind) => void;
  onBaseUrl: (value: string) => void;
  onModel: (value: string) => void;
  onApiKey: (value: string) => void;
}
