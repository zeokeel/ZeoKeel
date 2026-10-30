import { afterEach, describe, expect, it, vi } from "vitest";
import { UpgradeRequiredError } from "../src/upgrade";
import {
  downloadSkillArchive,
  fetchAppHtml,
  fetchApps,
  fetchConnectors,
  fetchSkills,
  fetchTombstones,
  openApp,
  sendInstallReceipt,
  toggleConnector,
  type ConnectorItem,
  type SkillItem,
} from "../src/store";
import type { AuthSession } from "../src/auth";

const SESSION: AuthSession = {
  access_token: "token",
  refresh_token: "refresh",
  expires_in: 900,
  user_id: "u1",
  role: "employee",
  username: "user",
  device_name: "device",
};

const SKILL: SkillItem = { id: "s1", name: "demo", version: "1.0.0", sha256: "deadbeef", enforced: false, min_client_version: "", can_install: true };
const CONNECTOR: ConnectorItem = { id: "c1", name: "demo", transport: "http", enabled: false, can_enable: true, oauth: null };
const APP = { appId: "a1", name: "app", description: "", version: "1.0.0", access: "all" };

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("store 426 -> forced upgrade (round-6 §1)", () => {
  it.each([
    ["skills", () => fetchSkills(SESSION, "http://srv")],
    ["connectors", () => fetchConnectors(SESSION, "http://srv")],
    ["skill archive", () => downloadSkillArchive(SESSION, "http://srv", SKILL)],
    ["connector toggle", () => toggleConnector(SESSION, "http://srv", CONNECTOR)],
    ["tombstones", () => fetchTombstones(SESSION, "http://srv")],
    ["install receipt", () => sendInstallReceipt(SESSION, "http://srv", SKILL, "tree")],
    ["apps", () => fetchApps(SESSION, "http://srv")],
    ["app open", () => openApp(SESSION, "http://srv", APP)],
    ["app asset", () => fetchAppHtml(SESSION, "http://srv", APP)],
  ])("maps %s 426 to UpgradeRequiredError", async (_name, call) => {
    vi.stubGlobal("fetch", () => Promise.resolve(new Response(JSON.stringify({ error: { code: "CLIENT_TOO_OLD", minimum: "9.9.9" } }), { status: 426 })));
    await expect(call()).rejects.toBeInstanceOf(UpgradeRequiredError);
  });
});
