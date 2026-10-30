import { defineConfig, devices } from "@playwright/test";

const MOCK_URL = "http://127.0.0.1:4173";

export default defineConfig({
  testDir: "e2e",
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
      reuseExistingServer: true,
      timeout: 180_000,
    },
    {
      command:
        "mkdir -p target/evidence/m1/home/demo-data && cp demo-data/hello.txt target/evidence/m1/home/demo-data/hello.txt && cargo build -q -p zk-desktop && ZEOKEEL_HOME=target/evidence/m1/home ZEOKEEL_SERVE_ADDR=127.0.0.1:8788 ./target/debug/zk-desktop --serve",
      url: "http://127.0.0.1:8788/healthz",
      reuseExistingServer: true,
      timeout: 240_000,
    },
  ],
});
