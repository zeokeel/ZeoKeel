import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { App } from "../src/App";
import { parseWorkspaces } from "../src/protocol";
import { appReducer, createInitialState } from "../src/state/reducer";
import { defaultSettings } from "../src/types";
import { FakeTransport } from "./fake-transport";
import { CONVERSATION, META, WORKSPACE, WORKSPACES } from "./fixtures";

const SECOND_WORKSPACE = {
  ...WORKSPACE,
  id: "ws-project",
  path: "/home/test/project",
  title: "project",
  is_default: false,
  created_at_ms: 2,
  updated_at_ms: 2,
};

describe("workspace protocol", () => {
  it("parses workspace records and rejects malformed ones", () => {
    expect(parseWorkspaces(WORKSPACES)).toEqual(WORKSPACES);
    expect(parseWorkspaces([{ id: "ws" }])).toBeNull();
    expect(parseWorkspaces("nope")).toBeNull();
  });
});

describe("workspace reducer", () => {
  it("hydrates the selected workspace conversations and keeps them across refresh", () => {
    let state = appReducer(createInitialState(), {
      type: "boot_loaded",
      meta: META,
      workspaces: [WORKSPACE, SECOND_WORKSPACE],
      activeWorkspaceId: WORKSPACE.id,
      conversationsByWorkspace: {
        [WORKSPACE.id]: [CONVERSATION],
        [SECOND_WORKSPACE.id]: [],
      },
      conversations: [CONVERSATION],
      settings: defaultSettings(),
      activeId: CONVERSATION.id,
    });
    expect(state.activeWorkspaceId).toBe(WORKSPACE.id);
    expect(state.conversations).toHaveLength(1);

    state = appReducer(state, { type: "workspace_selected", workspaceId: SECOND_WORKSPACE.id });
    expect(state.activeWorkspaceId).toBe(SECOND_WORKSPACE.id);
    expect(state.conversations).toHaveLength(0);
    expect(state.activeConversationId).toBeNull();

    state = appReducer(state, { type: "workspace_selected", workspaceId: WORKSPACE.id });
    expect(state.activeConversationId).toBe(CONVERSATION.id);
    expect(state.messagesState).toBe("loading");

    state = appReducer(state, {
      type: "messages_loaded",
      conversationId: CONVERSATION.id,
      messages: [],
    });
    expect(state.messagesState).toBe("idle");

    // A late session.list refresh must not reset an already loaded thread.
    state = appReducer(state, {
      type: "conversations_loaded",
      workspaceId: WORKSPACE.id,
      conversations: [CONVERSATION],
      activeId: CONVERSATION.id,
    });
    expect(state.messagesState).toBe("idle");
    expect(state.activeConversationId).toBe(CONVERSATION.id);
  });
});

describe("workspace first run", () => {
  it("creates the default ~ workspace without creating a conversation", async () => {
    const transport = new FakeTransport();
    transport.handle("meta.get", () => META);
    transport.handle("workspace.list", () => []);
    transport.handle("settings.get", () => defaultSettings());
    transport.handle("session.list", () => []);
    transport.handle("workspace.create", (payload) => ({
      id: "ws-created",
      path: payload["path"],
      title: payload["title"],
      is_default: false,
      created_at_ms: 1,
      updated_at_ms: 1,
    }));
    transport.handle("workspace.set_default", () => [{ ...WORKSPACE, id: "ws-created", path: "~", title: "主目录" }]);
    transport.handle("settings.set", (payload) => payload["settings"]);
    transport.handle("approval.decide", () => ({}));

    render(<App transport={transport} />);
    expect(await screen.findByRole("heading", { name: "先选择一个工作区" })).toBeTruthy();
    await userEvent.click(screen.getByRole("button", { name: "使用默认 ~" }));

    expect(await screen.findByRole("button", { name: /主目录/ })).toBeTruthy();
    expect(transport.callsFor("workspace.create")[0]).toMatchObject({ path: "~" });
    expect(transport.callsFor("session.create")).toHaveLength(0);
    expect(screen.queryByRole("textbox", { name: "消息输入框" })).toBeNull();
  });
});
