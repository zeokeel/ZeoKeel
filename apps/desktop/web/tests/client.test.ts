import { afterEach, describe, expect, it, vi } from "vitest";
import { clientHeaders, newRequestId, platformId, serverFetch, serverHeaders } from "../src/client";
import { UpgradeRequiredError } from "../src/upgrade";

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("client identity headers (round-6 §1/§4)", () => {
  it("sends X-ZeoKeel-Client with normalized platform and a X-Request-Id per request", async () => {
    const calls: { url: string; init: RequestInit | undefined }[] = [];
    vi.stubGlobal("fetch", (url: unknown, init?: RequestInit) => {
      calls.push({ url: String(url), init });
      return Promise.resolve(new Response("{}", { status: 200, headers: { "content-type": "application/json" } }));
    });
    const response = await serverFetch("http://server.example/api/client/v1/skills", {
      headers: { authorization: "Bearer token" },
    });
    expect(response.status).toBe(200);
    expect(calls).toHaveLength(1);
    expect(calls[0]?.url).toContain("/api/client/v1/skills");
    const headers = new Headers(calls[0]?.init?.headers);
    expect(headers.get("authorization")).toBe("Bearer token");
    const client = headers.get("X-ZeoKeel-Client") ?? "";
    expect(client.split(";")).toHaveLength(4);
    expect(client.split(";").at(0) ?? "").toMatch(/^\d+\.\d+\.\d+/);
    expect(client.split(";").at(2) ?? "").toMatch(/^(linux-x64|windows-x64|macos|other)$/);
    expect(headers.get("X-Request-Id")).toMatch(/^[0-9a-f-]{8,}$/i);
    expect(headers.get("X-Request-Id")).not.toBe(serverHeaders()["X-Request-Id"]);
  });

  it("normalizes platform ids and generates distinct request ids", () => {
    expect(["linux-x64", "windows-x64", "macos", "other"]).toContain(platformId());
    const identity = clientHeaders()["X-ZeoKeel-Client"] ?? "";
    expect(identity.split(";").at(2) ?? "").toBe(platformId());
    expect(newRequestId()).not.toBe(newRequestId());
  });

  it("maps any 426 to a typed UpgradeRequiredError", async () => {
    vi.stubGlobal("fetch", () => Promise.resolve(new Response(JSON.stringify({ error: { code: "CLIENT_TOO_OLD", current: "0.1.0", minimum: "9.9.9", portal_url: "/portal" } }), { status: 426 })));
    await expect(serverFetch("http://server.example/api/client/v1/skills")).rejects.toBeInstanceOf(UpgradeRequiredError);
  });
});
