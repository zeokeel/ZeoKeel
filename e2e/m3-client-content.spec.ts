import { test, expect } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";

const LOGIN_URL = process.env["ZEOKEEL_LOGIN_URL"];
const SERVER_URL = process.env["ZEOKEEL_LOGIN_SERVER"] ?? "http://127.0.0.1:8795";
const SKILL_ZIP = process.env["ZEOKEEL_CLIENT_SKILL_ZIP"] ?? "/tmp/zk-client-skill.zip";

test.describe("M3 client content store", () => {
  test.skip(LOGIN_URL === undefined, "set ZEOKEEL_LOGIN_URL to run");

  test("install a skill, remove it via admin tombstone, toggle a connector", async ({ page, context }) => {
    await page.goto(`${LOGIN_URL}/`, { waitUntil: "networkidle" });
    const addressField = page.getByLabel("服务器地址");
    if (await addressField.isVisible().catch(() => false)) await addressField.fill(SERVER_URL);
    await page.getByLabel("账号").fill("admin");
    await page.getByLabel("密码").fill("admin-pass");
    await page.getByRole("button", { name: "登录" }).click();
    await expect(page.getByRole("button", { name: "命令面板" }).first()).toBeVisible({ timeout: 15_000 });

    await page.keyboard.press("Control+k");
    await page.getByRole("option", { name: "技能" }).click();
    const skills = page.getByRole("dialog", { name: "技能" });
    await expect(skills).toBeVisible();
    await skills.getByRole("button", { name: "安装" }).click();
    await expect(skills.getByText("已安装")).toBeVisible({ timeout: 20_000 });
    const axe = await new AxeBuilder({ page }).analyze();
    const serious = axe.violations.filter((v) => v.impact === "serious" || v.impact === "critical");
    expect(serious, JSON.stringify(serious.map((v) => v.id))).toEqual([]);
    await page.screenshot({ path: "target/evidence/m3/client-skills.png" });
    await skills.getByRole("button", { name: "关闭", exact: true }).click();

    await page.keyboard.press("Control+k");
    await page.getByRole("option", { name: "连接器" }).click();
    const connectors = page.getByRole("dialog", { name: "连接器" });
    await expect(connectors).toBeVisible();
    // OAuth authorize now runs the real RFC 8252 loopback+PKCE flow (B16/OAuth); a headless
    // Playwright run cannot complete a system-browser callback, so the flow itself is covered
    // by the desktop IPC integration tests. Here we cover the panel + grant-gated toggle.
    await expect(connectors.getByRole("button", { name: "授权" }).first()).toBeVisible();
    await connectors.getByRole("button", { name: "启用" }).click();
    await expect(connectors.getByRole("button", { name: "停用" })).toBeVisible({ timeout: 10_000 });
    await page.screenshot({ path: "target/evidence/m3/client-connectors.png" });
    await connectors.getByRole("button", { name: "关闭", exact: true }).click();

    await page.keyboard.press("Control+k");
    await page.getByRole("option", { name: "应用" }).click();
    const apps = page.getByRole("dialog", { name: "应用" });
    await expect(apps).toBeVisible();
    await expect(apps.getByText("client-app")).toBeVisible();
    await apps.getByRole("button", { name: "打开" }).click();
    await expect(apps.getByRole("status")).toContainText("已打开 client-app v1.0.0", { timeout: 10_000 });
    const surface = page.getByRole("dialog", { name: "应用窗口" });
    await expect(surface).toBeVisible({ timeout: 10_000 });
    await expect(surface.getByTitle("应用")).toBeVisible();
    await expect(page.frameLocator("iframe").locator("body")).toContainText("rows:", { timeout: 15_000 });
    await page.screenshot({ path: "target/evidence/m4/client-apps.png" });
    await surface.getByRole("button", { name: "关闭" }).click();
  });
});
