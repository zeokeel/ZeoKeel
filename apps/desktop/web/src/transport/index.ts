/**
 * Transport selection: explicit `?mock=1` demo backend, Tauri IPC when running
 * inside the desktop shell, otherwise the HTTP `serve` carrier.
 */
import type { Transport } from "../types";
import { createHttpTransport } from "./http";
import { createMockTransport } from "./mock";
import { createTauriTransport } from "./tauri";

export function isTauriAvailable(): boolean {
  const api = window.__TAURI__;
  if (api === undefined) {
    return false;
  }
  return typeof api.core.invoke === "function" && typeof api.event.listen === "function";
}

export function createTransport(): Transport {
  const params = new URLSearchParams(window.location.search);
  if (params.get("mock") === "1") {
    return createMockTransport({
      startEmptyWorkspaces: params.get("no-workspace") === "1",
      seed: params.get("seed") === "1",
    });
  }
  if (isTauriAvailable()) {
    return createTauriTransport();
  }
  return createHttpTransport();
}
