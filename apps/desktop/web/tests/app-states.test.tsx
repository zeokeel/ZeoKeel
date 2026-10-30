import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it } from "vitest";
import { App } from "../src/App";
import { FakeTransport } from "./fake-transport";
import { CONVERSATION, readyTransport } from "./fixtures";

function renderApp(transport: FakeTransport) {
  return render(<App transport={transport} />);
}

afterEach(() => {
  Reflect.deleteProperty(navigator, "onLine");
});

describe("app six states", () => {
  it("shows the loading state while bootstrapping", () => {
    const transport = new FakeTransport();
    transport.handle("meta.get", () => new Promise(() => {}));
    transport.handle("session.list", () => new Promise(() => {}));
    transport.handle("settings.get", () => new Promise(() => {}));
    renderApp(transport);
    expect(screen.getByText("正在启动 ZeoKeel…")).toBeTruthy();
  });

  it("shows empty conversation + empty chat states", async () => {
    renderApp(readyTransport([]));
    expect(await screen.findByRole("heading", { name: "今天想完成什么？" })).toBeTruthy();
    expect(screen.getByText("还没有会话，点击“新建会话”开始。")).toBeTruthy();
    expect(screen.getByRole("button", { name: "发送" })).toHaveProperty("disabled", true);
  });

  it("shows a boot error and recovers through retry", async () => {
    const transport = readyTransport([]);
    transport.fail("session.list", "后端不可用");
    renderApp(transport);
    expect(await screen.findByRole("heading", { name: "无法连接本地后端" })).toBeTruthy();
    expect(screen.getByText("后端不可用")).toBeTruthy();

    transport.failures.delete("session.list");
    await userEvent.click(screen.getByRole("button", { name: "重试" }));
    expect(await screen.findByRole("heading", { name: "今天想完成什么？" })).toBeTruthy();
  });

  it("shows the offline banner and pauses sending", async () => {
    Object.defineProperty(navigator, "onLine", { configurable: true, value: false });
    renderApp(readyTransport([]));
    expect(
      await screen.findByText("当前处于离线状态：已有会话可查看，发送已暂停。"),
    ).toBeTruthy();
    expect(screen.getByRole("textbox", { name: "消息输入框" })).toHaveProperty("disabled", true);
    expect(screen.getByRole("button", { name: "发送" })).toHaveProperty("disabled", true);
  });

  it("restores messages for the boot conversation", async () => {
    const transport = readyTransport([CONVERSATION]);
    transport.handle("session.messages", () => [
      {
        id: "message-1",
        conversation_id: CONVERSATION.id,
        role: "user",
        content: "历史消息",
        created_at_ms: 1,
      },
    ]);
    renderApp(transport);
    expect(await screen.findByText("历史消息")).toBeTruthy();
    expect(screen.getByRole("button", { name: "演示会话" })).toBeTruthy();
  });

  it("shows a message-load error with retry", async () => {
    const transport = readyTransport([CONVERSATION]);
    transport.fail("session.messages", "读取消息失败");
    renderApp(transport);
    expect(await screen.findByRole("heading", { name: "消息加载失败" })).toBeTruthy();
    expect(screen.getByText("读取消息失败")).toBeTruthy();

    transport.failures.delete("session.messages");
    await userEvent.click(screen.getByRole("button", { name: "重新加载" }));
    expect(await screen.findByRole("heading", { name: "今天想完成什么？" })).toBeTruthy();
  });
});
