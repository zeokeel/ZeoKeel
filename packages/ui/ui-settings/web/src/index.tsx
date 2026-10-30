import type { ComponentType } from "react";

/** Placeholder; the real component is filled by the UI workstream. */
export const SettingsDialog = (() => null) as ComponentType<Record<string, never>>;

export const components = {
  "settings.dialog": SettingsDialog,
} as const;
