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
    transport.handle("meta.get", () => new Promise(() => undefined));
    transport.handle("workspace.list", () => new Promise(() => undefined));
    transport.handle("session.list", () => new Promise(() => undefined));
    transport.handle("settings.get", () => new Promise(() => undefined));
    renderApp(transport);
    expect(screen.getByText("正在启动 ZeoKeel…")).toBeTruthy();
  });

  it("shows empty workspace + new-conversation prompt", async () => {
    renderApp(readyTransport([]));
    expect(await screen.findByRole("heading", { name: "今天想完成什么？" })).toBeTruthy();
    expect(screen.getAllByText("暂无会话").length).toBeGreaterThan(0);
    expect(screen.queryByRole("textbox", { name: "消息输入框" })).toBeNull();
    expect(screen.getAllByRole("button", { name: "新建会话" }).length).toBeGreaterThan(0);
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
    renderApp(readyTransport([CONVERSATION]));
    expect(
      (await screen.findAllByText("当前处于离线状态：已有会话可查看，发送已暂停。")).length,
    ).toBeGreaterThan(0);
    const input = await screen.findByRole("textbox", { name: "消息输入框" });
    expect(input).toHaveProperty("disabled", true);
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
    expect(screen.getByRole("button", { name: /演示会话/ })).toBeTruthy();
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
