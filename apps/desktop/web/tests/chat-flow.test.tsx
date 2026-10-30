import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { App } from "../src/App";
import type { FakeTransport } from "./fake-transport";
import { readyTransport } from "./fixtures";

async function createConversation() {
  const buttons = await screen.findAllByRole("button", { name: "新建会话" });
  const button = buttons[buttons.length - 1] ?? buttons[0];
  if (button === undefined) {
    throw new Error("new conversation button missing");
  }
  await userEvent.click(button);
  return screen.findByRole("textbox", { name: "消息输入框" });
}

async function bootAndSend(transport: FakeTransport, text: string) {
  render(<App transport={transport} />);
  const input = await createConversation();
  await userEvent.type(input, text);
  await userEvent.keyboard("{Enter}");
  await waitFor(() => {
    expect(transport.callsFor("chat.send")).toHaveLength(1);
  });
  return input;
}

async function emitAgent(
  transport: FakeTransport,
  seq: number,
  agentEvent: Record<string, unknown>,
): Promise<void> {
  await act(async () => {
    transport.emitAgent("stream-1", seq, agentEvent);
  });
}

describe("chat flow", () => {
  it("streams markdown, renders tool cards, and completes the turn", async () => {
    const transport = readyTransport([]);
    await bootAndSend(transport, "读取 demo-data/hello.txt");
    expect(screen.getByTestId("user-text").textContent).toContain("读取 demo-data/hello.txt");
    expect(screen.getByRole("button", { name: "停止生成" })).toBeTruthy();
    expect(transport.callsFor("session.create")[0]).toMatchObject({
      workspace_id: "ws-home",
      title: "新会话",
    });
    expect(transport.callsFor("chat.send")[0]).toMatchObject({
      workspace_id: "ws-home",
      conversation_id: "conv-created-1",
    });

    await emitAgent(transport, 2, {
      type: "tool_call_started",
      name: "fs.read",
      args: '{"path":"demo-data/hello.txt"}',
    });
    await emitAgent(transport, 3, {
      type: "tool_call_finished",
      name: "fs.read",
      result: "Hello from ZeoKeel demo data.",
    });
    expect(screen.getByText("fs.read")).toBeTruthy();
    expect(screen.getByText("完成")).toBeTruthy();
    expect(screen.getByText("Hello from ZeoKeel demo data.")).toBeTruthy();

    await emitAgent(transport, 4, {
      type: "text_delta",
      delta: "**加粗** 和 `code`",
    });
    expect(screen.getByText("加粗").tagName).toBe("STRONG");
    expect(screen.getByText("code").tagName).toBe("CODE");

    await emitAgent(transport, 5, {
      type: "turn_completed",
      reply: {
        text: "最终回答：已读取 demo-data/hello.txt。",
        tool_calls: [],
      },
    });
    expect(screen.getByText("最终回答：已读取 demo-data/hello.txt。")).toBeTruthy();
    await waitFor(() => {
      expect(screen.getByRole("button", { name: "发送" })).toBeTruthy();
    });
    expect(screen.queryByRole("button", { name: "停止生成" })).toBeNull();
  });

  it("cancels a streaming turn and keeps the partial content", async () => {
    const transport = readyTransport([]);
    await bootAndSend(transport, "写点东西");
    await emitAgent(transport, 2, { type: "text_delta", delta: "已经生成的一部分" });
    expect(screen.getByText(/已经生成的一部分/)).toBeTruthy();

    await userEvent.click(screen.getByRole("button", { name: "停止生成" }));
    expect(transport.callsFor("chat.cancel")[0]).toMatchObject({ stream_id: "stream-1" });

    await emitAgent(transport, 3, { type: "turn_cancelled" });
    expect(screen.getByText("已取消")).toBeTruthy();
    expect(screen.getByText(/已经生成的一部分/)).toBeTruthy();
    await waitFor(() => {
      expect(screen.getByRole("button", { name: "发送" })).toBeTruthy();
    });
  });

  it("shows an inline error and retries the failed send", async () => {
    const transport = readyTransport([]);
    await bootAndSend(transport, "触发失败");
    await emitAgent(transport, 2, { type: "error", message: "boom" });
    const alert = await screen.findByRole("alert");
    expect(alert.textContent).toContain("boom");

    await userEvent.click(screen.getByRole("button", { name: "重试发送" }));
    await waitFor(() => {
      expect(transport.callsFor("chat.send")).toHaveLength(2);
    });
  });

  it("ignores unknown event kinds with a warning", async () => {
    const transport = readyTransport([]);
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
    await bootAndSend(transport, "你好");
    await act(async () => {
      transport.emit({ stream_id: "stream-1", seq: 9, kind: "heartbeat", payload: {} });
    });
    expect(warn).toHaveBeenCalledWith(expect.stringContaining("ignored unsupported event kind"));
    expect(screen.getByRole("button", { name: "停止生成" })).toBeTruthy();
  });
});
