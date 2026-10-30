import { describe, expect, it } from "vitest";
import { ADMIN_PAGES, AdminShellSlot } from "../src/generated/registry";

describe("admin registry", () => {
  it("exposes the generated admin composition contract", () => {
    // Edition-agnostic: the personal baseline emits an empty stub; enterprise binds all 10 pages.
    expect(typeof AdminShellSlot).toBe("function");
    expect(Object.values(ADMIN_PAGES).every((page) => typeof page === "function")).toBe(true);
    const keys = Object.keys(ADMIN_PAGES);
    expect(keys.length === 0 || keys.length === 10).toBe(true);
  });
});
