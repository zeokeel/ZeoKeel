/**
 * Connector OAuth glue tests: the PKCE/loopback flow itself is covered by the Rust
 * tests (`apps/desktop/src/mcp_oauth.rs`); here we test the IPC view adapter, token
 * parsing and status polling (docs/07 §6).
 */
import { describe, expect, it } from "vitest";
import {
  oauthMetadataPayload,
  parseOAuthFlowStatus,
  parseOAuthTokens,
  tokenNeedsRefresh,
  waitForOAuthFlow,
  type ConnectorOAuthView,
} from "../src/oauth";

const view: ConnectorOAuthView = {
  clientId: "public-client",
  scopes: ["mcp.read"],
  redirectUri: "",
  authorizeUrl: "https://auth.example/authorize",
  tokenUrl: "https://auth.example/token",
};

describe("connector oauth glue", () => {
  it("maps catalog camelCase metadata to the IPC contract snake_case shape", () => {
    expect(oauthMetadataPayload(view)).toEqual({
      client_id: "public-client",
      scopes: ["mcp.read"],
      redirect_uri: "",
      authorize_url: "https://auth.example/authorize",
      token_url: "https://auth.example/token",
    });
    // No client_secret ever crosses this seam (public client + PKCE, ADR-060).
    expect(Object.keys(oauthMetadataPayload(view))).not.toContain("client_secret");
  });

  it("parses keychain token payloads and refresh skew", () => {
    const tokens = parseOAuthTokens(
      JSON.stringify({ access_token: "a", refresh_token: "r", expires_at: 1_000 }),
    );
    expect(tokens?.access_token).toBe("a");
    expect(tokens?.refresh_token).toBe("r");
    expect(tokenNeedsRefresh({ access_token: "a", refresh_token: "r", expires_at: 1_000 })).toBe(true);
    expect(tokenNeedsRefresh({ access_token: "a", refresh_token: "r", expires_at: 3_600_000 }, 0)).toBe(false);
    expect(parseOAuthTokens("not-json")).toBeNull();
    expect(parseOAuthTokens(null)).toBeNull();
    // A token without a refresh token is reported as unusable for refresh decisions.
    expect(parseOAuthTokens(JSON.stringify({ access_token: "a", expires_at: 1 }))).toBeNull();
  });

  it("parses oauth.status snapshots and ignores malformed ones", () => {
    expect(parseOAuthFlowStatus({ state: "idle", error: null })).toEqual({ state: "idle", error: null });
    expect(parseOAuthFlowStatus({ state: "pending" })).toEqual({ state: "pending", error: null });
    expect(parseOAuthFlowStatus({ state: "failed", error: "OAUTH_TIMEOUT" })).toEqual({
      state: "failed",
      error: "OAUTH_TIMEOUT",
    });
    expect(parseOAuthFlowStatus({ state: "weird" })).toBeNull();
    expect(parseOAuthFlowStatus("nope")).toBeNull();
    expect(parseOAuthFlowStatus(null)).toBeNull();
  });

  it("polls oauth.status until the background flow stops being pending", async () => {
    const calls: string[] = [];
    let polls = 0;
    const call = (name: string): Promise<unknown> => {
      calls.push(name);
      polls += 1;
      if (polls < 3) return Promise.resolve({ state: "pending", error: null });
      return Promise.resolve({ state: "authorized", error: null });
    };
    const status = await waitForOAuthFlow(call, "mcp-demo", { pollMs: 0, timeoutMs: 5_000 });
    expect(status).toEqual({ state: "authorized", error: null });
    expect(calls.every((name) => name === "oauth.status")).toBe(true);
    expect(polls).toBe(3);
  });

  it("surfaces a failed flow error and times out honestly", async () => {
    const failed = await waitForOAuthFlow(
      () => Promise.resolve({ state: "failed", error: "OAUTH_TOKEN_HTTP_400" }),
      "mcp-demo",
      { pollMs: 0, timeoutMs: 1_000 },
    );
    expect(failed).toEqual({ state: "failed", error: "OAUTH_TOKEN_HTTP_400" });

    const timedOut = await waitForOAuthFlow(
      () => Promise.resolve({ state: "pending", error: null }),
      "mcp-demo",
      { pollMs: 0, timeoutMs: 1 },
    );
    expect(timedOut).toEqual({ state: "failed", error: "OAUTH_TIMEOUT" });
  });
});
