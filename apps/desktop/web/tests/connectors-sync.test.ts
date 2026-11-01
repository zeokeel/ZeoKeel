import { afterEach, describe, expect, it, vi } from "vitest";
import { clearConnectorCatalog, reloadConnectorSessions, syncConnectorCatalog } from "../src/connectors-sync";
import { fetchConnectorCatalog } from "../src/store";
import { FakeTransport } from "./fake-transport";

afterEach(() => {
  vi.restoreAllMocks();
});

describe("connector catalog → host wiring", () => {
  it("syncs the raw catalog through connectors.configure", async () => {
    const transport = new FakeTransport();
    transport.handle("connectors.configure", (payload) => ({ configured: 2, payload }));
    const catalog = { items: [], catalog: { items: [{ id: "mcp-demo", enabled: true }] } };
    await syncConnectorCatalog(transport, catalog);
    expect(transport.callsFor("connectors.configure")).toEqual([{ catalog: catalog.catalog }]);
  });

  it("reloads sessions and clears catalog config for logout", async () => {
    const transport = new FakeTransport();
    transport.handle("connectors.reload", () => ({ reloaded: true }));
    transport.handle("connectors.clear", () => ({ cleared: true }));
    await reloadConnectorSessions(transport);
    await clearConnectorCatalog(transport);
    expect(transport.calls.map((call) => call.name)).toEqual([
      "connectors.reload",
      "connectors.clear",
    ]);
  });

  it("keeps the raw catalog for the host while parsing items for the panel", async () => {
    const body = {
      items: [
        { id: "mcp-demo", name: "Demo", transport: "stdio", enabled: true, can_enable: true },
      ],
    };
    vi.spyOn(globalThis, "fetch").mockResolvedValue(new Response(JSON.stringify(body), { status: 200 }));
    const catalog = await fetchConnectorCatalog(
      { access_token: "a", refresh_token: "r", expires_in: 900, user_id: "u", role: "employee", username: "alice", device_name: "d" },
      "http://127.0.0.1:8795",
    );
    expect(catalog.items).toHaveLength(1);
    expect(catalog.catalog).toEqual(body);
  });
});
