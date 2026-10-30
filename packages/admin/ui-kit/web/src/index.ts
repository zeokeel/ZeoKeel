/** Shared admin API/CSRF/session state (no page/domain logic). */
let csrf = "";

export function setCsrf(token: string): void { csrf = token; }
export function clearCsrf(): void { csrf = ""; }

export interface ApiErrorBody { ok?: boolean; error?: string }

export async function api<T>(path: string, init: RequestInit = {}): Promise<T> {
  const headers = new Headers(init.headers);
  headers.set("content-type", "application/json");
  if (csrf !== "") headers.set("x-csrf-token", csrf);
  const response = await fetch(path, { ...init, headers, credentials: "include" });
  const body = (await response.json()) as ApiErrorBody & T;
  if (!response.ok || body.ok === false) throw new Error(body.error ?? `HTTP ${response.status}`);
  return body;
}

export interface User { id: string; username: string; display_name: string; role: string; status: string }
export interface Wallet { user_id: string; username: string; balance_micros: number }
export interface Price { model: string; input_per_1k_micros: number; output_per_1k_micros: number; enabled: boolean }
export interface AuditItem { id: number; action: string; target: string; actor_id: string | null }
export interface UsageItem { username: string; requests: number; input_tokens: number; output_tokens: number; cost_micros: number }
export interface JobItem { job: string; idempotency_key: string; status: string; detail: unknown }
export interface SettingItem { key: string; value: unknown; hot: boolean }

export { validateLogin, validateAdminLogin, validateCreateUser, validateAdjust, assertValid } from "./validation";

export interface SkillContent {
  id: string;
  kind: string;
  name: string;
  owner_id: string;
  status: string;
  visibility: string;
  current_release_id: string | null;
  force_update: boolean;
  min_client_version: string;
  version: string;
}

/** Multipart POST with CSRF (zip upload; no content-type override). */
export async function apiForm<T>(path: string, form: FormData): Promise<T> {
  const headers = new Headers();
  if (csrf !== "") headers.set("x-csrf-token", csrf);
  const response = await fetch(path, { method: "POST", headers, body: form, credentials: "include" });
  const body = (await response.json()) as ApiErrorBody & T;
  if (!response.ok || body.ok === false) throw new Error(body.error ?? `HTTP ${response.status}`);
  return body;
}

export interface ConnectorGrant { grantee: string; can_enable: boolean }
export interface ConnectorAdmin {
  id: string;
  name: string;
  transport: string;
  force_update: boolean;
  min_client_version: string;
  grants: ConnectorGrant[];
  oauth_client_id: string | null;
  redirect_uri: string | null;
}
