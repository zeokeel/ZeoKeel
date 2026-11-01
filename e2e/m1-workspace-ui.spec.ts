import { test, expect } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";

test.describe("workspace UI (mock)", () => {
  test("default workspace ~ is listed and the first run does not create sessions", async ({ page }) => {
    await page.goto("/?mock=1", { waitUntil: "networkidle" });
    await expect(page.getByRole("button", { name: /^主目录/ })).toBeVisible();
    await expect(page.getByText("默认").first()).toBeVisible();
    await expect(page.getByRole("textbox", { name: "消息输入框" })).toHaveCount(0);
    await expect(page.getByRole("button", { name: "新建会话" }).last()).toBeVisible();

    await page.goto("/?mock=1&no-workspace=1", { waitUntil: "networkidle" });
    await expect(page.getByRole("heading", { name: "先选择一个工作区" })).toBeVisible();
    await expect(page.getByRole("textbox")).toHaveCount(0);
    await page.getByRole("button", { name: "使用默认 ~" }).click();
    await expect(page.getByRole("button", { name: /^主目录/ })).toBeVisible();
    await expect(page.getByRole("textbox", { name: "消息输入框" })).toHaveCount(0);
    await expect(page.getByRole("button", { name: "新建会话" }).last()).toBeVisible();
    await page.screenshot({ path: "target/evidence/m1/e2e-workspace-first-run.png" });
  });

  test("create workspace, isolate its conversations, and switch back", async ({ page }) => {
    await page.goto("/?mock=1", { waitUntil: "networkidle" });
    await page.getByRole("button", { name: "添加工作区" }).first().click();
    await page.getByPlaceholder("输入已存在的目录路径").fill("/tmp/e2e-workspace");
    await page.getByLabel("名称（可选）").fill("e2e-ws");
    await page.getByRole("button", { name: "添加", exact: true }).click();

    const workspaceRow = page.getByRole("button", { name: /e2e-ws/ }).first();
    await expect(workspaceRow).toBeVisible();
    await page.getByRole("button", { name: "新建会话" }).last().click();
    const composer = page.getByRole("textbox", { name: "消息输入框" });
    await expect(composer).toBeVisible();
    await composer.fill("读取 demo-data/hello.txt（e2e 工作区隔离）");
    await page.keyboard.press("Enter");
    await expect(page.getByText(/e2e 工作区隔离/).first()).toBeVisible();
    await expect(page.getByText("Hello from ZeoKeel", { exact: false })).toBeVisible({ timeout: 15_000 });

    // The default workspace must not show the new conversation (isolation).
    await page.getByRole("button", { name: /^主目录/ }).first().click();
    await expect(page.getByRole("textbox", { name: "消息输入框" })).toHaveCount(0);
    await expect(page.getByText(/e2e 工作区隔离/)).toHaveCount(0);

    // Switch back and continue the conversation in its workspace.
    await workspaceRow.click();
    await expect(page.getByText(/e2e 工作区隔离/).first()).toBeVisible();
    await expect(page.getByRole("textbox", { name: "消息输入框" })).toBeVisible();
    await page.screenshot({ path: "target/evidence/m1/e2e-workspace-sidebar.png" });
  });

  test("removing a workspace asks for reassignment and a second confirmation", async ({ page }) => {
    await page.goto("/?mock=1", { waitUntil: "networkidle" });
    await page.getByRole("button", { name: "添加工作区" }).first().click();
    await page.getByPlaceholder("输入已存在的目录路径").fill("/tmp/e2e-remove");
    await page.getByLabel("名称（可选）").fill("remove-ws");
    await page.getByRole("button", { name: "添加", exact: true }).click();
    await page.getByRole("button", { name: "新建会话" }).last().click();
    await expect(page.getByRole("textbox", { name: "消息输入框" })).toBeVisible();

    const row = page.getByRole("button", { name: /remove-ws/ }).first();
    await row.hover();
    await page.getByRole("button", { name: "工作区操作" }).nth(1).click();
    await page.getByRole("menuitem", { name: "移除" }).click();
    const dialog = page.getByRole("dialog");
    await expect(dialog).toBeVisible();
    await expect(dialog.getByLabel("迁移会话到")).toHaveValue(/ws-/);
    await dialog.getByRole("button", { name: "确认移除" }).click();
    await expect(page.getByRole("button", { name: /remove-ws/ })).toHaveCount(0);
    // Conversations were reassigned to the default workspace.
    await expect(page.getByRole("button", { name: /^主目录/ }).first()).toBeVisible();

    // The default workspace cannot be removed directly.
    await page.getByRole("button", { name: /^主目录/ }).first().hover();
    await page.getByRole("button", { name: "工作区操作" }).first().click();
    await page.getByRole("menuitem", { name: "移除" }).click();
    await expect(page.getByText("默认工作区不能直接移除，请先把其它工作区设为默认。")).toBeVisible();
    await page.getByRole("button", { name: "取消" }).click();

    const results = await new AxeBuilder({ page }).analyze();
    const serious = results.violations.filter((v) => v.impact === "serious" || v.impact === "critical");
    expect(serious, JSON.stringify(serious.map((v) => v.id))).toEqual([]);
  });
});
