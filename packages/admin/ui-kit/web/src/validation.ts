/** Runtime validation generated from Rust JSON Schema (T22/C10). */
import Ajv from "ajv";
import { admin_loginSchema, auth_loginSchema, user_createSchema, wallet_adjustSchema } from "../../../../../shared/contracts/ts/contracts";

const ajv = new Ajv({ allErrors: true, strict: false });
export const validateLogin = ajv.compile(auth_loginSchema);
export const validateAdminLogin = ajv.compile(admin_loginSchema);
export const validateCreateUser = ajv.compile(user_createSchema);
export const validateAdjust = ajv.compile(wallet_adjustSchema);

export function assertValid(check: { (data: unknown): boolean; errors?: unknown }, value: unknown, label: string): void {
  if (!check(value)) throw new Error(`INVALID_${label}`);
}
