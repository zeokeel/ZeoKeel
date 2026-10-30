/** OAuth public-client + PKCE helpers (tokens stay client-side; ADR-030/060). */

export interface PkcePair {
  verifier: string;
  challenge: string;
}

export interface OAuthMetadata {
  authorizeUrl: string;
  tokenUrl: string;
  clientId: string;
  redirectUri: string;
  scopes: string[];
}

import { isRecord, readNumber, readString } from "@zeokeel/ui-kit";

export interface OAuthTokens {
  access_token: string;
  refresh_token: string;
  expires_at: number;
}

function base64Url(bytes: Uint8Array): string {
  let binary = "";
  for (const byte of bytes) binary += String.fromCharCode(byte);
  return btoa(binary).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}

function randomVerifier(): string {
  const bytes = new Uint8Array(32);
  crypto.getRandomValues(bytes);
  return base64Url(bytes);
}

export async function createPkcePair(): Promise<PkcePair> {
  const verifier = randomVerifier();
  const digest = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(verifier));
  return { verifier, challenge: base64Url(new Uint8Array(digest)) };
}

export function buildAuthorizeUrl(metadata: OAuthMetadata, challenge: string, state: string): string {
  const url = new URL(metadata.authorizeUrl);
  url.searchParams.set("response_type", "code");
  url.searchParams.set("client_id", metadata.clientId);
  url.searchParams.set("redirect_uri", metadata.redirectUri);
  url.searchParams.set("scope", metadata.scopes.join(" "));
  url.searchParams.set("code_challenge", challenge);
  url.searchParams.set("code_challenge_method", "S256");
  url.searchParams.set("state", state);
  return url.toString();
}

function parseTokens(body: unknown, now: number): OAuthTokens {
  if (!isRecord(body)) throw new Error("OAUTH_BAD_RESPONSE");
  const access = readString(body, "access_token");
  const refresh = readString(body, "refresh_token");
  if (access === null || refresh === null) throw new Error("OAUTH_BAD_RESPONSE");
  const expiresIn = readNumber(body, "expires_in") ?? 3600;
  return { access_token: access, refresh_token: refresh, expires_at: now + expiresIn * 1000 };
}

export async function exchangeCode(metadata: OAuthMetadata, code: string, verifier: string, now = Date.now()): Promise<OAuthTokens> {
  const response = await fetch(metadata.tokenUrl, {
    method: "POST",
    headers: { "content-type": "application/x-www-form-urlencoded" },
    body: new URLSearchParams({ grant_type: "authorization_code", code, client_id: metadata.clientId, redirect_uri: metadata.redirectUri, code_verifier: verifier }).toString(),
  });
  if (!response.ok) throw new Error(`OAUTH_HTTP_${String(response.status)}`);
  return parseTokens(await response.json(), now);
}

export async function refreshOAuthToken(metadata: OAuthMetadata, tokens: OAuthTokens, now = Date.now()): Promise<OAuthTokens> {
  const response = await fetch(metadata.tokenUrl, {
    method: "POST",
    headers: { "content-type": "application/x-www-form-urlencoded" },
    body: new URLSearchParams({ grant_type: "refresh_token", refresh_token: tokens.refresh_token, client_id: metadata.clientId }).toString(),
  });
  if (!response.ok) throw new Error(`OAUTH_HTTP_${String(response.status)}`);
  return parseTokens(await response.json(), now);
}

export function tokenNeedsRefresh(tokens: OAuthTokens, now = Date.now()): boolean {
  return tokens.expires_at - now < 60_000;
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
  } catch { return null; }
}

const PENDING_KEY = "zk.connector.pkce";

interface PendingAuth { connectorId: string; verifier: string; state: string; metadata: OAuthMetadata }

export function savePendingAuth(pending: PendingAuth): void {
  sessionStorage.setItem(PENDING_KEY, JSON.stringify(pending));
}

export function takePendingAuth(): PendingAuth | null {
  const raw = sessionStorage.getItem(PENDING_KEY);
  if (raw === null) return null;
  sessionStorage.removeItem(PENDING_KEY);
  try {
    const parsed: unknown = JSON.parse(raw);
    if (!isRecord(parsed)) return null;
    const connectorId = readString(parsed, "connectorId");
    const verifier = readString(parsed, "verifier");
    const stateValue = readString(parsed, "state");
    const rawMetadata = parsed["metadata"];
    if (connectorId === null || verifier === null || stateValue === null || !isRecord(rawMetadata)) return null;
    const authorizeUrl = readString(rawMetadata, "authorizeUrl");
    const tokenUrl = readString(rawMetadata, "tokenUrl");
    const clientId = readString(rawMetadata, "clientId");
    const redirectUri = readString(rawMetadata, "redirectUri");
    const rawScopes = rawMetadata["scopes"];
    if (authorizeUrl === null || tokenUrl === null || clientId === null || redirectUri === null || !Array.isArray(rawScopes)) return null;
    const scopes = rawScopes.flatMap((scope) => (typeof scope === "string" ? [scope] : []));
    return { connectorId, verifier, state: stateValue, metadata: { authorizeUrl, tokenUrl, clientId, redirectUri, scopes } };
  } catch { return null; }
}
