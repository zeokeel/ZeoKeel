import { describe, expect, it } from "vitest";
import { appReducer, createInitialState, type AppState } from "../src/state/reducer";
import { defaultSettings } from "../src/types";
import { CONVERSATION, META } from "./fixtures";

function loadedState(): AppState {
  let state = appReducer(createInitialState(), {
    type: "boot_loaded",
    meta: META,
    conversations: [CONVERSATION],
    settings: defaultSettings(),
    activeId: CONVERSATION.id,
  });
  state = appReducer(state, {
    type: "messages_loaded",
    conversationId: CONVERSATION.id,
    messages: [],
  });
  return state;
}

function startSend(state: AppState, text = "你好"): AppState {
  return appReducer(state, {
    type: "send_started",
    conversationId: CONVERSATION.id,
    text,
    messageId: "message-1",
    assistantId: "assistant-1",
    now: 10,
  });
}

describe("chat reducer", () => {
  it("accumulates Chinese and emoji deltas split across UTF-16 boundaries", () => {
    const wave = "👋";
    let state = startSend(loadedState(), "你好");
    state = appReducer(state, {
      type: "agent_event",
      event: { type: "turn_started", turn_id: "stream-1" },
      now: 11,
    });
    state = appReducer(state, {
      type: "agent_event",
      event: { type: "text_delta", delta: `你${wave.slice(0, 1)}` },
      now: 12,
    });
    state = appReducer(state, {
      type: "agent_event",
      event: { type: "text_delta", delta: `${wave.slice(1)}好🙂` },
      now: 13,
    });
    const assistant = state.messages[1];
    expect(assistant?.content).toBe("你👋好🙂");
    expect(assistant?.status).toBe("streaming");
    expect(state.streaming).toBe(true);
  });

  it("finishes the turn with the authoritative reply text and clears streaming", () => {
    let state = startSend(loadedState());
    state = appReducer(state, {
      type: "agent_event",
      event: { type: "text_delta", delta: "部分" },
      now: 11,
    });
    state = appReducer(state, {
      type: "agent_event",
      event: { type: "turn_completed", reply: { text: "最终回答", tool_calls: [] } },
      now: 12,
    });
    const assistant = state.messages[1];
    expect(assistant?.content).toBe("最终回答");
    expect(assistant?.status).toBe("done");
    expect(state.streaming).toBe(false);
    expect(state.streamId).toBeNull();
  });

  it("keeps partial content and labels a cancelled turn", () => {
    let state = startSend(loadedState());
    state = appReducer(state, {
      type: "agent_event",
      event: { type: "text_delta", delta: "写到一半" },
      now: 11,
    });
    state = appReducer(state, {
      type: "agent_event",
      event: { type: "turn_cancelled" },
      now: 12,
    });
    const assistant = state.messages[1];
    expect(assistant?.content).toBe("写到一半");
    expect(assistant?.status).toBe("cancelled");
    expect(state.streaming).toBe(false);
  });

  it("records a tool card from started/finished events", () => {
    let state = startSend(loadedState());
    state = appReducer(state, {
      type: "agent_event",
      event: { type: "tool_call_started", name: "fs.read", args: '{"path":"a"}' },
      now: 11,
    });
    state = appReducer(state, {
      type: "agent_event",
      event: { type: "tool_call_finished", name: "fs.read", result: "hello" },
      now: 12,
    });
    const tool = state.messages[1]?.toolCalls[0];
    expect(tool?.name).toBe("fs.read");
    expect(tool?.status).toBe("done");
    expect(tool?.result).toBe("hello");
    expect(state.messages[1]?.toolCalls).toHaveLength(1);
  });

  it("turns an agent error into an inline retryable error", () => {
    let state = startSend(loadedState());
    state = appReducer(state, {
      type: "agent_event",
      event: { type: "error", message: "upstream failed" },
      now: 11,
    });
    expect(state.messages[1]?.status).toBe("error");
    expect(state.messages[1]?.error).toBe("upstream failed");
    expect(state.streaming).toBe(false);
    expect(state.retryAction).toBe("send");

    state = appReducer(state, { type: "retry_started", now: 12 });
    expect(state.messages[1]?.status).toBe("streaming");
    expect(state.messages[1]?.content).toBe("");
    expect(state.streaming).toBe(true);
  });

  it("marks approval decisions without dropping the request", () => {
    let state = startSend(loadedState());
    state = appReducer(state, {
      type: "agent_event",
      event: {
        type: "approval_requested",
        approval_id: "approval-1",
        tool: "fs.write",
        args: "{}",
        reason: "write outside workspace",
      },
      now: 11,
    });
    expect(state.messages[1]?.approvals[0]?.decision).toBe("pending");
    state = appReducer(state, {
      type: "approval_decided",
      approvalId: "approval-1",
      decision: "approved",
    });
    expect(state.messages[1]?.approvals[0]?.decision).toBe("approved");
    expect(state.messages[1]?.approvals).toHaveLength(1);
  });

  it("starts loading messages for the boot conversation", () => {
    const state = appReducer(createInitialState(), {
      type: "boot_loaded",
      meta: META,
      conversations: [CONVERSATION],
      settings: defaultSettings(),
      activeId: CONVERSATION.id,
    });
    expect(state.boot).toBe("ready");
    expect(state.messagesState).toBe("loading");
  });
});
