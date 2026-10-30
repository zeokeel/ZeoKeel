/** Shared admin API/CSRF/session state (no page/domain logic). */
let csrf = "";

/** Narrow an untrusted JSON value to a record (no type assertions at call sites). */
export function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

/** Human-readable message for an unknown thrown value (never leaks an object). */
export function messageOf(error: unknown): string {
  if (error instanceof Error) return error.message;
  if (typeof error === "string") return error;
  return "request failed";
}

export function setCsrf(token: string): void { csrf = token; }
export function clearCsrf(): void { csrf = ""; }

export interface ApiErrorBody { ok?: boolean; error?: string }

function apiFailure(body: unknown, ok: boolean, status: number): string | null {
  const record = isRecord(body) ? body : null;
  const error = record !== null && typeof record["error"] === "string" ? record["error"] : null;
  const okField = record !== null && record["ok"] === false;
  if (ok && !okField) return null;
  return error ?? `HTTP ${String(status)}`;
}

export async function api<T>(path: string, init: RequestInit = {}): Promise<T> {
  const headers = new Headers(init.headers);
  headers.set("content-type", "application/json");
  if (csrf !== "") headers.set("x-csrf-token", csrf);
  const response = await fetch(path, { ...init, headers, credentials: "include" });
  const body: unknown = await response.json();
  const failure = apiFailure(body, response.ok, response.status);
  if (failure !== null) throw new Error(failure);
  // Generic decode boundary: callers declare T and pages validate their request payloads with Ajv (C10/T22).
  // eslint-disable-next-line @typescript-eslint/consistent-type-assertions -- ADR-059/T22 generic response boundary
  return body as T;
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
  const body: unknown = await response.json();
  const failure = apiFailure(body, response.ok, response.status);
  if (failure !== null) throw new Error(failure);
  // eslint-disable-next-line @typescript-eslint/consistent-type-assertions -- ADR-059/T22 generic response boundary
  return body as T;
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

export interface AppAdmin { id: string; name: string; status: string; owner_id: string; version: string; wasm_sha256: string }
