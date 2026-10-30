import { render, screen } from "@testing-library/react";
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
