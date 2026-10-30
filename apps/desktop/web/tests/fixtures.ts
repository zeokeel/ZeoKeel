import { defaultSettings, type Conversation, type Workspace } from "../src/types";
import { FakeTransport } from "./fake-transport";

export const WORKSPACE: Workspace = {
  id: "ws-home",
  path: "/home/test",
  title: "主目录",
  is_default: true,
  created_at_ms: 1,
  updated_at_ms: 1,
};

export const WORKSPACES: Workspace[] = [WORKSPACE];

export const CONVERSATION: Conversation = {
  id: "conv-1",
  workspace_id: WORKSPACE.id,
  title: "演示会话",
  created_at_ms: 1,
  updated_at_ms: 2,
};

export const META = {
  edition: "personal",
  version: "0.1.0-mock",
  packages: ["tool.fs", "llm.mock", "agent-loop"],
};

/** A transport that answers the workspace-scoped IPC in-memory. */
export function readyTransport(
  conversations: Conversation[] = [],
  workspaces: Workspace[] = WORKSPACES,
): FakeTransport {
  const transport = new FakeTransport();
  let created = 0;
  transport.handle("meta.get", () => META);
  transport.handle("workspace.list", () => workspaces);
  transport.handle("session.list", (payload) => {
    const workspaceId = payload["workspace_id"];
    return conversations.filter((conversation) => conversation.workspace_id === workspaceId);
  });
  transport.handle("settings.get", () => defaultSettings());
  transport.handle("session.messages", () => []);
  transport.handle("session.create", (payload) => {
    created += 1;
    const title = payload["title"];
    return {
      id: `conv-created-${created}`,
      workspace_id: payload["workspace_id"],
      title: typeof title === "string" ? title : "新会话",
      created_at_ms: 1,
      updated_at_ms: 1,
    };
  });
  transport.handle("chat.send", () => ({
    stream_id: "stream-1",
    conversation_id: "conv-created-1",
    workspace_id: WORKSPACE.id,
  }));
  transport.handle("chat.cancel", () => ({}));
  transport.handle("settings.set", (payload) => payload["settings"]);
  transport.handle("approval.decide", () => ({}));
  transport.handle("workspace.create", (payload) => ({
    id: "ws-created",
    path: payload["path"],
    title: payload["title"] === "" ? "新工作区" : payload["title"],
    is_default: false,
    created_at_ms: 1,
    updated_at_ms: 1,
  }));
  transport.handle("workspace.rename", (payload) => ({
    ...WORKSPACE,
    id: payload["workspace_id"],
    title: payload["title"],
  }));
  transport.handle("workspace.set_default", () => workspaces);
  transport.handle("workspace.remove", () => ({ removed: true, moved_sessions: 0 }));
  return transport;
}
