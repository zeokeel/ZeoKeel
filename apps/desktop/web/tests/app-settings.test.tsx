import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { App } from "../src/App";
import { parseSettings } from "../src/protocol";
import { readyTransport } from "./fixtures";

describe("settings integration", () => {
  it("applies locale and theme after saving and shows a toast", async () => {
    const transport = readyTransport([]);
    render(<App transport={transport} />);

    await userEvent.click(await screen.findByRole("button", { name: "打开设置" }));
    expect(await screen.findByRole("dialog")).toBeTruthy();

    await userEvent.selectOptions(screen.getByLabelText("语言"), "en-US");
    await userEvent.selectOptions(screen.getByLabelText("主题"), "dark");
    const maxSteps = screen.getByLabelText("最大步数（1-200）");
    await userEvent.clear(maxSteps);
    await userEvent.type(maxSteps, "12");
    const temperature = screen.getByLabelText("temperature（0-2）");
    await userEvent.clear(temperature);
    await userEvent.type(temperature, "0.7");
    await userEvent.click(screen.getByLabelText("启用会话记忆"));
    await userEvent.click(screen.getByRole("button", { name: "保存" }));

    await waitFor(() => {
      expect(transport.callsFor("settings.set")).toHaveLength(1);
    });
    const sentSettings = parseSettings(
      transport.callsFor("settings.set")[0]?.["settings"],
    );
    expect(sentSettings.locale).toBe("en-US");
    expect(sentSettings.theme).toBe("dark");
    expect(sentSettings.agent.max_steps).toBe(12);
    expect(sentSettings.agent.completion.temperature).toBe(0.7);
    expect(sentSettings.agent.memory.enabled).toBe(false);

    expect(document.documentElement.lang).toBe("en-US");
    expect(document.documentElement.dataset["theme"]).toBe("dark");
    expect(await screen.findByText("Settings saved")).toBeTruthy();
    expect(screen.getAllByRole("button", { name: "New conversation" }).length).toBeGreaterThan(0);
    await waitFor(() => {
      expect(screen.queryByRole("dialog")).toBeNull();
    });
  });
});

describe("update check", () => {
  it("shows the available version from the update feed", async () => {
    const transport = readyTransport([]);
    transport.handle("update.check", () => ({ release: { version: "9.9.9", notes: "" } }));
    render(<App transport={transport} />);
    await userEvent.click(await screen.findByRole("button", { name: "打开设置" }));
    await userEvent.click(await screen.findByRole("button", { name: "检查更新" }));
    expect(await screen.findByText("发现新版本 9.9.9")).toBeTruthy();
  });
});
