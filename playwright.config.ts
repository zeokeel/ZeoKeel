import { defineConfig, devices } from "@playwright/test";

const MOCK_URL = "http://127.0.0.1:4173";

export default defineConfig({
  testDir: "e2e",
  // Agent-rig suite owns its own compose/env lifecycle (scripts/agent-e2e.sh + playwright.agent.config.ts).
  testIgnore: ["**/agent-rig.spec.ts"],
  timeout: 60_000,
  expect: { timeout: 10_000 },
  fullyParallel: false,
  workers: 1,
  retries: 0,
  reporter: [["list"], ["html", { outputFolder: "target/evidence/m1/playwright-report", open: "never" }]],
  use: {
    baseURL: MOCK_URL,
    trace: "retain-on-failure",
    screenshot: "only-on-failure",
    video: "off",
  },
  projects: [{ name: "chromium", use: { ...devices["Desktop Chrome"], viewport: { width: 1280, height: 820 } } }],
  webServer: [
    {
      command: "pnpm --dir apps/desktop/web build && pnpm --dir apps/desktop/web exec vite preview --host 127.0.0.1 --port 4173",
      url: `${MOCK_URL}/`,
      // Never silently reuse a stale preview/build; CI must exercise the current tree.
      reuseExistingServer: false,
      timeout: 180_000,
    },
    // The mock-profile run sets ZEOKEEL_E2E_SKIP_SERVE=1: the real serve carrier must be
    // built from the committed personal baseline, not from the temporary mock composition.
    ...(process.env["ZEOKEEL_E2E_SKIP_SERVE"] === "1" ? [] : [{
      // Deterministic fake upstream + baseline git workspace + real serve/SQLite.
      command: "bash e2e/serve-m1.sh",
      url: "http://127.0.0.1:8788/healthz",
      reuseExistingServer: false,
      timeout: 300_000,
    }]),
  ],
});
