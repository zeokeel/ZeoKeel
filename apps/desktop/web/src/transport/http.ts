/**
 * HTTP `serve` carrier transport: `POST /ipc` + SSE `GET /events`.
 * Same command/event shapes as Tauri (transport-agnostic IPC contract).
 */
import { parseCommandResponse, parseUiEvent } from "../protocol";
import type { Transport, UiEvent } from "../types";

const IPC_PATH = "/ipc";
const EVENTS_PATH = "/events";

export function createHttpTransport(): Transport {
  return {
    async call(name, payload) {
      const response = await fetch(IPC_PATH, {
        method: "POST",
        credentials: "same-origin",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ name, payload }),
      });
      if (!response.ok) {
        throw new Error(`IPC request failed with HTTP ${String(response.status)}`);
      }
      const raw: unknown = await response.json();
      const parsed = parseCommandResponse(raw);
      if (!parsed.ok) {
        throw new Error(parsed.error ?? `${name} failed`);
      }
      return parsed.data;
    },

    subscribe(handler: (event: UiEvent) => void) {
      if (typeof EventSource === "undefined") {
        console.warn("[zk] EventSource unavailable; streaming events disabled");
        return () => undefined;
      }
      const source = new EventSource(EVENTS_PATH);
      source.onmessage = (event: MessageEvent) => {
        const raw: unknown = event.data;
        if (typeof raw !== "string") {
          return;
        }
        let decoded: unknown;
        try {
          decoded = JSON.parse(raw);
        } catch {
          console.warn("[zk] ignored malformed SSE payload");
          return;
        }
        const parsed = parseUiEvent(decoded);
        if (parsed === null) {
          console.warn("[zk] ignored malformed UiEvent from SSE");
          return;
        }
        handler(parsed);
      };
      source.onerror = () => {
        // EventSource reconnects automatically; keep this log low-noise.
        console.warn("[zk] SSE connection lost; reconnecting");
      };
      return () => {
        source.close();
      };
    },
  };
}
