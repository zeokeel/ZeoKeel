/** Tauri IPC transport: single `ipc_call` command + `zk://event` event channel. */
import { parseCommandResponse, parseUiEvent } from "../protocol";
import type { Transport, UiEvent } from "../types";

const EVENT_CHANNEL = "zk://event";

export function createTauriTransport(): Transport {
  return {
    async call(name, payload) {
      const api = window.__TAURI__;
      if (api === undefined) {
        throw new Error("Tauri runtime is not available");
      }
      const raw: unknown = await api.core.invoke("ipc_call", { name, payload });
      const response = parseCommandResponse(raw);
      if (!response.ok) {
        throw new Error(response.error ?? `${name} failed`);
      }
      return response.data;
    },

    subscribe(handler: (event: UiEvent) => void) {
      const api = window.__TAURI__;
      if (api === undefined) {
        return () => {};
      }
      let disposed = false;
      let unlisten: (() => void) | null = null;
      void api.event
        .listen(EVENT_CHANNEL, (event) => {
          const parsed = parseUiEvent(event.payload);
          if (parsed === null) {
            console.warn("[zk] ignored malformed UiEvent from Tauri");
            return;
          }
          handler(parsed);
        })
        .then((stop) => {
          if (disposed) {
            stop();
          } else {
            unlisten = stop;
          }
        })
        .catch((error: unknown) => {
          console.warn("[zk] failed to subscribe to Tauri events", error);
        });
      return () => {
        disposed = true;
        if (unlisten !== null) {
          unlisten();
          unlisten = null;
        }
      };
    },
  };
}
