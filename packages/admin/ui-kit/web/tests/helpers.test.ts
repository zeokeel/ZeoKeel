import { describe, expect, it, vi } from "vitest";
import {
  apiJson,
  assertValid,
  decodeAdminLogin,
  decodeAdminUsers,
  isRecord,
  messageOf,
  validateAdminLogin,
  validateCreateUser,
} from "../src/index";

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

  it("decodes admin responses through generated schemas (T22/C10)", () => {
    const users = decodeAdminUsers({
      ok: true,
      users: [{ id: "u1", username: "admin", display_name: "Admin", role: "super_admin", status: "active" }],
    });
    expect(users.users[0]?.username).toBe("admin");
    const login = decodeAdminLogin({ ok: true, csrf_token: "t", expires_in: 60, idle_timeout_secs: 60, role: "super_admin" });
    expect(login.csrf_token).toBe("t");
  });

  it("rejects out-of-contract response bodies and logs the reason (C10)", () => {
    const warn = vi.spyOn(console, "warn").mockImplementation(() => undefined);
    expect(() => decodeAdminUsers({ ok: true, users: "nope" })).toThrow("INVALID_ADMIN_USERS");
    expect(warn).toHaveBeenCalled();
    warn.mockRestore();
  });

  it("rejects an out-of-contract HTTP body at the fetch boundary (no assertion)", async () => {
    const warn = vi.spyOn(console, "warn").mockImplementation(() => undefined);
    vi.stubGlobal("fetch", () =>
      Promise.resolve(
        new Response(JSON.stringify({ ok: true, users: [{ id: 1 }] }), {
          status: 200,
          headers: { "content-type": "application/json" },
        }),
      ),
    );
    await expect(apiJson("/api/admin/users", decodeAdminUsers)).rejects.toThrow("INVALID_ADMIN_USERS");
    warn.mockRestore();
    vi.unstubAllGlobals();
  });
});
