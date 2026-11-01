/**
 * Tool-row grouping is implemented once in `@zeokeel/ui-kit` (the app shell may
 * only import ui-kit, docs/27 §3 rule 2); this module re-exports it for ui-chat
 * consumers/tests so the design's `ui-chat/toolRows.ts` entry point stays valid.
 */
export { groupToolRows } from "@zeokeel/ui-kit";
export type { ToolGrouping } from "@zeokeel/ui-kit";
