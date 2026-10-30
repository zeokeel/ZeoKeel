/** Enterprise content catalog (skills/connectors) API client. */
import { isRecord } from "@zeokeel/ui-kit";
import type { AuthSession } from "./auth";

export interface SkillItem {
  id: string;
  name: string;
  version: string;
  sha256: string;
  enforced: boolean;
  min_client_version: string;
  can_install: boolean;
}

export interface ConnectorOAuthInfo { clientId: string; scopes: string[]; redirectUri: string; authorizeUrl: string; tokenUrl: string }

export interface ConnectorItem {
  id: string;
  name: string;
  transport: string;
  enabled: boolean;
  can_enable: boolean;
  oauth: ConnectorOAuthInfo | null;
}

function record(value: unknown): Record<string, unknown> | null {
  return isRecord(value) ? value : null;
}

function text(value: unknown): string | null {
  return typeof value === "string" ? value : null;
}

function bool(value: unknown): boolean | null {
  return typeof value === "boolean" ? value : null;
}

export async function fetchSkills(session: AuthSession, serverUrl: string): Promise<SkillItem[]> {
  const response = await fetch(`${serverUrl.replace(/\/$/, "")}/api/client/v1/skills`, {
    headers: { authorization: `Bearer ${session.access_token}` },
  });
  if (!response.ok) throw new Error(`skills HTTP ${String(response.status)}`);
  const body: unknown = await response.json();
  const items = record(body)?.["items"];
  if (!Array.isArray(items)) return [];
  return items.flatMap((item) => {
    const value = record(item);
    if (value === null) return [];
    const name = text(value["name"]);
    const version = text(value["version"]);
    const sha256 = text(value["sha256"]);
    const canInstall = bool(value["can_install"]);
    if (name === null || version === null || sha256 === null || canInstall === null) return [];
    return [{ id: text(value["id"]) ?? name, name, version, sha256, enforced: bool(value["enforced"]) ?? false, min_client_version: text(value["min_client_version"]) ?? "", can_install: canInstall }];
  });
}

export async function fetchConnectors(session: AuthSession, serverUrl: string): Promise<ConnectorItem[]> {
  const response = await fetch(`${serverUrl.replace(/\/$/, "")}/api/client/v1/connectors`, {
    headers: { authorization: `Bearer ${session.access_token}` },
  });
  if (!response.ok) throw new Error(`connectors HTTP ${String(response.status)}`);
  const body: unknown = await response.json();
  const items = record(body)?.["items"];
  if (!Array.isArray(items)) return [];
  return items.flatMap((item) => {
    const value = record(item);
    if (value === null) return [];
    const name = text(value["name"]);
    const transport = text(value["transport"]);
    const canEnable = bool(value["can_enable"]);
    if (name === null || transport === null || canEnable === null) return [];
    const oauthValue = record(value["oauth"]);
    const oauth = oauthValue === null ? null : {
      clientId: text(oauthValue["client_id"]) ?? "",
      scopes: Array.isArray(oauthValue["scopes"]) ? oauthValue["scopes"].flatMap((scope) => typeof scope === "string" ? [scope] : []) : [],
      redirectUri: text(oauthValue["redirect_uri"]) ?? "",
      authorizeUrl: text(oauthValue["authorize_url"]) ?? "",
      tokenUrl: text(oauthValue["token_url"]) ?? "",
    };
    return [{ id: text(value["id"]) ?? name, name, transport, enabled: bool(value["enabled"]) ?? false, can_enable: canEnable, oauth: oauth?.clientId === "" ? null : oauth }];
  });
}

function toBase64(bytes: Uint8Array): string {
  let binary = "";
  const chunk = 0x8000;
  for (let index = 0; index < bytes.length; index += chunk) {
    binary += String.fromCharCode(...bytes.subarray(index, index + chunk));
  }
  return btoa(binary);
}

export async function downloadSkillArchive(session: AuthSession, serverUrl: string, item: SkillItem): Promise<{ archive_base64: string; sha256: string }> {
  const response = await fetch(`${serverUrl.replace(/\/$/, "")}/api/client/v1/skills/${encodeURIComponent(item.name)}/${encodeURIComponent(item.version)}/archive`, {
    headers: { authorization: `Bearer ${session.access_token}` },
  });
  if (!response.ok) throw new Error(`skills download HTTP ${String(response.status)}`);
  const header = response.headers.get("x-zeokeel-skill-sha256");
  if (header !== null && header !== item.sha256) throw new Error("CHECKSUM_MISMATCH");
  const bytes = new Uint8Array(await response.arrayBuffer());
  const digest = await crypto.subtle.digest("SHA-256", bytes);
  const actual = Array.from(new Uint8Array(digest)).map((byte) => byte.toString(16).padStart(2, "0")).join("");
  if (actual !== item.sha256) throw new Error("CHECKSUM_MISMATCH");
  return { archive_base64: toBase64(bytes), sha256: item.sha256 };
}

export async function toggleConnector(session: AuthSession, serverUrl: string, item: ConnectorItem): Promise<void> {
  const action = item.enabled ? "disable" : "enable";
  const response = await fetch(`${serverUrl.replace(/\/$/, "")}/api/client/v1/connectors/${encodeURIComponent(item.id)}/${action}`, {
    method: "POST",
    headers: { authorization: `Bearer ${session.access_token}` },
  });
  if (!response.ok) throw new Error(`connector HTTP ${String(response.status)}`);
}

export async function fetchTombstones(session: AuthSession, serverUrl: string): Promise<string[]> {
  const response = await fetch(`${serverUrl.replace(/\/$/, "")}/api/client/v1/skills/tombstones`, {
    headers: { authorization: `Bearer ${session.access_token}` },
  });
  if (!response.ok) return [];
  const body: unknown = await response.json();
  const items = record(body)?.["items"];
  return Array.isArray(items) ? items.flatMap((item) => (typeof item === "string" ? [item] : [])) : [];
}

export async function sendInstallReceipt(session: AuthSession, serverUrl: string, item: SkillItem, treeSha256: string): Promise<void> {
  const response = await fetch(`${serverUrl.replace(/\/$/, "")}/api/client/v1/skills/${encodeURIComponent(item.name)}/${encodeURIComponent(item.version)}/install`, {
    method: "POST",
    headers: { authorization: `Bearer ${session.access_token}`, "content-type": "application/json" },
    body: JSON.stringify({ tree_sha256: treeSha256 }),
  });
  if (!response.ok) throw new Error(`install receipt HTTP ${String(response.status)}`);
}

export interface AppItem { appId: string; name: string; description: string; version: string; access: string }

export async function fetchApps(session: AuthSession, serverUrl: string): Promise<AppItem[]> {
  const response = await fetch(`${serverUrl.replace(/\/$/, "")}/api/client/v1/apps`, { headers: { authorization: `Bearer ${session.access_token}` } });
  if (!response.ok) throw new Error(`apps HTTP ${String(response.status)}`);
  const body: unknown = await response.json();
  const items = record(body)?.["items"];
  if (!Array.isArray(items)) return [];
  return items.flatMap((item) => {
    const value = record(item);
    if (value === null) return [];
    const appId = text(value["app_id"]);
    const name = text(value["name"]);
    const version = text(value["version"]);
    if (appId === null || name === null || version === null) return [];
    return [{ appId, name, description: text(value["description"]) ?? "", version, access: text(value["access"]) ?? "" }];
  });
}

export async function openApp(session: AuthSession, serverUrl: string, item: AppItem): Promise<{ version: string; entry: string }> {
  const response = await fetch(`${serverUrl.replace(/\/$/, "")}/api/client/v1/apps/${encodeURIComponent(item.appId)}/open`, { method: "POST", headers: { authorization: `Bearer ${session.access_token}` } });
  if (!response.ok) throw new Error(`open HTTP ${String(response.status)}`);
  const version = response.headers.get("x-zeokeel-app-version") ?? item.version;
  let entry = "index.html";
  try {
    const body: unknown = await response.json();
    const manifest = isRecord(body) ? body["ui_manifest"] : undefined;
    if (typeof manifest === "string") {
      const parsed: unknown = JSON.parse(manifest);
      if (Array.isArray(parsed) && typeof parsed[0] === "string") entry = parsed[0];
    }
  } catch { /* keep default */ }
  return { version, entry };
}

export async function fetchAppHtml(session: AuthSession, serverUrl: string, item: AppItem, entry = "index.html"): Promise<string> {
  const response = await fetch(`${serverUrl.replace(/\/$/, "")}/api/client/v1/apps/${encodeURIComponent(item.appId)}/assets/${encodeURIComponent(entry)}`, { headers: { authorization: `Bearer ${session.access_token}` } });
  if (!response.ok) throw new Error(`app asset HTTP ${String(response.status)}`);
  return response.text();
}
