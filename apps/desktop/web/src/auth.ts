import { clientHeaders } from "./client";
import { isRecord, readNumber, readString } from "@zeokeel/ui-kit";
import { readJsonOrUpgrade } from "./upgrade";
import { serverFetch } from "./client";

/** Enterprise auth session (M2 P4). Tokens stay in local storage for now; keychain later. */
export interface AuthSession {
  access_token: string;
  refresh_token: string;
  expires_in: number;
  user_id: string;
  role: string;
  username: string;
  device_name: string;
}

const KEY = "zk.enterprise.session";
const SERVER_KEY = "zk.enterprise.server";
const MODEL_KEY = "zk.enterprise.model";

export function loadSession(): AuthSession | null {
  try {
    const raw = localStorage.getItem(KEY);
    if (raw === null) return null;
    const parsed: unknown = JSON.parse(raw);
    if (!isRecord(parsed)) return null;
    const access = readString(parsed, "access_token");
    const refresh = readString(parsed, "refresh_token");
    if (access === null || refresh === null) return null;
    return {
      access_token: access,
      refresh_token: refresh,
      expires_in: readNumber(parsed, "expires_in") ?? 900,
      user_id: readString(parsed, "user_id") ?? "",
      role: readString(parsed, "role") ?? "employee",
      username: readString(parsed, "username") ?? "",
      device_name: readString(parsed, "device_name") ?? "",
    };
  } catch {
    return null;
  }
}

export function saveSession(session: AuthSession): void {
  localStorage.setItem(KEY, JSON.stringify(session));
}

export function clearSession(): void {
  localStorage.removeItem(KEY);
  localStorage.removeItem(SERVER_KEY);
}

export function loadServerUrl(): string {
  return localStorage.getItem(SERVER_KEY) ?? "";
}

export function saveServerUrl(serverUrl: string): void {
  localStorage.setItem(SERVER_KEY, serverUrl.replace(/\/$/, ""));
}

export function loadGatewayModel(): string {
  return localStorage.getItem(MODEL_KEY) ?? "";
}

export function saveGatewayModel(model: string): void {
  localStorage.setItem(MODEL_KEY, model);
}

function parseTokenResponse(body: unknown): { access_token: string; refresh_token: string; expires_in: number } | null {
  if (!isRecord(body)) return null;
  const access = readString(body, "access_token");
  const refresh = readString(body, "refresh_token");
  if (access === null || refresh === null) return null;
  return { access_token: access, refresh_token: refresh, expires_in: readNumber(body, "expires_in") ?? 900 };
}

export interface BootstrapInfo {
  defaultModel: string;
  models: string[];
}

export async function fetchBootstrapInfo(session: AuthSession, serverUrl: string): Promise<BootstrapInfo> {
  const response = await serverFetch(`${serverUrl.replace(/\/$/, "")}/api/client/v1/config/bootstrap`, {
    headers: { ...clientHeaders(), authorization: `Bearer ${session.access_token}` },
  });
  if (!response.ok) return { defaultModel: "", models: [] };
  const body: unknown = await readJsonOrUpgrade(response);
  if (!isRecord(body)) return { defaultModel: "", models: [] };
  const gateway = body["gateway"];
  if (!isRecord(gateway)) return { defaultModel: "", models: [] };
  const defaultModel = readString(gateway, "default_model") ?? "";
  const rawModels = gateway["models"];
  const models = Array.isArray(rawModels)
    ? rawModels.flatMap((item) => {
        const id = isRecord(item) ? readString(item, "id") : null;
        return id === null ? [] : [id];
      })
    : [];
  return { defaultModel, models };
}

export async function refreshSession(session: AuthSession, serverUrl: string): Promise<AuthSession> {
  const response = await serverFetch(`${serverUrl.replace(/\/$/, "")}/api/client/v1/auth/refresh`, {
    method: "POST",
    headers: { ...clientHeaders(), "content-type": "application/json" },
    body: JSON.stringify({ refresh_token: session.refresh_token, device_id: session.device_name || "client" }),
  });
  const body: unknown = await readJsonOrUpgrade(response);
  const tokens = parseTokenResponse(body);
  if (!response.ok || tokens === null) {
    throw new Error(isRecord(body) ? (readString(body, "error") ?? `HTTP ${String(response.status)}`) : `HTTP ${String(response.status)}`);
  }
  return { ...session, ...tokens };
}

export async function login(serverUrl: string, username: string, password: string, deviceName: string): Promise<AuthSession> {
  const response = await serverFetch(`${serverUrl.replace(/\/$/, "")}/api/client/v1/auth/login`, {
    method: "POST",
    headers: { ...clientHeaders(), "content-type": "application/json" },
    body: JSON.stringify({ username, password, device_id: deviceName }),
  });
  const body: unknown = await readJsonOrUpgrade(response);
  const tokens = parseTokenResponse(body);
  if (!response.ok || tokens === null) {
    throw new Error(isRecord(body) ? (readString(body, "error") ?? `HTTP ${String(response.status)}`) : `HTTP ${String(response.status)}`);
  }
  const user = isRecord(body) && isRecord(body["user"]) ? body["user"] : null;
  return {
    ...tokens,
    user_id: user === null ? "" : (readString(user, "id") ?? ""),
    role: user === null ? "employee" : (readString(user, "role") ?? "employee"),
    username: user === null ? username : (readString(user, "username") ?? username),
    device_name: deviceName,
  };
}
