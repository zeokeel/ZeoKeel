/** Enterprise auth session (M2 P4). Tokens stay in local storage for now; keychain later. */
export interface AuthSession {
  access_token: string;
  refresh_token: string;
  expires_in: number;
  user_id: string;
  role: string;
  username: string;
}

const KEY = "zk.enterprise.session";

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
  };
}
