import { expect, test, type Page } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";
import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";

/** `/plan` plan mode interaction contract (`2026-11-01-client-plan-mode`). */
const SERVE = "http://127.0.0.1:8788";
const EVIDENCE = "target/evidence/2026-11-01-client-plan-mode/interaction";
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

async function openConversation(page: Page): Promise<void> {
  await page.goto(`${SERVE}/`, { waitUntil: "networkidle" });
  await expect(page.getByRole("complementary", { name: "工作区与会话" })).toBeVisible();
  await page.getByRole("button", { name: "新建会话", exact: true }).last().click();
  await expect(composer(page)).toBeVisible();
}

function composer(page: Page): ReturnType<Page["getByRole"]> {
  return page.getByRole("textbox", { name: "消息输入框" });
}

async function send(page: Page, text: string): Promise<void> {
  await composer(page).fill(text);
  await page.keyboard.press("Enter");
  await expect(page.getByTestId("user-text").last()).toHaveText(text);
  await expect(page.getByRole("button", { name: "停止生成" })).toHaveCount(0, { timeout: 30_000 });
}

async function enterPlanMode(page: Page): Promise<void> {
  await composer(page).fill("/plan");
  await expect(page.getByRole("option", { name: /计划/ })).toBeVisible();
  await page.keyboard.press("Enter");
  await expect(page.getByRole("region", { name: "会话计划" })).toBeVisible({ timeout: 15_000 });
}

async function state(page: Page): Promise<{ mode: string; status: string; plan: string; feedback?: string | null } | null> {
  const workspaceId = idOf(rowsOf(await callIpc(page, "workspace.list", {}))[0]);
  const conversationId = idOf(rowsOf(await callIpc(page, "session.list", { workspace_id: workspaceId }))[0]);
  const data = await callIpc(page, "plan.get", { workspace_id: workspaceId, conversation_id: conversationId });
  if (!isRecordValue(data) || !isRecordValue(data["plan"])) return null;
  const view = data["plan"];
  return {
    mode: typeof view["mode"] === "string" ? view["mode"] : "",
    status: typeof view["status"] === "string" ? view["status"] : "",
    plan: typeof view["plan"] === "string" ? view["plan"] : "",
    feedback: typeof view["feedback"] === "string" ? view["feedback"] : null,
  };
}

test.describe("command-plan", () => {
  test("CMD3P-PLAN-01: /plan enters plan mode, mutating tools stay blocked, /plan off exits", { tag: ["@interaction:CMD3P-PLAN-01"] }, async ({ page }) => {
    await openConversation(page);
    await enterPlanMode(page);
    expect(await state(page)).toMatchObject({ mode: "active", status: "draft" });
    // Mutating tool call in plan mode: blocked with a readable error (no file written).
    await send(page, "写入 demo-data/plan-blocked.txt");
    await expect(page.getByText(/plan\/mutating-blocked/).first()).toBeVisible({ timeout: 20_000 });
    // The workspace file was not created (blocked, not executed).
    const workspaceId = idOf(rowsOf(await callIpc(page, "workspace.list", {}))[0]);
    const files = rowsOf(await callIpc(page, "workspace.files", { workspace_id: workspaceId, path: "demo-data" }));
    expect(files.some((file) => file["path"] === "demo-data/plan-blocked.txt")).toBe(false);
    // `/plan off` exits and persists.
    await composer(page).fill("/plan off");
    await page.keyboard.press("Enter");
    await expect.poll(async () => (await state(page))?.mode).toBe("off");
    await page.reload({ waitUntil: "networkidle" });
    expect(await state(page)).toMatchObject({ mode: "off", status: "draft" });
    record("CMD3P-PLAN-01", "plan mode blocks mutating tools and /plan off exits");
  });

  test("CMD3P-PLAN-02: exit_plan_mode submits for local review; reject/continue/approve never execute", { tag: ["@interaction:CMD3P-PLAN-02"] }, async ({ page }) => {
    await openConversation(page);
    await enterPlanMode(page);
    await send(page, "提交计划");
    await expect(page.getByRole("region", { name: "计划内容" })).toContainText("step one", { timeout: 20_000 });
    expect(await state(page)).toMatchObject({ mode: "active", status: "pending_review" });
    // Reject with feedback keeps plan mode + feedback.
    await page.getByRole("textbox", { name: "审核反馈（可选）" }).fill("add tests");
    await page.getByRole("button", { name: "拒绝" }).first().click();
    await expect.poll(async () => (await state(page))?.status).toBe("rejected");
    expect(await state(page)).toMatchObject({ mode: "active", feedback: "add tests" });
    // Continue editing returns to draft, still in plan mode.
    await page.getByRole("button", { name: "继续编辑" }).first().click();
    await expect.poll(async () => (await state(page))?.status).toBe("draft");
    // Resubmit, then approve: approved + plan mode off, plan never auto-executed.
    await send(page, "提交计划");
    await expect.poll(async () => (await state(page))?.status).toBe("pending_review");
    await page.getByRole("button", { name: "批准" }).first().click();
    await expect.poll(async () => (await state(page))?.status).toBe("approved");
    expect(await state(page)).toMatchObject({ mode: "off" });
    // Persisted across reload.
    await page.reload({ waitUntil: "networkidle" });
    expect(await state(page)).toMatchObject({ status: "approved", mode: "off" });
    record("CMD3P-PLAN-02", "exit_plan_mode review lifecycle is local-only and persistent");
  });

  test("CMD3P-KEY-01: /pl filters, Esc keeps the draft, plan card is keyboard operable", { tag: ["@interaction:CMD3P-KEY-01"] }, async ({ page }) => {
    await openConversation(page);
    await composer(page).fill("/pl");
    await expect(page.getByRole("option", { name: /计划/ })).toBeVisible();
    await page.keyboard.press("Escape");
    await expect(page.getByRole("listbox")).toHaveCount(0);
    await expect(composer(page)).toHaveValue("/pl");
    await composer(page).fill("/plan");
    await page.keyboard.press("Enter");
    await expect(page.getByRole("region", { name: "会话计划" })).toBeVisible();
    await send(page, "提交计划");
    const approve = page.getByRole("button", { name: "批准" }).first();
    await approve.focus();
    await expect(approve).toBeFocused();
    await page.keyboard.press("Enter");
    await expect.poll(async () => (await state(page))?.status).toBe("approved");
    await expect(composer(page)).toBeVisible();
    record("CMD3P-KEY-01", "plan menu keys and review card keyboard operability");
  });

  test("CMD3P-ERR-01: failed plan actions keep state and never fake success", { tag: ["@interaction:CMD3P-ERR-01"] }, async ({ page }) => {
    await openConversation(page);
    await enterPlanMode(page);
    const before = await state(page);
    await page.route("**/ipc", async (route) => {
      const body: unknown = route.request().postDataJSON();
      const name = isRecordValue(body) ? body["name"] : undefined;
      if (name === "plan.update") {
        await route.fulfill({ status: 500, body: "boom" });
        return;
      }
      await route.continue();
    });
    await page.getByRole("button", { name: "退出计划模式" }).first().click();
    await expect(page.getByText("计划操作失败，状态未改变")).toBeVisible();
    await page.unroute("**/ipc");
    expect(await state(page)).toMatchObject({ mode: "active", status: before?.status ?? "draft" });
    // No plan card content was fabricated.
    await expect(page.getByRole("region", { name: "计划内容" })).toHaveCount(0);
    const axe = await new AxeBuilder({ page }).analyze();
    const serious = axe.violations.filter((violation) => violation.impact === "serious" || violation.impact === "critical").map((violation) => violation.id);
    expect(serious, JSON.stringify(serious)).toEqual([]);
    record("CMD3P-ERR-01", "failed plan actions keep state and never fake success");
  });
});
