import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { App } from "../src/App";
import { readyTransport } from "./fixtures";

describe("command palette", () => {
  it("opens with Cmd/Ctrl+K, filters and runs settings", async () => {
    render(<App transport={readyTransport([])} />);
    await screen.findByRole("heading", { name: "今天想完成什么？" });
    await userEvent.keyboard("{Meta>}k{/Meta}");
    expect(await screen.findByRole("dialog", { name: "命令面板" })).toBeTruthy();
    const search = screen.getByRole("textbox", { name: "搜索命令" });
    await userEvent.type(search, "设置");
    const option = screen.getByRole("option", { name: /设置/ });
    await userEvent.click(option);
    expect(await screen.findByRole("dialog", { name: "设置" })).toBeTruthy();
  });

  it("closes with Escape", async () => {
    render(<App transport={readyTransport([])} />);
    await screen.findByRole("heading", { name: "今天想完成什么？" });
    await userEvent.keyboard("{Control>}k{/Control}");
    expect(await screen.findByRole("dialog", { name: "命令面板" })).toBeTruthy();
    await userEvent.keyboard("{Escape}");
    expect(screen.queryByRole("dialog", { name: "命令面板" })).toBeNull();
  });
});


describe("sidebar command entry", () => {
  it("opens the shell command palette instead of a local copy (C17)", async () => {
    const transport = readyTransport([]);
    render(<App transport={transport} />);
    const buttons = await screen.findAllByRole("button", { name: "命令面板" });
    expect(buttons.length).toBeGreaterThan(0);
    const [firstButton] = buttons;
    if (firstButton === undefined) {
      throw new Error("command palette button missing");
    }
    await userEvent.click(firstButton);
    expect(await screen.findByRole("dialog", { name: "命令面板" })).toBeTruthy();
    expect(screen.getAllByRole("dialog")).toHaveLength(1);
    await userEvent.keyboard("{Escape}");
    await waitFor(() => { expect(screen.queryByRole("dialog")).toBeNull(); });
  });
});
