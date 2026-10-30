import { describe, expect, it } from "vitest";
import { components } from "../src/index";

describe("admin-wallet package contract", () => {
  it("registers a slot component per zk.toml contribution", () => {
    const values = Object.values(components);
    expect(values.length).toBeGreaterThan(0);
    expect(values.every((value) => typeof value === "function")).toBe(true);
    expect(components).toHaveProperty("page.wallet");
  });
});
