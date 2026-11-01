/**
 * App-level MCP test-connection: `mcp.test` + poll `mcp.test.status` (no-cache probe).
 *
 * The pane never calls IPC directly; it receives this as `app.testMcpServer`, so
 * the probe policy (timeout/polling/error mapping) has a single implementation.
 */
import type { McpServerConfig, McpTestStatusResult } from "@zeokeel/ui-kit";
import { isRecord } from "./protocol";
import type { Transport } from "./types";

const POLL_INTERVAL_MS = 100;
const MAX_POLLS = 160;

function readString(value: Record<string, unknown>, key: string): string | null {
  const raw = value[key];
  return typeof raw === "string" ? raw : null;
}

function delay(ms: number): Promise<void> {
  return new Promise((resolve) => {
    window.setTimeout(resolve, ms);
  });
}

export async function probeMcpServer(
  transport: Transport,
  server: McpServerConfig,
): Promise<McpTestStatusResult> {
  const started = await transport.call("mcp.test", { server });
  const testId = isRecord(started) ? readString(started, "test_id") : null;
  if (testId === null) {
    return { state: "error", tools: [], error: "mcp/unsupported: missing test_id" };
  }
  for (let attempt = 0; attempt < MAX_POLLS; attempt += 1) {
    const raw = await transport.call("mcp.test.status", { test_id: testId });
    const state = isRecord(raw) ? readString(raw, "state") : null;
    const tools = isRecord(raw) && Array.isArray(raw["tools"])
      ? raw["tools"].flatMap((tool) => (typeof tool === "string" ? [tool] : []))
      : [];
    const error = isRecord(raw) ? readString(raw, "error") : null;
    if (state === "ok") return { state: "ok", tools, error: null };
    if (state === "error") return { state: "error", tools, error: error ?? "mcp/connect-failed" };
    await delay(POLL_INTERVAL_MS);
  }
  return { state: "error", tools: [], error: "mcp/timeout" };
}
