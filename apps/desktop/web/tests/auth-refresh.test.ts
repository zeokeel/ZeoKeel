import { afterEach, describe, expect, it, vi } from "vitest";
import { loadServerUrl, refreshSession, saveServerUrl } from "../src/auth";

afterEach(() => {
  localStorage.clear();
  vi.restoreAllMocks();
});

describe("enterprise auth refresh", () => {
  it("rotates tokens against /api/client/v1/auth/refresh", async () => {
    saveServerUrl("http://127.0.0.1:8795/");
    expect(loadServerUrl()).toBe("http://127.0.0.1:8795");
    const fetchMock = vi.spyOn(globalThis, "fetch").mockResolvedValue(
      new Response(JSON.stringify({ ok: true, access_token: "a2", refresh_token: "r2", expires_in: 900 }), { status: 200 }),
    );
    const session = { access_token: "a1", refresh_token: "r1", expires_in: 900, user_id: "u1", role: "employee", username: "alice", device_name: "dev1" };
    const next = await refreshSession(session, "http://127.0.0.1:8795");
    expect(next.access_token).toBe("a2");
    expect(next.refresh_token).toBe("r2");
    expect(fetchMock).toHaveBeenCalledWith(
      "http://127.0.0.1:8795/api/client/v1/auth/refresh",
      expect.objectContaining({ method: "POST" }),
    );
  });

  it("rejects when refresh is revoked", async () => {
    vi.spyOn(globalThis, "fetch").mockResolvedValue(
      new Response(JSON.stringify({ ok: false, error: "REFRESH_REUSED" }), { status: 401 }),
    );
    const session = { access_token: "a1", refresh_token: "r1", expires_in: 900, user_id: "u1", role: "employee", username: "alice", device_name: "dev1" };
    await expect(refreshSession(session, "http://127.0.0.1:8795")).rejects.toThrow("REFRESH_REUSED");
  });
});
