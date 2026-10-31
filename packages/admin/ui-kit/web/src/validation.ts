/** Runtime validation generated from Rust JSON Schema (T22/C10). */
import Ajv, { type ValidateFunction } from "ajv";
import {
  admin_apps_responseSchema,
  admin_audit_responseSchema,
  admin_connectors_responseSchema,
  admin_jobs_responseSchema,
  admin_login_responseSchema,
  admin_ok_responseSchema,
  admin_price_list_responseSchema,
  admin_settings_responseSchema,
  admin_skills_responseSchema,
  admin_usage_responseSchema,
  admin_users_responseSchema,
  admin_wallet_responseSchema,
  admin_loginSchema,
  auth_loginSchema,
  user_createSchema,
  wallet_adjustSchema,
  type AdminAppsResponse,
  type AdminAuditResponse,
  type AdminConnectorsResponse,
  type AdminJobsResponse,
  type AdminLoginResponse,
  type AdminOkResponse,
  type AdminPriceListResponse,
  type AdminSettingsResponse,
  type AdminSkillsResponse,
  type AdminUsageResponse,
  type AdminUsersResponse,
  type AdminWalletResponse,
} from "../../../../../shared/contracts/ts/contracts";

const ajv = new Ajv({ allErrors: true, strict: false, logger: false });

/** Request validators (generated schemas; see `assertValid`). */
export const validateLogin = ajv.compile(auth_loginSchema);
export const validateAdminLogin = ajv.compile(admin_loginSchema);
export const validateCreateUser = ajv.compile(user_createSchema);
export const validateAdjust = ajv.compile(wallet_adjustSchema);

export function assertValid(check: { (data: unknown): boolean; errors?: unknown }, value: unknown, label: string): void {
  if (!check(value)) throw new Error(`INVALID_${label}`);
}

/** Runtime decoder produced from a generated JSON Schema. */
export type Decoder<T> = (value: unknown) => T;

function describe(errors: ValidateFunction["errors"]): string {
  if (errors === null || errors === undefined) {
    return "";
  }
  return errors
    .slice(0, 3)
    .map((error) => `${error.instancePath === "" ? "/" : error.instancePath} ${error.message ?? ""}`.trim())
    .join("; ");
}

/**
 * Decode an untrusted body through a generated-schema validator. Ajv compiles
 * schemas to type predicates, so a mismatch is logged (bounded field list) and
 * rejected with `INVALID_<label>`; success narrows `unknown` to `T` without an
 * assertion (C9/C10, replaces the former ADR-059 `as T` suppression).
 */
function decoder<T>(check: ValidateFunction<T>, label: string): Decoder<T> {
  return (value: unknown): T => {
    if (check(value)) {
      return value;
    }
    console.warn(`[zk-admin] response schema invalid: ${label}`, describe(check.errors));
    throw new Error(`INVALID_${label}`);
  };
}

/** `POST /api/admin/auth/login` */
export const decodeAdminLogin = decoder(ajv.compile<AdminLoginResponse>(admin_login_responseSchema), "ADMIN_LOGIN");
/** Success envelope of admin mutations whose payload pages ignore. */
export const decodeAdminOk = decoder(ajv.compile<AdminOkResponse>(admin_ok_responseSchema), "ADMIN_OK");
/** `GET /api/admin/users` */
export const decodeAdminUsers = decoder(ajv.compile<AdminUsersResponse>(admin_users_responseSchema), "ADMIN_USERS");
/** `GET /api/admin/wallet` */
export const decodeAdminWallet = decoder(ajv.compile<AdminWalletResponse>(admin_wallet_responseSchema), "ADMIN_WALLET");
/** `GET /api/admin/pricing` */
export const decodeAdminPrices = decoder(
  ajv.compile<AdminPriceListResponse>(admin_price_list_responseSchema),
  "ADMIN_PRICES",
);
/** `GET /api/admin/audit` */
export const decodeAdminAudit = decoder(ajv.compile<AdminAuditResponse>(admin_audit_responseSchema), "ADMIN_AUDIT");
/** `GET /api/admin/usage` */
export const decodeAdminUsage = decoder(ajv.compile<AdminUsageResponse>(admin_usage_responseSchema), "ADMIN_USAGE");
/** `GET /api/admin/jobs` */
export const decodeAdminJobs = decoder(ajv.compile<AdminJobsResponse>(admin_jobs_responseSchema), "ADMIN_JOBS");
/** `GET /api/admin/settings` */
export const decodeAdminSettings = decoder(
  ajv.compile<AdminSettingsResponse>(admin_settings_responseSchema),
  "ADMIN_SETTINGS",
);
/** `GET /api/admin/skills` */
export const decodeAdminSkills = decoder(ajv.compile<AdminSkillsResponse>(admin_skills_responseSchema), "ADMIN_SKILLS");
/** `GET /api/admin/connectors` */
export const decodeAdminConnectors = decoder(
  ajv.compile<AdminConnectorsResponse>(admin_connectors_responseSchema),
  "ADMIN_CONNECTORS",
);
/** `GET /api/admin/apps` */
export const decodeAdminApps = decoder(ajv.compile<AdminAppsResponse>(admin_apps_responseSchema), "ADMIN_APPS");
