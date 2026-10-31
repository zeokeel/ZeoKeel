/** Shared admin API/CSRF/session state (no page/domain logic). */
import { decodeAdminOk, type Decoder } from "./validation";

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

/** Fetch + parse + error-envelope check shared by every admin request. */
async function responseBody(path: string, init: RequestInit): Promise<unknown> {
  const response = await fetch(path, { ...init, credentials: "include" });
  const body: unknown = await response.json();
  const failure = apiFailure(body, response.ok, response.status);
  if (failure !== null) throw new Error(failure);
  return body;
}

async function jsonRequest(path: string, init: RequestInit): Promise<unknown> {
  const headers = new Headers(init.headers);
  headers.set("content-type", "application/json");
  if (csrf !== "") headers.set("x-csrf-token", csrf);
  return responseBody(path, { ...init, headers });
}

/**
 * Admin JSON mutation whose payload pages ignore: validates the contract `ok`
 * envelope before resolving, so a malformed body is reported instead of silently
 * accepted (C10). Rejections surface through `messageOf` in the page error slot.
 */
export async function api(path: string, init: RequestInit = {}): Promise<void> {
  decodeAdminOk(await jsonRequest(path, init));
}

/**
 * Admin JSON request decoded through a generated contract schema (C10/T22).
 * The caller supplies the generated decoder; no generic type assertion here.
 */
export async function apiJson<T>(path: string, decode: Decoder<T>, init: RequestInit = {}): Promise<T> {
  return decode(await jsonRequest(path, init));
}

/** Multipart POST with CSRF (zip upload; no content-type override). */
export async function apiForm(path: string, form: FormData): Promise<void> {
  const headers = new Headers();
  if (csrf !== "") headers.set("x-csrf-token", csrf);
  decodeAdminOk(await responseBody(path, { method: "POST", headers, body: form }));
}

// Generated wire types (T22: no hand-written DTO mirrors; the server contract is
// the single source of truth, pages consume these generated names).
export type {
  AdminAppItem,
  AdminAuditItem,
  AdminConnectorItem,
  AdminJobItem,
  AdminSettingItem,
  AdminSkillItem,
  AdminUsageItem,
  AdminUserItem,
  AdminWalletItem,
  ModelPrice,
} from "../../../../../shared/contracts/ts/contracts";

export {
  validateLogin,
  validateAdminLogin,
  validateCreateUser,
  validateAdjust,
  assertValid,
  decodeAdminLogin,
  decodeAdminUsers,
  decodeAdminWallet,
  decodeAdminPrices,
  decodeAdminAudit,
  decodeAdminUsage,
  decodeAdminJobs,
  decodeAdminSettings,
  decodeAdminSkills,
  decodeAdminConnectors,
  decodeAdminApps,
} from "./validation";
export { adminT, adminTranslate, getAdminLocale, setAdminLocale } from "./i18n";
export type { AdminLocale, AdminMessageKey } from "./i18n";
