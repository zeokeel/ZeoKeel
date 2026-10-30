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
    await userEvent.click(screen.getByRole("button", { name: "保存" }));

    await waitFor(() => {
      expect(transport.callsFor("settings.set")).toHaveLength(1);
    });
    const sentSettings = parseSettings(
      transport.callsFor("settings.set")[0]?.["settings"],
    );
    expect(sentSettings.locale).toBe("en-US");
    expect(sentSettings.theme).toBe("dark");

    expect(document.documentElement.lang).toBe("en-US");
    expect(document.documentElement.dataset["theme"]).toBe("dark");
    expect(await screen.findByText("Settings saved")).toBeTruthy();
    expect(screen.getAllByRole("button", { name: "New conversation" }).length).toBeGreaterThan(0);
    await waitFor(() => {
      expect(screen.queryByRole("dialog")).toBeNull();
    });
  });
});
