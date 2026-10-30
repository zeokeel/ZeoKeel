import type { ComponentType } from "react";

/** Placeholder; the real component is filled by the UI workstream. */
export const Sidebar = (() => null) as ComponentType<Record<string, never>>;

export const components = {
  "sessions.sidebar": Sidebar,
} as const;
