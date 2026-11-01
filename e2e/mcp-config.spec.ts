import { test, expect } from "@playwright/test";
import { join } from "node:path";
import { writeFileSync } from "node:fs";
import AxeBuilder from "@axe-core/playwright";

const SERVE = "http://127.0.0.1:8788";
const SCRIPT = join(process.cwd(), "e2e", "fixtures", "fake_mcp.py");

test.describe("MCP local config (real personal serve)", () => {
  // P2-N4: the mock stage sets ZEOKEEL_E2E_SKIP_SERVE=1; the serve stage runs this.
  test.skip(process.env["ZEOKEEL_E2E_SKIP_SERVE"] === "1", "mock stage");

  test("add a stdio server, test connection, save and survive reload", async ({ page }) => {
    await page.goto(`${SERVE}/`, { waitUntil: "networkidle" });
    await page.getByRole("button", { name: /设置|Settings/ }).first().click();
    const dialog = page.getByRole("dialog");
    await dialog.getByRole("button", { name: /MCP/ }).click();
    // a11y evidence for the new pane (0 serious/critical, same threshold as other e2e).
    const axe = await new AxeBuilder({ page }).include("[role=dialog]").analyze();
    const serious = axe.violations.filter((violation) => violation.impact === "serious" || violation.impact === "critical");
    writeFileSync("target/evidence/2026-11-01-client-mcp-config/axe.json", JSON.stringify({ serious }, null, 2));
    expect(serious, JSON.stringify(serious.map((violation) => violation.id))).toEqual([]);

    await dialog.getByRole("button", { name: /添加 MCP server|Add MCP server/ }).click();
    await dialog.getByLabel(/ID/).fill("e2e-mcp");
    await dialog.getByLabel(/名称|Name/).fill("E2E MCP");
    await dialog.getByLabel(/命令|Command/).fill("python3");
    await dialog.getByLabel(/参数|Arguments/).fill(SCRIPT);
    await dialog.getByRole("button", { name: /应用|Apply/ }).click();
    await expect(dialog.getByText("E2E MCP")).toBeVisible();

    // No-cache probe through mcp.test + mcp.test.status polling.
    await dialog.getByRole("button", { name: /测试连接|Test connection/ }).click();
    await expect(dialog.getByText(/连接成功|Connected/)).toBeVisible({ timeout: 30_000 });
    await page.screenshot({ path: "target/evidence/2026-11-01-client-mcp-config/screenshots/mcp-pane-test-connection.png" });

    await dialog.getByRole("button", { name: /保存|Save/ }).click();
    await expect(page.getByText(/设置已保存|Settings saved/)).toBeVisible({ timeout: 20_000 });
    await expect(dialog).toBeHidden({ timeout: 20_000 });

    // Reload: the persisted local server is listed again (settings.json).
    await page.reload({ waitUntil: "networkidle" });
    await page.getByRole("button", { name: /设置|Settings/ }).first().click();
    const reloaded = page.getByRole("dialog");
    await reloaded.getByRole("button", { name: /MCP/ }).click();
    await expect(reloaded.getByText("E2E MCP")).toBeVisible();

    // Cleanup so later specs observe pristine settings.
    await reloaded.getByRole("button", { name: /删除|Delete/ }).click();
    await reloaded.getByRole("button", { name: /保存|Save/ }).click();
    await expect(page.getByText(/设置已保存|Settings saved/)).toBeVisible({ timeout: 20_000 });
  });
});
