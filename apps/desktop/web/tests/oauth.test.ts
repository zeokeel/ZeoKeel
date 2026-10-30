import { describe, expect, it, vi, afterEach } from "vitest";
import { buildAuthorizeUrl, createPkcePair, exchangeCode, parseOAuthTokens, refreshOAuthToken, savePendingAuth, takePendingAuth, tokenNeedsRefresh, type OAuthMetadata } from "../src/oauth";

const metadata: OAuthMetadata = {
  authorizeUrl: "https://auth.example/authorize",
  tokenUrl: "https://auth.example/token",
  clientId: "public-client",
  redirectUri: "http://127.0.0.1/callback",
  scopes: ["mcp", "offline_access"],
};

afterEach(() => { vi.restoreAllMocks(); });

describe("connector oauth", () => {
  it("creates PKCE challenge and authorize URL", async () => {
    const pair = await createPkcePair();
    expect(pair.verifier.length).toBeGreaterThan(20);
    expect(pair.challenge).not.toBe(pair.verifier);
    const url = new URL(buildAuthorizeUrl(metadata, pair.challenge, "state-1"));
    expect(url.searchParams.get("code_challenge_method")).toBe("S256");
    expect(url.searchParams.get("client_id")).toBe("public-client");
    expect(url.searchParams.get("state")).toBe("state-1");
  });

  it("exchanges and refreshes tokens with expiry", async () => {
    const fetchMock = vi.spyOn(globalThis, "fetch")
      .mockResolvedValueOnce(new Response(JSON.stringify({ access_token: "a1", refresh_token: "r1", expires_in: 3600 }), { status: 200 }))
      .mockResolvedValueOnce(new Response(JSON.stringify({ access_token: "a2", refresh_token: "r2", expires_in: 3600 }), { status: 200 }));
    const tokens = await exchangeCode(metadata, "code", "verifier", 1_000);
    expect(tokens.access_token).toBe("a1");
    expect(tokenNeedsRefresh(tokens, 1_000)).toBe(false);
    const refreshed = await refreshOAuthToken(metadata, tokens, 2_000);
    expect(refreshed.access_token).toBe("a2");
    expect(fetchMock).toHaveBeenCalledTimes(2);
  });

  it("parses keychain token payloads", () => {
    expect(parseOAuthTokens(JSON.stringify({ access_token: "a", refresh_token: "r", expires_at: 1 }))?.access_token).toBe("a");
    expect(parseOAuthTokens("not-json")).toBeNull();
    expect(parseOAuthTokens(null)).toBeNull();
  });

  it("round-trips pending PKCE state", () => {
    sessionStorage.clear();
    savePendingAuth({ connectorId: "c1", verifier: "v", state: "s", metadata });
    const pending = takePendingAuth();
    expect(pending?.connectorId).toBe("c1");
    expect(takePendingAuth()).toBeNull();
  });

  it("fails closed on refresh error", async () => {
    vi.spyOn(globalThis, "fetch").mockResolvedValue(new Response("nope", { status: 401 }));
    await expect(refreshOAuthToken(metadata, { access_token: "a", refresh_token: "r", expires_at: 0 })).rejects.toThrow("OAUTH_HTTP_401");
  });
});
