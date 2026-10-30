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
  if (typeof body !== "object" || body === null) throw new Error("OAUTH_BAD_RESPONSE");
  const value = body as Record<string, unknown>;
  if (typeof value["access_token"] !== "string" || typeof value["refresh_token"] !== "string") throw new Error("OAUTH_BAD_RESPONSE");
  const expiresIn = typeof value["expires_in"] === "number" ? value["expires_in"] : 3600;
  return { access_token: value["access_token"], refresh_token: value["refresh_token"], expires_at: now + expiresIn * 1000 };
}

export async function exchangeCode(metadata: OAuthMetadata, code: string, verifier: string, now = Date.now()): Promise<OAuthTokens> {
  const response = await fetch(metadata.tokenUrl, {
    method: "POST",
    headers: { "content-type": "application/x-www-form-urlencoded" },
    body: new URLSearchParams({ grant_type: "authorization_code", code, client_id: metadata.clientId, redirect_uri: metadata.redirectUri, code_verifier: verifier }).toString(),
  });
  if (!response.ok) throw new Error(`OAUTH_HTTP_${response.status}`);
  return parseTokens(await response.json(), now);
}

export async function refreshOAuthToken(metadata: OAuthMetadata, tokens: OAuthTokens, now = Date.now()): Promise<OAuthTokens> {
  const response = await fetch(metadata.tokenUrl, {
    method: "POST",
    headers: { "content-type": "application/x-www-form-urlencoded" },
    body: new URLSearchParams({ grant_type: "refresh_token", refresh_token: tokens.refresh_token, client_id: metadata.clientId }).toString(),
  });
  if (!response.ok) throw new Error(`OAUTH_HTTP_${response.status}`);
  return parseTokens(await response.json(), now);
}

export function tokenNeedsRefresh(tokens: OAuthTokens, now = Date.now()): boolean {
  return tokens.expires_at - now < 60_000;
}
