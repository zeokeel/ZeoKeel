import { expect, test, type Page } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";
import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";

/** Workspace-scoped new conversation (`2026-11-01-client-workspace-new-session`). */
const SERVE = "http://127.0.0.1:8788";
const HOME = "target/evidence/m1/home";
const EVIDENCE = "target/evidence/2026-11-01-client-workspace-new-session/interaction";
mkdirSync(EVIDENCE, { recursive: true });

function record(id: string, title: string): void {
  writeFileSync(join(EVIDENCE, `${id}.json`), `${JSON.stringify({ id, title, at: new Date().toISOString(), passed: true }, null, 2)}\n`, "utf8");
}

function isRecordValue(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function rowsOf(data: unknown): Record<string, unknown>[] {
  const entries: unknown[] = Array.isArray(data) ? data : [];
  return entries.filter(isRecordValue);
}

function idOf(row: Record<string, unknown> | undefined): string {
  const id = row?.["id"];
  return typeof id === "string" ? id : "";
}

async function callIpc(page: Page, name: string, payload: Record<string, unknown>): Promise<unknown> {
  const response = await page.request.post(`${SERVE}/ipc`, { data: { name, payload } });
  const body: unknown = await response.json();
  return isRecordValue(body) ? body["data"] : undefined;
}

async function seedSecondWorkspace(page: Page): Promise<string> {
  const path = join(HOME, "second-ws");
  mkdirSync(path, { recursive: true });
  await callIpc(page, "workspace.create", { path, title: "second-ws" });
  const workspaces = rowsOf(await callIpc(page, "workspace.list", {}));
  const second = workspaces.find((workspace) => workspace["title"] === "second-ws");
  return idOf(second);
}

async function openClient(page: Page): Promise<void> {
  await page.goto(`${SERVE}/`, { waitUntil: "networkidle" });
  await expect(page.getByRole("complementary", { name: "工作区与会话" })).toBeVisible();
}

test.describe("workspace-scoped new conversation", () => {
  test("SES2-WSNEW-01: row + creates and selects in that workspace (no picker)", { tag: ["@interaction:SES2-WSNEW-01"] }, async ({ page }) => {
    await openClient(page);
    const secondId = await seedSecondWorkspace(page);
    await page.reload({ waitUntil: "networkidle" });
    const defaultWs = rowsOf(await callIpc(page, "workspace.list", {})).find((workspace) => idOf(workspace) !== secondId);
    const defaultId = idOf(defaultWs);
    const beforeSecond = rowsOf(await callIpc(page, "session.list", { workspace_id: secondId })).length;
    const beforeDefault = rowsOf(await callIpc(page, "session.list", { workspace_id: defaultId })).length;
    const row = page.getByRole("listitem").filter({ hasText: "second-ws" }).first();
    await row.hover();
    const plus = page.getByRole("button", { name: "在 second-ws 新建会话" });
    await expect(plus).toBeVisible();
    await plus.click();
    // Created in the target workspace, not the previously active one.
    await expect
      .poll(async () => rowsOf(await callIpc(page, "session.list", { workspace_id: secondId })).length)
      .toBe(beforeSecond + 1);
    expect(rowsOf(await callIpc(page, "session.list", { workspace_id: defaultId })).length).toBe(beforeDefault);
    // No workspace picker/modal; the target workspace row is highlighted.
    await expect(page.getByRole("dialog")).toHaveCount(0);
    const activeRow = page.getByRole("listitem").filter({ hasText: "second-ws" }).first();
    await expect(activeRow.getByText("second-ws", { exact: true })).toHaveClass(/font-medium/);
    record("SES2-WSNEW-01", "row + creates and selects in that workspace (no picker)");
  });

  test("SES2-WSNEW-02: keyboard/focus visibility, aria label and no row toggle", { tag: ["@interaction:SES2-WSNEW-02"] }, async ({ page }) => {
    await openClient(page);
    await seedSecondWorkspace(page);
    await page.reload({ waitUntil: "networkidle" });
    const plus = page.getByRole("button", { name: "在 second-ws 新建会话" });
    await expect(plus).toHaveCount(1);
    // Non-active row: hidden by default (opacity 0), visible on hover/focus.
    expect(await plus.evaluate((element) => getComputedStyle(element).opacity)).toBe("0");
    await plus.focus();
    await expect.poll(async () => plus.evaluate((element) => getComputedStyle(element).opacity)).toBe("1");
    // Clicking the row title only selects; it must not collapse/expand the row.
    const row = page.getByRole("listitem").filter({ hasText: "second-ws" }).first();
    const toggle = row.getByRole("button", { name: "折叠会话" });
    await expect(toggle).toHaveAttribute("aria-expanded", "true");
    await row.getByRole("button", { name: /^second-ws/ }).click();
    await expect(toggle).toHaveAttribute("aria-expanded", "true");
    // Keyboard activation creates a conversation in the target workspace.
    await plus.focus();
    await page.keyboard.press("Enter");
    await expect
      .poll(async () => rowsOf(await callIpc(page, "session.list", { workspace_id: idOf(rowsOf(await callIpc(page, "workspace.list", {})).find((workspace) => workspace["title"] === "second-ws")) })).length)
      .toBeGreaterThan(0);
    // Active row keeps the button always visible (once the busy state clears).
    await expect.poll(async () => plus.evaluate((element) => getComputedStyle(element).opacity)).toBe("1");
    const axe = await new AxeBuilder({ page }).analyze();
    const serious = axe.violations.filter((violation) => violation.impact === "serious" || violation.impact === "critical").map((violation) => violation.id);
    expect(serious, JSON.stringify(serious)).toEqual([]);
    // Visual evidence only (pixel/aria snapshot baseline needs human confirmation).
    await page.screenshot({ path: join(EVIDENCE, "SES2-WSNEW-VIS-01.png") });
    record("SES2-WSNEW-02", "keyboard/focus visibility, aria label and no row toggle");
  });

  test("SES2-WSNEW-03: scoped/global share one streaming guard and recover", { tag: ["@interaction:SES2-WSNEW-03"] }, async ({ page }) => {
    await openClient(page);
    await seedSecondWorkspace(page);
    await page.reload({ waitUntil: "networkidle" });
    await page.getByRole("button", { name: "新建会话" }).last().click();
    const composer = page.getByRole("textbox", { name: "消息输入框" });
    const secondId = idOf(rowsOf(await callIpc(page, "workspace.list", {})).find((workspace) => workspace["title"] === "second-ws"));
    const before = rowsOf(await callIpc(page, "session.list", { workspace_id: secondId })).length;
    // Capture the guard atomically from inside the page: the streaming window of the
    // fake-upstream turn is very short, so a MutationObserver records the first
    // moment both the scoped row button and the global new button are disabled.
    await page.evaluate(() => {
      delete document.documentElement.dataset["zkGuard"];
      const capture = (): void => {
        if (document.documentElement.dataset["zkGuard"] !== undefined) return;
        const scoped = document.querySelector('button[aria-label="在 second-ws 新建会话"]');
        const globalButtons = Array.from(document.querySelectorAll('button[aria-label="新建会话"]'));
        const scopedDisabled = scoped instanceof HTMLButtonElement && scoped.disabled;
        const globalsDisabled = globalButtons.length > 0 && globalButtons.every((button) => button instanceof HTMLButtonElement && button.disabled);
        if (!scopedDisabled || !globalsDisabled) return;
        const scopedTitle = scoped instanceof HTMLButtonElement ? scoped.title : "";
        const scopedAria = scoped instanceof HTMLButtonElement ? scoped.getAttribute("aria-disabled") : null;
        document.documentElement.dataset["zkGuard"] = JSON.stringify({
          scopedDisabled,
          globalsDisabled,
          globalCount: globalButtons.length,
          scopedTitle,
          scopedAria,
        });
      };
      const observer = new MutationObserver(capture);
      observer.observe(document.body, { subtree: true, attributes: true, childList: true });
      capture();
    });
    await composer.fill("读取 demo-data/hello.txt 并总结");
    await page.keyboard.press("Enter");
    await expect(page.getByText("Hello from ZeoKeel", { exact: false }).last()).toBeVisible({ timeout: 30_000 });
    const guardRaw = await page.evaluate(() => document.documentElement.dataset["zkGuard"] ?? "");
    expect(guardRaw).not.toBe("");
    const guard: unknown = JSON.parse(guardRaw);
    expect(isRecordValue(guard)).toBe(true);
    expect(guard).toMatchObject({
      scopedDisabled: true,
      globalsDisabled: true,
      scopedTitle: "当前正在生成，停止后可新建",
      scopedAria: "true",
    });
    // Nothing was created in the target workspace by the disabled entries.
    expect(rowsOf(await callIpc(page, "session.list", { workspace_id: secondId })).length).toBe(before);
    // After the turn the entries recover and the scoped + works again.
    const scoped = page.getByRole("button", { name: "在 second-ws 新建会话" });
    await expect(scoped).toBeEnabled({ timeout: 10_000 });
    await expect(page.getByRole("button", { name: "新建会话" }).last()).toBeEnabled();
    await scoped.click();
    await expect
      .poll(async () => rowsOf(await callIpc(page, "session.list", { workspace_id: secondId })).length)
      .toBe(before + 1);
    // ⌘K shares the same guard: while streaming its `new-conversation` row is disabled
    // (descriptor-level disabledReason is unit-checked in command-catalog.test.ts).
    await expect(page.getByRole("button", { name: "新建会话" }).last()).toBeEnabled();
    record("SES2-WSNEW-03", "scoped/global share one streaming guard and recover");
  });

});
