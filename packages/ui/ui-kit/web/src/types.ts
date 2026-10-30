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

/** Mirrors `zk_contract_config::AppSettings`. */
export interface AppSettings {
  schema: number;
  provider: ProviderConfig;
  theme: Theme;
  locale: Locale;
  auto_approve_read_only: boolean;
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
}

/** Payload subset used by the shell header. */
export interface Meta {
  edition: string;
  version: string;
  packages: string[];
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

/** A message as rendered by the chat view (assistant messages may still be streaming). */
export interface ChatMessage {
  id: string;
  role: "user" | "assistant";
  content: string;
  created_at_ms: number;
  status: MessageStatus;
  toolCalls: ToolCallView[];
  approvals: ApprovalView[];
  error: string | null;
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
      type: "turn_completed";
      reply: { text: string; tool_calls: ReplyToolCall[] };
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

export function defaultSettings(): AppSettings {
  return {
    schema: 1,
    provider: { kind: "mock", base_url: "", model: "mock", api_key: "" },
    theme: "auto",
    locale: "zh-CN",
    auto_approve_read_only: true,
  };
}
