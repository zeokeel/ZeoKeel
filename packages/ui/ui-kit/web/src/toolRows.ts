/**
 * Parse persisted `role="tool"` rows and group them onto assistant messages by
 * `turn_id` (design §2.5.4). Pure helpers: the app reducer owns state changes.
 */
import type { Message, ToolCallStatus, ToolCallView } from "./types";

function isTerminalStatus(value: string): value is ToolCallStatus {
  return (
    value === "success" || value === "failed" || value === "denied" || value === "cancelled"
  );
}

function parseStatus(raw: unknown): ToolCallStatus {
  if (typeof raw === "string" && isTerminalStatus(raw)) {
    return raw;
  }
  // Legacy rows had no status; they were successful tool rows per the old format.
  return "success";
}

function parseToolRow(message: Message): ToolCallView {
  const base: ToolCallView = {
    id: message.id,
    name: "",
    args: "",
    result: null,
    status: "success",
  };
  let parsed: unknown;
  try {
    parsed = JSON.parse(message.content);
  } catch {
    return { ...base, name: "tool", result: message.content, status: "failed", unparsed: true };
  }
  if (!isRecord(parsed)) {
    return { ...base, name: "tool", result: message.content, status: "failed", unparsed: true };
  }
  const name = typeof parsed["name"] === "string" ? parsed["name"] : "tool";
  const args = typeof parsed["args"] === "string" ? parsed["args"] : "";
  const result = typeof parsed["result"] === "string" ? parsed["result"] : "";
  return {
    id: message.id,
    name,
    args,
    result,
    status: parseStatus(parsed["status"]),
  };
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

/** Grouping result consumed by the app reducer. */
export interface ToolGrouping {
  /** Non-tool messages in original order. */
  visible: Message[];
  /** Tool rows keyed by the assistant message they belong to. */
  toolsByAssistant: Map<string, ToolCallView[]>;
  /** Tool rows with no owning assistant (degraded group). */
  unassigned: ToolCallView[];
}

/**
 * Group persisted tool rows by turn:
 * - rows with a `turn_id` attach to the assistant row of the same turn;
 * - legacy rows (empty `turn_id`) attach to the next assistant row in the same
 *   conversation (real persistence order is `user → tool* → assistant`);
 * - a trailing legacy run, or one followed by a non-assistant row, is unassigned.
 */
export function groupToolRows(messages: Message[]): ToolGrouping {
  const visible: Message[] = [];
  const toolsByAssistant = new Map<string, ToolCallView[]>();
  const unassigned: ToolCallView[] = [];
  const byTurn = new Map<string, ToolCallView[]>();
  for (const message of messages) {
    if (message.role !== "tool") {
      continue;
    }
    const turnId = message.turn_id ?? "";
    if (turnId !== "") {
      const list = byTurn.get(turnId) ?? [];
      list.push(parseToolRow(message));
      byTurn.set(turnId, list);
    }
  }
  let legacyBuffer: ToolCallView[] = [];
  const flushLegacy = (targetAssistant: Message | null): void => {
    if (legacyBuffer.length === 0) {
      return;
    }
    if (targetAssistant === null) {
      unassigned.push(...legacyBuffer);
    } else {
      const list = toolsByAssistant.get(targetAssistant.id) ?? [];
      list.push(...legacyBuffer);
      toolsByAssistant.set(targetAssistant.id, list);
    }
    legacyBuffer = [];
  };
  for (const message of messages) {
    if (message.role === "tool") {
      const turnId = message.turn_id ?? "";
      if (turnId === "") {
        legacyBuffer.push(parseToolRow(message));
      }
      continue;
    }
    flushLegacy(message.role === "assistant" ? message : null);
    visible.push(message);
    if (message.role === "assistant") {
      const turnTools = byTurn.get(message.turn_id ?? "");
      if (turnTools !== undefined) {
        const existing = toolsByAssistant.get(message.id) ?? [];
        toolsByAssistant.set(message.id, [...turnTools, ...existing]);
      }
    }
  }
  flushLegacy(null);
  return { visible, toolsByAssistant, unassigned };
}
