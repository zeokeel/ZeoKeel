import { describe, expect, it } from "vitest";
import { appReducer, createInitialState, type AppState } from "../src/state/reducer";
import { defaultSettings } from "../src/types";
import { CONVERSATION, META, WORKSPACE, WORKSPACES } from "./fixtures";

function loadedState(): AppState {
  let state = appReducer(createInitialState(), {
    type: "boot_loaded",
    meta: META,
    workspaces: WORKSPACES,
    activeWorkspaceId: WORKSPACE.id,
    conversationsByWorkspace: { [WORKSPACE.id]: [CONVERSATION] },
    conversations: [CONVERSATION],
    settings: defaultSettings(),

    settingsRevision: 0,
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
      event: { type: "turn_completed", reply: { text: "最终回答", tool_calls: [], usage: null } },
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
    expect(tool?.status).toBe("success");
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

  it("records question asked and answered", () => {
    let state = startSend(loadedState());
    state = appReducer(state, {
      type: "agent_event",
      event: { type: "question_asked", question_id: "q1", questions: [{ id: "name", question: "Name?" }] },
      now: 11,
    });
    expect(state.messages[1]?.questions[0]?.answers).toBeNull();
    state = appReducer(state, {
      type: "question_answered",
      questionId: "q1",
      answers: { answers: [{ id: "name", custom: "Ada" }] },
    });
    expect(state.messages[1]?.questions[0]?.answers).not.toBeNull();
  });

  it("starts loading messages for the boot conversation", () => {
    const state = appReducer(createInitialState(), {
      type: "boot_loaded",
      meta: META,
      workspaces: WORKSPACES,
      activeWorkspaceId: WORKSPACE.id,
      conversationsByWorkspace: { [WORKSPACE.id]: [CONVERSATION] },
      conversations: [CONVERSATION],
      settings: defaultSettings(),

      settingsRevision: 0,
      activeId: CONVERSATION.id,
    });
    expect(state.boot).toBe("ready");
    expect(state.messagesState).toBe("loading");
  });

  it("applies terminal tool statuses from finish events", () => {
    let state = startSend(loadedState());
    state = appReducer(state, {
      type: "agent_event",
      event: { type: "tool_call_started", name: "bash", args: "{}" },
      now: 11,
    });
    state = appReducer(state, {
      type: "agent_event",
      event: {
        type: "tool_call_finished",
        name: "bash",
        result: "denied by approval policy",
        args: '{"command":"rm"}',
        status: "denied",
      },
      now: 12,
    });
    expect(state.messages[1]?.toolCalls[0]?.status).toBe("denied");
    expect(state.messages[1]?.toolCalls[0]?.args).toBe('{"command":"rm"}');
  });

  it("groups persisted tool rows by turn and keeps degraded leftovers", () => {
    let state = loadedState();
    state = appReducer(state, {
      type: "messages_loaded",
      conversationId: CONVERSATION.id,
      messages: [
        {
          id: "u1",
          conversation_id: CONVERSATION.id,
          role: "user",
          content: "go",
          created_at_ms: 1,
          status: "done",
          usage: null,
          turn_id: "turn-1",
        },
        {
          id: "t1",
          conversation_id: CONVERSATION.id,
          role: "tool",
          content: JSON.stringify({ name: "fs.write", args: "{}", status: "cancelled", result: "wrote" }),
          created_at_ms: 2,
          status: "done",
          usage: null,
          turn_id: "turn-1",
        },
        {
          id: "a1",
          conversation_id: CONVERSATION.id,
          role: "assistant",
          content: "done",
          created_at_ms: 3,
          status: "done",
          usage: null,
          turn_id: "turn-1",
        },
      ],
    });
    expect(state.messages.map((message) => message.id)).toEqual(["u1", "a1"]);
    expect(state.messages[1]?.toolCalls[0]?.status).toBe("cancelled");
    expect(state.messages[1]?.turnId).toBe("turn-1");
  });

  it("records approval metadata and typed decisions", () => {
    let state = startSend(loadedState());
    state = appReducer(state, {
      type: "agent_event",
      event: {
        type: "approval_requested",
        approval_id: "approval-1",
        tool: "fs.write",
        args: '{"path":"a.txt"}',
        reason: "mutating tool",
        auto: false,
        deadline_ms: 1234,
        metadata: { summary: "write a.txt", risk: "medium", write_paths: ["a.txt"] },
      },
      now: 11,
    });
    const pending = state.messages[1]?.approvals[0];
    expect(pending?.decision).toBe("pending");
    expect(pending?.metadata?.summary).toBe("write a.txt");
    expect(pending?.deadline_ms).toBe(1234);
    state = appReducer(state, {
      type: "agent_event",
      event: { type: "approval_resolved", approval_id: "approval-1", decision: "allow_session", timed_out: false },
      now: 12,
    });
    expect(state.messages[1]?.approvals[0]?.decision).toBe("allow_session");
  });

  it("applies live usage events to the streaming turn", () => {
    let state = startSend(loadedState());
    state = appReducer(state, {
      type: "agent_event",
      event: {
        type: "usage",
        usage: {
          input_tokens: 3,
          output_tokens: 2,
          total_tokens: 5,
          cached_tokens: 0,
          reasoning_tokens: 0,
          cost_micros: null,
        },
      },
      now: 12,
    });
    expect(state.messages[1]?.usage?.total_tokens).toBe(5);
  });
});
