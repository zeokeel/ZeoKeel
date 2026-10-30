/** Client identity headers required by the server business surface (`docs/04` §2). */
import { BRAND } from "./generated/brand";

const DEVICE_KEY = "zeokeel.device_id";

function deviceId(): string {
  try {
    const existing = window.localStorage.getItem(DEVICE_KEY);
    if (existing !== null && existing !== "") return existing;
    const generated = typeof crypto.randomUUID === "function" ? crypto.randomUUID() : `${String(Date.now())}-${String(Math.random())}`;
    window.localStorage.setItem(DEVICE_KEY, generated);
    return generated;
  } catch {
    return "desktop-client";
  }
}

/** `X-ZeoKeel-Client: <semver>;<channel>;<platform>;<device_id>`. */
export function clientHeaders(): Record<string, string> {
  return { "X-ZeoKeel-Client": `${BRAND.clientVersion};${BRAND.channel};desktop;${deviceId()}` };
}
