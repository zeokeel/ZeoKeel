/**
 * Forced-upgrade (HTTP 426 `CLIENT_TOO_OLD`) boundary helpers (round-5 §1/§2).
 *
 * The enterprise server rejects too-old clients with 426 and a body carrying the
 * current/minimum versions and (optionally) the portal URL. Any 426 is treated as a
 * forced upgrade (fail-closed); the app shows a non-dismissable full-screen gate and
 * never retries business calls automatically.
 */

import { isRecord } from "@zeokeel/ui-kit";

export interface UpgradeInfo {
  /** Client version reported by the server, when present. */
  current: string | null;
  /** Minimum supported client version, when present. */
  minimum: string | null;
  /** Absolute or relative download portal URL, when present. */
  portalUrl: string | null;
}

export class UpgradeRequiredError extends Error {
  readonly info: UpgradeInfo;

  constructor(info: UpgradeInfo) {
    super("CLIENT_TOO_OLD");
    this.name = "UpgradeRequiredError";
    this.info = info;
  }
}

function readStringField(record: Record<string, unknown>, keys: readonly string[]): string | null {
  for (const key of keys) {
    const value = record[key];
    if (typeof value === "string" && value.trim() !== "") {
      return value.trim();
    }
  }
  return null;
}

function infoFromRecord(record: Record<string, unknown>): UpgradeInfo {
  return {
    current: readStringField(record, ["current", "current_version", "client_version", "version"]),
    minimum: readStringField(record, ["minimum", "min", "min_client_version", "minimum_version"]),
    portalUrl: readStringField(record, ["portal_url", "portalUrl", "portal", "download_url"]),
  };
}

function hasTooOldCode(record: Record<string, unknown>): boolean {
  const candidates = [record["error"], record["code"], record["message"]];
  for (const candidate of candidates) {
    if (typeof candidate === "string" && candidate.includes("CLIENT_TOO_OLD")) {
      return true;
    }
    if (typeof candidate === "object" && candidate !== null) continue;
  }
  return false;
}

/**
 * Parses a 426 response body. Supports both `{error:"CLIENT_TOO_OLD",…}` and
 * `{error:{code:"CLIENT_TOO_OLD",…}}` shapes; any 426 is a forced upgrade even when the
 * body is empty/malformed (status is authoritative; versions become unknown).
 */
export function parseUpgradeInfo(status: number, rawBody: string): UpgradeInfo | null {
  if (status !== 426) {
    return null;
  }
  let parsed: unknown = null;
  try {
    parsed = rawBody === "" ? null : JSON.parse(rawBody);
  } catch {
    parsed = null;
  }
  if (!isRecord(parsed)) {
    return { current: null, minimum: null, portalUrl: null };
  }
  const root = parsed;
  const error = root["error"];
  if (isRecord(error)) {
    const info = infoFromRecord(error);
    return {
      current: info.current ?? infoFromRecord(root).current,
      minimum: info.minimum ?? infoFromRecord(root).minimum,
      portalUrl: info.portalUrl ?? infoFromRecord(root).portalUrl,
    };
  }
  // Flat body or unrecognized shape: still a forced upgrade (status 426).
  const info = infoFromRecord(root);
  return { current: info.current, minimum: info.minimum, portalUrl: info.portalUrl };
}

/** Extracts upgrade info from any thrown transport/auth error, if it is a 426. */
export function upgradeInfoOf(value: unknown): UpgradeInfo | null {
  if (value instanceof UpgradeRequiredError) {
    return value.info;
  }
  if (typeof value === "string" && value.includes("CLIENT_TOO_OLD")) {
    return { current: null, minimum: null, portalUrl: null };
  }
  if (value instanceof Error) {
    if (value.message.includes("CLIENT_TOO_OLD")) {
      return { current: null, minimum: null, portalUrl: null };
    }
    return null;
  }
  if (isRecord(value) && hasTooOldCode(value)) {
    const nested = isRecord(value["error"]) ? value["error"] : value;
    return infoFromRecord(nested);
  }
  return null;
}

/** Builds the portal URL for the current server (`<server_url>/portal`). */
export function portalHref(serverUrl: string): string | null {
  const base = serverUrl.trim().replace(/\/$/, "");
  return base === "" ? null : `${base}/portal`;
}

/** http(s) origin of the active server, or null when unset/invalid (round-6 P2-1). */
function serverOrigin(serverUrl: string): string | null {
  const base = serverUrl.trim();
  if (base === "") return null;
  try {
    const url = new URL(base);
    if (url.protocol !== "http:" && url.protocol !== "https:") return null;
    return url.origin;
  } catch {
    return null;
  }
}

/**
 * Resolves the 426 `portal_url` with scheme/origin hardening (round-6 review P2-1):
 * - only http/https is accepted;
 * - the resolved URL must be same-origin as the active server URL;
 * - `javascript:`/`data:`/`file:`/offsite https/protocol-relative (`//evil`) all fall back to
 *   `<server_origin>/portal`;
 * - same-origin relative (`/portal`, `portal`) and absolute URLs are allowed;
 * - no usable server origin → no portal link (`null`).
 */
export function resolvePortalHref(portalUrl: string | null, serverUrl: string): string | null {
  const origin = serverOrigin(serverUrl);
  if (origin === null) return null;
  const fallback = `${origin}/portal`;
  if (portalUrl === null || portalUrl.trim() === "") return fallback;
  try {
    const candidate = new URL(portalUrl.trim(), `${origin}/`);
    if (candidate.protocol !== "http:" && candidate.protocol !== "https:") return fallback;
    if (candidate.origin !== origin) return fallback;
    return candidate.toString();
  } catch {
    return fallback;
  }
}

/** Fetches a server response body, throwing `UpgradeRequiredError` on HTTP 426. */
export async function readJsonOrUpgrade(response: Response): Promise<unknown> {
  if (response.status === 426) {
    const text = await response.text();
    throw new UpgradeRequiredError(parseUpgradeInfo(426, text) ?? { current: null, minimum: null, portalUrl: null });
  }
  if (response.status === 204) {
    return null;
  }
  const text = await response.text();
  if (text === "") {
    return null;
  }
  try {
    return JSON.parse(text);
  } catch {
    return null;
  }
}
