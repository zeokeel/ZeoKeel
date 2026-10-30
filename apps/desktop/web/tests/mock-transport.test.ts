import { describe, expect, it } from "vitest";
import {
  parseAgentEvent,
  parseConversation,
  parseMessages,
  parseStreamId,
  parseWorkspaces,
} from "../src/protocol";
import { createMockTransport } from "../src/transport/mock";
import type { AgentEvent, UiEvent } from "../src/types";

async function waitUntil(predicate: () => boolean, timeoutMs = 2000): Promise<void> {
  const started = Date.now();
  while (!predicate()) {
    if (Date.now() - started > timeoutMs) {
      throw new Error("timed out waiting for mock streaming");
    }
    await new Promise((resolve) => {
      setTimeout(resolve, 5);
    });
  }
}

function agentEvents(events: UiEvent[]): AgentEvent[] {
  const parsed: AgentEvent[] = [];
  for (const event of events) {
    const agentEvent = parseAgentEvent(event.payload);
    if (agentEvent !== null) {
      parsed.push(agentEvent);
    }
  }
  return parsed;
}

describe("mock transport (?mock=1)", () => {
  it("streams tool events, completes, and persists the conversation", async () => {
    const transport = createMockTransport({ chunkDelayMs: 1 });
    const events: UiEvent[] = [];
    transport.subscribe((event) => events.push(event));

    const workspaces = await transport.call("workspace.list", {});
    const defaultWorkspace = parseWorkspaces(workspaces)?.[0];
    if (defaultWorkspace === undefined) {
      throw new Error("mock workspace.list returned no default workspace");
    }
    const conversation = parseConversation(
      await transport.call("session.create", {
        workspace_id: defaultWorkspace.id,
        title: "演示",
      }),
    );
    expect(conversation).not.toBeNull();
    if (conversation === null) {
      return;
    }

    const sent = await transport.call("chat.send", {
      workspace_id: defaultWorkspace.id,
      conversation_id: conversation.id,
      text: "读取 demo-data/hello.txt",
    });
    expect(parseStreamId(sent)).toBe("mock-stream-1");

    await waitUntil(() =>
      events.some((event) => {
        const parsed = parseAgentEvent(event.payload);
        return parsed !== null && parsed.type === "turn_completed";
      }),
    );

    const parsedEvents = agentEvents(events);
    expect(parsedEvents.some((event) => event.type === "tool_call_started")).toBe(true);
    expect(parsedEvents.some((event) => event.type === "tool_call_finished")).toBe(true);

    let finalText = "";
    for (const event of parsedEvents) {
      if (event.type === "turn_completed") {
        finalText = event.reply.text;
      }
    }
    expect(finalText).toContain("已读取");

    const messages = parseMessages(
      await transport.call("session.messages", {
        workspace_id: defaultWorkspace.id,
        conversation_id: conversation.id,
      }),
    );
    expect(messages?.map((message) => message.role)).toEqual(["user", "assistant"]);
  });

  it("cancels a turn and keeps the partial assistant text", async () => {
    const transport = createMockTransport({ chunkDelayMs: 5 });
    const events: UiEvent[] = [];
    transport.subscribe((event) => events.push(event));

    const workspaces = await transport.call("workspace.list", {});
    const defaultWorkspace = parseWorkspaces(workspaces)?.[0];
    if (defaultWorkspace === undefined) {
      throw new Error("mock workspace.list returned no default workspace");
    }
    const conversation = parseConversation(
      await transport.call("session.create", {
        workspace_id: defaultWorkspace.id,
        title: "取消演示",
      }),
    );
    if (conversation === null) {
      throw new Error("mock session.create returned an invalid conversation");
    }
    const streamId = parseStreamId(
      await transport.call("chat.send", {
        workspace_id: defaultWorkspace.id,
        conversation_id: conversation.id,
        text: "写一段长文字",
      }),
    );
    if (streamId === null) {
      throw new Error("mock chat.send returned no stream id");
    }

    await waitUntil(() => agentEvents(events).some((event) => event.type === "text_delta"));
    await transport.call("chat.cancel", { stream_id: streamId });

    expect(agentEvents(events).some((event) => event.type === "turn_cancelled")).toBe(true);
    const messages = parseMessages(
      await transport.call("session.messages", {
        workspace_id: defaultWorkspace.id,
        conversation_id: conversation.id,
      }),
    );
    expect(messages).toHaveLength(2);
    expect(messages?.[1]?.role).toBe("assistant");
    expect(messages?.[1]?.content).not.toBe("");
  });

  it("reports unknown commands as errors", async () => {
    const transport = createMockTransport({ chunkDelayMs: 1 });
    await expect(transport.call("does.not.exist", {})).rejects.toThrow("unknown command");
  });
});
