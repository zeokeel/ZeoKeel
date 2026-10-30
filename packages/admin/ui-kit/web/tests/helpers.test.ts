import { describe, expect, it } from "vitest";
import { assertValid, isRecord, messageOf, validateAdminLogin, validateCreateUser } from "../src/index";

describe("admin-ui-kit helpers", () => {
  it("narrows records and formats unknown errors", () => {
    expect(isRecord({ ok: true })).toBe(true);
    expect(isRecord("x")).toBe(false);
    expect(messageOf(new Error("boom"))).toBe("boom");
    expect(messageOf("text")).toBe("text");
    expect(messageOf({ code: 1 })).toBe("request failed");
  });

  it("validates requests against generated schemas (C10)", () => {
    expect(() => { assertValid(validateAdminLogin, { username: "admin", password: "pw" }, "LOGIN"); }).not.toThrow();
    expect(() => { assertValid(validateCreateUser, { username: "" }, "USER"); }).toThrow("INVALID_USER");
  });
});
