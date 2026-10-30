import { test, expect } from "@playwright/test";

const UI_URL = process.env["ZEOKEEL_UPGRADE_URL"];
const SERVER_URL = process.env["ZEOKEEL_UPGRADE_SERVER"] ?? "http://127.0.0.1:8798";

test.describe("forced upgrade gate (real 426 server)", () => {
  test.skip(UI_URL === undefined, "set ZEOKEEL_UPGRADE_URL to run via scripts/client-upgrade-e2e.sh");

  test("login succeeds, business call 426s, gate shows versions/portal and never retries", async ({ page }) => {
    const upgradeResponses: string[] = [];
    page.on("response", (response) => {
      if (response.status() === 426) upgradeResponses.push(response.url());
    });

    await page.goto(`${UI_URL}/`, { waitUntil: "networkidle" });
    await expect(page.getByRole("heading", { name: "连接企业服务器" })).toBeVisible();
    await page.getByLabel("服务器地址").fill(SERVER_URL);
    await page.getByLabel("账号").fill("admin");
    await page.getByLabel("密码").fill("admin-pass");
    await page.getByRole("button", { name: "登录" }).click();

    // Login (exempt from the business gate) succeeds; the next business call returns 426.
    const gate = page.getByTestId("upgrade-gate");
    await expect(gate).toBeVisible({ timeout: 20_000 });
    await expect(page.getByRole("heading", { name: "需要更新客户端" })).toBeVisible();
    await expect(gate).toContainText("0.1.0");
    await expect(gate).toContainText("9.9.9");
    const portal = page.getByRole("link", { name: "打开下载页" });
    await expect(portal).toBeVisible();
    await expect(portal).toHaveAttribute("href", `${SERVER_URL}/portal`);
    await page.screenshot({ path: "target/evidence/client-upgrade/gate.png" });

    // No business UI, no dismiss/retry controls.
    await expect(page.getByRole("button", { name: "命令面板" })).toHaveCount(0);
    await expect(page.getByRole("button", { name: "登录" })).toHaveCount(0);
    await expect(gate.getByRole("button")).toHaveCount(0);

    // No infinite automatic retry: the 426 request count stays stable while the gate is shown.
    const before = upgradeResponses.length;
    await page.waitForTimeout(1500);
    const after = upgradeResponses.length;
    expect(after).toBeLessThanOrEqual(before);
    await expect(gate).toBeVisible();
    expect(upgradeResponses.length).toBeGreaterThanOrEqual(1);
  });
});
