import { test, expect } from "@playwright/test";

const SERVE = "http://127.0.0.1:8788";

test.describe("serve carrier security", () => {
  test("rejects unauthenticated IPC and does not leak the provider key", async ({ request }) => {
    const unauthorized = await request.post(`${SERVE}/ipc`, {
      data: { name: "workspace.list", payload: {} },
    });
    expect(unauthorized.status()).toBe(401);

    const index = await request.get(`${SERVE}/`);
    expect(index.ok()).toBeTruthy();

    const set = await request.post(`${SERVE}/ipc`, {
      data: {
        name: "settings.set",
        payload: {
          settings: {
            schema: 1,
            provider: { kind: "open_ai_compatible", base_url: "https://example.com/v1", model: "m", api_key: "sk-SECRET-TEST" },
            theme: "light",
            locale: "zh-CN",
            auto_approve_read_only: true,
          },
        },
      },
    });
    expect(set.ok()).toBeTruthy();
    expect(JSON.stringify(await set.json())).not.toContain("sk-SECRET-TEST");
    const got = await request.post(`${SERVE}/ipc`, {
      data: { name: "settings.get", payload: {} },
    });
    const body = (await got.json()) as { data: { provider: { api_key: string } } };
    expect(body.data.provider.api_key).toBe("");
    expect(JSON.stringify(body)).not.toContain("sk-SECRET-TEST");

    // Restore a provider that does not hot-swap away the env fake upstream: later suites in
    // this shared serve process must not inherit the example.com provider persisted here.
    const restore = await request.post(`${SERVE}/ipc`, {
      data: {
        name: "settings.set",
        payload: {
          settings: {
            schema: 1,
            // Point back at the deterministic fake upstream started by e2e/serve-m1.sh.
            provider: { kind: "open_ai_compatible", base_url: "http://127.0.0.1:8998/v1", model: "test-model", api_key: "test" },
            theme: "light",
            locale: "zh-CN",
            auto_approve_read_only: true,
          },
        },
      },
    });
    expect(restore.ok()).toBeTruthy();
  });

  test("does not ship design prototypes in production", async ({ request }) => {
    for (const path of ["/prototype.html", "/dense.html"]) {
      const response = await request.get(`${SERVE}${path}`);
      expect(response.status()).toBe(404);
    }
  });

  test("does not reflect arbitrary cross-origin access", async ({ request }) => {
    const response = await request.post(`${SERVE}/ipc`, {
      headers: { Origin: "http://evil.example" },
      data: { name: "workspace.list", payload: {} },
    });
    expect(response.headers()["access-control-allow-origin"]).toBeUndefined();
  });
});
