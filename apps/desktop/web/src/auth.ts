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
    if (typeof parsed !== "object" || parsed === null) return null;
    const value = parsed as Record<string, unknown>;
    if (typeof value["access_token"] !== "string" || typeof value["refresh_token"] !== "string") return null;
    return {
      access_token: value["access_token"],
      refresh_token: value["refresh_token"],
      expires_in: typeof value["expires_in"] === "number" ? value["expires_in"] : 900,
      user_id: typeof value["user_id"] === "string" ? value["user_id"] : "",
      role: typeof value["role"] === "string" ? value["role"] : "employee",
      username: typeof value["username"] === "string" ? value["username"] : "",
      device_name: typeof value["device_name"] === "string" ? value["device_name"] : "",
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

export async function fetchBootstrapModel(session: AuthSession, serverUrl: string): Promise<string> {
  const response = await fetch(`${serverUrl.replace(/\/$/, "")}/api/client/v1/config/bootstrap`, {
    headers: { authorization: `Bearer ${session.access_token}` },
  });
  if (!response.ok) return "";
  const body: unknown = await response.json();
  if (typeof body !== "object" || body === null) return "";
  const gateway = (body as { gateway?: { default_model?: unknown } }).gateway;
  return typeof gateway?.default_model === "string" ? gateway.default_model : "";
}

export async function refreshSession(session: AuthSession, serverUrl: string): Promise<AuthSession> {
  const response = await fetch(`${serverUrl.replace(/\/$/, "")}/api/client/v1/auth/refresh`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ refresh_token: session.refresh_token, device_id: session.device_name || "client" }),
  });
  const body = (await response.json()) as { ok?: boolean; error?: string; access_token?: string; refresh_token?: string; expires_in?: number };
  if (!response.ok || body.ok === false || typeof body.access_token !== "string" || typeof body.refresh_token !== "string") {
    throw new Error(body.error ?? `HTTP ${response.status}`);
  }
  return {
    ...session,
    access_token: body.access_token,
    refresh_token: body.refresh_token,
    expires_in: typeof body.expires_in === "number" ? body.expires_in : 900,
  };
}

export async function login(serverUrl: string, username: string, password: string, deviceName: string): Promise<AuthSession> {
  const response = await fetch(`${serverUrl.replace(/\/$/, "")}/api/client/v1/auth/login`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ username, password, device_id: deviceName }),
  });
  const body = (await response.json()) as { ok?: boolean; error?: string; access_token?: string; refresh_token?: string; expires_in?: number; user?: { id?: string; role?: string; username?: string } };
  if (!response.ok || body.ok === false || typeof body.access_token !== "string" || typeof body.refresh_token !== "string") {
    throw new Error(body.error ?? `HTTP ${response.status}`);
  }
  return {
    access_token: body.access_token,
    refresh_token: body.refresh_token,
    expires_in: typeof body.expires_in === "number" ? body.expires_in : 900,
    user_id: body.user?.id ?? "",
    role: body.user?.role ?? "employee",
    username: body.user?.username ?? username,
    device_name: deviceName,
  };
}
