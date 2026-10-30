import type { ComponentType } from "react";

/** Placeholder; the real component is filled by the UI workstream. */
export const ChatView = (() => null) as ComponentType<Record<string, never>>;

export const components = {
  "chat.thread": ChatView,
} as const;
