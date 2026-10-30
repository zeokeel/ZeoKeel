import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { App } from "../src/App";
import { parseMessages } from "../src/protocol";
import { appReducer, createInitialState } from "../src/state/reducer";
import { defaultSettings } from "../src/types";
import { CONVERSATION, META, WORKSPACE, WORKSPACES, readyTransport } from "./fixtures";

const USAGE = {
  input_tokens: 5,
  output_tokens: 2,
  total_tokens: 7,
  cached_tokens: 1,
  reasoning_tokens: 0,
  cost_micros: 10,
};

const HISTORY = {
  id: "history-1",
  conversation_id: CONVERSATION.id,
  role: "assistant",
  content: "历史回复",
  created_at_ms: 5,
  status: "done",
  usage: USAGE,
};

function bootedState() {
  return appReducer(createInitialState(), {
    type: "boot_loaded",
    meta: META,
    workspaces: WORKSPACES,
    activeWorkspaceId: WORKSPACE.id,
    conversationsByWorkspace: { [WORKSPACE.id]: [CONVERSATION] },
    conversations: [CONVERSATION],
    settings: defaultSettings(),
    activeId: CONVERSATION.id,
  });
}

describe("persisted usage restore (round-4 P1-1)", () => {
  it("parses Message.usage from a persisted session message", () => {
    const messages = parseMessages([HISTORY]);
    expect(messages?.[0]?.usage).toEqual(USAGE);
    expect(parseMessages([{ ...HISTORY, usage: undefined }])?.[0]?.usage).toBeNull();
    expect(parseMessages([{ ...HISTORY, usage: null }])?.[0]?.usage).toBeNull();
  });

  it("degrades malformed persisted usage to null instead of dropping the message", () => {
    const warn = vi.spyOn(console, "warn").mockImplementation(() => undefined);
    const messages = parseMessages([{ ...HISTORY, usage: { input_tokens: 5 } }]);
    expect(messages).toHaveLength(1);
    expect(messages?.[0]?.usage).toBeNull();
    expect(warn).toHaveBeenCalled();
    expect(parseMessages([{ ...HISTORY, content: 42 }])).toBeNull();
  });

  it("reducer messages_loaded maps usage onto ChatMessage (absent → null)", () => {
    const state = appReducer(bootedState(), {
      type: "messages_loaded",
      conversationId: CONVERSATION.id,
      messages: parseMessages([HISTORY, { ...HISTORY, id: "history-2", usage: undefined }]) ?? [],
    });
    expect(state.messages[0]?.usage).toEqual(USAGE);
    expect(state.messages[1]?.usage).toBeNull();
  });

  it("restored messages render the usage part after reload", async () => {
    const transport = readyTransport([CONVERSATION]);
    transport.handle("session.messages", () => [HISTORY]);
    render(<App transport={transport} />);
    expect(await screen.findByText("历史回复")).toBeTruthy();
    expect(await screen.findByText("输入 5")).toBeTruthy();
    expect(screen.getByText("输出 2")).toBeTruthy();
    expect(screen.getByText("缓存 1")).toBeTruthy();
    expect(screen.getByText("合计 7")).toBeTruthy();
    expect(screen.getByText("成本 $0.000010")).toBeTruthy();
  });
});
