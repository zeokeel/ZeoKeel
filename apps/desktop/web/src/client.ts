/** Client identity headers required by the server business surface (`docs/04` §2). */
import { isRecord } from "@zeokeel/ui-kit";
import { BRAND } from "./generated/brand";
import { parseUpgradeInfo, UpgradeRequiredError } from "./upgrade";

const DEVICE_KEY = "zeokeel.device_id";

function deviceId(): string {
  try {
    const existing = window.localStorage.getItem(DEVICE_KEY);
    if (existing !== null && existing !== "") return existing;
    const generated = newRequestId();
    window.localStorage.setItem(DEVICE_KEY, generated);
    return generated;
  } catch {
    return "desktop-client";
  }
}

/** uuid used for `X-Request-Id` (server echoes it back). */
export function newRequestId(): string {
  if (typeof crypto.randomUUID === "function") return crypto.randomUUID();
  return `${String(Date.now())}-${String(Math.random())}`;
}

/** Normalized platform id: `linux-x64` | `windows-x64` | `macos` | `other`. */
export function platformId(): string {
  const data: unknown = Reflect.get(navigator, "userAgentData");
  const fromData = isRecord(data) ? data["platform"] : null;
  const legacy: unknown = Reflect.get(navigator, "platform");
  const raw = typeof fromData === "string" ? fromData : typeof legacy === "string" ? legacy : "";
  const value = raw.toLowerCase();
  if (value.includes("linux") || value.includes("x11")) return "linux-x64";
  if (value.includes("win")) return "windows-x64";
  if (value.includes("mac") || value.includes("darwin")) return "macos";
  return "other";
}

/** `X-ZeoKeel-Client: <semver>;<channel>;<platform>;<device_id>`. */
export function clientHeaders(): Record<string, string> {
  return { "X-ZeoKeel-Client": `${BRAND.clientVersion};${BRAND.channel};${platformId()};${deviceId()}` };
}

/** All headers for a direct web→server request (identity + request id). */
export function serverHeaders(): Record<string, string> {
  return { ...clientHeaders(), "X-Request-Id": newRequestId() };
}

/**
 * Unified direct web→server fetch (round-6 §1):
 * - always sends `X-ZeoKeel-Client` + `X-Request-Id`;
 * - HTTP 426 always maps to `UpgradeRequiredError`, so no 426 can silently fall into
 *   `contentError`/generic HTTP errors.
 * Callers then parse JSON/text/binary from the returned response as needed.
 */
export async function serverFetch(url: string, init?: RequestInit): Promise<Response> {
  const headers = new Headers(init?.headers);
  for (const [name, value] of Object.entries(serverHeaders())) {
    headers.set(name, value);
  }
  const response = await fetch(url, { ...init, headers });
  if (response.status === 426) {
    const text = await response.clone().text();
    throw new UpgradeRequiredError(parseUpgradeInfo(426, text) ?? { current: null, minimum: null, portalUrl: null });
  }
  return response;
}
