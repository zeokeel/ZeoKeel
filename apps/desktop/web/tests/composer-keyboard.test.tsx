import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { App } from "../src/App";
import { readyTransport } from "./fixtures";

async function boot() {
  const transport = readyTransport([]);
  render(<App transport={transport} />);
  const input = await screen.findByRole("textbox", { name: "消息输入框" });
  return { transport, input };
}

describe("composer keyboard", () => {
  it("disables send for empty input", async () => {
    await boot();
    expect(screen.getByRole("button", { name: "发送" })).toHaveProperty("disabled", true);
  });

  it("sends on Enter and clears the field", async () => {
    const { transport, input } = await boot();
    await userEvent.type(input, "你好");
    await userEvent.keyboard("{Enter}");
    await waitFor(() => {
      expect(transport.callsFor("chat.send")).toHaveLength(1);
    });
    expect(transport.callsFor("chat.send")[0]).toMatchObject({ text: "你好" });
    await waitFor(() => {
      expect(input).toHaveProperty("value", "");
    });
  });

  it("keeps a newline with Shift+Enter instead of sending", async () => {
    const { transport, input } = await boot();
    await userEvent.type(input, "第一行");
    await userEvent.keyboard("{Shift>}{Enter}{/Shift}");
    expect(transport.callsFor("chat.send")).toHaveLength(0);
    expect(input).toHaveProperty("value", expect.stringContaining("第一行"));
  });

  it("does not send while an IME composition is active", async () => {
    const { transport, input } = await boot();
    fireEvent.compositionStart(input);
    await userEvent.type(input, "中文");
    fireEvent.keyDown(input, { key: "Enter", isComposing: true });
    expect(transport.callsFor("chat.send")).toHaveLength(0);

    fireEvent.compositionEnd(input);
    await userEvent.keyboard("{Enter}");
    await waitFor(() => {
      expect(transport.callsFor("chat.send")).toHaveLength(1);
    });
  });

  it("swaps send for a stop button while streaming", async () => {
    const { transport, input } = await boot();
    await userEvent.type(input, "你好");
    await userEvent.keyboard("{Enter}");
    await waitFor(() => {
      expect(transport.callsFor("chat.send")).toHaveLength(1);
    });
    expect(await screen.findByRole("button", { name: "停止生成" })).toBeTruthy();
  });
});
