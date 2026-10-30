/**
 * App-side type surface. Domain DTOs live in `@zeokeel/ui-kit` (shared with UI
 * packages); transport shapes stay here because they are app infrastructure.
 */
export type * from "@zeokeel/ui-kit";
export { defaultSettings } from "@zeokeel/ui-kit";

import type { UiEvent } from "@zeokeel/ui-kit";

/** The only seam the UI knows about; implemented by tauri/http/mock transports. */
export interface Transport {
  call(name: string, payload: Record<string, unknown>): Promise<unknown>;
  subscribe(handler: (event: UiEvent) => void): () => void;
}

/**
 * Minimal shape of the Tauri 2 global injected by `withGlobalTauri`.
 * Exactly the two entry points the shell uses; nothing else is trusted.
 */
export interface TauriGlobal {
  core: {
    invoke(command: string, args?: Record<string, unknown>): Promise<unknown>;
  };
  event: {
    listen(
      event: string,
      handler: (event: { payload: unknown }) => void,
    ): Promise<() => void>;
  };
}

declare global {
  interface Window {
    __TAURI__?: TauriGlobal;
  }
}
