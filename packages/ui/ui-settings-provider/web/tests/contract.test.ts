import { describe, expect, it } from "vitest";
import { components } from "../src/index";

describe("ui-settings-provider package contract", () => {
  it("registers its ui.settings.pane contribution per zk.toml", () => {
    const values = Object.values(components);
    expect(values.length).toBeGreaterThan(0);
    expect(values.every((value) => typeof value === "function")).toBe(true);
    expect(components).toHaveProperty("settings.pane.provider");
  });
});
