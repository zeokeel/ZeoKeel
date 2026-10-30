/** Mock view models for the UX skeleton only (no backend). */
export interface WorkspaceView {
  id: string;
  name: string;
  detail: string;
  isDefault: boolean;
  sessions: SessionView[];
}
export interface SessionView {
  id: string;
  title: string;
  time: string;
  status: "idle" | "running" | "unread" | "pending" | "error";
}
export interface ToolView {
  id: string;
  name: string;
  detail: string;
  time: string;
  status: "running" | "done" | "error";
  args: string;
  result: string;
}
export interface PlanStep {
  id: string;
  label: string;
  status: "done" | "running" | "pending";
}
export interface ApprovalView {
  id: string;
  tool: string;
  args: string;
  reason: string;
  decision: "pending" | "approved" | "rejected";
}
export interface MessageView {
  id: string;
  role: "user" | "assistant";
  text: string;
  status: "done" | "streaming" | "cancelled" | "error";
  tools: ToolView[];
  plan?: PlanStep[];
  approval?: ApprovalView;
}
export interface FileNode { path: string; depth: number; kind: "dir" | "file"; badge?: "M" | "A" }
export interface ChangeView { path: string; add: number; del: number }
