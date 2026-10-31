/**
 * Connector OAuth glue (docs/07 §6).
 *
 * The OAuth 2.0 public-client + PKCE flow itself lives in the Rust host
 * (`apps/desktop/src/mcp_oauth.rs`): loopback redirect (RFC 8252), system browser,
 * token exchange/refresh and the keychain write. This module only:
 * - maps the catalog's camelCase metadata to the IPC contract shape;
 * - polls `oauth.status` (never returns tokens);
 * - parses keychain token payloads for the "needs refresh" decision.
 *
 * Single implementation (C4/C17): no PKCE/crypto here.
 */
import { isRecord, readNumber, readString } from "@zeokeel/ui-kit";

export interface OAuthTokens {
  access_token: string;
  refresh_token: string;
  expires_at: number;
}

/** Catalog OAuth metadata as delivered to the web UI (camelCase view type). */
export interface ConnectorOAuthView {
  clientId: string;
  scopes: string[];
  redirectUri: string;
  authorizeUrl: string;
  tokenUrl: string;
}

/** IPC `oauth.authorize`/`oauth.refresh` payload (`zk_contract_connector::ConnectorOAuth`). */
export function oauthMetadataPayload(view: ConnectorOAuthView): Record<string, unknown> {
  return {
    client_id: view.clientId,
    scopes: view.scopes,
    redirect_uri: view.redirectUri,
    authorize_url: view.authorizeUrl,
    token_url: view.tokenUrl,
  };
}

export type OAuthFlowState = "idle" | "pending" | "authorized" | "failed";

/** One `oauth.status` snapshot (idle when the host has no flow for the connector). */
export interface OAuthFlowStatus {
  state: OAuthFlowState;
  error: string | null;
}

export type OAuthCall = (name: string, payload: Record<string, unknown>) => Promise<unknown>;

const FLOW_STATES: ReadonlySet<string> = new Set(["idle", "pending", "authorized", "failed"]);

/** Parse an `oauth.status` payload; `null` when malformed (ignored by callers). */
export function parseOAuthFlowStatus(value: unknown): OAuthFlowStatus | null {
  if (!isRecord(value)) return null;
  const rawState = readString(value, "state");
  if (rawState === null || !FLOW_STATES.has(rawState)) return null;
  const state: OAuthFlowState =
    rawState === "pending" || rawState === "authorized" || rawState === "failed" ? rawState : "idle";
  const error = value["error"];
  return { state, error: typeof error === "string" ? error : null };
}

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => { window.setTimeout(resolve, Math.max(0, ms)); });
}

/**
 * Poll `oauth.status` until the background flow finishes (or the deadline passes).
 *
 * `timeoutMs` bounds the *polling*; the host bounds the browser interaction itself
 * (`timeout_seconds` in `oauth.authorize`).
 */
export async function waitForOAuthFlow(
  call: OAuthCall,
  connectorId: string,
  options: { timeoutMs?: number; pollMs?: number } = {},
): Promise<OAuthFlowStatus> {
  const timeoutMs = options.timeoutMs ?? 190_000;
  const pollMs = options.pollMs ?? 500;
  const deadline = Date.now() + timeoutMs;
  for (;;) {
    const status = parseOAuthFlowStatus(
      await call("oauth.status", { connector_id: connectorId }).catch(() => null),
    );
    if (status !== null && status.state !== "pending") {
      return status;
    }
    if (Date.now() >= deadline) {
      return { state: "failed", error: "OAUTH_TIMEOUT" };
    }
    await sleep(pollMs);
  }
}

/** Parse tokens returned by the host keychain (opaque JSON string). */
export function parseOAuthTokens(raw: unknown): OAuthTokens | null {
  if (typeof raw !== "string") return null;
  try {
    const value: unknown = JSON.parse(raw);
    if (!isRecord(value)) return null;
    const access = readString(value, "access_token");
    const refresh = readString(value, "refresh_token");
    const expiresAt = readNumber(value, "expires_at");
    return access !== null && refresh !== null && expiresAt !== null
      ? { access_token: access, refresh_token: refresh, expires_at: expiresAt }
      : null;
  } catch {
    return null;
  }
}

/** Whether the access token is expired or expires within the 60s refresh skew. */
export function tokenNeedsRefresh(tokens: OAuthTokens, now = Date.now()): boolean {
  return tokens.expires_at - now < 60_000;
}
